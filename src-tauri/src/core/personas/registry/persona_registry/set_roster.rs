use super::*;

impl PersonaRegistry {
    /// Set which worker types this assistant staffs when it delegates.
    ///
    /// An empty roster means "whatever is installed" — the same as never having
    /// set one, so clearing it is how you go back to the default rather than a
    /// state that means nothing.
    pub fn set_roster(&mut self, id: &str, roster: Vec<String>) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown assistant: {id}"));
        }
        let roster: Vec<String> =
            roster.into_iter().map(|r| r.trim().to_string()).filter(|r| !r.is_empty()).collect();
        self.state.subagents.insert(id.to_string(), roster.clone());
        if let Some(m) = self.entries.get_mut(id) {
            m.subagents = roster;
        }
        self.save();
        Ok(())
    }

    /// Who this assistant may hand work to. Never set — the default — is every other
    /// assistant, so they can all ask each other (owner's call, 2026-10-13). The exception is
    /// the Email Assistant: it reads strangers' mail unattended, and a hop from it is how an
    /// email would reach an assistant that acts (`email_bot::drafter_problem`). Once set, even
    /// to nobody, it is exactly what the person ticked — and an assistant with nobody to ask
    /// is never offered the tool at all.
    pub fn asks(&self, id: &str) -> Vec<String> {
        match self.state.asks.get(id) {
            Some(who) => who.clone(),
            None if id == crate::core::email_bot::ASSISTANT => Vec::new(),
            None => self.order.iter().filter(|o| *o != id).cloned().collect(),
        }
    }

    /// Set that list. Unknown ids, itself and repeats are dropped rather than refused: a
    /// grant to an assistant that isn't there is nothing, and saying so would make the
    /// caller police a list the registry can check for itself.
    pub fn set_asks(&mut self, id: &str, who: Vec<String>) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown assistant: {id}"));
        }
        let mut kept: Vec<String> = Vec::new();
        for want in who {
            let want = want.trim().to_string();
            if want.is_empty() || want == id || !self.entries.contains_key(&want) || kept.contains(&want) {
                continue;
            }
            kept.push(want);
        }
        self.state.asks.insert(id.to_string(), kept);
        self.save();
        Ok(())
    }

    /// The remote agent this assistant's turns go to, if it has one.
    pub fn remote(&self, id: &str) -> Option<String> {
        self.state.remote.get(id).filter(|u| !u.trim().is_empty()).cloned()
    }

    /// Point this assistant at a remote agent, or bring it back to running here with an
    /// empty address. An address the floor refuses is refused here too, with the reason —
    /// saving it and failing every turn later would be the same refusal said worse.
    pub fn set_remote(&mut self, id: &str, endpoint: &str) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown assistant: {id}"));
        }
        let endpoint = endpoint.trim();
        if endpoint.is_empty() {
            self.state.remote.remove(id);
        } else {
            if let Some(problem) = crate::core::aguiclient::endpoint_problem(endpoint) {
                return Err(format!("Not saved: {problem}"));
            }
            self.state.remote.insert(id.to_string(), endpoint.to_string());
        }
        self.save();
        Ok(())
    }

    /// Give this assistant its own browser, or take it away. Whether the browser is
    /// switched on at all is a separate, app-wide setting; this is only who may use it.
    pub fn set_browser(&mut self, id: &str, on: bool) -> Result<(), String> {
        let Some(m) = self.entries.get_mut(id) else {
            return Err(format!("unknown assistant: {id}"));
        };
        apply_browser_grant(m, on);
        self.state.browser.insert(id.to_string(), on);
        self.save();
        Ok(())
    }

    /// Remove a user-installed assistant: its manifest, its bundled files, and its
    /// lifecycle state.
    ///
    /// Built-ins are refused. Their manifests are compiled into the binary, so
    /// "deleting" one would remove a `personas.json` entry and change nothing
    /// visible — the assistant would still be there on the next load, which is a
    /// worse answer than saying no.
    pub fn uninstall(&mut self, id: &str) -> Result<(), String> {
        let Some(m) = self.entries.get(id) else {
            return Err(format!("unknown assistant: {id}"));
        };
        if m.builtin {
            return Err(format!("{} is built in and cannot be deleted — disable it instead", m.name));
        }
        // `source` is where the manifest was read from, so deletion follows the file
        // that actually produced this assistant rather than a path rebuilt from the
        // id — those differ whenever the id was derived from a filename.
        if let Some(src) = m.source.clone() {
            let path = PathBuf::from(&src);
            let _ = fs::remove_file(&path);
            // Bundled files live beside it as `<stem>-files/`.
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let _ = fs::remove_dir_all(path.with_file_name(format!("{stem}-files")));
            }
        }
        self.entries.remove(id);
        self.order.retain(|o| o != id);
        self.state.enabled.remove(id);
        self.state.surfaced.remove(id);
        self.state.mode.remove(id);
        self.state.subagents.remove(id);
        if self.state.default.as_deref() == Some(id) {
            self.state.default = None;
        }
        self.save();
        Ok(())
    }

    pub fn set_default(&mut self, id: &str) -> Result<(), String> {
        if !self.entries.contains_key(id) {
            return Err(format!("unknown persona: {id}"));
        }
        self.state.default = Some(id.to_string());
        self.state.enabled.insert(id.to_string(), true); // a default must be enabled
        self.save();
        Ok(())
    }
}
