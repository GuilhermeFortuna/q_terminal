//! Coalesced change notification from the stream thread to Qt objects.
//!
//! A trade burst changes the feed hundreds of times a second; a panel needs to repaint a
//! few times. [`Coalescer`] runs its callback at most once per interval no matter how often
//! it is triggered, on its own thread, and stops when the callback says its target is gone.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

struct State {
    dirty: Mutex<bool>,
    wake: Condvar,
    alive: AtomicBool,
}

pub struct Coalescer {
    state: Arc<State>,
}

impl Coalescer {
    /// `run` returns false once its target no longer exists.
    pub fn spawn(interval: Duration, run: impl Fn() -> bool + Send + 'static) -> Self {
        let state = Arc::new(State {
            dirty: Mutex::new(false),
            wake: Condvar::new(),
            alive: AtomicBool::new(true),
        });
        let worker = state.clone();
        let _ = std::thread::Builder::new()
            .name("trade-notify".into())
            .spawn(move || loop {
                {
                    let mut dirty = worker.dirty.lock().unwrap_or_else(|e| e.into_inner());
                    while !*dirty {
                        if !worker.alive.load(Ordering::SeqCst) {
                            return;
                        }
                        dirty = worker
                            .wake
                            .wait_timeout(dirty, Duration::from_millis(500))
                            .unwrap_or_else(|e| e.into_inner())
                            .0;
                    }
                    *dirty = false;
                }
                if !run() {
                    worker.alive.store(false, Ordering::SeqCst);
                    return;
                }
                std::thread::sleep(interval);
            });
        Self { state }
    }

    /// Asks for a run. Returns false once the callback's target is gone.
    pub fn trigger(&self) -> bool {
        *self.state.dirty.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.state.wake.notify_one();
        self.state.alive.load(Ordering::SeqCst)
    }
}

impl Drop for Coalescer {
    fn drop(&mut self) {
        self.state.alive.store(false, Ordering::SeqCst);
        self.state.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn a_burst_of_triggers_runs_the_callback_far_fewer_times() {
        let runs = Arc::new(AtomicUsize::new(0));
        let r = runs.clone();
        let c = Coalescer::spawn(Duration::from_millis(40), move || {
            r.fetch_add(1, Ordering::SeqCst);
            true
        });
        for _ in 0..1000 {
            c.trigger();
        }
        std::thread::sleep(Duration::from_millis(150));
        let n = runs.load(Ordering::SeqCst);
        assert!((1..=5).contains(&n), "ran {n} times");
    }

    #[test]
    fn a_callback_that_reports_its_target_gone_stops_the_worker() {
        let c = Coalescer::spawn(Duration::from_millis(1), || false);
        c.trigger();
        std::thread::sleep(Duration::from_millis(60));
        assert!(!c.trigger());
    }
}
