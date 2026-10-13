//! per-provider endpoint overrides

use super::*;

//
// The registry hardcodes each vendor's public endpoint, which is right up until the
// traffic is meant to go somewhere else: an OpenAI-compatible gateway, a self-hosted
// vLLM, LM Studio, a proxy on loopback. Those all speak the same wire format and
// differ only in address, so the only thing in the way was that the address was a
// constant.
//
// It lives beside the keys rather than in Settings because it belongs with the key
// it is used with, and because an endpoint URL is not as harmless as it looks — it
// can carry a token in its path or name an internal host. This store is owner-only.
pub(super) fn provider_base(id: &str) -> String { format!("provider_base_{id}") }

pub fn set_provider_base_url(id: &str, url: &str) -> bool { set(&provider_base(id), url) }

pub fn get_provider_base_url(id: &str) -> Option<String> { get(&provider_base(id)) }

pub fn clear_provider_base_url(id: &str) -> bool { clear(&provider_base(id)) }

/// Model ids for an endpoint we can't have a curated list for, comma-separated.
/// Without this a custom endpoint is reachable but unpickable: the composer only
/// offers ids it was given, so "ready" would mean nothing the user could select.
pub(super) fn provider_models(id: &str) -> String { format!("provider_models_{id}") }

pub fn set_provider_models(id: &str, csv: &str) -> bool { set(&provider_models(id), csv) }

pub fn get_provider_models(id: &str) -> Option<String> { get(&provider_models(id)) }

pub fn clear_provider_models(id: &str) -> bool { clear(&provider_models(id)) }
