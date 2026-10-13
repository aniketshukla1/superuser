use super::*;

impl MemoryEngine {
    /// The memories recall is allowed to see.
    ///
    /// One definition, so the keyword path and the vector path cannot disagree about
    /// what exists — which they would, and silently, if each filtered for itself.
    /// Two exclusions, both of them declared rather than inferred:
    ///
    /// * **Superseded** — some other memory says it replaced this one.
    /// * **Suspect** — external content that reads like an instruction. It is out of
    ///   *automatic* recall only. `list` still returns it, so it is still in the UI,
    ///   and the user can still open and read it.
    ///
    /// Neither is a deletion and neither is reversible only by us: both are lines in
    /// the file, which the user owns and can edit.
    pub fn recallable(&self) -> Vec<Memory> {
        let all = self.list();
        let dead = Self::superseded(&all);
        all.into_iter()
            // `suspect` is the stored verdict, written when the memory was written.
            // `screen` is run again here because the stored one only ever covers
            // memories saved *since the screen existed*: measured on the author's
            // real vault, all 125 predate it and every one of them parses as
            // `suspect: false`. A defence that covers only future writes is not a
            // defence of the vault that exists.
            //
            // The cost is fifteen `contains` over a body this call has already read
            // and parsed, which is far below the file I/O it sits on.
            .filter(|m| !m.suspect && !screen(m.origin, &m.title, &m.body) && !dead.contains(&m.filename))
            .collect()
    }

    /// Memories one hop from `hits` through a name they share, and — for every hit —
    /// the other hit it is linked to and the name that links them.
    ///
    /// Built for `joins_two_memories`, and the failure it answers is precise. On
    /// `gemma4:12b` the task went 1/3, and both failures stopped after the first hop:
    /// *"Project Perseus was transferred to the Atlas team. They own the service, the
    /// runbook and the pager."* Correct, and not the question, which was where to
    /// escalate. One memory holds the owning team and the other holds that team's
    /// channel, and the only thing joining them is one word a person would underline:
    /// "Atlas". A similarity score has no reason to pair them, and a prompt nudge
    /// ("recall again on that name") earned a second recall once in three.
    ///
    /// So the join is made here, mechanically, in two parts:
    ///
    /// * **Expansion.** A distinctive name in a hit that the question did not mention
    ///   pulls in up to `max` other memories carrying the same name — the linked
    ///   half of the answer arrives with the first half instead of waiting for the
    ///   model to think of asking.
    /// * **Annotation.** Two memories in the result that share such a name are
    ///   labelled with it, so the connection is stated rather than left to be
    ///   noticed — which covers the case where both halves were retrieved and the
    ///   model still did not put them together.
    ///
    /// **Distinctive** is doing real work. A synced vault mentions "GitHub" in half
    /// its memories, and following that would drag in noise on every recall, so a
    /// name only links when it appears in at least two memories and at most
    /// `max(3, 5%)` of them — the inverse-document-frequency idea without the maths.
    /// Local, deterministic, no model and no index: a HippoRAG-style one-hop
    /// expansion for a vault of plain files.
    pub fn entity_hops(
        pool: &[Memory],
        hits: &[Memory],
        query: &str,
        max: usize,
    ) -> (Vec<(Memory, String)>, std::collections::HashMap<String, (String, String)>) {
        use std::collections::{HashMap, HashSet};
        let asked: HashSet<String> = entities(query).into_iter().collect();
        let ents = |m: &Memory| entities(&m.body);
        let cap = (pool.len() / 20).max(3);
        // How many memories carry each name, across everything recall may see.
        let mut df: HashMap<String, usize> = HashMap::new();
        for m in pool {
            for e in ents(m) {
                *df.entry(e).or_default() += 1;
            }
        }
        let distinctive = |e: &str| df.get(e).is_some_and(|n| *n >= 2 && *n <= cap);

        let have: HashSet<&str> = hits.iter().map(|m| m.filename.as_str()).collect();
        let mut added: Vec<(Memory, String)> = Vec::new();
        for h in hits {
            for e in ents(h) {
                if added.len() >= max || asked.contains(&e) || !distinctive(&e) {
                    continue;
                }
                for m in pool {
                    if added.len() >= max {
                        break;
                    }
                    if have.contains(m.filename.as_str()) || added.iter().any(|(a, _)| a.filename == m.filename) {
                        continue;
                    }
                    if ents(m).contains(&e) {
                        added.push((m.clone(), e.clone()));
                    }
                }
            }
        }

        // Every pair in the final result that shares a distinctive, un-asked name.
        let all: Vec<&Memory> = hits.iter().chain(added.iter().map(|(m, _)| m)).collect();
        let mut links: HashMap<String, (String, String)> = HashMap::new();
        for a in &all {
            let ea: HashSet<String> = ents(a).into_iter().collect();
            for b in &all {
                if a.filename == b.filename || links.contains_key(&a.filename) {
                    continue;
                }
                if let Some(e) = ents(b).into_iter().find(|e| ea.contains(e) && !asked.contains(e) && distinctive(e)) {
                    links.insert(a.filename.clone(), (b.title.clone(), e));
                }
            }
        }
        (added, links)
    }

    /// The memory carrying this upstream key, if we already hold it.
    ///
    /// Linear over the vault, like everything else here. At 5,000 memories a full
    /// parse is ~200 ms and this runs on a sync, not in front of a token.
    pub fn find_by_key(&self, key: &str) -> Option<Memory> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }
        self.list().into_iter().find(|m| m.key == key)
    }

    /// Filenames that some other memory has explicitly replaced.
    ///
    /// Declared, never inferred. The tempting rule — two memories with the same
    /// title, so the later date wins — is wrong often enough to matter: "Meeting
    /// notes" written every Tuesday is not six stale copies of one note, and a
    /// vault that quietly stopped returning five of them would be losing the user's
    /// data on a heuristic. Recency between same-titled memories is handled where it
    /// can be handled without hiding anything, in [`Self::freshness`].
    pub fn superseded(all: &[Memory]) -> std::collections::HashSet<String> {
        let mut dead: std::collections::HashSet<String> =
            all.iter().filter(|m| !m.supersedes.trim().is_empty()).map(|m| m.supersedes.trim().to_string()).collect();

        // A shared `key` is the other declared form, and it is safe where a shared
        // *title* is not. A key is the writer asserting identity — "this is
        // github:issue:1421" — so two live memories carrying the same one are two
        // versions of one thing, and only the newest is true. A title is a label a
        // person chose, and "Meeting notes" every Tuesday is five different notes.
        // That distinction is the whole reason this is keyed and not titled.
        //
        // `save` upserts on a key, so two copies normally cannot coexist. They can
        // when a file is written by hand or by a writer that set an explicit
        // filename, and then recall would rank two versions of one fact against each
        // other — the failure `prefers_the_newer_fact` exists to catch.
        let mut newest: std::collections::HashMap<&str, &Memory> = std::collections::HashMap::new();
        for m in all.iter().filter(|m| !m.key.trim().is_empty()) {
            let k = m.key.trim();
            match newest.get(k) {
                Some(cur) if cur.date >= m.date => {
                    dead.insert(m.filename.clone());
                }
                Some(cur) => {
                    dead.insert(cur.filename.clone());
                    newest.insert(k, m);
                }
                None => {
                    newest.insert(k, m);
                }
            }
        }
        dead
    }
}
