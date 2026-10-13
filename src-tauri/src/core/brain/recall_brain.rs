use super::*;

/// `@brain what did we decide about X?` → `Some("what did we decide about X?")`.
///
/// Returns `Some("")` for a bare `@brain`, which the caller answers with the help
/// text rather than sending an empty question to a model. A message that merely
/// *mentions* the brain ("I asked @brain earlier") is not a command — the mention
/// has to open the message, the way `/context` does.
pub fn parse_mention(text: &str) -> Option<String> {
    let t = text.trim_start();
    let rest = t.strip_prefix(MENTION).or_else(|| {
        // Case-insensitive, but only pay for the allocation when the cheap check
        // above missed and the length still makes a match possible.
        (t.len() >= MENTION.len() && t[..MENTION.len()].eq_ignore_ascii_case(MENTION))
            .then(|| &t[MENTION.len()..])
    })?;
    // `@brainstorm` is a word, not a mention. Only a boundary ends the token.
    if !rest.is_empty() && !rest.starts_with(|c: char| c.is_whitespace() || c == ':' || c == ',') {
        return None;
    }
    Some(rest.trim_start_matches([':', ',']).trim().to_string())
}

/// What a bare `@brain` says back. Short, and about this brain rather than about
/// brains in general — the counts make it obvious whether there is anything to ask
/// about yet.
pub fn help(census: &Census) -> String {
    let mut s = String::from(
        "**Ask my brain.** Type `@brain` followed by a question and I'll search everything I hold \
         about you — memories, timeline, knowledge pages and your profile — and answer from it, \
         with sources.\n\nRight now I'm holding:\n",
    );
    s.push_str(&format!(
        "- **{}** memories{}\n- **{}** days of timeline{}\n- **{}** knowledge pages\n- **{}** things learned about how you work\n",
        census.memories,
        if census.categories.is_empty() {
            String::new()
        } else {
            format!(" across {} categories", census.categories.len())
        },
        census.journal_days,
        if census.has_core_memory { ", plus distilled core memory" } else { "" },
        census.knowledge_pages,
        census.profile_facets,
    ));
    s.push_str("\nFor example: `@brain what have I been working on this month?`");
    s
}

/// Count everything. Cheap: the memory list is already parsed frontmatter, and the
/// other three are directory listings.
pub fn census(engine: &MemoryEngine, vault: &Path, data_dir: &Path) -> Census {
    census_of(&engine.list(), vault, data_dir)
}

/// The census over an already-read memory list.
///
/// Split out because `recall` needs the same list to build its inventories, and
/// parsing every file in the vault twice for one question is a cost that grows with
/// exactly the thing this feature is for — a large vault.
pub(super) fn census_of(all: &[Memory], vault: &Path, data_dir: &Path) -> Census {
    let mut categories: BTreeMap<String, usize> = BTreeMap::new();
    let mut oldest = String::new();
    let mut newest = String::new();
    for m in all {
        *categories.entry(m.category.clone()).or_default() += 1;
        // Dates are ISO, so string ordering is date ordering. Blank dates (a memory
        // saved without one) are skipped rather than sorting to the front and
        // reporting a range that starts at nothing.
        if m.date.is_empty() {
            continue;
        }
        if oldest.is_empty() || m.date < oldest {
            oldest = m.date.clone();
        }
        if newest.is_empty() || m.date > newest {
            newest = m.date.clone();
        }
    }
    let journal = Journal::new(vault);
    let knowledge = Knowledge::new(vault);
    let learned = crate::core::profile::Learned::load(data_dir);
    Census {
        memories: all.len(),
        categories: categories.into_iter().collect(),
        oldest,
        newest,
        journal_days: journal.dailies().len(),
        has_core_memory: !journal.read_core().trim().is_empty(),
        knowledge_pages: knowledge.list().len(),
        profile_facets: learned.active_by_class().iter().map(|(_, f)| f.len()).sum(),
    }
}

/// Read all four stores for one question.
///
/// The two semantic searches are the slow part and they are independent, so they
/// run concurrently — the vault and the journal have separate embedding indexes and
/// neither waits on the other. Knowledge and profile are local reads.
pub async fn recall(
    engine: &MemoryEngine,
    settings: &Settings,
    vault: &Path,
    data_dir: &Path,
    query: &str,
    budget: Budget,
) -> Recall {
    let all = engine.list();
    let census = census_of(&all, vault, data_dir);
    let inventories = inventories(&all, query, &census, budget);
    drop(all);

    // Listing a category in full changes what the excerpts are worth, so everything
    // after this point runs on the reduced budget — including rendering, which reads
    // it back off the `Recall`.
    let budget = if inventories.is_empty() { budget } else { budget.displaced() };

    let journal = Journal::new(vault);
    let ((mem_hits, degraded), journal_hits) = futures_util::future::join(
        embeddings::recall_explained(engine, settings, data_dir, query, budget.memories),
        embeddings::recall_journal(&journal, settings, data_dir, query, budget.journal),
    )
    .await;

    // Reading a citation back into a full memory can fail — the file may have been
    // deleted between indexing and now. A missing one is dropped rather than
    // rendered as an empty section, and `reinforce` records that recall touched it,
    // the same bookkeeping the chat path does, so asking about something keeps it
    // alive in the vault's weighting.
    let memories: Vec<Memory> = mem_hits
        .iter()
        .filter_map(|c| {
            let m = engine.parse_file(&engine.vault_path.join(&c.filename))?;
            let _ = engine.reinforce(&c.filename);
            Some(m)
        })
        .collect();

    Recall {
        memories,
        journal: journal_hits,
        pages: Knowledge::new(vault).search(query, budget.pages),
        profile: crate::core::profile::Learned::load(data_dir).system_block(),
        census,
        inventories,
        degraded,
        budget,
    }
}
