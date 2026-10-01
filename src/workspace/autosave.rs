//! Autosave orchestration and debouncing (Q-077).

use std::time::{Duration, Instant};

pub const DEFAULT_DEBOUNCE_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutosaveState {
    Clean,
    Dirty,
    Saving,
}

#[derive(Debug)]
pub struct Autosaver {
    debounce: Duration,
    state: AutosaveState,
    dirty_revision: u64,
    saved_revision: u64,
    scheduled_at: Option<Instant>,
    suppressed: bool,
    last_error: Option<String>,
    save_count: usize,
}

impl Default for Autosaver {
    fn default() -> Self {
        Self::new(Duration::from_millis(DEFAULT_DEBOUNCE_MS))
    }
}

impl Autosaver {
    pub fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            state: AutosaveState::Clean,
            dirty_revision: 0,
            saved_revision: 0,
            scheduled_at: None,
            suppressed: false,
            last_error: None,
            save_count: 0,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.state == AutosaveState::Dirty || self.dirty_revision > self.saved_revision
    }

    pub fn is_suppressed(&self) -> bool {
        self.suppressed
    }

    pub fn save_count(&self) -> usize {
        self.save_count
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn dirty_revision(&self) -> u64 {
        self.dirty_revision
    }

    pub fn saved_revision(&self) -> u64 {
        self.saved_revision
    }

    pub fn suppress(&mut self) {
        self.suppressed = true;
    }

    pub fn resume(&mut self) {
        self.suppressed = false;
    }

    /// Marks dirty and schedules a save at `now + debounce`.
    /// If an earlier save was already scheduled, resets the deadline to `now + debounce` (single-shot debounce).
    pub fn mark_dirty(&mut self, now: Instant) {
        if self.suppressed {
            return;
        }
        self.dirty_revision += 1;
        self.state = AutosaveState::Dirty;
        self.scheduled_at = Some(now + self.debounce);
    }

    /// Checks if the debounce deadline has arrived at `now`.
    pub fn is_deadline_reached(&self, now: Instant) -> bool {
        if self.suppressed || self.state != AutosaveState::Dirty {
            return false;
        }
        match self.scheduled_at {
            Some(deadline) => now >= deadline,
            None => false,
        }
    }

    /// Prepares to flush: returns the revision to save if a save is needed.
    pub fn prepare_flush(&mut self) -> Option<u64> {
        if self.suppressed || !self.is_dirty() {
            return None;
        }
        self.state = AutosaveState::Saving;
        self.scheduled_at = None;
        Some(self.dirty_revision)
    }

    pub fn on_save_success(&mut self, revision: u64) {
        self.saved_revision = std::cmp::max(self.saved_revision, revision);
        if self.dirty_revision == revision {
            self.state = AutosaveState::Clean;
            self.scheduled_at = None;
        } else {
            // Edits occurred while saving was in flight
            self.state = AutosaveState::Dirty;
        }
        self.last_error = None;
        self.save_count += 1;
    }

    pub fn on_save_failure(&mut self, err: String) {
        // Dirty revision stays dirty until the write succeeds!
        self.state = AutosaveState::Dirty;
        self.last_error = Some(err);
    }
}
