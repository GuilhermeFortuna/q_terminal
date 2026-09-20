#[cxx::bridge]
#[allow(clippy::missing_safety_doc, clippy::too_many_arguments)]
pub mod chart {
    #[derive(Clone, Debug, PartialEq)]
    struct ProbeVertex {
        x: f32,
        y: f32,
        direction: f32,
        forming: f32,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ProbeResult {
        vertices: Vec<ProbeVertex>,
        revision: i64,
        uploads: i32,
        geometry_allocations: i32,
        rising_vertex_count: i32,
        falling_vertex_count: i32,
        forming_vertex_count: i32,
        paint_node_call_count: i32,
        update_request_count: i32,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ViewportProbeResult {
        pub first_bar: i32,
        pub last_bar: i32,
        pub low_price: f64,
        pub high_price: f64,
        pub empty: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct ChartPaneProbeResult {
        pub top_price_label: String,
        pub bottom_price_label: String,
        pub vertex_count: i32,
        pub empty: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct StatusStripProbeResult {
        pub connection_state: String,
        pub last_error: String,
        pub history_text: String,
        pub stale_text: String,
        pub stale_visible: bool,
        pub live_only_visible: bool,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct EmptyStateProbeResult {
        pub message: String,
        pub reason: String,
    }

    #[derive(Default)]
    struct ProbeState {
        uploaded_revision: i64,
        upload_count: i32,
        geometry_allocations: i32,
    }

    unsafe extern "C++" {
        include!("cpp/chart_cxx.h");

        type BarSeries;
        type BarChartItem;
        type BarFeed;
        type ViewportProbe;
        type ChartPaneProbe;
        type StatusStripProbe;
        type EmptyStateProbe;

        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        fn make_status_strip_probe() -> UniquePtr<StatusStripProbe>;
        unsafe fn set_feed(self: Pin<&mut StatusStripProbe>, feed: *mut BarFeed);
        fn result(self: &StatusStripProbe) -> StatusStripProbeResult;

        fn make_empty_state_probe() -> UniquePtr<EmptyStateProbe>;
        unsafe fn set_feed(self: Pin<&mut EmptyStateProbe>, feed: *mut BarFeed);
        fn result(self: &EmptyStateProbe) -> EmptyStateProbeResult;

        unsafe fn make_test_feed() -> *mut BarFeed;
        unsafe fn feed_set_connection_state(feed: *mut BarFeed, state: &str);
        unsafe fn feed_set_last_error(feed: *mut BarFeed, error: &str);
        unsafe fn feed_set_stale(feed: *mut BarFeed, stale: bool);
        unsafe fn feed_set_data_age_ms(feed: *mut BarFeed, ms: i64);
        unsafe fn feed_set_live_only(feed: *mut BarFeed, live_only: bool);
        unsafe fn feed_set_history(feed: *mut BarFeed, source: &str, shortfall: i64);
        unsafe fn feed_set_bar_count(feed: *mut BarFeed, count: i64);

        unsafe fn find_window_feed(engine: Pin<&mut QQmlApplicationEngine>) -> *mut BarFeed;
        unsafe fn setup_window_feed(engine: Pin<&mut QQmlApplicationEngine>, feed: *mut BarFeed);
        fn setup_window_auto_close(engine: Pin<&mut QQmlApplicationEngine>, ms: i32);

        unsafe fn feed_set_symbol(feed: *mut BarFeed, symbol: &str);
        unsafe fn feed_set_timeframe(feed: *mut BarFeed, timeframe: &str, timeframe_ms: i64);
        unsafe fn feed_setup_and_load(
            feed: *mut BarFeed,
            api_base: &str,
            symbol: &str,
            timeframe: &str,
        );

        unsafe fn post_feed_stream_state(
            feed: *mut BarFeed,
            state: &str,
            last_error: &str,
            applied: i64,
            dropped: i64,
            gaps_closed: i64,
            resnapshots: i64,
            rest_calls: i64,
        );

        unsafe fn post_feed_completed_bar(
            feed: *mut BarFeed,
            generation: i64,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        unsafe fn post_feed_forming_bar(
            feed: *mut BarFeed,
            generation: i64,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
        );

        fn make_viewport_probe() -> UniquePtr<ViewportProbe>;
        fn set_bars_visible(self: Pin<&mut ViewportProbe>, count: i32);
        fn set_price_margin(self: Pin<&mut ViewportProbe>, margin: f64);
        fn update(
            self: Pin<&mut ViewportProbe>,
            bar_count: i32,
            low: f64,
            high: f64,
            revision: i32,
        );
        fn result(self: &ViewportProbe) -> ViewportProbeResult;

        fn make_chart_pane_probe() -> UniquePtr<ChartPaneProbe>;
        unsafe fn set_series(self: Pin<&mut ChartPaneProbe>, series: *mut BarSeries);
        fn set_size(self: Pin<&mut ChartPaneProbe>, width: f32, height: f32);
        fn set_bars_visible(self: Pin<&mut ChartPaneProbe>, count: i32);
        fn set_price_margin(self: Pin<&mut ChartPaneProbe>, margin: f64);
        fn result(self: &ChartPaneProbe) -> ChartPaneProbeResult;

        fn register_bar_chart_types();
        unsafe fn make_test_series(bar_count: i32) -> *mut BarSeries;
        unsafe fn chart_probe_sync(
            series: *mut BarSeries,
            first_bar: i32,
            last_bar: i32,
            low: f64,
            high: f64,
            width_px: f32,
            height_px: f32,
            state: &mut ProbeState,
        ) -> ProbeResult;
        unsafe fn chart_probe_item_paint(item: *mut BarChartItem, frames: i32) -> ProbeResult;
        unsafe fn make_test_chart_item(
            series: *mut BarSeries,
            first_bar: i32,
            last_bar: i32,
            low: f64,
            high: f64,
            width_px: f32,
            height_px: f32,
        ) -> *mut BarChartItem;
        unsafe fn chart_item_set_size(item: *mut BarChartItem, width_px: f32, height_px: f32);
        unsafe fn chart_item_apply_view(
            item: *mut BarChartItem,
            first_bar: i32,
            last_bar: i32,
            low: f64,
            high: f64,
            width_px: f32,
            height_px: f32,
        );
        unsafe fn chart_series_ingest_completed(
            series: *mut BarSeries,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: f64,
        );
        unsafe fn chart_series_low(series: *mut BarSeries) -> f64;
        unsafe fn chart_series_high(series: *mut BarSeries) -> f64;
        unsafe fn chart_series_ingest_forming(
            series: *mut BarSeries,
            time: i64,
            open: f64,
            high: f64,
            low: f64,
            close: f64,
            volume: f64,
        );
        unsafe fn chart_series_rebuild(series: *mut BarSeries);
        unsafe fn chart_series_geometry_revision(series: *mut BarSeries) -> i64;
        unsafe fn chart_series_vertex_len(series: *mut BarSeries) -> i32;
        unsafe fn chart_series_vertex_at(series: *mut BarSeries, index: usize) -> ProbeVertex;
        fn ensure_application();
        fn exec_application() -> i32;
        fn process_events();
        fn ensure_test_app();
        fn reset_chart_probe_state();

        unsafe fn find_window_execution_models(
            engine: Pin<&mut QQmlApplicationEngine>,
        ) -> *mut ExecutionModels;
        unsafe fn post_execution_models_sync(models: *mut ExecutionModels);

        unsafe fn feed_retarget(
            feed: *mut BarFeed,
            symbol: &str,
            timeframe: &str,
            generation: i64,
        ) -> bool;
        unsafe fn feed_target_generation(feed: *mut BarFeed) -> i64;
        unsafe fn feed_stale_dropped(feed: *mut BarFeed) -> i64;
        unsafe fn feed_symbol(feed: *mut BarFeed) -> String;
        unsafe fn post_feed_overlays(feed: *mut BarFeed, generation: i64, json: &str);
        unsafe fn feed_set_execution_rows(feed: *mut BarFeed, decisions: &str, fills: &str);
        unsafe fn feed_marker_count(feed: *mut BarFeed) -> i64;
        unsafe fn feed_rebuild_overlays(
            feed: *mut BarFeed,
            first_bar: i32,
            last_bar: i32,
            low: f64,
            high: f64,
            width: f32,
            height: f32,
        ) -> i64;

        unsafe fn feed_bar_times_len(feed: *mut BarFeed) -> i32;
        unsafe fn feed_bar_time_at(feed: *mut BarFeed, index: i32) -> i64;
        unsafe fn feed_vertex_len(feed: *mut BarFeed) -> i32;
        unsafe fn feed_vertex_at(feed: *mut BarFeed, index: usize) -> ProbeVertex;
        unsafe fn feed_rebuild_geometry(
            feed: *mut BarFeed,
            first_bar: i32,
            last_bar: i32,
            low: f64,
            high: f64,
            width: f32,
            height: f32,
        );
        unsafe fn feed_history_source(feed: *mut BarFeed) -> String;
        unsafe fn feed_history_error(feed: *mut BarFeed) -> String;
        unsafe fn feed_history_loading(feed: *mut BarFeed) -> bool;
        unsafe fn feed_bar_count(feed: *mut BarFeed) -> i64;
        unsafe fn feed_rest_calls(feed: *mut BarFeed) -> i64;

        type ExecutionModels;

        fn make_test_execution_models() -> *mut ExecutionModels;
        unsafe fn delete_test_execution_models(models: *mut ExecutionModels);
        unsafe fn execution_models_sync(models: *mut ExecutionModels);
        unsafe fn execution_models_revision(models: *mut ExecutionModels) -> i64;
        unsafe fn execution_models_redraw_count(models: *mut ExecutionModels) -> i64;
        unsafe fn execution_models_select_deployment(models: *mut ExecutionModels, id: &str);
        unsafe fn execution_models_select_account(models: *mut ExecutionModels, id: &str);
        unsafe fn execution_models_load_older(models: *mut ExecutionModels, table: &str);
    }
}

pub use chart::{
    delete_test_execution_models, ensure_application, exec_application,
    execution_models_load_older, execution_models_redraw_count, execution_models_revision,
    execution_models_select_account, execution_models_select_deployment, execution_models_sync,
    feed_bar_count, feed_bar_time_at, feed_bar_times_len, feed_history_error, feed_history_loading,
    feed_history_source, feed_marker_count, feed_rebuild_geometry, feed_rebuild_overlays,
    feed_rest_calls, feed_retarget, feed_set_bar_count, feed_set_connection_state,
    feed_set_data_age_ms, feed_set_execution_rows, feed_set_history, feed_set_last_error,
    feed_set_live_only, feed_set_stale, feed_set_symbol, feed_set_timeframe, feed_setup_and_load,
    feed_stale_dropped, feed_symbol, feed_target_generation, feed_vertex_at, feed_vertex_len,
    find_window_execution_models, find_window_feed, make_chart_pane_probe, make_empty_state_probe,
    make_status_strip_probe, make_test_execution_models, make_test_feed, make_viewport_probe,
    post_execution_models_sync, post_feed_completed_bar, post_feed_forming_bar, post_feed_overlays,
    post_feed_stream_state, process_events, setup_window_auto_close, setup_window_feed, BarFeed,
    ChartPaneProbe, ChartPaneProbeResult, EmptyStateProbe, EmptyStateProbeResult, ExecutionModels,
    ProbeResult, ProbeVertex, StatusStripProbe, StatusStripProbeResult, ViewportProbe,
    ViewportProbeResult,
};

pub fn register_chart_types() {
    chart::register_bar_chart_types();
}

/// # Safety
/// `series` must be a valid `BarSeries` pointer produced by this crate's test helpers.
#[allow(clippy::too_many_arguments)]
pub unsafe fn probe_sync_series(
    series: *mut chart::BarSeries,
    first_bar: i32,
    last_bar: i32,
    low: f64,
    high: f64,
    width_px: f32,
    height_px: f32,
    state: &mut chart::ProbeState,
) -> chart::ProbeResult {
    chart::chart_probe_sync(
        series, first_bar, last_bar, low, high, width_px, height_px, state,
    )
}

/// # Safety
/// The returned pointer is owned by the C++ test harness and must not be freed from Rust.
pub unsafe fn make_test_series(bar_count: i32) -> *mut chart::BarSeries {
    chart::make_test_series(bar_count)
}

pub static QT_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
