//! the person's accounts

use super::*;

//
// Connected apps' credentials, their account labels, and remote agents' tokens. A scratch
// instance (`SUPERUSER_DATA_DIR`) keeps these in a store of its own, in its own folder and never
// in the keychain: it starts with no account connected, and whatever it connects, changes or
// disconnects stays in that folder. So a harness task can't post in the person's Slack, read
// their mail, or disconnect their GitHub. Model keys are still read from
// the app's store — that is how the eval harness runs on cloud models.
//
// `SUPERUSER_SHARE_ACCOUNTS=1` gives a scratch instance the person's real accounts, for a trial
// that needs them. It is set where the instance is launched, so nothing running inside it can
// switch it on.

pub(super) fn accounts_place() -> Place {
    let scratch = std::env::var_os("SUPERUSER_DATA_DIR").map(PathBuf::from).filter(|d| !d.as_os_str().is_empty());
    let shared = std::env::var_os("SUPERUSER_SHARE_ACCOUNTS").is_some_and(|v| v == "1");
    accounts_place_for(scratch, shared, Place::app())
}

/// A subfolder rather than the scratch folder itself, so that a `SUPERUSER_DATA_DIR` pointed at
/// the real data folder still can't land on the real `secrets.json`.
pub(super) fn accounts_place_for(scratch: Option<PathBuf>, shared: bool, app: Place) -> Place {
    match scratch {
        Some(dir) if !shared => Place { dir: dir.join("accounts"), keychain: false, migrate: true, read_only: false },
        _ => app,
    }
}
