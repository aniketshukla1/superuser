//! cloud (Anthropic)

use super::*;

pub fn set_cloud_key(key: &str) -> bool { set(CLOUD_KEY, key) }

pub fn get_cloud_key() -> Option<String> { get(CLOUD_KEY) }

pub fn has_cloud_key() -> bool { get_cloud_key().is_some() }

pub fn clear_cloud_key() -> bool { clear(CLOUD_KEY) }
