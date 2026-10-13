//! per-integration tokens

use super::*;

pub(super) fn integ_key(id: &str) -> String { format!("integration_{id}") }

pub(super) fn account_key(id: &str) -> String { format!("account_{id}") }

pub fn set_integration_token(id: &str, token: &str) -> bool { set_in(&accounts_place(), &integ_key(id), token) }

pub fn get_integration_token(id: &str) -> Option<String> { get_in(&accounts_place(), &integ_key(id)) }

pub fn has_integration_token(id: &str) -> bool { get_integration_token(id).is_some() }

pub fn clear_integration_token(id: &str) -> bool {
    let _ = clear_in(&accounts_place(), &account_key(id));
    clear_in(&accounts_place(), &integ_key(id))
}
