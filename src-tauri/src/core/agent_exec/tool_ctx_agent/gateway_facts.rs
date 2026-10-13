use super::*;

impl ToolCtx<'_> {
    /// Who is acting, where, and what started it — what `core::gateway` records.
    pub fn gateway_facts(&self) -> crate::core::gateway::Facts {
        crate::core::gateway::Facts {
            persona: self.persona.clone(),
            session: self.session_id.clone(),
            initiator: self
                .initiator
                .clone()
                .unwrap_or_else(|| crate::core::audit::Initiator::from_session(&self.session_id)),
        }
    }

    /// Record that external content entered this run.
    pub fn taint(&self) {
        self.tainted.store(true, Ordering::SeqCst);
    }

    pub fn is_tainted(&self) -> bool {
        self.tainted.load(Ordering::SeqCst)
    }

    /// Record that this run has actually looked at something.
    pub fn note_seen(&self, what: &str) {
        let what = what.trim();
        if what.is_empty() {
            return;
        }
        if let Ok(mut s) = self.seen.lock() {
            // Bounded: a run that greps a thousand files must not carry a thousand
            // strings into every later gate check.
            if !s.iter().any(|e| e == what) && s.len() < 200 {
                s.push(what.to_string());
            }
        }
    }

    pub fn seen_list(&self) -> Vec<String> {
        self.seen.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// How many times this exact call has already failed in this run.
    pub fn failure_count(&self, signature: &str) -> u32 {
        self.calls.lock().ok().and_then(|c| c.get(signature).copied()).unwrap_or(0)
    }

    pub fn note_failure(&self, signature: &str) {
        if let Ok(mut c) = self.calls.lock() {
            *c.entry(signature.to_string()).or_insert(0) += 1;
        }
    }

    /// Record a link this run came across.
    pub fn note_url(&self, url: &str) {
        let url = url.trim().to_lowercase();
        if url.len() < 9 {
            return;
        }
        if let Ok(mut u) = self.urls.lock() {
            if !u.iter().any(|e| *e == url) && u.len() < 400 {
                u.push(url);
            }
        }
    }

    /// Everything this run could honestly cite: what it read, plus what it saw.
    pub fn citable(&self) -> Vec<String> {
        let mut out = self.seen_list();
        out.extend(self.urls.lock().map(|u| u.clone()).unwrap_or_default());
        out
    }

    /// Keep a visual this run produced, in the shape the transcript replays it from.
    ///
    /// Charts were kept from the start and the others were not, which made the answer
    /// above them a liar on reload: "as the table shows", with no table. All four are the
    /// same kind of thing — a spec the transcript redraws — so all four are kept.
    pub fn note_visual(&self, name: &str, args: &Value) {
        if let Ok(mut c) = self.charts.lock() {
            // A turn that draws twenty of them is a turn with a bug in it; the cap
            // keeps one from bloating the session file.
            if c.iter().filter(|v| v["kind"] != "computer").count() < 12 {
                c.push(serde_json::json!({ "kind": "tool", "name": name, "args": args, "status": "ok" }));
            }
        }
    }

    /// How many coworkers this turn has asked.
    pub fn asked_count(&self) -> usize {
        self.asked.lock().map(|a| a.len()).unwrap_or(0)
    }

    /// Record one ask. `false` when this turn has already made exactly this one — which is
    /// a duplicate to refuse, not a second hop to make.
    pub fn note_ask(&self, key: &str) -> bool {
        let Ok(mut asked) = self.asked.lock() else { return true };
        if asked.iter().any(|k| k == key) {
            return false;
        }
        asked.push(key.to_string());
        true
    }

    pub fn charts_drawn(&self) -> Vec<Value> {
        self.charts.lock().map(|c| c.clone()).unwrap_or_default()
    }

    /// Keep one transcript item that must survive the turn — a browser screen the turn
    /// opened, beside the charts it drew. Same store and the same reason: the transcript
    /// is rebuilt from what the assistant message carries.
    pub fn note_item(&self, item: Value) {
        if let Ok(mut c) = self.charts.lock() {
            // Counted apart from the charts, so a long browsing turn can't use up theirs.
            if c.iter().filter(|v| v["kind"] == item["kind"]).count() < 12 {
                c.push(item);
            }
        }
    }

    /// A call that worked clears its FAILURE history: the model found the way
    /// through, and the next identical call is a legitimate re-read, not a retry.
    ///
    /// It does not clear the stall history — see [`ToolCtx::note_result`]. Those are
    /// different questions: "has this been failing" and "is this still telling us
    /// anything", and answering only the first is what let a turn run for 22 steps.
    pub fn note_success(&self, signature: &str) {
        if let Ok(mut c) = self.calls.lock() {
            c.remove(signature);
        }
    }

    /// Record what a successful call returned, and report how many times in a row
    /// this exact call has now returned this exact thing.
    ///
    /// # Why this is separate from the failure counter
    ///
    /// [`crate::core::gates::repetition`] catches a call that keeps *failing*. It
    /// cannot catch the more expensive shape: a call that keeps **succeeding** and
    /// keeps saying the same thing. `note_success` deletes the counter on every
    /// success, so a model that greps the same pattern twenty times — each call
    /// perfectly valid, each result identical — is invisible to it.
    ///
    /// Measured in the 2026-09-06 harness sweep: one `find_in_haystack` turn on
    /// `gemma4:12b` ran **22 steps over 1,036 seconds** and returned nothing. Every
    /// call in it succeeded. Seventeen minutes is not a slow answer, it is a hang
    /// with a progress bar.
    ///
    /// # Why "same output", not "same call"
    ///
    /// Because repeating a call is often correct and the result is what says whether
    /// it was. Re-reading a file after editing it, re-running a test after a fix,
    /// listing a directory after writing to it — all identical signatures, all
    /// legitimate, and all returning something *different*. Keying on the output
    /// means the legitimate cases never trip this, and the only thing that does is a
    /// call that has genuinely stopped adding information.
    ///
    /// Hashed rather than stored: a turn can produce megabytes of tool output and
    /// this is only ever asked whether two of them are equal.
    /// Fold one finished call into the groping counter, and return the count.
    ///
    /// A look resets it to zero; a miss increments it; anything else leaves it alone
    /// — a successful read in between does not clear a hunt, because the model may
    /// have read something unrelated, but a `list_dir` does, because that is the act
    /// the gate exists to demand.
    pub fn note_locator(&self, name: &str, result: &str) -> u32 {
        use std::sync::atomic::Ordering;
        if crate::core::gates::is_a_look(name) {
            self.gropes.store(0, Ordering::Relaxed);
            return 0;
        }
        if crate::core::gates::is_a_miss(result) {
            return self.gropes.fetch_add(1, Ordering::Relaxed) + 1;
        }
        self.gropes.load(Ordering::Relaxed)
    }

    pub fn grope_count(&self) -> u32 {
        self.gropes.load(std::sync::atomic::Ordering::Relaxed)
    }
}
