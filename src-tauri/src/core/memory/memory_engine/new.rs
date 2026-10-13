use super::*;

impl MemoryEngine {
    pub fn new(vault_path: PathBuf) -> Self {
        let _ = fs::create_dir_all(&vault_path);
        Self { vault_path }
    }

    pub(in crate::core::memory) fn decay(&self, last_interaction: &str, base: f64, reinforce: u32) -> f64 {
        let parsed = DateTime::parse_from_rfc3339(last_interaction)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let hours = (Utc::now() - parsed).num_seconds() as f64 / 3600.0;
        let mut w = base * (-DECAY_RATE * hours.max(0.0)).exp();
        w += reinforce as f64 * 0.02;
        w.clamp(0.12, 1.0)
    }

    pub fn parse_file(&self, path: &Path) -> Option<Memory> {
        let content = fs::read_to_string(path).ok()?;
        let filename = path.file_name()?.to_string_lossy().to_string();
        let meta = fs::metadata(path).ok()?;
        let mtime: DateTime<Utc> = meta.modified().ok()?.into();

        let mut title = filename.trim_end_matches(".md").to_string();
        let mut category = "General".to_string();
        let mut tags: Vec<String> = vec![];
        let mut date = mtime.to_rfc3339();
        let mut summary = String::new();
        let mut weight = 1.0_f64;
        let mut reinforce_count = 0_u32;
        let mut last_interaction = mtime.to_rfc3339();
        let mut mem_key = String::new();
        let mut supersedes = String::new();
        let mut suspect = false;
        // Fail closed: no `origin:` line (every memory predating the field, and any
        // file a user dropped into the vault by hand) reads as External.
        let mut origin = Origin::External;

        // Split frontmatter (--- ... ---) from body.
        let body = if let Some(rest) = content.strip_prefix("---") {
            if let Some(end) = rest.find("\n---") {
                let fm = &rest[..end];
                let body = rest[end + 4..].trim_start_matches('\n');
                for line in fm.lines() {
                    if let Some(idx) = line.find(':') {
                        let key = line[..idx].trim();
                        let val = unquote(line[idx + 1..].trim());
                        match key {
                            "title" => title = val,
                            "category" => category = val,
                            "summary" => summary = val,
                            "date" => date = val,
                            "weight" => weight = val.parse().unwrap_or(1.0),
                            "reinforce_count" => reinforce_count = val.parse().unwrap_or(0),
                            "last_interaction" => last_interaction = val,
                            "origin" => origin = Origin::parse(&val),
                            "key" => mem_key = val,
                            "supersedes" => supersedes = val,
                            "suspect" => suspect = val.trim() == "true",
                            "tags" => {
                                tags = val
                                    .trim_matches(|c| c == '[' || c == ']')
                                    .split(',')
                                    .map(|t| unquote(t.trim()))
                                    .filter(|t| !t.is_empty())
                                    .collect()
                            }
                            _ => {}
                        }
                    }
                }
                body.to_string()
            } else {
                content.clone()
            }
        } else {
            content.clone()
        };

        let dynamic_weight = self.decay(&last_interaction, weight, reinforce_count);

        Some(Memory {
            id: filename.clone(),
            filename,
            key: mem_key,
            supersedes,
            suspect,
            title,
            category,
            tags,
            summary,
            body: body.trim().to_string(),
            date,
            weight: dynamic_weight,
            origin,
            reinforce_count,
            last_interaction,
        })
    }

    pub fn list(&self) -> Vec<Memory> {
        let mut out = vec![];
        if let Ok(entries) = fs::read_dir(&self.vault_path) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "md").unwrap_or(false) {
                    if let Some(m) = self.parse_file(&p) {
                        out.push(m);
                    }
                }
            }
        }
        out
    }

    /// Write a memory, updating in place when it carries a key we already hold.
    ///
    /// The append-only version of this was the root defect in the vault: a filename
    /// built from a slug and `timestamp_millis()` means every write is a new file,
    /// so a connector that pulls the same issue on every sync leaves one memory per
    /// sync, and the retriever ranks the copies against each other.
    ///
    /// A key changes the identity question from "what shall we call this file" to
    /// "have we already got this thing", which is the question the writer can
    /// actually answer.
    pub fn save(&self, input: &MemoryInput) -> std::io::Result<String> {
        // An explicit filename still wins: some callers own their filename
        // (`welcome.md`, `user-profile.md`) and a key must not move them.
        let by_key = input
            .filename
            .is_none()
            .then(|| input.key.as_deref().filter(|k| !k.trim().is_empty()).and_then(|k| self.find_by_key(k)))
            .flatten();
        let kept_reinforce = by_key.as_ref().map(|m| m.reinforce_count).unwrap_or(0);
        let filename = input.filename.clone().or(by_key.map(|m| m.filename)).unwrap_or_else(|| {
            let slug: String = input
                .title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect();
            let slug = slug.trim_matches('-');
            format!("{}-{}.md", if slug.is_empty() { "memory" } else { slug }, Utc::now().timestamp_millis())
        });
        let path = self.vault_path.join(&filename);
        let now = Utc::now().to_rfc3339();
        // `date` is when this content arrived, not when it was last written. A sync rewrites
        // what it already has on every pass; unchanged, it keeps its date — or every synced
        // email would look new every fifteen minutes, and "what came in since yesterday" would
        // be everything (the Chief of Staff's digest reads exactly that). Changed content — an
        // issue with a new comment — is new, and dated now.
        let date = path
            .exists()
            .then(|| self.parse_file(&path))
            .flatten()
            .filter(|m| same_content(m, input))
            .map(|m| m.date)
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| now.clone());
        let tags = format!("[{}]", input.tags.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(", "));
        let content = format!(
            "---\ntitle: \"{}\"\ncategory: \"{}\"\ntags: {}\ndate: \"{}\"\nsummary: \"{}\"\nweight: 1.000\nreinforce_count: {}\nlast_interaction: \"{}\"\norigin: \"{}\"\nkey: \"{}\"\nsupersedes: \"{}\"\nsuspect: {}\n---\n\n{}",
            esc(&input.title),
            esc(&input.category),
            tags,
            date,
            esc(&input.summary),
            // An update in place keeps what using it earned. Reinforcement is a
            // record of the user's attention, not of the bytes, and a connector
            // re-syncing the same issue must not reset it to zero.
            kept_reinforce,
            now,
            input.origin.as_str(),
            esc(input.key.as_deref().unwrap_or("")),
            esc(input.supersedes.as_deref().unwrap_or("")),
            screen(input.origin, &input.title, &input.body),
            input.body
        );
        fs::write(&path, content)?;
        Ok(filename)
    }

    pub fn delete(&self, filename: &str) -> bool {
        let path = self.vault_path.join(filename);
        path.exists() && fs::remove_file(path).is_ok()
    }
}
