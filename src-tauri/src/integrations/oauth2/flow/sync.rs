use super::super::*;

/// Pull the connector's feeds into the Memory Tree.
pub async fn sync(spec: &OAuthSpec, memory: &MemoryEngine) -> Result<SyncOutcome, String> {
    let token = access_token(spec).await?;
    let mut out = SyncOutcome::default();
    if spec.feeds.is_empty() {
        return Err(format!("{} is connected, but reading its content isn't wired up yet.", spec.name));
    }
    let mut last_err = None;
    for f in spec.feeds {
        let body = match fetch(spec, &token, f.path, f.post_body).await {
            Ok(v) => v,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };
        if let Value::Array(items) = crate::integrations::rest::dig_path(&body, f.items_at) {
            crate::integrations::rest::save_items(spec.id, spec.name, f, items, memory, &mut out);
        }
    }
    if out.created == 0 {
        if let Some(e) = last_err {
            return Err(e);
        }
    }
    Ok(out)
}
