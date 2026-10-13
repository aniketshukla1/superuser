use super::*;

impl Control {
    pub fn new(now: f64) -> Self {
        Control { holder: Holder::Assistant, since: now, reason: None, requested_at: None, secret: None, releases: 0, supplied: 0, declines: 0 }
    }

    /// The user saying no to a request for help. Only an open request can be declined, and
    /// declining never touches a wheel the user already holds.
    pub fn decline(&mut self, now: f64) -> bool {
        self.expire(now);
        if self.holder != Holder::Assistant || self.requested_at.is_none() {
            return false;
        }
        self.requested_at = None;
        self.reason = None;
        self.declines += 1;
        true
    }

    pub fn declines(&self) -> u64 {
        self.declines
    }

    pub(super) fn expire(&mut self, now: f64) {
        // Only ever an ASK expires. A user actually holding the wheel is never timed out
        // from under them: they may be halfway through typing a code.
        if self.holder == Holder::Assistant {
            if let Some(at) = self.requested_at {
                if now - at > ASK_TTL_SECS {
                    self.requested_at = None;
                    self.reason = None;
                }
            }
        }
        if self.secret.as_ref().is_some_and(|s| now - s.at > ASK_TTL_SECS) {
            self.secret = None;
        }
    }

    pub fn view(&mut self, now: f64) -> ControlView {
        self.expire(now);
        ControlView {
            holder: self.holder,
            since: self.since,
            reason: self.reason.clone(),
            requested: self.requested_at.is_some() && self.holder == Holder::Assistant,
            secret_wanted: self.secret.as_ref().map(|s| s.label.clone()),
        }
    }

    /// The assistant asking for help. It does not take control: an assistant that could
    /// hand itself to a person could also hand a person a page they never asked to see.
    pub fn request_help(&mut self, reason: &str, now: f64) {
        let reason = reason.trim();
        self.requested_at = Some(now);
        self.reason = Some(if reason.is_empty() { "The assistant needs you to continue.".into() } else { reason.to_string() });
    }

    /// The assistant asking for one value, naming the field it goes in.
    pub fn request_secret(&mut self, label: &str, r#ref: &str, backend_node_id: i64, now: f64) {
        let label = label.trim();
        self.secret = Some(SecretAsk {
            label: if label.is_empty() { "the value this page is asking for".into() } else { label.to_string() },
            r#ref: r#ref.to_string(),
            backend_node_id,
            at: now,
        });
    }

    /// What a typed value may go into — or `None`, which is what keeps the masked box from
    /// being a general way to type into the page. Stale requests stop being answerable at
    /// the same moment they stop being shown.
    pub fn pending_secret(&mut self, now: f64) -> Option<SecretAsk> {
        self.expire(now);
        self.secret.clone()
    }

    /// The value landed. Called only after it reached the field; a failure leaves the
    /// request open so the user can try again.
    pub fn secret_supplied(&mut self) {
        self.secret = None;
        self.supplied += 1;
    }

    /// The field is gone, so the request is closed rather than left for someone to type a
    /// password into a dead ref forever.
    pub fn secret_abandoned(&mut self) {
        self.secret = None;
    }

    /// The user taking the wheel. The reason survives — it is what they were asked to do —
    /// and any pending secret goes: someone driving can type it into the page themselves.
    pub fn take(&mut self, now: f64) {
        self.holder = Holder::User;
        self.since = now;
        self.requested_at = None;
        self.secret = None;
    }

    /// The user handing back. The reason goes with it; left set, the screen would still be
    /// showing a request that has been dealt with.
    pub fn release(&mut self, now: f64) {
        let was_user = self.holder == Holder::User;
        self.holder = Holder::Assistant;
        self.since = now;
        self.reason = None;
        self.requested_at = None;
        self.secret = None;
        if was_user {
            self.releases += 1;
        }
    }

    pub fn bot_may_act(&self) -> Result<(), &'static str> {
        match self.holder {
            Holder::User => Err(HUMAN_HAS_CONTROL),
            Holder::Assistant => Ok(()),
        }
    }

    /// Whether the user's input should be applied. A socket being open is not permission.
    pub fn user_may_drive(&self) -> bool {
        self.holder == Holder::User
    }

    pub fn releases(&self) -> u64 {
        self.releases
    }

    pub fn supplied(&self) -> u64 {
        self.supplied
    }

    pub fn holder(&self) -> Holder {
        self.holder
    }
}
