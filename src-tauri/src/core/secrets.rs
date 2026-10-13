//! Secret storage. The OS keychain is the preferred home, but unsigned dev
//! builds hit keychain ACL failures on every rebuild — so we keep an
//! authoritative local file (0600) in the app data dir and treat the keychain
//! as a best-effort mirror. Reads prefer the file, then fall back to keychain.
//!
//! **The file is encrypted at rest**: ChaCha20-Poly1305 over the
//! whole map, so neither the keys nor which services they are for can be read off disk.
//! The key follows the same rule as the secrets, for the same reason — its own owner-only
//! file, `secrets.key`, mirrored to the keychain and read from there only when the file is
//! gone. A key the keychain held alone would prompt, or fail, on every rebuild of an
//! unsigned build, and a key that fails to load is every saved credential gone at once.
//!
//! What that buys, said plainly: the secrets file on its own — copied, synced, attached to a
//! bug report, pasted, or read by something scanning files for token shapes — gives nothing
//! away. Something that can read this whole folder as this user can read the key beside it;
//! stopping that needs a key only the keychain holds.
//!
//! A store that is encrypted and cannot be opened is **locked**, not empty: nothing is
//! written over it, so saving one key can never destroy the others it failed to read.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use keyring::Entry;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

mod read_at;
mod write_at;
mod person_accounts;
mod cloud;
mod model_provider_keys;
mod per_provider_endpoint;
mod per_integration_tokens;
mod remote_agents;

pub use read_at::*;
pub use write_at::*;
use person_accounts::*;
pub use cloud::*;
pub use model_provider_keys::*;
pub use per_provider_endpoint::*;
pub use per_integration_tokens::*;
pub use remote_agents::*;
