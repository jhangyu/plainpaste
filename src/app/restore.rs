//! Pending clipboard restore: the runtime on/off switch, the snapshot to put back, and the
//! session that scheduled it. App is the only owner.

pub struct RestoreSchedule<S> {
    enabled: bool,
    pending: Option<Pending<S>>,
    next_session: u64,
}

struct Pending<S> {
    session: u64,
    snapshot: S,
}

impl<S> RestoreSchedule<S> {
    pub fn new(enabled: bool) -> Self {
        RestoreSchedule {
            enabled,
            pending: None,
            next_session: 0,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Flips the switch and returns the new state. Turning it off drops the pending
    /// snapshot, so the pasted text stays on the clipboard.
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        if !self.enabled && self.pending.take().is_some() {
            log::info!("pending clipboard restore cancelled: restore turned off");
        }
        self.enabled
    }

    /// Keeps `snapshot` for the next timer expiry and returns the session to hand to the timer.
    pub fn schedule(&mut self, snapshot: S) -> u64 {
        let session = self.next_session;
        self.next_session += 1;
        self.pending = Some(Pending { session, snapshot });
        session
    }

    /// Takes the pending snapshot whatever its session: a newer paste or quit supersedes it.
    pub fn take(&mut self) -> Option<S> {
        self.pending.take().map(|p| p.snapshot)
    }

    /// Takes the pending snapshot only if it was scheduled by `session`.
    pub fn take_due(&mut self, session: u64) -> Option<S> {
        if self.pending.as_ref().is_some_and(|p| p.session == session) {
            return self.take();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::RestoreSchedule;

    #[test]
    fn toggle_off_cancels_scheduled_restore() {
        let mut restore = RestoreSchedule::new(true);
        let session = restore.schedule("snapshot");
        assert!(!restore.toggle());
        assert_eq!(restore.take_due(session), None);
    }

    #[test]
    fn take_due_ignores_other_session() {
        let mut restore = RestoreSchedule::new(true);
        let stale = restore.schedule("first");
        let current = restore.schedule("second");
        assert_eq!(restore.take_due(stale), None);
        assert_eq!(restore.take_due(current), Some("second"));
    }
}
