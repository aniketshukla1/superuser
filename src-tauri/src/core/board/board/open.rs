use super::*;

impl Board {
    pub fn open(data_dir: &Path, workspace: &Path) -> Self {
        Self { path: data_dir.join("boards").join(format!("{}.jsonl", slug(workspace))) }
    }

    pub(in crate::core::board) fn append(&self, ev: &Event) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let line = serde_json::to_string(ev).map_err(|e| e.to_string())?;
        let mut f = OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| e.to_string())?;
        writeln!(f, "{line}").map_err(|e| e.to_string())
    }

    /// Every event, oldest first. A line that will not parse is skipped rather
    /// than failing the read: one corrupt append must not take the board with it.
    pub fn events(&self) -> Vec<Event> {
        let Ok(text) = fs::read_to_string(&self.path) else { return Vec::new() };
        text.lines().filter_map(|l| serde_json::from_str::<Event>(l).ok()).collect()
    }

    /// Fold the log into the board.
    pub fn items(&self) -> Vec<Item> {
        let mut by_id: BTreeMap<String, Item> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for ev in self.events() {
            match ev.kind {
                EventKind::Created { title, detail, acceptance } => {
                    if !by_id.contains_key(&ev.item) {
                        order.push(ev.item.clone());
                    }
                    by_id.insert(
                        ev.item.clone(),
                        Item {
                            id: ev.item,
                            title,
                            detail,
                            acceptance,
                            state: ItemState::Open,
                            assignee: None,
                            builder: None,
                            notes: Vec::new(),
                            reviews: Vec::new(),
                        },
                    );
                }
                // Everything else is meaningless without a Created — a log that
                // starts mid-item is a log we cannot project, so it is skipped.
                _ => {
                    let Some(item) = by_id.get_mut(&ev.item) else { continue };
                    match ev.kind {
                        EventKind::Assigned { to } => item.assignee = Some(to),
                        EventKind::Moved { to } => {
                            // The builder is whoever put it up for review, recorded
                            // at the moment it happens: asking later who "built" it
                            // means guessing from a list of edits.
                            if to == ItemState::Review {
                                item.builder = Some(ev.actor.clone());
                            }
                            item.state = to;
                        }
                        EventKind::Noted { text } => item.notes.push(text),
                        EventKind::Reviewed { pass, evidence } => {
                            item.reviews.push(Review { by: ev.actor.clone(), pass, evidence });
                        }
                        EventKind::Created { .. } => {}
                    }
                }
            }
        }
        order.into_iter().filter_map(|id| by_id.remove(&id)).collect()
    }

    pub fn get(&self, id: &str) -> Option<Item> {
        self.items().into_iter().find(|i| i.id == id)
    }

    pub fn add(&self, actor: &str, title: &str, detail: &str, acceptance: &str) -> Result<String, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("an item needs a title".into());
        }
        let id = self.next_id();
        self.append(&Event {
            at: now(),
            item: id.clone(),
            actor: actor.to_string(),
            kind: EventKind::Created {
                title: title.to_string(),
                detail: detail.trim().to_string(),
                acceptance: acceptance.trim().to_string(),
            },
        })?;
        Ok(id)
    }

    /// A fresh item id.
    ///
    /// Milliseconds alone were not enough: a lead decomposing a plan writes several
    /// items in one breath, they landed in the same millisecond, and the second
    /// `Created` silently replaced the first in the projection — work vanishing off
    /// the board at the exact moment someone was filling it. The counter separates
    /// items within a millisecond; the existence check covers a restart landing on
    /// a millisecond the previous process already used.
    pub(in crate::core::board) fn next_id(&self) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let taken: Vec<String> = self.items().into_iter().map(|i| i.id).collect();
        loop {
            let id = format!("i{:x}{:03x}", (now() * 1000.0) as u64, N.fetch_add(1, Ordering::SeqCst) & 0xfff);
            if !taken.contains(&id) {
                return id;
            }
        }
    }

    pub fn assign(&self, actor: &str, id: &str, to: &str) -> Result<(), String> {
        self.require(id)?;
        self.append(&Event { at: now(), item: id.into(), actor: actor.into(), kind: EventKind::Assigned { to: to.trim().into() } })
    }

    pub fn note(&self, actor: &str, id: &str, text: &str) -> Result<(), String> {
        self.require(id)?;
        if text.trim().is_empty() {
            return Err("a note with no text is not a note".into());
        }
        self.append(&Event { at: now(), item: id.into(), actor: actor.into(), kind: EventKind::Noted { text: text.trim().into() } })
    }

    /// Move an item. The one transition that is not free is into `Done`.
    pub fn move_to(&self, actor: &str, id: &str, to: ItemState) -> Result<(), String> {
        let item = self.require(id)?;
        if to == ItemState::Done && !item.verified() {
            return Err(format!(
                "\"{}\" cannot be marked done: it has no passing review from anyone other than whoever built it. \
                 Move it to `review` and have a different assistant check it against its acceptance criteria.",
                item.title
            ));
        }
        self.append(&Event { at: now(), item: id.into(), actor: actor.into(), kind: EventKind::Moved { to } })
    }

    /// Record a review. The builder's own verdict is refused.
    pub fn review(&self, actor: &str, id: &str, pass: bool, evidence: &str) -> Result<(), String> {
        let item = self.require(id)?;
        if item.builder.as_deref() == Some(actor) {
            return Err(format!(
                "You put \"{}\" up for review, so you cannot be the one to pass it. \
                 A different assistant has to check it — that is the whole point of the review column.",
                item.title
            ));
        }
        if evidence.trim().is_empty() {
            return Err("a review needs evidence: what you ran, what you read, what you saw".into());
        }
        self.append(&Event {
            at: now(),
            item: id.into(),
            actor: actor.into(),
            kind: EventKind::Reviewed { pass, evidence: evidence.trim().into() },
        })
    }

    pub(in crate::core::board) fn require(&self, id: &str) -> Result<Item, String> {
        self.get(id).ok_or_else(|| format!("no item `{id}` on this board"))
    }
}
