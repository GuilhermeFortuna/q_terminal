#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/chart_bridge.rs"]
pub mod chart_bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/ops_session.rs"]
pub mod ops_session;
pub use bridge::execution_controls;
pub use bridge::execution_models;
#[path = "../src/startup.rs"]
pub mod startup;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use chart_bridge::chart::{self, make_test_chart_item, make_test_series, ProbeState};

fn setup() -> std::sync::MutexGuard<'static, ()> {
    let guard = chart_bridge::QT_TEST_MUTEX.lock().unwrap();
    chart::ensure_test_app();
    chart::reset_chart_probe_state();
    guard
}

#[allow(clippy::too_many_arguments)]
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
    let _guard = setup();
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
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        let mut state = ProbeState::default();

        let first = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
        assert_eq!(first.uploads, 1);

        let second = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
        assert_eq!(second.uploads, 0);

        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
        let third = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(third.uploads, 1);
    }
}

#[test]
fn ten_mutations_between_two_frames_produce_one_upload() {
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
    unsafe {
        let series = make_test_series(0);
        let mut state = ProbeState::default();
        let result = sync_series(series, 0, 0, 0.0, 0.0, 400.0, 200.0, &mut state);
        assert!(result.vertices.is_empty());
    }
}

#[test]
fn flat_bar_draws_visible_mark() {
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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
    let _guard = setup();
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

#[test]
fn status_strip_unavailable_stream_shows_stale_and_reason() {
    let _guard = setup();
    unsafe {
        let feed = chart::make_test_feed();
        chart::feed_set_connection_state(feed, "reconnecting");
        chart::feed_set_last_error(feed, "stream disconnected");
        chart::feed_set_stale(feed, true);
        chart::feed_set_data_age_ms(feed, 125_000);

        let mut probe = chart::make_status_strip_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_feed(feed);
        let r = pin.result();

        assert_eq!(r.connection_state, "RECONNECTING");
        assert_eq!(r.last_error, "stream disconnected");
        assert!(r.stale_visible);
        assert_eq!(r.stale_text, "STALE (2m 5s)");
    }
}

#[test]
fn status_strip_recovers_to_live_and_clears_stale() {
    let _guard = setup();
    unsafe {
        let feed = chart::make_test_feed();
        chart::feed_set_connection_state(feed, "live");
        chart::feed_set_last_error(feed, "");
        chart::feed_set_stale(feed, false);
        chart::feed_set_data_age_ms(feed, 0);

        let mut probe = chart::make_status_strip_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_feed(feed);
        let r = pin.result();

        assert_eq!(r.connection_state, "LIVE");
        assert_eq!(r.last_error, "");
        assert!(!r.stale_visible);
    }
}

#[test]
fn status_strip_shows_history_source_and_shortfall() {
    let _guard = setup();
    unsafe {
        let feed = chart::make_test_feed();
        chart::feed_set_history(feed, "api", 15);

        let mut probe = chart::make_status_strip_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut().set_feed(feed);
        let r = pin.result();

        assert_eq!(r.history_text, "History: API (shortfall: 15 bars)");
    }
}

#[test]
fn status_strip_and_empty_state_show_live_only() {
    let _guard = setup();
    unsafe {
        let feed = chart::make_test_feed();
        chart::feed_set_live_only(feed, true);

        let mut strip_probe = chart::make_status_strip_probe();
        strip_probe.pin_mut().set_feed(feed);
        let strip_res = strip_probe.result();
        assert!(strip_res.live_only_visible);

        let mut empty_probe = chart::make_empty_state_probe();
        empty_probe.pin_mut().set_feed(feed);
        let empty_res = empty_probe.result();
        assert_eq!(empty_res.message, "Waiting for live bars (live only)...");
    }
}

#[test]
fn empty_state_shows_retrying_when_api_down() {
    let _guard = setup();
    unsafe {
        let feed = chart::make_test_feed();
        chart::feed_set_connection_state(feed, "retrying");
        chart::feed_set_last_error(feed, "connection refused");

        let mut empty_probe = chart::make_empty_state_probe();
        empty_probe.pin_mut().set_feed(feed);
        let empty_res = empty_probe.result();
        assert_eq!(empty_res.message, "API unreachable, retrying...");
        assert_eq!(empty_res.reason, "connection refused");
    }
}
