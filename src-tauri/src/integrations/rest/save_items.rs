use super::*;

/// Map a feed's JSON items into the Memory Tree. Shared by the token-based REST
/// engine and the OAuth engine (which differ only in how they authenticate).
pub fn save_items(connector_id: &str, connector_name: &str, feed: &Feed, items: &[Value], memory: &MemoryEngine, out: &mut SyncOutcome) {
    for item in items.iter().take(40) {
        let id = text_of(&item[feed.id_key]);
        if id.is_empty() {
            continue;
        }
        let title = feed
            .title_keys
            .iter()
            .map(|k| text_of(&item[*k]))
            .find(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("{} {id}", feed.kind));

        let mut lines = Vec::new();
        for (label, key) in feed.body_keys {
            let val = text_of(&item[*key]);
            if !val.trim().is_empty() {
                lines.push(format!("- {label}: {}", val.chars().take(300).collect::<String>()));
            }
        }
        let detail = lines.join("\n");
        let slug: String = id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).take(40).collect();
        let filename = format!("{connector_id}-{}-{slug}.md", feed.kind);
        let existed = memory.vault_path.join(&filename).exists();

        let input = MemoryInput {
            filename: Some(filename),
            title: format!("{connector_name}: {}", title.chars().take(120).collect::<String>()),
            category: connector_name.to_string(),
            tags: vec![connector_id.to_string(), feed.kind.to_string()],
            summary: lines.first().cloned().unwrap_or_else(|| format!("{connector_name} {}", feed.kind)),
            body: format!("**{title}**\n\n{detail}"),
            origin: Origin::External,
            key: None,
            supersedes: None,
        };
        if memory.save(&input).is_ok() && !existed {
            out.created += 1;
            out.titles.push(format!("{connector_name}: {title}"));
        }
    }
}

/// Follow a dotted path into a JSON value (shared with the OAuth engine).
pub fn dig_path<'a>(v: &'a Value, path: &str) -> &'a Value {
    dig(v, path)
}

/// Pull every feed and write new items into the Memory Tree.
pub async fn sync(spec: &RestSpec, stored: &str, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let creds = Creds::parse(stored);
    let mut out = SyncOutcome::default();
    let mut last_err: Option<String> = None;

    for feed in spec.feeds {
        let body = match get(spec, &creds, feed.path).await {
            Ok(v) => v,
            Err(e) => {
                last_err = Some(e);
                continue; // one failing feed shouldn't kill the whole sync
            }
        };
        let items = match dig(&body, feed.items_at) {
            Value::Array(a) => a.clone(),
            _ => continue,
        };
        save_items(spec.id, spec.name, feed, &items, memory, &mut out);
    }

    if out.created == 0 {
        if let Some(e) = last_err {
            return Err(e);
        }
    }
    Ok(out)
}
