use super::*;

impl Knowledge {
    /// `vault/knowledge`. Nothing is created until something is written.
    pub fn new(vault: &Path) -> Self {
        Self { root: vault.join("knowledge") }
    }

    pub(super) fn page_path(&self, category: &str, slug: &str) -> PathBuf {
        let cat = slugify(if category.trim().is_empty() { "general" } else { category });
        self.root.join(cat).join(format!("{slug}.md"))
    }

    pub(super) fn parse(path: &Path) -> Option<Page> {
        let text = fs::read_to_string(path).ok()?;
        let slug = path.file_stem()?.to_string_lossy().to_string();
        let category = path.parent()?.file_name()?.to_string_lossy().to_string();
        // Front line is `# Title`; everything after is the body.
        let (title, body) = match text.strip_prefix("# ") {
            Some(rest) => {
                let (t, b) = rest.split_once('\n').unwrap_or((rest, ""));
                (t.trim().to_string(), b.trim_start().to_string())
            }
            None => (slug.replace('-', " "), text.clone()),
        };
        let updated_at = fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
            .unwrap_or_default();
        Some(Page { links: wikilinks(&body), slug, title, category, body, updated_at })
    }

    /// Create or update the page for `title`. Upsert by slug: writing about the
    /// same topic twice improves one page instead of making two.
    ///
    /// Moving a page between categories rewrites it in the new one and removes the
    /// old file, so a topic never exists in two places at once.
    pub fn write(&self, title: &str, category: &str, body: &str) -> std::io::Result<Page> {
        let title = title.trim();
        if title.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "a page needs a title"));
        }
        let slug = slugify(title);
        let existing = self.find(&slug);
        let path = self.page_path(category, &slug);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, format!("# {title}\n\n{}\n", strip_leading_h1(body)))?;
        if let Some(old) = existing {
            let old_path = self.page_path(&old.category, &old.slug);
            if old_path != path {
                let _ = fs::remove_file(&old_path);
            }
        }
        let page = Self::parse(&path).ok_or_else(|| std::io::Error::other("page written but unreadable"))?;
        self.write_index();
        self.log(&format!("wrote [[{}]] in {}", page.title, page.category));
        Ok(page)
    }

    /// Find a page by slug, whatever category it sits in.
    pub fn find(&self, slug: &str) -> Option<Page> {
        let slug = slugify(slug);
        self.list().into_iter().find(|p| p.slug == slug)
    }

    /// Every page, newest first.
    pub fn list(&self) -> Vec<Page> {
        let mut out: Vec<Page> = walkdir::WalkDir::new(&self.root)
            .max_depth(2)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file() && e.path().extension().map(|x| x == "md").unwrap_or(false))
            // index.md and log.md are generated scaffolding, not knowledge.
            .filter(|e| !matches!(e.path().file_name().and_then(|n| n.to_str()), Some("index.md") | Some("log.md")))
            .filter_map(|e| Self::parse(e.path()))
            .collect();
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        out
    }

    pub fn delete(&self, slug: &str) -> bool {
        match self.find(slug) {
            Some(p) => {
                let ok = fs::remove_file(self.page_path(&p.category, &p.slug)).is_ok();
                if ok {
                    self.write_index();
                    self.log(&format!("removed {}", p.title));
                }
                ok
            }
            None => false,
        }
    }

    /// Keyword search over title, category and body. Titles weigh most: a page
    /// *about* a thing should beat one that merely mentions it.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Page> {
        let terms: Vec<String> = query
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 2)
            .map(String::from)
            .collect();
        if terms.is_empty() {
            return vec![];
        }
        let mut scored: Vec<(i64, Page)> = self
            .list()
            .into_iter()
            .map(|p| {
                let (title, body, cat) = (p.title.to_lowercase(), p.body.to_lowercase(), p.category.to_lowercase());
                let score: i64 = terms
                    .iter()
                    .map(|t| {
                        let mut s = 0;
                        if title.contains(t.as_str()) {
                            s += 10;
                        }
                        if cat.contains(t.as_str()) {
                            s += 3;
                        }
                        s + (body.matches(t.as_str()).count() as i64).min(5)
                    })
                    .sum();
                (score, p)
            })
            .filter(|(s, _)| *s > 0)
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(limit).map(|(_, p)| p).collect()
    }

    /// The generated table of contents. Rewritten on every change rather than
    /// edited, so it can never drift out of step with what's on disk.
    pub fn write_index(&self) {
        let pages = self.list();
        let mut by_cat: std::collections::BTreeMap<String, Vec<Page>> = Default::default();
        for p in pages {
            by_cat.entry(p.category.clone()).or_default().push(p);
        }
        let mut s = String::from("# Knowledge\n\n_Generated — edit the pages, not this file._\n");
        for (cat, mut pages) in by_cat {
            pages.sort_by(|a, b| a.title.cmp(&b.title));
            s.push_str(&format!("\n## {cat}\n\n"));
            for p in pages {
                s.push_str(&format!("- [[{}]] — {}\n", p.title, summary_line(&p.body)));
            }
        }
        let _ = fs::create_dir_all(&self.root);
        let _ = fs::write(self.root.join("index.md"), s);
    }

    /// Append to the change log — a plain record of what the agent wrote and when,
    /// so a knowledge base that edits itself stays reviewable.
    pub(super) fn log(&self, line: &str) {
        let _ = fs::create_dir_all(&self.root);
        let path = self.root.join("log.md");
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M");
        let mut s = fs::read_to_string(&path).unwrap_or_else(|_| String::from("# Change log\n"));
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&format!("- {stamp} — {line}\n"));
        let _ = fs::write(&path, s);
    }

    /// Nodes and edges built from *real* `[[links]]` between pages, plus the
    /// category each sits in. Links to pages that don't exist yet are dropped
    /// rather than drawn as phantom nodes.
    pub fn graph(&self) -> serde_json::Value {
        use serde_json::json;
        let pages = self.list();
        let by_slug: std::collections::HashMap<String, &Page> = pages.iter().map(|p| (p.slug.clone(), p)).collect();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut cats: std::collections::BTreeSet<String> = Default::default();

        for p in &pages {
            nodes.push(json!({ "id": format!("k:{}", p.slug), "label": p.title, "kind": "page" }));
            if cats.insert(p.category.clone()) {
                nodes.push(json!({ "id": format!("kc:{}", p.category), "label": p.category, "kind": "category" }));
            }
            edges.push(json!({ "s": format!("kc:{}", p.category), "t": format!("k:{}", p.slug) }));
            for link in &p.links {
                if let Some(target) = by_slug.get(&slugify(link)) {
                    if target.slug != p.slug {
                        edges.push(json!({ "s": format!("k:{}", p.slug), "t": format!("k:{}", target.slug) }));
                    }
                }
            }
        }
        json!({ "nodes": nodes, "edges": edges })
    }
}
