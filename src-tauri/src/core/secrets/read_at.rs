use super::*;

pub(super) const SERVICE: &str = "ai.superuser.app";

/// The keychain service used under the old name, BigBrainAI. Only consulted on
/// read: the authoritative store is `secrets.json` in the data dir, which the
/// startup migration copies across, so this is the belt to that braces — it covers
/// an install whose file was lost but whose keychain entries survived.
pub(super) const LEGACY_SERVICE: &str = "ai.bigbrain.app";

pub(super) const CLOUD_KEY: &str = "anthropic_api_key";

/// The keychain account that mirrors the store's key.
pub(super) const KEY_ACCOUNT: &str = "secrets-store-key";

pub(super) const CIPHER: &str = "chacha20-poly1305";

/// Bound into every seal, so a blob sealed for anything else never opens as this store.
pub(super) const AAD: &[u8] = b"superuser secrets store v2";

pub(super) fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let d = base.join("ai.superuser.app");
    let _ = fs::create_dir_all(&d);
    d
}

/// Where a store lives and how it may be treated. The app's is [`Place::app`]; tests use a
/// folder of their own and never touch the keychain, whose entries are global to this user.
pub(super) struct Place {
    pub(super) dir: PathBuf,
    /// Mirror the key to the keychain, and look there when the key file is gone.
    pub(super) keychain: bool,
    /// Encrypt a plaintext store found on read. Off for a scratch instance: this file is the
    /// real one whatever `SUPERUSER_DATA_DIR` says, and an older copy of the app still
    /// running beside it reads it in the old format.
    pub(super) migrate: bool,
    /// Refuse every write. Set for the app's own store under `cargo test`: a test that
    /// reaches the real credentials may read them as the app does, and must never change them.
    pub(super) read_only: bool,
}

impl Place {
    pub(super) fn app() -> Self {
        let scratch = crate::core::instance::is_scratch();
        let testing = cfg!(test);
        Place { dir: data_dir(), keychain: !testing, migrate: !scratch && !testing, read_only: testing }
    }
    pub(super) fn store(&self) -> PathBuf {
        self.dir.join("secrets.json")
    }
    pub(super) fn key_file(&self) -> PathBuf {
        self.dir.join("secrets.key")
    }
}

/// How the store on disk is kept, for Settings and the health check.
#[derive(Debug, Clone, PartialEq)]
pub enum Sealing {
    /// Nothing saved yet.
    Empty,
    Encrypted,
    /// Plaintext, from before encryption, and not converted — a scratch instance, or a
    /// conversion that failed and left the file as it was.
    Plain,
    /// Encrypted, and it would not open. The reason is for a person.
    Locked(String),
}

impl Sealing {
    pub fn label(&self) -> String {
        match self {
            Sealing::Empty => "empty".into(),
            Sealing::Encrypted => "encrypted".into(),
            Sealing::Plain => "plain".into(),
            Sealing::Locked(why) => format!("locked: {why}"),
        }
    }
}

pub(super) fn decode_key(text: &str) -> Option<[u8; 32]> {
    B64.decode(text.trim()).ok()?.try_into().ok()
}

pub(super) fn load_key(place: &Place) -> Option<[u8; 32]> {
    if let Some(k) = fs::read_to_string(place.key_file()).ok().and_then(|t| decode_key(&t)) {
        return Some(k);
    }
    if place.keychain {
        let mirrored = Entry::new(SERVICE, KEY_ACCOUNT).ok().and_then(|e| e.get_password().ok()).and_then(|t| decode_key(&t));
        if let Some(k) = mirrored {
            // Put the file back, so the keychain is asked once rather than on every read.
            let _ = crate::core::store::write_atomic_private(&place.key_file(), &B64.encode(k));
            return Some(k);
        }
    }
    None
}

pub(super) fn create_key(place: &Place) -> Result<[u8; 32], String> {
    use ring::rand::SecureRandom;
    let mut key = [0u8; 32];
    ring::rand::SystemRandom::new().fill(&mut key).map_err(|_| "no randomness to make a key with".to_string())?;
    crate::core::store::write_atomic_private(&place.key_file(), &B64.encode(key)).map_err(|e| format!("couldn't save the key: {e}"))?;
    // Read back before anything is sealed with it: a key that didn't land is every secret lost.
    if load_key(&Place { dir: place.dir.clone(), keychain: false, migrate: false, read_only: true }) != Some(key) {
        return Err("the key didn't read back as written".into());
    }
    if place.keychain {
        if let Ok(e) = Entry::new(SERVICE, KEY_ACCOUNT) {
            let _ = e.set_password(&B64.encode(key));
        }
    }
    Ok(key)
}

pub(super) fn cipher_key(key: &[u8; 32]) -> Result<ring::aead::LessSafeKey, String> {
    ring::aead::UnboundKey::new(&ring::aead::CHACHA20_POLY1305, key)
        .map(ring::aead::LessSafeKey::new)
        .map_err(|_| "the key is the wrong shape".to_string())
}

pub(super) fn seal(key: &[u8; 32], plain: &[u8]) -> Result<String, String> {
    use ring::rand::SecureRandom;
    let mut nonce = [0u8; ring::aead::NONCE_LEN];
    ring::rand::SystemRandom::new().fill(&mut nonce).map_err(|_| "no randomness for a nonce".to_string())?;
    let mut data = plain.to_vec();
    cipher_key(key)?
        .seal_in_place_append_tag(ring::aead::Nonce::assume_unique_for_key(nonce), ring::aead::Aad::from(AAD), &mut data)
        .map_err(|_| "sealing failed".to_string())?;
    Ok(json!({ "v": 2, "cipher": CIPHER, "nonce": B64.encode(nonce), "data": B64.encode(data) }).to_string())
}

pub(super) fn open(key: &[u8; 32], envelope: &Map<String, Value>) -> Result<Vec<u8>, String> {
    if envelope.get("cipher").and_then(Value::as_str) != Some(CIPHER) {
        return Err("it was sealed with a cipher this build doesn't know".into());
    }
    let nonce: [u8; ring::aead::NONCE_LEN] = envelope
        .get("nonce")
        .and_then(Value::as_str)
        .and_then(|n| B64.decode(n).ok())
        .and_then(|n| n.try_into().ok())
        .ok_or("its nonce is damaged")?;
    let mut data = envelope.get("data").and_then(Value::as_str).and_then(|d| B64.decode(d).ok()).ok_or("its contents are damaged")?;
    let plain = cipher_key(key)?
        .open_in_place(ring::aead::Nonce::assume_unique_for_key(nonce), ring::aead::Aad::from(AAD), &mut data)
        .map_err(|_| "the key doesn't open it — the key was replaced, or the file was altered".to_string())?;
    Ok(plain.to_vec())
}

pub(super) fn is_sealed(obj: &Map<String, Value>) -> bool {
    obj.get("v").and_then(Value::as_u64) == Some(2)
}

/// The store's contents, or why they can't be had.
pub(super) fn read_at(place: &Place) -> Result<(Map<String, Value>, Sealing), String> {
    let Ok(text) = fs::read_to_string(place.store()) else { return Ok((Map::new(), Sealing::Empty)) };
    // A file that isn't JSON at all reads as empty, as it always has: the atomic writes below
    // mean a torn read can't produce one, so this is a file damaged by something else.
    let Some(obj) = serde_json::from_str::<Value>(&text).ok().and_then(|v| v.as_object().cloned()) else {
        return Ok((Map::new(), Sealing::Empty));
    };
    if is_sealed(&obj) {
        let key = load_key(place).ok_or("the key that opens it is missing")?;
        let plain = open(&key, &obj)?;
        let map = serde_json::from_slice::<Value>(&plain)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .ok_or("it opened, and what was inside isn't a list of secrets")?;
        return Ok((map, Sealing::Encrypted));
    }
    if obj.is_empty() {
        return Ok((obj, Sealing::Empty));
    }
    // Plaintext from before encryption: converted where it is found, unless this is a copy
    // of the app that must leave the shared file alone. A conversion that fails leaves the
    // file exactly as it was.
    if place.migrate && write_at(place, &obj).is_ok() {
        return Ok((obj, Sealing::Encrypted));
    }
    Ok((obj, Sealing::Plain))
}
