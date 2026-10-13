//! One running copy of the app per data folder.
//!
//! Two copies on one data folder is not "two windows". Both poll the same Telegram bot and
//! the same Slack token, so each steals the other's updates; both run the automation
//! scheduler, so a routine fires twice; both write the same session files and the same
//! audit chain, and a hash chain written by two processes breaks. Launching from Finder
//! usually reaches the running copy anyway, but a dev build, a terminal launch or `open -n`
//! does not.
//!
//! So the first copy takes an exclusive lock on `instance.lock` in its data folder and holds
//! it for as long as it runs — the OS lets go when the process ends, however it ends, so a
//! crash never leaves the app unable to start. A second copy that finds the lock held asks
//! the first to come to the front, over a socket only this user can open, and exits.
//!
//! The lock is per data folder on purpose: a scratch instance started with
//! `SUPERUSER_DATA_DIR` is a different app with different files, and the eval harness
//! depends on being able to run one beside the real one.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Started with `SUPERUSER_DATA_DIR`: a copy of the app on a folder of its own, for the eval
/// harness or a trial run. Its data and its connected accounts are its own — it starts with none
/// connected (`secrets::accounts_place`) — while its model keys are the
/// real app's. It still starts nothing in the background that reaches an account: launched with
/// `SUPERUSER_SHARE_ACCOUNTS=1` it holds the real credentials, and would reply in the person's
/// Slack, answer their Telegram bot beside the real app, or copy their mail and documents into a
/// throwaway folder. See `lib.rs::start_slack_watcher` and its neighbours.
pub fn is_scratch() -> bool {
    std::env::var_os("SUPERUSER_DATA_DIR").is_some_and(|d| !d.is_empty())
}

/// What a launch found.
pub enum Claim {
    /// This is the only copy. `wakes` receives one message each time somebody opens the app
    /// again, which is the cue to bring the window forward.
    First { wakes: mpsc::Receiver<()> },
    /// Another copy holds this data folder, and has been asked to come forward.
    Running,
}

/// Where the second copy knocks. Beside the lock when the path fits in a socket address
/// (104 bytes on macOS), otherwise in this user's temp folder under a name derived from the
/// data folder — still one per folder, still this user's.
fn socket_path(data_dir: &Path) -> PathBuf {
    let beside = data_dir.join("instance.sock");
    if beside.as_os_str().len() < 100 {
        return beside;
    }
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data_dir.as_os_str().as_encoded_bytes());
    let short: String = digest.iter().take(6).map(|b| format!("{b:02x}")).collect();
    std::env::temp_dir().join(format!("superuser-{short}.sock"))
}

#[cfg(unix)]
pub fn claim(data_dir: &Path) -> Claim {
    use std::os::unix::io::AsRawFd;
    let _ = std::fs::create_dir_all(data_dir);
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(data_dir.join("instance.lock"));
    let Ok(lock) = lock else {
        // A folder the app can't write a lock into is one it can't run from either; starting
        // is how the person finds out why, where refusing here would say nothing useful.
        return Claim::First { wakes: mpsc::channel().1 };
    };
    // SAFETY: `flock` on a descriptor this function owns; no memory is involved.
    let held = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0;
    if !held {
        wake(data_dir);
        return Claim::Running;
    }
    // Held for the life of the process. Dropping the file would release the lock; the OS
    // releases it anyway when the process ends, which is exactly the lifetime wanted.
    std::mem::forget(lock);
    Claim::First { wakes: listen(data_dir) }
}

/// No lock on this platform yet: every launch is a first one.
#[cfg(not(unix))]
pub fn claim(_data_dir: &Path) -> Claim {
    Claim::First { wakes: mpsc::channel().1 }
}

#[cfg(unix)]
fn listen(data_dir: &Path) -> mpsc::Receiver<()> {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;
    let (tx, rx) = mpsc::channel();
    let path = socket_path(data_dir);
    // Whatever is there is stale: the lock says no other copy is running.
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = std::os::unix::net::UnixListener::bind(&path) else { return rx };
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    std::thread::Builder::new()
        .name("instance-wakes".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                let mut buf = [0u8; 8];
                let n = stream.read(&mut buf).unwrap_or(0);
                if buf[..n].starts_with(b"wake") && tx.send(()).is_err() {
                    return;
                }
            }
        })
        .ok();
    rx
}

#[cfg(unix)]
fn wake(data_dir: &Path) {
    use std::io::Write;
    if let Ok(mut s) = std::os::unix::net::UnixStream::connect(socket_path(data_dir)) {
        let _ = s.write_all(b"wake\n");
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        // Short on purpose: the socket goes beside the lock only when the path fits.
        let d = PathBuf::from("/tmp").join(format!("su-inst-{tag}-{}", super::super::inbox::short_id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_second_launch_brings_the_first_forward_and_does_not_start() {
        let d = temp("a");
        // A socket left by a copy that crashed is not a running copy.
        std::fs::write(d.join("instance.sock"), b"stale").unwrap();
        let Claim::First { wakes } = claim(&d) else { panic!("nothing else holds this folder") };
        assert!(matches!(claim(&d), Claim::Running), "the lock is held while the first copy runs");
        wakes.recv_timeout(std::time::Duration::from_secs(3)).expect("the second launch asked the first to come forward");

        // A different data folder is a different app — the scratch instance beside the real one.
        let other = temp("b");
        assert!(matches!(claim(&other), Claim::First { .. }));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_long_data_folder_still_gets_a_socket_it_can_bind() {
        let long = PathBuf::from("/tmp").join("x".repeat(120));
        let path = socket_path(&long);
        assert!(path.as_os_str().len() < 104, "{}", path.display());
        assert_eq!(path, socket_path(&long), "the same folder knocks on the same door");
        assert_ne!(path, socket_path(&PathBuf::from("/tmp").join("y".repeat(120))));
    }
}
