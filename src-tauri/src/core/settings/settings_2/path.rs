use super::*;

impl Settings {
    pub(in crate::core::settings) fn path(data_dir: &PathBuf) -> PathBuf {
        data_dir.join("settings.json")
    }

    /// Read the settings, and be careful about what "missing" is allowed to mean.
    ///
    /// `unwrap_or_default()` is right for a first run and wrong for a corrupt
    /// file: the defaults are not neutral here, they decide `local_only`. Silently
    /// adopting them because a few bytes were truncated means file corruption can
    /// turn the privacy switch off, which is the one thing this file must never do
    /// quietly. So a parse failure falls back to the last good copy, and if even
    /// that is gone it fails *closed* — local-only on — and leaves the unreadable
    /// file next to it rather than overwriting the evidence.
    pub fn load(data_dir: &PathBuf) -> Self {
        let p = Self::path(data_dir);
        let parse = |path: &PathBuf| fs::read_to_string(path).ok().and_then(|c| serde_json::from_str::<Settings>(&c).ok());

        let mut s = match parse(&p) {
            Some(s) => s,
            None if !p.exists() => Settings::default(), // genuine first run
            None => {
                let bak = Self::backup_path(data_dir);
                let _ = fs::rename(&p, p.with_extension("json.unreadable"));
                match parse(&bak) {
                    Some(s) => s,
                    None => Settings { local_only: true, cloud_enabled: false, ..Default::default() },
                }
            }
        };
        s.has_cloud_key = crate::core::secrets::has_cloud_key();
        crate::core::egress::set_local_only(s.local_only);
        crate::core::stall::set_limits(s.stall_seconds, s.stall_seconds_local);
        s.push_browser_switches();
        s
    }

    /// The browser's switches live where its proxy reads them per connection, so a change
    /// applies to the next request rather than the next browser — Bug Reproduction's
    /// staging address among them, the one private site its browser may reach.
    pub(in crate::core::settings) fn push_browser_switches(&self) {
        use std::sync::atomic::Ordering;
        crate::core::agentbrowser::computer::allow_loopback().store(self.browser_allow_loopback, Ordering::SeqCst);
        crate::core::agentbrowser::computer::headed().store(self.browser_headed, Ordering::SeqCst);
        crate::core::repro::set_staging(&self.repro_staging);
    }

    pub(in crate::core::settings) fn backup_path(data_dir: &Path) -> PathBuf {
        data_dir.join("settings.bak.json")
    }

    pub fn save(&self, data_dir: &PathBuf) {
        let _ = fs::create_dir_all(data_dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            // Atomic: a torn settings.json parses as nothing and `load` falls back
            // to the defaults, which would silently turn `local_only` back on —
            // or, worse, off. The privacy switch must not be decided by a race.
            if crate::core::store::write_atomic(&Self::path(data_dir), &json).is_ok() {
                // Keep the last good copy so `load` has something to recover from
                // if the live file is ever damaged by something outside our control.
                let _ = crate::core::store::write_atomic(&Self::backup_path(data_dir), &json);
            }
        }
        // Load and save are the two functions every settings change passes
        // through, which is why the egress policy is refreshed *here* rather than
        // at the twenty call sites that flip the switch. A hard switch enforced in
        // some of the places it's changed is not a hard switch.
        crate::core::egress::set_local_only(self.local_only);
        crate::core::stall::set_limits(self.stall_seconds, self.stall_seconds_local);
        self.push_browser_switches();
    }

    /// Standing instructions, as a turn receives them: trimmed, bounded, and with
    /// the rule for when they conflict with the assistant's role. Empty when there
    /// are none, so a user who never wrote any sends nothing.
    ///
    /// The precedence sentence is the part that makes this safe
    /// to put beside any role: "the role decides what you do and these decide how
    /// you do it." Without it, "always reply in three bullets" would fight a
    /// security reviewer's report format, and the model would pick one at random.
    pub fn standing_block(&self) -> String {
        let text = self.standing_instructions.trim();
        if text.is_empty() {
            return String::new();
        }
        let text: String = text.chars().take(STANDING_INSTRUCTIONS_MAX).collect();
        format!(
            "\n\n## The user's standing instructions\n\n\
             The user wrote these instructions for every assistant and every task, alongside your role:\n\n\
             {text}\n\n\
             Where the two conflict, the role decides what you do and these decide how you do it."
        )
    }
}
