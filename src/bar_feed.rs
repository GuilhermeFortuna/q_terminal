use std::sync::Arc;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use q_buffers::frame::{BarColumns, TimeLabel};

use crate::config::Config;
use crate::history::{HistoryController, Loaded, DEFAULT_HISTORY_BARS};

#[derive(Debug, Clone)]
pub enum BarDelivery {
    Completed(BarColumns),
    Forming(BarColumns),
}

impl PartialEq for BarDelivery {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BarDelivery::Completed(a), BarDelivery::Completed(b))
            | (BarDelivery::Forming(a), BarDelivery::Forming(b)) => {
                a.time == b.time
                    && a.open
                        .iter()
                        .zip(&b.open)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.high
                        .iter()
                        .zip(&b.high)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.low
                        .iter()
                        .zip(&b.low)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.close
                        .iter()
                        .zip(&b.close)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
            }
            _ => false,
        }
    }
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, connection_state)]
        #[qproperty(QString, last_error)]
        #[qproperty(i64, data_age_ms)]
        #[qproperty(i64, applied)]
        #[qproperty(i64, dropped)]
        #[qproperty(i64, gaps_closed)]
        #[qproperty(i64, resnapshots)]
        #[qproperty(bool, history_loading)]
        #[qproperty(f64, history_progress)]
        #[qproperty(QString, history_source)]
        #[qproperty(QString, history_dataset_id)]
        #[qproperty(QString, history_published_at)]
        #[qproperty(QString, history_error)]
        #[qproperty(i64, history_bars)]
        #[qproperty(i64, history_shortfall)]
        #[qproperty(i64, timeframe_ms)]
        #[qproperty(bool, stale)]
        #[qproperty(bool, live_only)]
        #[qproperty(i64, rest_calls)]
        #[qproperty(QString, symbol)]
        #[qproperty(QString, timeframe)]
        #[qproperty(i64, bar_count)]
        #[qproperty(i64, revision)]
        #[qproperty(f64, last_price)]
        #[qproperty(i64, first_time)]
        #[qproperty(i64, last_time)]
        #[qproperty(f64, low)]
        #[qproperty(f64, high)]
        type BarFeed = super::BarFeedRust;

        #[qinvokable]
        fn load_history(self: Pin<&mut BarFeed>) -> bool;

        #[qinvokable]
        fn setup_config(
            self: Pin<&mut BarFeed>,
            api_base: QString,
            symbol: QString,
            timeframe: QString,
        );

        #[qinvokable]
        fn ingest_completed_bar(
            self: Pin<&mut BarFeed>,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        #[qinvokable]
        fn ingest_forming_bar(
            self: Pin<&mut BarFeed>,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        #[qinvokable]
        fn set_viewport(
            self: Pin<&mut BarFeed>,
            first_bar: i64,
            last_bar: i64,
            low: f64,
            high: f64,
        );

        #[qinvokable]
        fn set_surface(self: Pin<&mut BarFeed>, width_px: f32, height_px: f32);

        #[qinvokable]
        fn rebuild_geometry(self: Pin<&mut BarFeed>);

        #[qinvokable]
        fn vertex_ptr(self: &BarFeed) -> i64;

        #[qinvokable]
        fn vertex_len(self: &BarFeed) -> i64;

        #[qinvokable]
        fn geometry_revision(self: &BarFeed) -> i64;
    }

    impl cxx_qt::Threading for BarFeed {}
}

pub fn parse_timeframe_ms(timeframe: &str) -> i64 {
    let tf = timeframe.trim();
    if tf.is_empty() {
        return 60_000;
    }
    let first = tf.chars().next().unwrap();
    if first.is_alphabetic() && tf.len() > 1 && tf[1..].chars().all(|c| c.is_ascii_digit()) {
        let n: i64 = tf[1..].parse().unwrap_or(1);
        match first.to_ascii_uppercase() {
            'S' => return n * 1_000,
            'M' => return n * 60_000,
            'H' => return n * 3_600_000,
            'D' => return n * 86_400_000,
            'W' => return n * 604_800_000,
            _ => {}
        }
    }
    let last = tf.chars().last().unwrap();
    if last.is_alphabetic()
        && tf.len() > 1
        && tf[..tf.len() - 1].chars().all(|c| c.is_ascii_digit())
    {
        let n: i64 = tf[..tf.len() - 1].parse().unwrap_or(1);
        match last.to_ascii_lowercase() {
            's' => return n * 1_000,
            'm' => return n * 60_000,
            'h' => return n * 3_600_000,
            'd' => return n * 86_400_000,
            'w' => return n * 604_800_000,
            _ => {}
        }
    }
    60_000
}

pub struct BarFeedRust {
    pub connection_state: QString,
    pub last_error: QString,
    pub data_age_ms: i64,
    pub applied: i64,
    pub dropped: i64,
    pub gaps_closed: i64,
    pub resnapshots: i64,

    pub history_loading: bool,
    pub history_progress: f64,
    pub history_source: QString,
    pub history_dataset_id: QString,
    pub history_published_at: QString,
    pub history_error: QString,
    pub history_bars: i64,
    pub history_shortfall: i64,

    pub timeframe_ms: i64,
    pub stale: bool,
    pub live_only: bool,
    pub rest_calls: i64,

    pub symbol: QString,
    pub timeframe: QString,
    pub bar_count: i64,
    pub revision: i64,
    pub last_price: f64,
    pub first_time: i64,
    pub last_time: i64,
    pub low: f64,
    pub high: f64,

    pub series: q_qt::BarSeriesRust,
    pub last_applied_instant: Option<Instant>,
    history: Arc<HistoryController>,
    config: Option<Config>,
    history_bar_count: usize,
    pending_deliveries: Vec<BarDelivery>,
}

impl BarFeedRust {
    pub fn new(symbol: &str, timeframe: &str) -> Self {
        Self::with_config(symbol, timeframe, None)
    }

    pub fn with_config(symbol: &str, timeframe: &str, config: Option<Config>) -> Self {
        let tf_ms = parse_timeframe_ms(timeframe);
        let series = q_qt::BarSeriesRust::new(symbol, timeframe, 500_000);
        Self {
            connection_state: QString::from("connecting"),
            last_error: QString::from(""),
            data_age_ms: -1,
            applied: 0,
            dropped: 0,
            gaps_closed: 0,
            resnapshots: 0,
            history_loading: false,
            history_progress: 0.0,
            history_source: QString::from("none"),
            history_dataset_id: QString::from(""),
            history_published_at: QString::from(""),
            history_error: QString::from(""),
            history_bars: 0,
            history_shortfall: 0,
            timeframe_ms: tf_ms,
            stale: false,
            live_only: false,
            rest_calls: 0,
            symbol: QString::from(symbol),
            timeframe: QString::from(timeframe),
            bar_count: series.bar_count,
            revision: series.revision,
            last_price: series.last_price,
            first_time: series.first_time,
            last_time: series.last_time,
            low: series.low,
            high: series.high,
            series,
            last_applied_instant: None,
            history: Arc::new(HistoryController::new()),
            config,
            history_bar_count: DEFAULT_HISTORY_BARS,
            pending_deliveries: Vec::new(),
        }
    }

    pub fn history_controller(&self) -> &Arc<HistoryController> {
        &self.history
    }

    pub fn set_connection_state(&mut self, state: &str) {
        self.connection_state = QString::from(state);
    }

    pub fn set_last_error(&mut self, error: &str) {
        self.last_error = QString::from(error);
    }

    pub fn update_counters(
        &mut self,
        dropped: u64,
        gaps_closed: u64,
        resnapshots: u64,
        rest_calls: u64,
    ) {
        self.dropped = dropped as i64;
        self.gaps_closed = gaps_closed as i64;
        self.resnapshots = resnapshots as i64;
        self.rest_calls = rest_calls as i64;
    }

    pub fn inc_rest_calls(&mut self) {
        self.rest_calls += 1;
    }

    pub fn set_rest_calls(&mut self, count: i64) {
        self.rest_calls = count;
    }

    pub fn update_data_age(&mut self) {
        if let Some(inst) = self.last_applied_instant {
            self.data_age_ms = inst.elapsed().as_millis() as i64;
        } else {
            self.data_age_ms = -1;
        }
        self.stale = self.data_age_ms > self.timeframe_ms;
    }

    pub fn set_data_age_ms(&mut self, ms: i64) {
        self.data_age_ms = ms;
        self.stale = self.data_age_ms > self.timeframe_ms;
    }

    pub fn update_live_only(&mut self) {
        let source = self.history_source.to_string();
        let is_none = source == "none" || source.is_empty();
        self.live_only = is_none && self.history_bars == 0 && self.applied > 0;
    }

    pub fn sync_series_properties(&mut self) {
        self.symbol = self.series.symbol.clone();
        self.timeframe = self.series.timeframe.clone();
        self.bar_count = self.series.bar_count;
        self.revision = self.series.revision;
        self.last_price = self.series.last_price;
        self.first_time = self.series.first_time;
        self.last_time = self.series.last_time;
        self.low = self.series.low;
        self.high = self.series.high;
    }

    pub fn sync_history_properties(&mut self) {
        let snapshot = self.history.snapshot();
        self.history_loading = snapshot.loading;
        self.history_progress = snapshot.progress;
        self.history_source = QString::from(&snapshot.source);
        self.history_dataset_id = QString::from(&snapshot.dataset_id);
        self.history_published_at = QString::from(&snapshot.published_at);
        self.history_error = QString::from(&snapshot.error);
        self.history_bars = snapshot.bars;
        self.history_shortfall = snapshot.shortfall;
        self.update_live_only();
    }

    pub fn start_history_load(&mut self) -> Result<(), String> {
        let config = self
            .config
            .clone()
            .ok_or_else(|| "configuration missing".to_string())?;

        self.history.begin_load()?;
        self.sync_history_properties();

        let bars = self.history_bar_count;
        let controller = Arc::clone(&self.history);
        let progress_controller = Arc::clone(&self.history);
        controller.spawn_load(
            config,
            bars,
            move |loaded| {
                let _ = loaded;
            },
            move |progress| {
                progress_controller.set_progress(progress);
            },
        );
        Ok(())
    }

    pub fn start_history_load_with_handlers(
        &mut self,
        on_loaded: impl FnOnce(Loaded) + Send + 'static,
        on_progress: impl Fn(f64) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let config = self
            .config
            .clone()
            .ok_or_else(|| "configuration missing".to_string())?;

        self.history.begin_load()?;
        self.sync_history_properties();

        let controller = Arc::clone(&self.history);
        controller.spawn_load(config, self.history_bar_count, on_loaded, on_progress);
        Ok(())
    }

    pub fn complete_history_load(&mut self, loaded: Loaded) {
        let Loaded {
            bars,
            source,
            dataset,
            shortfall,
            reason,
        } = loaded;
        let bar_count = bars.time.len();
        if bar_count > 0 {
            let _ = self.series.load_history(bars);
        }
        self.history.finish_load(
            Loaded {
                bars: empty_bar_columns(),
                source,
                dataset,
                shortfall,
                reason,
            },
            bar_count,
        );
        self.sync_history_properties();
        self.sync_series_properties();
        self.update_live_only();

        let pending = std::mem::take(&mut self.pending_deliveries);
        for delivery in pending {
            self.apply_delivery_now(delivery);
        }
    }

    pub fn apply_delivery(&mut self, delivery: BarDelivery) {
        if let Some(time) = delivery_first_time(&delivery) {
            self.history.record_stream_time(time);
        }
        if !self.history.gate_open() {
            self.pending_deliveries.push(delivery);
            return;
        }
        self.apply_delivery_now(delivery);
    }

    fn apply_delivery_now(&mut self, delivery: BarDelivery) {
        match delivery {
            BarDelivery::Completed(bars) => {
                let count = bars.time.len() as i64;
                if self.series.ingest_completed(bars).is_ok() {
                    self.applied += count;
                    self.last_applied_instant = Some(Instant::now());
                    self.data_age_ms = 0;
                    self.stale = false;
                    self.sync_series_properties();
                    self.update_live_only();
                }
            }
            BarDelivery::Forming(bars) => {
                if self.series.ingest_forming(bars).is_ok() {
                    self.applied += 1;
                    self.last_applied_instant = Some(Instant::now());
                    self.data_age_ms = 0;
                    self.stale = false;
                    self.sync_series_properties();
                    self.update_live_only();
                }
            }
        }
    }

    pub fn apply_deliveries(&mut self, deliveries: impl IntoIterator<Item = BarDelivery>) {
        for delivery in deliveries {
            self.apply_delivery(delivery);
        }
    }
}

impl ffi::BarFeed {
    pub fn load_history(mut self: std::pin::Pin<&mut Self>) -> bool {
        let qt_thread = self.qt_thread();
        let config = match self.as_ref().rust().config.clone() {
            Some(config) => config,
            None => {
                self.as_mut().rust_mut().history_error = QString::from("configuration missing");
                return false;
            }
        };

        {
            let mut rust = self.as_mut().rust_mut();
            if rust.history.begin_load().is_err() {
                rust.history_error = QString::from("history load already running");
                return false;
            }
            rust.sync_history_properties();
        }

        let controller = Arc::clone(&self.as_ref().rust().history);
        let progress_controller = Arc::clone(&self.as_ref().rust().history);
        let bars = self.as_ref().rust().history_bar_count;
        let qt_for_loaded = qt_thread.clone();
        controller.spawn_load(
            config,
            bars,
            move |loaded| {
                qt_for_loaded
                    .queue(move |mut feed| {
                        feed.as_mut().rust_mut().complete_history_load(loaded);
                    })
                    .ok();
            },
            move |progress| {
                progress_controller.set_progress(progress);
                qt_thread
                    .queue(move |mut feed| {
                        feed.as_mut().rust_mut().sync_history_properties();
                    })
                    .ok();
            },
        );
        true
    }

    pub fn setup_config(
        mut self: std::pin::Pin<&mut Self>,
        api_base: QString,
        symbol: QString,
        timeframe: QString,
    ) {
        let tf_str = timeframe.to_string();
        let tf_ms = parse_timeframe_ms(&tf_str);
        let cfg = Config {
            api_base: api_base.to_string(),
            symbol: symbol.to_string(),
            timeframe: tf_str,
        };
        let mut rust = self.as_mut().rust_mut();
        rust.config = Some(cfg);
        rust.symbol = symbol;
        rust.timeframe = timeframe;
        rust.timeframe_ms = tf_ms;
    }

    pub fn ingest_completed_bar(
        mut self: std::pin::Pin<&mut Self>,
        time: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    ) {
        let col = make_bar_columns(time, open, high, low, close);
        self.as_mut()
            .rust_mut()
            .apply_delivery(BarDelivery::Completed(col));
    }

    pub fn ingest_forming_bar(
        mut self: std::pin::Pin<&mut Self>,
        time: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    ) {
        let col = make_bar_columns(time, open, high, low, close);
        self.as_mut()
            .rust_mut()
            .apply_delivery(BarDelivery::Forming(col));
    }

    pub fn set_viewport(
        mut self: std::pin::Pin<&mut Self>,
        first_bar: i64,
        last_bar: i64,
        low: f64,
        high: f64,
    ) {
        self.as_mut()
            .rust_mut()
            .series
            .set_viewport(first_bar, last_bar, low, high);
    }

    pub fn set_surface(mut self: std::pin::Pin<&mut Self>, width_px: f32, height_px: f32) {
        self.as_mut()
            .rust_mut()
            .series
            .set_surface(width_px, height_px);
    }

    pub fn rebuild_geometry(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().series.rebuild_geometry();
    }

    pub fn vertex_ptr(&self) -> i64 {
        self.rust().series.vertex_ptr() as usize as i64
    }

    pub fn vertex_len(&self) -> i64 {
        self.rust().series.vertex_len() as i64
    }

    pub fn geometry_revision(&self) -> i64 {
        self.rust().series.geometry_revision()
    }
}

impl Default for BarFeedRust {
    fn default() -> Self {
        Self::new("DEFAULT", "1m")
    }
}

fn delivery_first_time(delivery: &BarDelivery) -> Option<i64> {
    match delivery {
        BarDelivery::Completed(bars) | BarDelivery::Forming(bars) => bars.time.first().copied(),
    }
}

pub fn make_bar_columns(t: i64, open: f64, high: f64, low: f64, close: f64) -> BarColumns {
    BarColumns {
        time: vec![t],
        open: vec![open],
        high: vec![high],
        low: vec![low],
        close: vec![close],
        tick_volume: None,
        spread: None,
        real_volume: None,
        label: TimeLabel::Utc,
    }
}

pub fn empty_bar_columns() -> BarColumns {
    BarColumns {
        time: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        tick_volume: None,
        spread: None,
        real_volume: None,
        label: TimeLabel::Utc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{source_label, Source};
    use std::sync::Mutex;
    use std::time::Duration;

    fn test_bar(t: i64, close: f64) -> BarColumns {
        make_bar_columns(t, 10.0, close + 1.0, 9.0, close)
    }

    fn test_config(api_base: &str) -> Config {
        Config {
            api_base: api_base.to_string(),
            symbol: "PETR4".to_string(),
            timeframe: "1m".to_string(),
        }
    }

    #[test]
    fn test_bar_feed_initial_state() {
        let feed = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed.connection_state.to_string(), "connecting");
        assert_eq!(feed.last_error.to_string(), "");
        assert_eq!(feed.data_age_ms, -1);
        assert_eq!(feed.applied, 0);
        assert_eq!(feed.dropped, 0);
        assert_eq!(feed.gaps_closed, 0);
        assert_eq!(feed.resnapshots, 0);
        assert_eq!(feed.series.bar_count, 0);
        assert_eq!(feed.series.revision, 0);
        assert_eq!(feed.series.symbol.to_string(), "PETR4");
        assert_eq!(feed.series.timeframe.to_string(), "1m");
        assert_eq!(feed.history_source.to_string(), "none");
        assert!(!feed.history_loading);
    }

    #[test]
    fn test_bar_feed_drain_completed_advances_series_and_counters() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        let deliveries: Vec<_> = (1..=5)
            .map(|i| BarDelivery::Completed(test_bar(i * 60, 10.0 + i as f64)))
            .collect();

        feed.history.open_gate();
        feed.apply_deliveries(deliveries);

        assert_eq!(feed.applied, 5);
        assert_eq!(feed.series.bar_count, 5);
        assert_eq!(feed.series.revision, 5);
        assert_eq!(feed.series.last_time, 300);
        assert_eq!(feed.series.last_price.to_bits(), 15.0f64.to_bits());
        assert_eq!(feed.data_age_ms, 0);
    }

    #[test]
    fn test_bar_feed_drain_forming_advances_series() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        feed.history.open_gate();

        feed.apply_delivery(BarDelivery::Completed(test_bar(60, 10.0)));
        assert_eq!(feed.series.bar_count, 1);
        assert_eq!(feed.series.revision, 1);
        assert!(!feed.series.has_forming);

        feed.apply_delivery(BarDelivery::Forming(test_bar(120, 10.5)));
        assert_eq!(feed.applied, 2);
        assert_eq!(feed.series.bar_count, 1);
        assert!(feed.series.has_forming);
        assert_eq!(feed.series.revision, 2);
        assert_eq!(feed.series.last_price.to_bits(), 10.5f64.to_bits());
    }

    #[test]
    fn test_bar_feed_forming_coalescing_and_completed_in_order() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        feed.history.open_gate();

        let deliveries = vec![
            BarDelivery::Completed(test_bar(60, 10.0)),
            BarDelivery::Forming(test_bar(120, 19.99)),
        ];

        feed.apply_deliveries(deliveries);

        assert_eq!(feed.applied, 2);
        assert_eq!(feed.series.bar_count, 1);
        assert!(feed.series.has_forming);
        assert_eq!(feed.series.revision, 2);
        assert_eq!(feed.series.last_price.to_bits(), 19.99f64.to_bits());
    }

    #[test]
    fn test_bar_feed_setters_and_counters() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        feed.set_connection_state("live");
        assert_eq!(feed.connection_state.to_string(), "live");

        feed.set_last_error("stream_unavailable");
        assert_eq!(feed.last_error.to_string(), "stream_unavailable");

        feed.update_counters(7, 3, 2, 4);
        assert_eq!(feed.dropped, 7);
        assert_eq!(feed.gaps_closed, 3);
        assert_eq!(feed.resnapshots, 2);
        assert_eq!(feed.rest_calls, 4);
    }

    #[test]
    fn test_bar_feed_update_data_age() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed.data_age_ms, -1);
        feed.update_data_age();
        assert_eq!(feed.data_age_ms, -1);

        feed.history.open_gate();
        feed.apply_delivery(BarDelivery::Completed(test_bar(60, 10.0)));
        assert_eq!(feed.data_age_ms, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        feed.update_data_age();
        assert!(feed.data_age_ms >= 4);
    }

    #[test]
    fn test_history_load_refuses_concurrent_request() {
        let mut feed =
            BarFeedRust::with_config("PETR4", "1m", Some(test_config("http://127.0.0.1:1")));
        feed.history.begin_load().unwrap();
        let err = feed.start_history_load().unwrap_err();
        assert!(err.contains("already running"));
    }

    #[test]
    fn test_history_load_runs_off_ui_thread() {
        let mut feed =
            BarFeedRust::with_config("PETR4", "1m", Some(test_config("http://127.0.0.1:1")));
        feed.start_history_load().unwrap();
        for _ in 0..100 {
            if feed.history_controller().ran_off_ui_thread() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("history load did not run off the UI thread");
    }

    #[test]
    fn test_history_holds_stream_until_loaded() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        feed.apply_delivery(BarDelivery::Completed(test_bar(300, 12.0)));
        assert_eq!(feed.series.bar_count, 0);
        assert_eq!(feed.applied, 0);

        feed.complete_history_load(Loaded {
            bars: make_bar_columns(60, 10.0, 11.0, 9.0, 10.5),
            source: Source::Api,
            dataset: None,
            shortfall: 0,
            reason: None,
        });

        assert_eq!(feed.series.bar_count, 2);
        assert_eq!(feed.series.last_time, 300);
        assert_eq!(feed.applied, 1);
    }

    #[test]
    fn test_history_progress_is_monotonic() {
        let mut feed =
            BarFeedRust::with_config("PETR4", "1m", Some(test_config("http://127.0.0.1:1")));
        let progress = Arc::new(Mutex::new(Vec::new()));
        let progress_capture = Arc::clone(&progress);
        let controller = Arc::clone(feed.history_controller());
        feed.start_history_load_with_handlers(
            move |_| {},
            move |value| {
                progress_capture.lock().unwrap().push(value);
                controller.set_progress(value);
            },
        )
        .unwrap();

        for _ in 0..200 {
            feed.sync_history_properties();
            if (feed.history_progress - 1.0).abs() < f64::EPSILON {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        let values = progress.lock().unwrap().clone();
        assert!(!values.is_empty());
        for window in values.windows(2) {
            assert!(window[0] <= window[1]);
        }
        assert!((values.last().copied().unwrap_or(0.0) - 1.0).abs() < f64::EPSILON);
        assert_eq!(feed.history_source.to_string(), source_label(Source::None));
    }

    #[test]
    fn test_bar_feed_staleness_flips_at_timeframe_interval() {
        let mut feed_1m = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed_1m.timeframe_ms, 60_000);
        assert_eq!(feed_1m.data_age_ms, -1);
        assert!(!feed_1m.stale);

        // Age <= timeframe_ms is not stale
        feed_1m.set_data_age_ms(59_999);
        assert!(!feed_1m.stale);
        feed_1m.set_data_age_ms(60_000);
        assert!(!feed_1m.stale);

        // Age > timeframe_ms flips to stale
        feed_1m.set_data_age_ms(60_001);
        assert!(feed_1m.stale);

        // Arriving bar resets staleness
        feed_1m.history.open_gate();
        feed_1m.apply_delivery(BarDelivery::Completed(test_bar(60, 10.0)));
        assert_eq!(feed_1m.data_age_ms, 0);
        assert!(!feed_1m.stale);

        // Test other timeframes: 5m, 1h, 1d, D1
        let mut feed_5m = BarFeedRust::new("VALE3", "5m");
        assert_eq!(feed_5m.timeframe_ms, 300_000);
        feed_5m.set_data_age_ms(300_000);
        assert!(!feed_5m.stale);
        feed_5m.set_data_age_ms(300_001);
        assert!(feed_5m.stale);

        let feed_1h = BarFeedRust::new("VALE3", "1h");
        assert_eq!(feed_1h.timeframe_ms, 3_600_000);

        let feed_1d = BarFeedRust::new("VALE3", "1d");
        assert_eq!(feed_1d.timeframe_ms, 86_400_000);

        let feed_d1 = BarFeedRust::new("VALE3", "D1");
        assert_eq!(feed_d1.timeframe_ms, 86_400_000);
    }

    #[test]
    fn test_bar_feed_live_only_transitions() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        assert!(!feed.live_only);

        // Case 1: History loaded with bars -> live_only stays false even when bars arrive
        feed.complete_history_load(Loaded {
            bars: make_bar_columns(60, 10.0, 11.0, 9.0, 10.5),
            source: Source::Lake,
            dataset: None,
            shortfall: 0,
            reason: None,
        });
        assert_eq!(feed.history_bars, 1);
        assert_eq!(feed.history_source.to_string(), "lake");
        assert!(!feed.live_only);

        feed.apply_delivery(BarDelivery::Completed(test_bar(120, 11.0)));
        assert_eq!(feed.applied, 1);
        assert!(!feed.live_only);

        // Case 2: History produced no bars / source is none -> live_only is true once bars arrive
        let mut live_feed = BarFeedRust::new("PETR4", "1m");
        assert!(!live_feed.live_only);

        live_feed.complete_history_load(Loaded {
            bars: empty_bar_columns(),
            source: Source::None,
            dataset: None,
            shortfall: 0,
            reason: None,
        });
        assert_eq!(live_feed.history_bars, 0);
        assert_eq!(live_feed.history_source.to_string(), "none");
        // No arriving bars yet -> live_only is false
        assert!(!live_feed.live_only);

        // Once a live bar arrives -> live_only becomes true
        live_feed.apply_delivery(BarDelivery::Completed(test_bar(180, 12.0)));
        assert_eq!(live_feed.applied, 1);
        assert!(live_feed.live_only);
    }

    #[test]
    fn test_bar_feed_rest_calls_tracks_protocol_calls() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed.rest_calls, 0);

        feed.inc_rest_calls();
        assert_eq!(feed.rest_calls, 1);

        feed.set_rest_calls(10);
        assert_eq!(feed.rest_calls, 10);

        feed.update_counters(2, 3, 4, 12);
        assert_eq!(feed.rest_calls, 12);
    }
}
