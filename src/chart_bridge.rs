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
        type ViewportProbe;
        type ChartPaneProbe;

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
        fn ensure_test_app();
        fn reset_chart_probe_state();
    }
}

pub use chart::{
    make_chart_pane_probe, make_viewport_probe, ChartPaneProbe, ChartPaneProbeResult, ProbeResult,
    ViewportProbe, ViewportProbeResult,
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

#[cfg(test)]
#[allow(clippy::missing_safety_doc, clippy::too_many_arguments)]
mod tests {
    use super::chart::{self, make_test_chart_item, make_test_series, ProbeState};

    fn setup() {
        chart::ensure_test_app();
        chart::reset_chart_probe_state();
    }

    unsafe fn sync_series(
        series: *mut chart::BarSeries,
        first_bar: i32,
        last_bar: i32,
        low: f64,
        high: f64,
        width_px: f32,
        height_px: f32,
        state: &mut ProbeState,
    ) -> chart::ProbeResult {
        chart::chart_probe_sync(
            series, first_bar, last_bar, low, high, width_px, height_px, state,
        )
    }

    #[test]
    fn probe_vertices_match_series_pointer_for_four_bars() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let mut state = ProbeState::default();
            let result = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);

            assert_eq!(result.vertices.len(), 48);
            for (index, uploaded) in result.vertices.iter().enumerate() {
                let source = chart::chart_series_vertex_at(series, index);
                assert_eq!(uploaded, &source, "vertex mismatch at index {index}");
            }
        }
    }

    #[test]
    fn upload_rules_first_uploads_unchanged_revision_skips_mutation_uploads_once() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let mut state = ProbeState::default();

            let first = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
            assert_eq!(first.uploads, 1);

            let second = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
            assert_eq!(second.uploads, 0);

            chart::chart_series_ingest_forming(
                series,
                1_000_000_240,
                100.0,
                103.0,
                98.0,
                101.0,
                10.0,
            );
            let third = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
            assert_eq!(third.uploads, 1);
        }
    }

    #[test]
    fn ten_mutations_between_two_frames_produce_one_upload() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let mut state = ProbeState::default();
            sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);

            for offset in 0..10 {
                chart::chart_series_ingest_forming(
                    series,
                    1_000_000 + offset,
                    100.0,
                    103.0,
                    98.0,
                    101.0,
                    10.0,
                );
            }

            let after_mutations = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
            assert_eq!(after_mutations.uploads, 1);
        }
    }

    #[test]
    fn constant_bucket_count_reuses_geometry() {
        setup();
        unsafe {
            let series = make_test_series(8);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let mut state = ProbeState::default();
            let mut total_allocations = 0;

            let first = sync_series(series, 0, 8, low, high, 400.0, 200.0, &mut state);
            total_allocations += first.geometry_allocations;
            assert!(first.geometry_allocations > 0);

            for _ in 1..10 {
                let frame = sync_series(series, 0, 8, low, high, 400.0, 200.0, &mut state);
                assert_eq!(frame.geometry_allocations, 0);
                total_allocations += frame.geometry_allocations;
            }

            assert_eq!(total_allocations, first.geometry_allocations);

            let changed = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
            assert_eq!(changed.geometry_allocations, first.geometry_allocations);
        }
    }

    #[test]
    fn empty_series_draws_nothing_without_error() {
        setup();
        unsafe {
            let series = make_test_series(0);
            let mut state = ProbeState::default();
            let result = sync_series(series, 0, 0, 0.0, 0.0, 400.0, 200.0, &mut state);
            assert!(result.vertices.is_empty());
        }
    }

    #[test]
    fn flat_bar_draws_visible_mark() {
        setup();
        unsafe {
            let series = make_test_series(0);
            chart::chart_series_ingest_completed(series, 60, 100.0, 100.0, 100.0, 100.0, 1.0);
            let mut state = ProbeState::default();
            let result = sync_series(series, 0, 1, 100.0, 100.0, 400.0, 200.0, &mut state);
            assert!(!result.vertices.is_empty());
        }
    }

    #[test]
    fn subpixel_bucket_viewport_still_draws_every_bucket() {
        setup();
        unsafe {
            let series = make_test_series(8000);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let mut state = ProbeState::default();
            let result = sync_series(series, 0, 8000, low, high, 400.0, 200.0, &mut state);
            assert_eq!(result.vertices.len(), 400 * 12);
        }
    }

    #[test]
    fn zero_sized_item_produces_no_vertices() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let item = make_test_chart_item(series, 0, 4, low, high, 0.0, 0.0);
            let result = chart::chart_probe_item_paint(item, 1);
            assert!(result.vertices.is_empty());
        }
    }

    #[test]
    fn property_and_size_changes_coalesce_to_one_paint_node_call() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            let item = make_test_chart_item(series, 0, 4, low, high, 400.0, 200.0);
            chart::chart_item_apply_view(item, 1, 3, 95.0, 115.0, 320.0, 160.0);

            let result = chart::chart_probe_item_paint(item, 1);
            assert_eq!(result.paint_node_call_count, 1);
            assert!(result.update_request_count > 1);
        }
    }

    #[test]
    fn rising_falling_and_forming_vertex_counts_match_flags() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let low = chart::chart_series_low(series);
            let high = chart::chart_series_high(series);
            chart::chart_series_ingest_forming(
                series,
                1_000_000_240,
                100.0,
                103.0,
                98.0,
                101.0,
                10.0,
            );
            let mut state = ProbeState::default();
            let result = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);

            let mut rising = 0;
            let mut falling = 0;
            let mut forming = 0;
            for vertex in &result.vertices {
                if vertex.forming >= 0.5 {
                    forming += 1;
                } else if vertex.direction >= 0.0 {
                    rising += 1;
                } else {
                    falling += 1;
                }
            }

            assert_eq!(result.rising_vertex_count, rising);
            assert_eq!(result.falling_vertex_count, falling);
            assert_eq!(result.forming_vertex_count, forming);
            assert_eq!(forming, 12);
        }
    }

    #[test]
    fn viewport_ends_at_newest_bar_and_holds_bars_visible() {
        setup();
        let mut probe = chart::make_viewport_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        pin.as_mut().update(50, 100.0, 200.0, 1);
        let result = pin.result();

        assert!(!result.empty);
        assert_eq!(result.last_bar, 50);
        assert_eq!(result.first_bar, 30);
        assert_eq!(result.last_bar - result.first_bar, 20);
    }

    #[test]
    fn viewport_completed_bar_advances_by_one() {
        setup();
        let mut probe = chart::make_viewport_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        pin.as_mut().update(50, 100.0, 200.0, 1);
        let first = pin.result();
        assert_eq!(first.last_bar, 50);
        assert_eq!(first.first_bar, 30);

        pin.as_mut().update(51, 100.0, 200.0, 2);
        let second = pin.result();
        assert_eq!(second.last_bar, 51);
        assert_eq!(second.first_bar, 31);
    }

    #[test]
    fn viewport_forming_update_inside_range_leaves_price_range_untouched() {
        setup();
        let mut probe = chart::make_viewport_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        pin.as_mut().set_price_margin(0.05);
        pin.as_mut().update(50, 100.0, 200.0, 1);
        let initial = pin.result();
        assert!((initial.low_price - 95.0).abs() < 1e-6);
        assert!((initial.high_price - 205.0).abs() < 1e-6);

        pin.as_mut().update(50, 98.0, 202.0, 2);
        let after = pin.result();
        assert!((after.low_price - 95.0).abs() < 1e-6);
        assert!((after.high_price - 205.0).abs() < 1e-6);
    }

    #[test]
    fn viewport_forming_update_outside_recomputes_with_margin() {
        setup();
        let mut probe = chart::make_viewport_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        pin.as_mut().set_price_margin(0.05);
        pin.as_mut().update(50, 100.0, 200.0, 1);
        let initial = pin.result();
        assert!((initial.low_price - 95.0).abs() < 1e-6);
        assert!((initial.high_price - 205.0).abs() < 1e-6);

        pin.as_mut().update(50, 98.0, 210.0, 2);
        let after_high = pin.result();
        assert!((after_high.high_price - 215.6).abs() < 1e-6);
        assert!((after_high.low_price - 92.4).abs() < 1e-6);

        pin.as_mut().update(50, 90.0, 210.0, 3);
        let after_low = pin.result();
        assert!((after_low.low_price - 84.0).abs() < 1e-6);
        assert!((after_low.high_price - 216.0).abs() < 1e-6);
    }

    #[test]
    fn viewport_empty_series_yields_empty_state() {
        setup();
        let mut probe = chart::make_viewport_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().update(0, 0.0, 0.0, 0);
        let result = pin.result();

        assert!(result.empty);
        assert_eq!(result.first_bar, 0);
        assert_eq!(result.last_bar, 0);
        assert!(result.low_price.abs() < 1e-6);
        assert!(result.high_price.abs() < 1e-6);
    }

    #[test]
    fn chart_pane_axis_labels_match_viewport_bounds() {
        setup();
        unsafe {
            let series = make_test_series(4);
            let mut probe = chart::make_chart_pane_probe();
            let mut pin = probe.pin_mut();
            pin.as_mut().set_size(400.0, 200.0);
            pin.as_mut().set_series(series);
            let r = pin.result();
            assert!(!r.empty);
            assert_eq!(r.vertex_count, 48);
            assert!(!r.top_price_label.is_empty());
            assert!(!r.bottom_price_label.is_empty());
            let top: f64 = r.top_price_label.parse().expect("valid top price number");
            let bottom: f64 = r
                .bottom_price_label
                .parse()
                .expect("valid bottom price number");
            assert!(top > bottom);
        }
    }

    #[test]
    fn chart_pane_draws_nothing_when_series_is_empty() {
        setup();
        unsafe {
            let series = make_test_series(0);
            let mut probe = chart::make_chart_pane_probe();
            let mut pin = probe.pin_mut();
            pin.as_mut().set_size(400.0, 200.0);
            pin.as_mut().set_series(series);
            let r = pin.result();
            assert!(r.empty);
            assert_eq!(r.vertex_count, 0);
            assert_eq!(r.top_price_label, "");
            assert_eq!(r.bottom_price_label, "");
        }
    }
}
