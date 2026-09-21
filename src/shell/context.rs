//! Named selection context (Q-052). Global by default, so selecting a deployment in one
//! window retargets every other; a window may detach and hold its own selection.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionContext {
    global: String,
    /// Windows that detached, with the selection each one holds.
    detached: HashMap<String, String>,
}

impl SelectionContext {
    pub fn global(&self) -> &str {
        &self.global
    }

    pub fn is_detached(&self, window: &str) -> bool {
        self.detached.contains_key(window)
    }

    /// What `window` shows: its own selection when detached, the global one otherwise.
    pub fn effective(&self, window: &str) -> &str {
        self.detached
            .get(window)
            .map(String::as_str)
            .unwrap_or(&self.global)
    }

    /// Selects in `window`. A detached window keeps the change to itself; an attached one
    /// changes the global selection. Returns whether the global selection changed.
    pub fn select(&mut self, window: &str, deployment: &str) -> bool {
        if let Some(own) = self.detached.get_mut(window) {
            *own = deployment.to_string();
            return false;
        }
        let changed = self.global != deployment;
        self.global = deployment.to_string();
        changed
    }

    /// Detaches `window`, starting from what it currently shows.
    pub fn detach(&mut self, window: &str) {
        let current = self.global.clone();
        self.detached.entry(window.to_string()).or_insert(current);
    }

    /// Reattaches `window`; it adopts the global selection.
    pub fn attach(&mut self, window: &str) {
        self.detached.remove(window);
    }

    /// Forget a closed window.
    pub fn forget(&mut self, window: &str) {
        self.detached.remove(window);
    }

    pub fn restore(&mut self, global: &str, detached: &HashMap<String, String>) {
        self.global = global.to_string();
        self.detached = detached.clone();
    }

    pub fn detached_map(&self) -> &HashMap<String, String> {
        &self.detached
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_windows_share_the_global_selection() {
        let mut ctx = SelectionContext::default();
        assert!(ctx.select("w1", "dep-a"));
        assert_eq!(ctx.effective("w1"), "dep-a");
        assert_eq!(ctx.effective("w2"), "dep-a");
    }

    #[test]
    fn detached_window_is_independent_and_global_is_unaffected() {
        let mut ctx = SelectionContext::default();
        ctx.select("w1", "dep-a");
        ctx.detach("w2");
        assert!(ctx.is_detached("w2"));
        assert_eq!(ctx.effective("w2"), "dep-a");

        assert!(!ctx.select("w2", "dep-b"));
        assert_eq!(ctx.effective("w2"), "dep-b");
        assert_eq!(ctx.global(), "dep-a");
        assert_eq!(ctx.effective("w1"), "dep-a");

        ctx.select("w1", "dep-c");
        assert_eq!(ctx.effective("w2"), "dep-b");
    }

    #[test]
    fn reattach_adopts_the_global_selection() {
        let mut ctx = SelectionContext::default();
        ctx.select("w1", "dep-a");
        ctx.detach("w2");
        ctx.select("w2", "dep-b");
        ctx.attach("w2");
        assert!(!ctx.is_detached("w2"));
        assert_eq!(ctx.effective("w2"), "dep-a");
    }
}
