use super::*;

pub(super) fn write_at(place: &Place, map: &Map<String, Value>) -> Result<(), String> {
    if place.read_only {
        return Err("this store is read-only here".into());
    }
    let key = match load_key(place) {
        Some(k) => k,
        None => {
            // Never mint a new key over a sealed store: whatever it holds would be unreadable
            // for good. The callers check first; this is the second lock on the same door.
            let sealed_already = fs::read_to_string(place.store())
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .and_then(|v| v.as_object().map(is_sealed))
                .unwrap_or(false);
            if sealed_already {
                return Err("the store is sealed and its key is missing".into());
            }
            create_key(place)?
        }
    };
    let plain = serde_json::to_vec(map).map_err(|e| e.to_string())?;
    let sealed = seal(&key, &plain)?;
    // Proved to open before it replaces anything.
    let reopened = serde_json::from_str::<Value>(&sealed)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .map(|obj| open(&key, &obj))
        .ok_or("the sealed store didn't parse")??;
    if reopened != plain {
        return Err("the sealed store didn't open to what was sealed".into());
    }
    // Atomic, because this is the file that holds every API key: a torn read yields
    // nothing, and the next `set` would write that back. Owner-only from creation, so it
    // is never briefly world-readable. See `core::store`.
    crate::core::store::write_atomic_private(&place.store(), &sealed).map_err(|e| e.to_string())
}

pub(super) fn set_at(place: &Place, key: &str, val: &str) -> bool {
    match read_at(place) {
        Ok((mut m, _)) => {
            m.insert(key.to_string(), Value::String(val.to_string()));
            write_at(place, &m).is_ok()
        }
        Err(_) => false,
    }
}

pub(super) fn get_at(place: &Place, key: &str) -> Option<String> {
    match read_at(place) {
        Ok((m, _)) => m.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string),
        Err(_) => None,
    }
}

pub(super) fn clear_at(place: &Place, key: &str) -> bool {
    match read_at(place) {
        Ok((mut m, _)) => {
            if m.remove(key).is_none() {
                return true;
            }
            write_at(place, &m).is_ok()
        }
        Err(_) => false,
    }
}

/// Move a store that won't open out of the way, so keys can be saved again. Renamed, never
/// deleted: if its key turns up, the old file still opens with it.
pub fn set_aside_locked() -> Result<PathBuf, String> {
    set_aside_at(&Place::app())
}

pub(super) fn set_aside_at(place: &Place) -> Result<PathBuf, String> {
    if place.read_only {
        return Err("this store is read-only here".into());
    }
    if read_at(place).is_ok() {
        return Err("the saved keys open, so there is nothing to set aside".into());
    }
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let to = place.dir.join(format!("secrets.locked-{stamp}.json"));
    fs::rename(place.store(), &to).map_err(|e| format!("couldn't move it: {e}"))?;
    Ok(to)
}

/// How the app's own store is kept. Reading it converts a plaintext store, as any read does.
pub fn sealing() -> Sealing {
    match read_at(&Place::app()) {
        Ok((_, s)) => s,
        Err(why) => Sealing::Locked(why),
    }
}

pub(super) fn set(key: &str, val: &str) -> bool {
    set_in(&Place::app(), key, val)
}

pub(super) fn get(key: &str) -> Option<String> {
    get_in(&Place::app(), key)
}

pub(super) fn clear(key: &str) -> bool {
    clear_in(&Place::app(), key)
}

/// Save to a store, mirrored to the keychain when the store keeps a mirror there.
pub(super) fn set_in(place: &Place, key: &str, val: &str) -> bool {
    if place.keychain {
        // best-effort keychain mirror (may fail for unsigned builds — that's fine)
        if let Ok(e) = Entry::new(SERVICE, key) {
            let _ = e.set_password(val);
        }
    }
    set_at(place, key, val)
}

/// Read from a store's file, then — only for a store mirrored there — from the keychain. A
/// store without a mirror never looks: the keychain's entries are this user's everywhere, so
/// looking would hand a store of its own the real app's credentials.
pub(super) fn get_in(place: &Place, key: &str) -> Option<String> {
    if let Some(s) = get_at(place, key) {
        return Some(s);
    }
    if !place.keychain {
        return None;
    }
    let from = |service: &str| Entry::new(service, key).ok().and_then(|e| e.get_password().ok()).filter(|s| !s.is_empty());
    from(SERVICE).or_else(|| from(LEGACY_SERVICE))
}

pub(super) fn clear_in(place: &Place, key: &str) -> bool {
    if place.keychain {
        if let Ok(e) = Entry::new(SERVICE, key) {
            let _ = e.delete_credential();
        }
    }
    clear_at(place, key)
}
