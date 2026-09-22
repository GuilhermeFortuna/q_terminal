use std::sync::Arc;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use q_buffers::frame::{BarColumns, TimeLabel, VolumeSet};

use crate::config::Config;
use crate::contracts_stream::{ExecutionDecisionState, ExecutionFillEvent};
use crate::execution::markers::{self, Marker};
use crate::execution::overlays::{self, Hit, Layer, OverlaySeries, View};
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
        #[qproperty(bool, has_forming)]
        #[qproperty(i64, revision)]
        #[qproperty(f64, last_price)]
        #[qproperty(i64, first_time)]
        #[qproperty(i64, last_time)]
        #[qproperty(f64, low)]
        #[qproperty(f64, high)]
        #[qproperty(i64, overlay_revision)]
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
        fn ingest_completed_bar_gen(
            self: Pin<&mut BarFeed>,
            generation: i64,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        #[qinvokable]
        fn ingest_forming_bar_gen(
            self: Pin<&mut BarFeed>,
            generation: i64,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        /// Points the feed at another symbol and timeframe: drops the previous target's
        /// bars, history load and pending deliveries, and loads the new target's history.
        /// Anything stamped with an older generation is dropped from then on.
        #[qinvokable]
        fn retarget(
            self: Pin<&mut BarFeed>,
            symbol: QString,
            timeframe: QString,
            generation: i64,
        ) -> bool;

        /// Replaces the selected deployment's decisions and fills (JSON arrays of the stream
        /// contract's states). Markers are placed on the bars at the next rebuild.
        #[qinvokable]
        fn set_execution_rows(
            self: Pin<&mut BarFeed>,
            decisions_json: QString,
            fills_json: QString,
        );

        /// Sets the overlays from a deployment chart response, unless it was requested for
        /// an older target.
        #[qinvokable]
        fn set_overlays_json(self: Pin<&mut BarFeed>, generation: i64, json: QString) -> bool;

        #[qinvokable]
        fn rebuild_overlays(
            self: Pin<&mut BarFeed>,
            first_bar: i64,
            last_bar: i64,
            low: f64,
            high: f64,
            width_px: f32,
            height_px: f32,
        );

        #[qinvokable]
        fn overlay_layer_count(self: &BarFeed) -> i32;

        #[qinvokable]
        fn overlay_layer_ptr(self: &BarFeed, layer: i32) -> i64;

        #[qinvokable]
        fn overlay_layer_len(self: &BarFeed, layer: i32) -> i64;

        #[qinvokable]
        fn overlay_layer_mode(self: &BarFeed, layer: i32) -> i32;

        #[qinvokable]
        fn overlay_layer_color(self: &BarFeed, layer: i32) -> i64;

        #[qinvokable]
        fn marker_count(self: &BarFeed) -> i32;

        /// Detail text of the marker drawn within `radius` px of (x, y), or empty.
        #[qinvokable]
        fn marker_detail_at(self: &BarFeed, x: f32, y: f32, radius: f32) -> QString;

        /// Fills the feed with synthetic bars, markers and overlays for the frame benchmark.
        #[qinvokable]
        fn bench_populate(self: Pin<&mut BarFeed>, bars: i64, markers: i64, overlays: i64);

        #[qinvokable]
        fn target_generation(self: &BarFeed) -> i64;

        #[qinvokable]
        fn stale_dropped(self: &BarFeed) -> i64;

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
        fn bar_times_len(self: &BarFeed) -> i32;

        #[qinvokable]
        fn bar_time_at(self: &BarFeed, index: i32) -> i64;

        #[qinvokable]
        fn visible_range_json(self: &BarFeed, first_bar: i32, last_bar: i32) -> QString;

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
    pub has_forming: bool,
    pub revision: i64,
    pub last_price: f64,
    pub first_time: i64,
    pub last_time: i64,
    pub low: f64,
    pub high: f64,
    pub overlay_revision: i64,

    pub bar_times: Vec<i64>,
    pub series: q_qt::BarSeriesRust,
    pub series_label: TimeLabel,
    pub series_vols: VolumeSet,
    pub last_applied_instant: Option<Instant>,
    history: Arc<HistoryController>,
    config: Option<Config>,
    history_bar_count: usize,
    pending_deliveries: Vec<BarDelivery>,
    /// Chart target generation. Bars and history results stamped with another one belong
    /// to a previous target and are dropped.
    /// High, low and close of each completed bar, indexed like `bar_times`.
    pub bar_hlc: Vec<(f64, f64, f64)>,
    /// The forming bar is not part of `bar_hlc` until it completes.
    pub forming: Option<(i64, f64, f64, f64, f64)>,
    decisions: Vec<ExecutionDecisionState>,
    fills: Vec<ExecutionFillEvent>,
    overlay_series: Vec<OverlaySeries>,
    markers: Vec<Marker>,
    /// What the cached markers were placed against: bar count, first bar time, rows revision.
    markers_key: Option<(usize, i64, u64)>,
    rows_rev: u64,
    layers: Vec<Layer>,
    hits: Vec<Hit>,
    /// Called on the Qt thread with the target generation after each completed bar is
    /// applied to the chart. The overlay fetcher hangs off it.
    pub on_completed: Option<Arc<dyn Fn(u64) + Send + Sync>>,
    pub generation: u64,
    /// Deliveries and history results dropped for carrying an older generation.
    pub stale_dropped: u64,
}

/// The properties QML reads, captured so a change can be announced (Q-049).
#[derive(Clone, PartialEq)]
struct PropSnapshot {
    symbol: QString,
    timeframe: QString,
    timeframe_ms: i64,
    bar_count: i64,
    has_forming: bool,
    revision: i64,
    last_price: f64,
    first_time: i64,
    last_time: i64,
    low: f64,
    high: f64,
    applied: i64,
    data_age_ms: i64,
    stale: bool,
    live_only: bool,
    history_loading: bool,
    history_progress: f64,
    history_source: QString,
    history_dataset_id: QString,
    history_published_at: QString,
    history_error: QString,
    history_bars: i64,
    history_shortfall: i64,
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
            has_forming: series.has_forming,
            revision: series.revision,
            last_price: series.last_price,
            first_time: series.first_time,
            last_time: series.last_time,
            low: series.low,
            high: series.high,
            bar_times: Vec::new(),
            series,
            series_label: TimeLabel::Utc,
            series_vols: VolumeSet {
                tick_volume: false,
                spread: false,
                real_volume: false,
            },
            last_applied_instant: None,
            history: Arc::new(HistoryController::new()),
            config,
            history_bar_count: DEFAULT_HISTORY_BARS,
            pending_deliveries: Vec::new(),
            generation: 0,
            stale_dropped: 0,
            overlay_revision: 0,
            bar_hlc: Vec::new(),
            forming: None,
            decisions: Vec::new(),
            fills: Vec::new(),
            overlay_series: Vec::new(),
            markers: Vec::new(),
            markers_key: None,
            rows_rev: 0,
            layers: Vec::new(),
            hits: Vec::new(),
            on_completed: None,
        }
    }

    fn snapshot_props(&self) -> PropSnapshot {
        PropSnapshot {
            symbol: self.symbol.clone(),
            timeframe: self.timeframe.clone(),
            timeframe_ms: self.timeframe_ms,
            bar_count: self.bar_count,
            has_forming: self.has_forming,
            revision: self.revision,
            last_price: self.last_price,
            first_time: self.first_time,
            last_time: self.last_time,
            low: self.low,
            high: self.high,
            applied: self.applied,
            data_age_ms: self.data_age_ms,
            stale: self.stale,
            live_only: self.live_only,
            history_loading: self.history_loading,
            history_progress: self.history_progress,
            history_source: self.history_source.clone(),
            history_dataset_id: self.history_dataset_id.clone(),
            history_published_at: self.history_published_at.clone(),
            history_error: self.history_error.clone(),
            history_bars: self.history_bars,
            history_shortfall: self.history_shortfall,
        }
    }

    fn restore_props(&mut self, p: PropSnapshot) {
        self.symbol = p.symbol;
        self.timeframe = p.timeframe;
        self.timeframe_ms = p.timeframe_ms;
        self.bar_count = p.bar_count;
        self.has_forming = p.has_forming;
        self.revision = p.revision;
        self.last_price = p.last_price;
        self.first_time = p.first_time;
        self.last_time = p.last_time;
        self.low = p.low;
        self.high = p.high;
        self.applied = p.applied;
        self.data_age_ms = p.data_age_ms;
        self.stale = p.stale;
        self.live_only = p.live_only;
        self.history_loading = p.history_loading;
        self.history_progress = p.history_progress;
        self.history_source = p.history_source;
        self.history_dataset_id = p.history_dataset_id;
        self.history_published_at = p.history_published_at;
        self.history_error = p.history_error;
        self.history_bars = p.history_bars;
        self.history_shortfall = p.history_shortfall;
    }

    /// Switches the feed to a new symbol and timeframe (Q-049). Everything of the previous
    /// target goes: its bars, its history controller (a load still running finishes into
    /// the old one and is dropped by generation), and deliveries waiting for the gate.
    pub fn reset_for_target(&mut self, symbol: &str, timeframe: &str, generation: u64) {
        self.generation = generation;
        self.series = q_qt::BarSeriesRust::new(symbol, timeframe, 500_000);
        self.series_label = TimeLabel::Utc;
        self.series_vols = VolumeSet {
            tick_volume: false,
            spread: false,
            real_volume: false,
        };
        self.bar_times.clear();
        self.bar_hlc.clear();
        self.forming = None;
        self.overlay_series.clear();
        self.markers.clear();
        self.markers_key = None;
        self.layers.clear();
        self.hits.clear();
        self.pending_deliveries.clear();
        self.history = Arc::new(HistoryController::new());
        self.applied = 0;
        self.last_applied_instant = None;
        self.data_age_ms = -1;
        self.stale = false;
        self.timeframe_ms = parse_timeframe_ms(timeframe);
        if let Some(cfg) = self.config.as_mut() {
            cfg.symbol = symbol.to_string();
            cfg.timeframe = timeframe.to_string();
        }
        self.sync_series_properties();
        self.sync_history_properties();
    }

    /// Bar opens in milliseconds, ascending, whatever unit the feed's times use.
    fn bar_opens_ms(&self) -> Vec<i64> {
        self.bar_times
            .iter()
            .map(|t| markers::normalize_ms(*t))
            .collect()
    }

    fn refresh_markers(&mut self) {
        let key = (
            self.bar_times.len(),
            self.bar_times.first().copied().unwrap_or(0),
            self.rows_rev,
        );
        if self.markers_key == Some(key) {
            return;
        }
        let opens = self.bar_opens_ms();
        let tf = self.timeframe_ms;
        let mut m = markers::decision_markers(&self.decisions, &opens, tf);
        m.extend(markers::fill_markers(&self.fills, &opens));
        self.markers = m;
        self.markers_key = Some(key);
    }

    /// Packs markers and overlay lines for the view into `layers` and `hits`.
    pub fn build_overlays(&mut self, view: View) {
        self.refresh_markers();
        let opens = self.bar_opens_ms();
        let mut layers = overlays::line_layers(&self.overlay_series, &opens, view);
        let (marker_layers, hits) = overlays::marker_layers(&self.markers, &self.bar_hlc, view);
        layers.extend(marker_layers);
        self.layers = layers;
        self.hits = hits;
    }

    /// Synthetic bars, `markers` markers spread over them and `overlays` overlay lines.
    pub fn populate_bench(&mut self, bars: usize, markers: usize, overlays: usize) {
        let t0 = 1_789_725_600_000i64;
        self.bar_times = (0..bars as i64)
            .map(|i| (t0 + i * 60_000) * 1_000)
            .collect();
        let close = |i: usize| {
            (i as f64 * 0.01)
                .sin()
                .mul_add(5.0, 100.0 + ((i * 7) % 13) as f64)
        };
        self.bar_hlc = (0..bars)
            .map(|i| (close(i) + 1.0, close(i) - 1.0, close(i)))
            .collect();
        let kinds = [
            markers::MarkerKind::Buy,
            markers::MarkerKind::Sell,
            markers::MarkerKind::Close,
            markers::MarkerKind::Fill,
        ];
        self.markers = (0..markers)
            .map(|n| {
                let i = (n * bars)
                    .checked_div(markers)
                    .unwrap_or(0)
                    .min(bars.saturating_sub(1));
                Marker {
                    id: format!("bench-{n}"),
                    bar_index: i,
                    bar_open_ms: t0 + i as i64 * 60_000,
                    price: Some(close(i)),
                    kind: kinds[n % 4],
                    label: String::new(),
                    detail: format!("bench marker {n}"),
                }
            })
            .collect();
        self.markers_key = Some((
            self.bar_times.len(),
            self.bar_times.first().copied().unwrap_or(0),
            self.rows_rev,
        ));
        self.overlay_series = (0..overlays)
            .map(|k| OverlaySeries {
                key: format!("bench-{k}"),
                label: format!("bench {k}"),
                pane: if k % 2 == 0 {
                    overlays::Pane::Price
                } else {
                    overlays::Pane::Oscillator
                },
                rgba: 0x2962ffff,
                points: (0..bars)
                    .map(|i| (t0 + i as i64 * 60_000, Some(close(i) + k as f64)))
                    .collect(),
            })
            .collect();
        let lo = self.bar_hlc.iter().map(|h| h.1).fold(f64::MAX, f64::min);
        let hi = self.bar_hlc.iter().map(|h| h.0).fold(f64::MIN, f64::max);
        self.low = lo;
        self.high = hi;
        self.bar_count = bars as i64;
        self.overlay_revision += 1;
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    pub fn set_rows(
        &mut self,
        decisions: Vec<ExecutionDecisionState>,
        fills: Vec<ExecutionFillEvent>,
    ) {
        self.decisions = decisions;
        self.fills = fills;
        self.rows_rev += 1;
        self.overlay_revision += 1;
    }

    /// Sets the overlays unless they were requested for a previous target.
    pub fn set_overlays(&mut self, generation: u64, series: Vec<OverlaySeries>) -> bool {
        if generation != self.generation {
            self.stale_dropped += 1;
            return false;
        }
        self.overlay_series = series;
        self.overlay_revision += 1;
        true
    }

    pub fn overlay_series(&self) -> &[OverlaySeries] {
        &self.overlay_series
    }

    /// Applies a delivery stamped with the generation it was produced for. Returns false,
    /// and counts it, when it belongs to a previous target.
    pub fn apply_stamped(&mut self, generation: u64, delivery: BarDelivery) -> bool {
        if generation != self.generation {
            self.stale_dropped += 1;
            return false;
        }
        self.apply_delivery(delivery);
        true
    }

    /// Completes a history load requested for `generation`. A result for a previous target
    /// is dropped and counted.
    pub fn complete_history_load_for(&mut self, generation: u64, loaded: Loaded) -> bool {
        if generation != self.generation {
            self.stale_dropped += 1;
            return false;
        }
        self.complete_history_load(loaded);
        true
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
        self.has_forming = self.series.has_forming;
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
            self.series_label = bars.label;
            self.series_vols = VolumeSet::from_bars(&bars);
            self.bar_times.extend_from_slice(&bars.time);
            self.bar_hlc
                .extend((0..bar_count).map(|i| (bars.high[i], bars.low[i], bars.close[i])));
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

    fn adapt_bars(&self, bars: BarColumns) -> BarColumns {
        let c = bars.time.len();
        let tick_volume = if self.series_vols.tick_volume {
            bars.tick_volume.or_else(|| Some(vec![0; c]))
        } else {
            None
        };
        let spread = if self.series_vols.spread {
            bars.spread.or_else(|| Some(vec![0; c]))
        } else {
            None
        };
        let real_volume = if self.series_vols.real_volume {
            bars.real_volume.or_else(|| Some(vec![0; c]))
        } else {
            None
        };
        BarColumns {
            time: bars.time,
            open: bars.open,
            high: bars.high,
            low: bars.low,
            close: bars.close,
            tick_volume,
            spread,
            real_volume,
            label: self.series_label,
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
        let completed = matches!(delivery, BarDelivery::Completed(_));
        self.apply_delivery_now(delivery);
        if completed {
            if let Some(hook) = &self.on_completed {
                hook(self.generation);
            }
        }
    }

    fn apply_delivery_now(&mut self, delivery: BarDelivery) {
        match delivery {
            BarDelivery::Completed(bars) => {
                self.forming = None;
                let count = bars.time.len() as i64;
                let adapted = self.adapt_bars(bars);
                let times = adapted.time.clone();
                let (highs, lows, closes) = (
                    adapted.high.clone(),
                    adapted.low.clone(),
                    adapted.close.clone(),
                );
                if self.series.ingest_completed(adapted).is_ok() {
                    for (i, t) in times.into_iter().enumerate() {
                        let hlc = (highs[i], lows[i], closes[i]);
                        match self.bar_times.last().copied() {
                            Some(last) if t > last => {
                                self.bar_times.push(t);
                                self.bar_hlc.push(hlc);
                            }
                            Some(last) if t == last => {
                                *self.bar_times.last_mut().unwrap() = t;
                                *self.bar_hlc.last_mut().unwrap() = hlc;
                            }
                            Some(_) => {}
                            None => {
                                self.bar_times.push(t);
                                self.bar_hlc.push(hlc);
                            }
                        }
                    }
                    self.applied += count;
                    self.last_applied_instant = Some(Instant::now());
                    self.data_age_ms = 0;
                    self.stale = false;
                    self.sync_series_properties();
                    self.update_live_only();
                }
            }
            BarDelivery::Forming(bars) => {
                if let (Some(&time), Some(&open), Some(&high), Some(&low), Some(&close)) = (
                    bars.time.first(),
                    bars.open.first(),
                    bars.high.first(),
                    bars.low.first(),
                    bars.close.first(),
                ) {
                    self.forming = Some((time, open, high, low, close));
                }
                let adapted = self.adapt_bars(bars);
                if self.series.ingest_forming(adapted).is_ok() {
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
    /// Announces what changed since `before`. Rust-side updates assign the fields directly,
    /// which raises no signal, so the fields are put back and set through the property
    /// setters, which do.
    fn notify_props(mut self: std::pin::Pin<&mut Self>, before: PropSnapshot) {
        let after = self.as_ref().rust().snapshot_props();
        if after == before {
            return;
        }
        self.as_mut().rust_mut().restore_props(before);
        self.as_mut().set_symbol(after.symbol);
        self.as_mut().set_timeframe(after.timeframe);
        self.as_mut().set_timeframe_ms(after.timeframe_ms);
        self.as_mut().set_bar_count(after.bar_count);
        self.as_mut().set_has_forming(after.has_forming);
        self.as_mut().set_last_price(after.last_price);
        self.as_mut().set_first_time(after.first_time);
        self.as_mut().set_last_time(after.last_time);
        self.as_mut().set_low(after.low);
        self.as_mut().set_high(after.high);
        self.as_mut().set_applied(after.applied);
        self.as_mut().set_data_age_ms(after.data_age_ms);
        self.as_mut().set_stale(after.stale);
        self.as_mut().set_live_only(after.live_only);
        self.as_mut().set_history_loading(after.history_loading);
        self.as_mut().set_history_progress(after.history_progress);
        self.as_mut().set_history_source(after.history_source);
        self.as_mut()
            .set_history_dataset_id(after.history_dataset_id);
        self.as_mut()
            .set_history_published_at(after.history_published_at);
        self.as_mut().set_history_error(after.history_error);
        self.as_mut().set_history_bars(after.history_bars);
        self.as_mut().set_history_shortfall(after.history_shortfall);
        // Last, so the chart items repaint once the rest is consistent.
        self.as_mut().set_revision(after.revision);
    }

    pub fn ingest_completed_bar_gen(
        mut self: std::pin::Pin<&mut Self>,
        generation: i64,
        time: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    ) {
        let before = self.as_ref().rust().snapshot_props();
        let col = make_bar_columns(time, open, high, low, close);
        self.as_mut()
            .rust_mut()
            .apply_stamped(generation as u64, BarDelivery::Completed(col));
        self.notify_props(before);
    }

    pub fn ingest_forming_bar_gen(
        mut self: std::pin::Pin<&mut Self>,
        generation: i64,
        time: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    ) {
        let before = self.as_ref().rust().snapshot_props();
        let col = make_bar_columns(time, open, high, low, close);
        self.as_mut()
            .rust_mut()
            .apply_stamped(generation as u64, BarDelivery::Forming(col));
        self.notify_props(before);
    }

    pub fn retarget(
        mut self: std::pin::Pin<&mut Self>,
        symbol: QString,
        timeframe: QString,
        generation: i64,
    ) -> bool {
        let before = self.as_ref().rust().snapshot_props();
        self.as_mut().rust_mut().reset_for_target(
            &symbol.to_string(),
            &timeframe.to_string(),
            generation as u64,
        );
        self.as_mut().notify_props(before);
        self.load_history()
    }

    pub fn set_execution_rows(
        mut self: std::pin::Pin<&mut Self>,
        decisions_json: QString,
        fills_json: QString,
    ) {
        let decisions = serde_json::from_str(&decisions_json.to_string()).unwrap_or_default();
        let fills = serde_json::from_str(&fills_json.to_string()).unwrap_or_default();
        self.as_mut().rust_mut().set_rows(decisions, fills);
        let rev = self.rust().overlay_revision;
        self.as_mut().rust_mut().overlay_revision = rev - 1;
        self.as_mut().set_overlay_revision(rev);
    }

    pub fn set_overlays_json(
        mut self: std::pin::Pin<&mut Self>,
        generation: i64,
        json: QString,
    ) -> bool {
        let Ok(series) = overlays::parse_chart(&json.to_string()) else {
            return false;
        };
        let ok = self
            .as_mut()
            .rust_mut()
            .set_overlays(generation as u64, series);
        if ok {
            let rev = self.rust().overlay_revision;
            self.as_mut().rust_mut().overlay_revision = rev - 1;
            self.as_mut().set_overlay_revision(rev);
        }
        ok
    }

    pub fn rebuild_overlays(
        mut self: std::pin::Pin<&mut Self>,
        first_bar: i64,
        last_bar: i64,
        low: f64,
        high: f64,
        width_px: f32,
        height_px: f32,
    ) {
        self.as_mut().rust_mut().build_overlays(View {
            first: first_bar.max(0) as usize,
            last: last_bar.max(0) as usize,
            low,
            high,
            width: width_px,
            height: height_px,
        });
    }

    pub fn overlay_layer_count(&self) -> i32 {
        self.rust().layers().len() as i32
    }

    pub fn overlay_layer_ptr(&self, layer: i32) -> i64 {
        self.rust()
            .layers()
            .get(layer as usize)
            .map_or(0, |l| l.xy.as_ptr() as usize as i64)
    }

    pub fn overlay_layer_len(&self, layer: i32) -> i64 {
        self.rust()
            .layers()
            .get(layer as usize)
            .map_or(0, |l| (l.xy.len() / 2) as i64)
    }

    pub fn overlay_layer_mode(&self, layer: i32) -> i32 {
        self.rust()
            .layers()
            .get(layer as usize)
            .map_or(0, |l| i32::from(l.mode))
    }

    pub fn overlay_layer_color(&self, layer: i32) -> i64 {
        self.rust()
            .layers()
            .get(layer as usize)
            .map_or(0, |l| i64::from(l.rgba))
    }

    pub fn marker_count(&self) -> i32 {
        self.rust().markers().len() as i32
    }

    pub fn marker_detail_at(&self, x: f32, y: f32, radius: f32) -> QString {
        let best = self
            .rust()
            .hits()
            .iter()
            .map(|h| ((h.y - y).mul_add(h.y - y, (h.x - x).powi(2)), h))
            .filter(|(d, _)| *d <= radius * radius)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        QString::from(best.map_or("", |(_, h)| h.detail.as_str()))
    }

    pub fn bench_populate(
        mut self: std::pin::Pin<&mut Self>,
        bars: i64,
        markers: i64,
        overlays: i64,
    ) {
        let before = self.as_ref().rust().snapshot_props();
        self.as_mut().rust_mut().populate_bench(
            bars.max(0) as usize,
            markers.max(0) as usize,
            overlays.max(0) as usize,
        );
        self.as_mut().notify_props(before);
    }

    pub fn target_generation(&self) -> i64 {
        self.rust().generation as i64
    }

    pub fn stale_dropped(&self) -> i64 {
        self.rust().stale_dropped as i64
    }

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
        let generation = self.as_ref().rust().generation;
        let qt_for_loaded = qt_thread.clone();
        controller.spawn_load(
            config,
            bars,
            move |loaded| {
                qt_for_loaded
                    .queue(move |mut feed| {
                        let before = feed.as_ref().rust().snapshot_props();
                        feed.as_mut()
                            .rust_mut()
                            .complete_history_load_for(generation, loaded);
                        feed.notify_props(before);
                    })
                    .ok();
            },
            move |progress| {
                progress_controller.set_progress(progress);
                qt_thread
                    .queue(move |mut feed| {
                        if feed.as_ref().rust().generation != generation {
                            return;
                        }
                        let before = feed.as_ref().rust().snapshot_props();
                        feed.as_mut().rust_mut().sync_history_properties();
                        feed.notify_props(before);
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
            operator: "operator".to_string(),
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

    pub fn bar_times_len(&self) -> i32 {
        self.rust().bar_times.len() as i32
    }

    pub fn bar_time_at(&self, index: i32) -> i64 {
        if index >= 0 && (index as usize) < self.rust().bar_times.len() {
            self.rust().bar_times[index as usize]
        } else if index == self.rust().bar_times.len() as i32 {
            self.rust().forming.map(|bar| bar.0).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn visible_range_json(&self, first_bar: i32, last_bar: i32) -> QString {
        let feed = self.rust();
        let first = first_bar.max(0) as usize;
        let last = last_bar.max(0) as usize;
        if first >= last || (first >= feed.bar_count.max(0) as usize && feed.forming.is_none()) {
            return QString::from(r#"{"valid":false}"#);
        }
        let completed_last = last.min(feed.bar_hlc.len());
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for (bar_high, bar_low, _) in &feed.bar_hlc[first.min(completed_last)..completed_last] {
            low = low.min(*bar_low);
            high = high.max(*bar_high);
        }
        if last > feed.bar_hlc.len() {
            if let Some((_, _, forming_high, forming_low, _)) = feed.forming {
                low = low.min(forming_low);
                high = high.max(forming_high);
            }
        }
        let first_time = feed
            .bar_times
            .get(first)
            .copied()
            .or_else(|| feed.forming.map(|bar| bar.0))
            .unwrap_or(0);
        let last_time = feed
            .bar_times
            .get(last - 1)
            .copied()
            .or_else(|| feed.forming.map(|bar| bar.0))
            .unwrap_or(0);
        if !low.is_finite() || !high.is_finite() || first_time == 0 || last_time == 0 {
            return QString::from(r#"{"valid":false}"#);
        }
        QString::from(format!(
            r#"{{"valid":true,"first_time":{},"last_time":{},"low":{},"high":{}}}"#,
            first_time, last_time, low, high
        ))
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
            operator: "operator".to_string(),
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

        for _ in 0..1500 {
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
