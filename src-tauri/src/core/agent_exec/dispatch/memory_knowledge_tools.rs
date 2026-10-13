use super::*;

pub(super) async fn memory_knowledge_tools(ctx: &ToolCtx<'_>, name: &str, args: &Value) -> String {
    match name {
        "recall_memory" => {
            let q = args["query"].as_str().unwrap_or("");
            let limit = args["limit"].as_u64().unwrap_or(5) as usize;
            let mut out = String::new();

            // Two stores, deliberately searched together. Vault notes answer "what do
            // I know about X"; the journal answers "what did we establish, and when".
            // Searching only the notes was why a preference stated last week was
            // invisible to a tool call this week.
            let journal = crate::core::journal::Journal::new(&ctx.memory.vault_path);
            let hits = crate::core::embeddings::recall_journal(&journal, &ctx.settings, &ctx.data_dir, q, limit).await;
            if !hits.is_empty() {
                out.push_str("## From your memory journal\n\n");
                for h in &hits {
                    match h.tier {
                        crate::core::journal::Tier::Core => out.push_str(&format!("- {}\n", h.text.trim())),
                        crate::core::journal::Tier::Daily => out.push_str(&format!("- ({}) {}\n", h.date, h.text.trim())),
                    }
                }
                out.push('\n');
            }

            // Semantic, with the keyword path as the fallback it always was —
            // not keyword-only. This tool used `recall_scored` directly, which
            // meant the agent loop, the one thing the scoreboard actually grades,
            // was the single path in the app that never reached the vectors.
            let (vault_hits, _why) =
                crate::core::embeddings::recall_memories(&ctx.memory, &ctx.settings, &ctx.data_dir, q, limit).await;
            // Which of these a newer memory with the same title has overtaken. Worked
            // out here, from the dates, and never asked of the model: given two
            // versions of a fact and no help, models pick inconsistently and often
            // hedge by naming both — which on a question like "who is on call" is the
            // one answer that is useless.
            let mut hits: Vec<crate::core::memory::Memory> = vault_hits.into_iter().map(|(m, _)| m).collect();
            // One hop along a shared name, and the link stated. See
            // `MemoryEngine::entity_hops` for why: two memories that each hold half
            // an answer are joined by a word, not by similarity, and a model handed
            // the first half named the team and stopped. Expanded memories go
            // through the same loop below, so an external one taints exactly as a
            // direct hit does — the hop widens what is read, never what is trusted.
            let pool = ctx.memory.recallable();
            let (hopped, links) = crate::core::memory::MemoryEngine::entity_hops(&pool, &hits, q, 3);
            hits.extend(hopped.into_iter().map(|(m, _)| m));
            let overtaken = crate::core::memory::MemoryEngine::freshness(&hits);
            // Newest first within a title, so the current entry is the one the model
            // reads first and the stale one arrives already labelled.
            hits.sort_by(|a, b| {
                a.title.trim().to_lowercase().cmp(&b.title.trim().to_lowercase()).then_with(|| b.date.cmp(&a.date))
            });
            let mut notes = String::new();
            for m in hits {
                // The moment a stranger's words enter the run. Everything the
                // model does from here on could be following them, so the run
                // loses its pre-approvals for acting outward.
                if m.origin == crate::core::memory::Origin::External {
                    ctx.taint();
                }
                // Nothing is hidden — "same title" is evidence about recency, not
                // proof of contradiction, and a vault that silently dropped last
                // Tuesday's "Meeting notes" would be losing the user's data on a
                // guess. It is stated, and it is stated as a rule so the model has
                // nothing left to decide.
                let stamp = match overtaken.get(&m.filename).and_then(|o| o.clone()) {
                    Some(newer) => format!(
                        " — dated {}, SUPERSEDED by the entry dated {newer} above; use the newer one",
                        m.date.split('T').next().unwrap_or(&m.date)
                    ),
                    None => format!(" — dated {}", m.date.split('T').next().unwrap_or(&m.date)),
                };
                // A superseded entry keeps its place and loses its body.
                //
                // It was 160 characters, on the reasoning that "things changed" is
                // worth some context. Measured against that: `prefers_the_newer_fact`
                // went 0/3, once by naming the superseded person outright. A hundred
                // and sixty characters of a stale fact is a hundred and sixty
                // characters the model can quote, and the entry's *job* here is to
                // say that a newer one exists — which the heading already does.
                // Nothing is hidden: the file is still in the vault and
                // `recall_memory` will return it if asked about history.
                let superseded = overtaken.get(&m.filename).is_some_and(|o| o.is_some());
                let room = if superseded { 0 } else { 500 };
                // The filename, so a wrong answer can be traced to the memory that
                // caused it. There is no other record of which memories entered a
                // prompt — `runlog` records the run, not the recall — and without
                // one, "why did it say that" and "what else did that bad sync
                // touch" are both unanswerable. The transcript is already kept, so
                // this costs a few tokens and needs no new store.
                let link = links
                    .get(&m.filename)
                    .map(|(partner, through)| format!(" — linked to “{partner}” through “{through}”"))
                    .unwrap_or_default();
                notes.push_str(&format!(
                    "### {} ({}){stamp}{link}\n{}\n{}\n\n",
                    m.title,
                    m.filename,
                    if superseded { String::new() } else { m.summary.clone() },
                    m.body.chars().take(room).collect::<String>()
                ));
            }
            if !notes.trim().is_empty() {
                out.push_str("## Saved memories\n\n");
                out.push_str(&notes);
            }
            if out.trim().is_empty() { "no relevant memories found".into() } else { out }
        }
        "knowledge_write" => {
            let k = crate::core::knowledge::Knowledge::new(&ctx.memory.vault_path);
            let title = args["title"].as_str().unwrap_or("");
            let body = args["body"].as_str().unwrap_or("");
            if body.trim().is_empty() {
                return "error: `body` is the page content — write the whole page, not just what changed".into();
            }
            match k.write(title, args["category"].as_str().unwrap_or("general"), body) {
                Ok(p) => {
                    let dangling: Vec<String> = p.links.iter().filter(|l| k.find(l).is_none()).cloned().collect();
                    format!(
                        "Saved \"{}\" under {}.{}",
                        p.title,
                        p.category,
                        // Say which links point nowhere yet. Silently dropping them
                        // from the graph would leave the agent thinking it had
                        // cross-referenced something it hadn't.
                        if dangling.is_empty() {
                            String::new()
                        } else {
                            format!(" These links have no page yet: {}.", dangling.join(", "))
                        }
                    )
                }
                Err(e) => format!("error: {e}"),
            }
        }
        "knowledge_search" => {
            let k = crate::core::knowledge::Knowledge::new(&ctx.memory.vault_path);
            let hits = k.search(args["query"].as_str().unwrap_or(""), 5);
            if hits.is_empty() {
                return "no knowledge pages match that.".into();
            }
            hits.iter()
                .map(|p| format!("### {} [{}]\n{}\n", p.title, p.category, p.body.chars().take(500).collect::<String>()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "knowledge_read" => {
            let k = crate::core::knowledge::Knowledge::new(&ctx.memory.vault_path);
            match k.find(args["title"].as_str().unwrap_or("")) {
                Some(p) => format!("# {}\n[{}]\n\n{}", p.title, p.category, p.body),
                None => {
                    let known = k.list();
                    if known.is_empty() {
                        "no page by that title — the knowledge base is empty.".into()
                    } else {
                        format!(
                            "no page by that title. Existing pages: {}",
                            known.iter().take(20).map(|p| p.title.clone()).collect::<Vec<_>>().join(", ")
                        )
                    }
                }
            }
        }
        _ => unreachable!("routed here only for its own tools"),
    }
}
