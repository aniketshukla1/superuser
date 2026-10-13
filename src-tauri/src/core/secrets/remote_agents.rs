//! remote agents (AG-UI)

use super::*;

// The bearer token a remote agent issued for one assistant. Keyed by the assistant, not the
// address: two assistants on one agent may well hold different grants.
pub(super) fn agent_key(persona: &str) -> String { format!("agent_token_{persona}") }

pub fn set_agent_token(persona: &str, token: &str) -> bool { set_in(&accounts_place(), &agent_key(persona), token) }

pub fn get_agent_token(persona: &str) -> Option<String> { get_in(&accounts_place(), &agent_key(persona)) }

pub fn clear_agent_token(persona: &str) -> bool { clear_in(&accounts_place(), &agent_key(persona)) }

/// A human label for the connected account, e.g. "@octocat" or an email.
pub fn set_account(id: &str, label: &str) -> bool { set_in(&accounts_place(), &account_key(id), label) }

pub fn get_account(id: &str) -> Option<String> { get_in(&accounts_place(), &account_key(id)) }
