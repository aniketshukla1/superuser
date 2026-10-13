use super::*;

impl ToolCtx<'_> {
    pub fn note_result(&self, signature: &str, out: &str) -> u32 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        out.hash(&mut h);
        let digest = h.finish();
        let Ok(mut s) = self.stalls.lock() else { return 0 };
        match s.get_mut(signature) {
            Some((prev, n)) if *prev == digest => {
                *n += 1;
                *n
            }
            slot => {
                let fresh = (digest, 1);
                match slot {
                    Some(v) => *v = fresh,
                    None => {
                        // Bounded like `seen`: a turn calling hundreds of distinct
                        // tools must not carry a map of all of them.
                        if s.len() < 200 {
                            s.insert(signature.to_string(), fresh);
                        }
                    }
                }
                1
            }
        }
    }

    /// How many times running this call would tell us what we already know.
    pub fn stalled_count(&self, signature: &str) -> u32 {
        self.stalls.lock().ok().and_then(|s| s.get(signature).map(|(_, n)| *n)).unwrap_or(0)
    }
}
