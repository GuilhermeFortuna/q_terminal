use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::config::Config;
use crate::history::{load, source_label, trim_to_seam, ApiClient, Loaded, Source, VerifiedSet};

pub const DEFAULT_HISTORY_BARS: usize = 10_000;

#[derive(Debug, Clone)]
pub struct HistorySnapshot {
    pub loading: bool,
    pub progress: f64,
    pub source: String,
    pub dataset_id: String,
    pub published_at: String,
    pub error: String,
    pub bars: i64,
    pub shortfall: i64,
}

impl Default for HistorySnapshot {
    fn default() -> Self {
        Self {
            loading: false,
            progress: 0.0,
            source: source_label(Source::None).to_string(),
            dataset_id: String::new(),
            published_at: String::new(),
            error: String::new(),
            bars: 0,
            shortfall: 0,
        }
    }
}

pub struct HistoryController {
    loading: AtomicBool,
    gate_open: AtomicBool,
    first_streamed_time: Mutex<Option<i64>>,
    verify_cache: Mutex<VerifiedSet>,
    snapshot: Mutex<HistorySnapshot>,
    ran_off_ui_thread: AtomicBool,
}

impl Default for HistoryController {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryController {
    pub fn new() -> Self {
        Self {
            loading: AtomicBool::new(false),
            gate_open: AtomicBool::new(false),
            first_streamed_time: Mutex::new(None),
            verify_cache: Mutex::new(VerifiedSet::default()),
            snapshot: Mutex::new(HistorySnapshot::default()),
            ran_off_ui_thread: AtomicBool::new(false),
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::SeqCst)
    }

    pub fn gate_open(&self) -> bool {
        self.gate_open.load(Ordering::SeqCst)
    }

    pub fn open_gate(&self) {
        self.gate_open.store(true, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> HistorySnapshot {
        self.snapshot.lock().unwrap().clone()
    }

    pub fn ran_off_ui_thread(&self) -> bool {
        self.ran_off_ui_thread.load(Ordering::SeqCst)
    }

    pub fn record_stream_time(&self, time: i64) {
        let mut guard = self.first_streamed_time.lock().unwrap();
        if guard.is_none() {
            *guard = Some(time);
        }
    }

    pub fn begin_load(&self) -> Result<(), String> {
        if self
            .loading
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("history load already running".to_string());
        }

        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.loading = true;
        snapshot.progress = 0.0;
        snapshot.error.clear();
        Ok(())
    }

    pub fn set_progress(&self, progress: f64) {
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.progress = progress.clamp(0.0, 1.0);
    }

    pub fn finish_load(&self, loaded: Loaded, bar_count: usize) {
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.loading = false;
        snapshot.progress = 1.0;
        snapshot.source = source_label(loaded.source).to_string();
        snapshot.bars = bar_count as i64;
        snapshot.shortfall = loaded.shortfall as i64;
        snapshot.error = loaded.reason.clone().unwrap_or_default();
        if let Some(dataset) = &loaded.dataset {
            snapshot.dataset_id = dataset.id.clone();
            snapshot.published_at = dataset.published_at.clone();
        } else {
            snapshot.dataset_id.clear();
            snapshot.published_at.clear();
        }
        self.loading.store(false, Ordering::SeqCst);
        self.gate_open.store(true, Ordering::SeqCst);
    }

    pub fn fail_load(&self, reason: String) {
        let mut snapshot = self.snapshot.lock().unwrap();
        snapshot.loading = false;
        snapshot.progress = 1.0;
        snapshot.source = source_label(Source::None).to_string();
        snapshot.error = reason;
        self.loading.store(false, Ordering::SeqCst);
        self.gate_open.store(true, Ordering::SeqCst);
    }

    pub fn spawn_load(
        self: &Arc<Self>,
        config: Config,
        bars: usize,
        on_loaded: impl FnOnce(Loaded) + Send + 'static,
        on_progress: impl Fn(f64) + Send + Sync + 'static,
    ) {
        let controller = Arc::clone(self);
        thread::spawn(move || {
            controller.ran_off_ui_thread.store(true, Ordering::SeqCst);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("history runtime");

            rt.block_on(async move {
                on_progress(0.05);
                let api = ApiClient::new(&config.api_base);
                let loaded = load(&api, &config, bars, &controller.verify_cache).await;
                on_progress(0.9);
                let first_streamed = *controller.first_streamed_time.lock().unwrap();
                let trimmed = Loaded {
                    bars: trim_to_seam(loaded.bars, first_streamed),
                    source: loaded.source,
                    dataset: loaded.dataset,
                    shortfall: loaded.shortfall,
                    reason: loaded.reason,
                };
                on_progress(1.0);
                on_loaded(trimmed);
            });
        });
    }
}
