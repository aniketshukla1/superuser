//! secrets-backed wrappers

use super::*;

pub(super) fn registry_key(connector: &str) -> String {
    format!("accounts_{connector}")
}

pub(super) fn cred_key(connector: &str, account: &str) -> String {
    format!("{connector}__{account}")
}

pub(super) fn load(connector: &str) -> Registry {
    Registry::parse(secrets::get_integration_token(&registry_key(connector)).as_deref())
}

pub(super) fn save(connector: &str, reg: &Registry) {
    secrets::set_integration_token(&registry_key(connector), &reg.to_json());
}

/// Every account registered for a connector. A pre-multi-account connection shows
/// up as a single implicit "default" account.
pub fn list(connector: &str) -> Vec<Account> {
    let reg = load(connector);
    if !reg.accounts.is_empty() {
        return reg.accounts;
    }
    if secrets::has_integration_token(connector) {
        let label = secrets::get_account(connector).unwrap_or_else(|| "Connected".into());
        return vec![Account { id: "default".into(), label, default: true }];
    }
    Vec::new()
}

/// Register (or relabel) an account and store its credential.
pub fn add(connector: &str, account_id: &str, label: &str, credential: &str) -> Result<(), String> {
    if account_id.trim().is_empty() || credential.trim().is_empty() {
        return Err("an account needs an id and a credential".into());
    }
    let mut reg = load(connector);
    reg.upsert(account_id, label);
    save(connector, &reg);
    secrets::set_integration_token(&cred_key(connector, account_id), credential);
    Ok(())
}

pub fn remove(connector: &str, account_id: &str) -> bool {
    let mut reg = load(connector);
    let removed = reg.remove(account_id);
    if removed {
        save(connector, &reg);
        secrets::clear_integration_token(&cred_key(connector, account_id));
    }
    removed
}

pub fn set_default(connector: &str, account_id: &str) -> Result<(), String> {
    let mut reg = load(connector);
    if !reg.set_default(account_id) {
        return Err(format!("unknown account: {account_id}"));
    }
    save(connector, &reg);
    Ok(())
}

/// The credential to use: a named account, else the default, else the legacy
/// single-connection credential.
pub fn credential(connector: &str, account_id: Option<&str>) -> Option<String> {
    let reg = load(connector);
    let wanted = account_id.map(String::from).or_else(|| reg.default_id());
    if let Some(id) = wanted {
        if id != "default" {
            if let Some(c) = secrets::get_integration_token(&cred_key(connector, &id)) {
                return Some(c);
            }
        }
    }
    secrets::get_integration_token(connector)
}
