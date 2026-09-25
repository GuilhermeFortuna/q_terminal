pub use q_terminal::{
    bridge, bridge::bar_feed, chart_bridge, chart_context, chart_target, config, contracts_stream,
    execution, execution_controls, execution_models, history, ops_session, ops_status, startup,
    stream,
};

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
fn forming_tick_updates_only_the_forming_scene_graph_layer() {
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        let mut state = ProbeState::default();

        let initial = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(initial.completed_layer_updates, 1);
        assert_eq!(initial.forming_layer_updates, 0);

        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
        let first_forming = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(first_forming.completed_layer_updates, 0);
        assert_eq!(first_forming.forming_layer_updates, 1);

        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 104.0, 97.0, 102.0, 10.0);
        let forming_tick = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(forming_tick.completed_layer_updates, 0);
        assert_eq!(forming_tick.forming_layer_updates, 1);
    }
}

#[test]
fn completed_bar_transition_invalidates_completed_and_forming_layers() {
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
        let mut state = ProbeState::default();
        sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);

        chart::chart_series_ingest_completed(
            series,
            1_000_000_240,
            100.0,
            103.0,
            98.0,
            101.0,
            10.0,
        );
        let completed = sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(completed.completed_layer_updates, 1);
        assert_eq!(completed.forming_layer_updates, 1);
        assert_eq!(completed.forming_vertex_count, 0);
    }
}

#[test]
fn viewport_price_and_surface_changes_invalidate_both_layers() {
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
        let mut state = ProbeState::default();
        sync_series(series, 0, 5, low, high, 400.0, 200.0, &mut state);

        let panned = sync_series(series, 1, 5, low, high, 400.0, 200.0, &mut state);
        assert_eq!(panned.completed_layer_updates, 1);
        assert_eq!(panned.forming_layer_updates, 1);

        let expanded = sync_series(
            series,
            1,
            5,
            low - 5.0,
            high + 5.0,
            400.0,
            200.0,
            &mut state,
        );
        assert_eq!(expanded.completed_layer_updates, 1);
        assert_eq!(expanded.forming_layer_updates, 1);

        let resized = sync_series(
            series,
            1,
            5,
            low - 5.0,
            high + 5.0,
            320.0,
            160.0,
            &mut state,
        );
        assert_eq!(resized.completed_layer_updates, 1);
        assert_eq!(resized.forming_layer_updates, 1);
    }
}

#[test]
fn offscreen_forming_tick_does_not_update_scene_graph_geometry() {
    let _guard = setup();
    unsafe {
        let series = make_test_series(4);
        let low = chart::chart_series_low(series);
        let high = chart::chart_series_high(series);
        let mut state = ProbeState::default();
        let initial = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
        assert_eq!(initial.forming_vertex_count, 0);

        chart::chart_series_ingest_forming(series, 1_000_000_240, 100.0, 103.0, 98.0, 101.0, 10.0);
        let offscreen_tick = sync_series(series, 0, 4, low, high, 400.0, 200.0, &mut state);
        assert_eq!(offscreen_tick.completed_layer_updates, 0);
        assert_eq!(offscreen_tick.forming_layer_updates, 0);
        assert_eq!(offscreen_tick.forming_vertex_count, 0);
    }
}

#[test]
fn source_identity_change_rebuilds_completed_layer() {
    let _guard = setup();
    unsafe {
        let first_series = make_test_series(4);
        let second_series = make_test_series(4);
        let low = chart::chart_series_low(first_series);
        let high = chart::chart_series_high(first_series);
        let mut state = ProbeState::default();
        sync_series(first_series, 0, 4, low, high, 400.0, 200.0, &mut state);

        let retargeted = sync_series(second_series, 0, 4, low, high, 400.0, 200.0, &mut state);
        assert_eq!(retargeted.completed_layer_updates, 1);
        assert_eq!(retargeted.forming_layer_updates, 0);
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
fn viewport_navigation_clamps_pan_and_reports_history_mode() {
    let _guard = setup();
    let mut probe = chart::make_viewport_probe();
    let mut pin = probe.pin_mut();
    pin.as_mut().set_bars_visible(5);
    pin.as_mut().update(20, 100.0, 200.0, 1);
    assert_eq!(pin.result().first_bar, 15);
    assert_eq!(pin.result().last_bar, 20);
    assert_eq!(pin.result().mode, "following");

    pin.as_mut().pan_bars(-3);
    let inspected = pin.result();
    assert_eq!((inspected.first_bar, inspected.last_bar), (12, 17));
    assert_eq!(inspected.mode, "inspecting");

    pin.as_mut().pan_bars(-100);
    let left = pin.result();
    assert_eq!((left.first_bar, left.last_bar), (0, 5));
    pin.as_mut().pan_bars(100);
    let right = pin.result();
    assert_eq!((right.first_bar, right.last_bar), (15, 20));
    assert_eq!(right.mode, "inspecting");
}

#[test]
fn viewport_zoom_keeps_anchor_bar_and_return_to_live_resumes_following() {
    let _guard = setup();
    let mut probe = chart::make_viewport_probe();
    let mut pin = probe.pin_mut();
    pin.as_mut().set_bars_visible(10);
    pin.as_mut().update(40, 100.0, 200.0, 1);

    pin.as_mut().zoom_at(0.5, 1);
    let zoomed = pin.result();
    assert_eq!((zoomed.first_bar, zoomed.last_bar), (31, 39));
    assert_eq!(zoomed.mode, "inspecting");

    pin.as_mut().update(41, 100.0, 200.0, 2);
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (31, 39));

    pin.as_mut().return_to_live();
    let live = pin.result();
    assert_eq!((live.first_bar, live.last_bar), (31, 41));
    assert_eq!(live.mode, "following");

    pin.as_mut().update(42, 100.0, 200.0, 3);
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (32, 42));
}

#[test]
fn viewport_moves_that_keep_the_live_edge_keep_following() {
    let _guard = setup();
    let mut probe = chart::make_viewport_probe();
    let mut pin = probe.pin_mut();
    pin.as_mut().set_bars_visible(10);
    pin.as_mut().update(40, 100.0, 200.0, 1);

    // Dragging toward the future at the live edge moves nothing and freezes nothing.
    pin.as_mut().pan_bars(3);
    assert_eq!(pin.result().mode, "following");
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (30, 40));

    // Zooming anchored on the newest bar keeps it visible, so live following continues.
    pin.as_mut().zoom_at(1.0, 1);
    let zoomed = pin.result();
    assert_eq!(zoomed.mode, "following");
    assert_eq!(zoomed.last_bar, 40);
    assert_eq!(zoomed.last_bar - zoomed.first_bar, 8);
    pin.as_mut().update(41, 100.0, 200.0, 2);
    assert_eq!(pin.result().last_bar, 41);
    assert_eq!(pin.result().last_bar - pin.result().first_bar, 8);

    // Zoom steps scale the visible bar count rather than adding two bars each.
    pin.as_mut().zoom_at(1.0, -3);
    let out = pin.result();
    assert_eq!(out.last_bar - out.first_bar, 16);
}

#[test]
fn viewport_price_bounds_fit_visible_bars_without_tick_oscillation() {
    let _guard = setup();
    let mut probe = chart::make_viewport_probe();
    let mut pin = probe.pin_mut();
    pin.as_mut().set_bars_visible(10);
    pin.as_mut().set_price_margin(0.05);
    pin.as_mut().update(20, 100.0, 200.0, 1);
    assert_eq!(
        (pin.result().low_price, pin.result().high_price),
        (95.0, 205.0)
    );

    // Same visible bars, narrower range: the axis holds still.
    pin.as_mut().update(20, 100.0, 150.0, 2);
    assert_eq!(
        (pin.result().low_price, pin.result().high_price),
        (95.0, 205.0)
    );

    // Same visible bars, a new high: the axis widens.
    pin.as_mut().update(20, 100.0, 250.0, 3);
    assert_eq!(
        (pin.result().low_price, pin.result().high_price),
        (92.5, 257.5)
    );

    // A new bar changes the visible bars: the axis refits, shrinking if it can.
    pin.as_mut().update(21, 100.0, 150.0, 4);
    assert_eq!(
        (pin.result().low_price, pin.result().high_price),
        (97.5, 152.5)
    );
}

#[test]
fn viewport_empty_singleton_and_target_reset_are_safe() {
    let _guard = setup();
    let mut probe = chart::make_viewport_probe();
    let mut pin = probe.pin_mut();
    pin.as_mut().set_bars_visible(10);
    pin.as_mut().update(0, 0.0, 0.0, 0);
    assert_eq!(pin.result().mode, "following");

    pin.as_mut().update(1, 100.0, 100.0, 1);
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (0, 1));
    pin.as_mut().pan_bars(-2);
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (0, 1));
    pin.as_mut().zoom_at(0.0, 1);
    assert_eq!((pin.result().first_bar, pin.result().last_bar), (0, 1));

    pin.as_mut().reset_for_target();
    let reset = pin.result();
    assert_eq!((reset.first_bar, reset.last_bar), (0, 0));
    assert_eq!(reset.mode, "following");
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

unsafe fn setup_chart_pane_with_context(
    probe: &mut cxx::UniquePtr<chart::ChartPaneProbe>,
) -> *mut chart_context::ffi::ChartContext {
    let feed = chart::make_test_feed();
    chart::feed_set_symbol(feed, "PETR4");
    chart::feed_set_timeframe(feed, "1m", 60_000);
    chart::feed_set_bar_count(feed, 4);

    let ctx = chart::make_test_chart_context() as *mut chart_context::ffi::ChartContext;
    chart_context::set_configured(ctx, "PETR4", "1m");
    chart_context::sync_ptr(ctx);

    let mut pin = probe.pin_mut();
    pin.as_mut().set_size(640.0, 360.0);
    pin.as_mut().set_feed(feed);
    pin.as_mut().set_context(ctx as *mut chart::ChartContext);
    chart::process_events();
    ctx
}

#[test]
fn chart_pane_target_prompt_symbol_only_requests_pair() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        pin.as_mut().set_target_prompt_draft("VALE3");
        let preview = pin.target_prompt_preview();
        assert!(preview.contains("VALE3"));
        assert!(preview.contains("1m"));
        pin.as_mut().submit_target_prompt();
        assert_eq!(chart_context::active_symbol(ctx), "VALE3");
        assert_eq!(chart_context::active_timeframe(ctx), "1m");
        assert!(!pin.target_prompt_open());
    }
}

#[test]
fn chart_pane_target_prompt_timeframe_only_requests_pair() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        pin.as_mut().set_target_prompt_draft("5m");
        let preview = pin.target_prompt_preview();
        assert!(preview.contains("PETR4"));
        assert!(preview.contains("5m"));
        pin.as_mut().submit_target_prompt();
        assert_eq!(chart_context::active_symbol(ctx), "PETR4");
        assert_eq!(chart_context::active_timeframe(ctx), "5m");
    }
}

#[test]
fn chart_pane_target_prompt_combined_input_requests_pair() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        pin.as_mut().set_target_prompt_draft("VALE3 5m");
        pin.as_mut().submit_target_prompt();
        assert_eq!(chart_context::active_symbol(ctx), "VALE3");
        assert_eq!(chart_context::active_timeframe(ctx), "5m");
    }
}

#[test]
fn chart_pane_target_prompt_escape_leaves_target_unchanged() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        pin.as_mut().set_target_prompt_draft("VALE3 5m");
        pin.as_mut().cancel_target_prompt();
        assert_eq!(chart_context::active_symbol(ctx), "PETR4");
        assert_eq!(chart_context::active_timeframe(ctx), "1m");
        assert!(!pin.target_prompt_open());
    }
}

#[test]
fn chart_pane_target_prompt_invalid_input_keeps_settled_target() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        pin.as_mut().set_target_prompt_draft("PETR4 99m");
        let preview = pin.target_prompt_preview();
        assert!(preview.contains("99m"));
        assert!(preview.contains("Unsupported timeframe"));
        pin.as_mut().submit_target_prompt();
        assert_eq!(chart_context::active_symbol(ctx), "PETR4");
        assert_eq!(chart_context::active_timeframe(ctx), "1m");
        assert!(chart_context::target_error(ctx).is_empty());
    }
}

#[test]
fn chart_pane_canvas_without_focus_does_not_open_prompt() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        assert!(!pin.as_mut().canvas_type_key("P"));
        assert!(!pin.target_prompt_open());
    }
}

unsafe fn setup_chart_pane_with_bars(
    probe: &mut cxx::UniquePtr<chart::ChartPaneProbe>,
    bar_count: i64,
    marker_count: i64,
) -> (*mut chart::BarFeed, *mut chart_context::ffi::ChartContext) {
    let feed = chart::make_test_feed();
    chart::feed_set_symbol(feed, "PETR4");
    chart::feed_set_timeframe(feed, "1m", 60_000);
    chart::feed_populate_bench(feed, bar_count, marker_count, 0);

    let ctx = chart::make_test_chart_context() as *mut chart_context::ffi::ChartContext;
    chart_context::set_configured(ctx, "PETR4", "1m");
    chart_context::sync_ptr(ctx);

    let mut pin = probe.pin_mut();
    pin.as_mut().set_size(640.0, 360.0);
    pin.as_mut().set_feed(feed);
    pin.as_mut().set_context(ctx as *mut chart::ChartContext);
    chart::process_events();
    chart::process_events();
    (feed, ctx)
}

#[test]
fn chart_pane_crosshair_and_readout_on_hover() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let (_feed, _ctx) = setup_chart_pane_with_bars(&mut probe, 20, 0);
        let mut pin = probe.pin_mut();

        assert!(!pin.crosshair_visible());
        assert!(pin.readout_pointer_price().is_empty());

        // Hover within canvas
        pin.as_mut().hover_canvas(100.0, 150.0);
        assert!(pin.crosshair_visible());
        assert!(pin.active_bar_index() >= 0);
        assert!(pin.readout_time().contains("UTC"));
        assert!(!pin.readout_open().is_empty());
        assert_ne!(pin.readout_open(), "--");
        assert!(!pin.readout_high().is_empty());
        assert!(!pin.readout_low().is_empty());
        assert!(!pin.readout_close().is_empty());
        assert!(!pin.readout_forming());
        assert!(!pin.readout_pointer_price().is_empty());
        assert!(pin.accessible_text().contains("Bar"));

        // Clear hover hides crosshair
        pin.as_mut().clear_hover();
        assert!(!pin.crosshair_visible());
    }
}

#[test]
fn chart_pane_crosshair_tracks_pan_and_zoom() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let (_feed, _ctx) = setup_chart_pane_with_bars(&mut probe, 100, 0);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        chart::process_events();

        pin.as_mut().hover_canvas(200.0, 100.0);
        assert!(pin.crosshair_visible());
        let initial_bar = pin.active_bar_index();

        // Pan viewport left (negative delta moves viewport earlier in history)
        pin.as_mut().pan_bars(-10);
        chart::process_events();
        let panned_bar = pin.active_bar_index();
        assert_ne!(initial_bar, panned_bar);
        assert!(pin.crosshair_visible());

        // Zoom at anchor 0.5
        pin.as_mut().zoom_at(0.5, 1);
        chart::process_events();
        assert!(pin.crosshair_visible());
    }
}

#[test]
fn chart_pane_empty_feed_hides_crosshair_and_readout() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let feed = chart::make_test_feed();
        chart::feed_set_symbol(feed, "PETR4");
        chart::feed_set_timeframe(feed, "1m", 60_000);
        chart::feed_set_bar_count(feed, 0);

        let ctx = chart::make_test_chart_context() as *mut chart_context::ffi::ChartContext;
        chart_context::set_configured(ctx, "PETR4", "1m");
        chart_context::sync_ptr(ctx);

        let mut pin = probe.pin_mut();
        pin.as_mut().set_size(640.0, 360.0);
        pin.as_mut().set_feed(feed);
        pin.as_mut().set_context(ctx as *mut chart::ChartContext);
        chart::process_events();

        // Hover over empty canvas
        pin.as_mut().hover_canvas(100.0, 100.0);
        assert!(!pin.crosshair_visible());
        assert!(pin.readout_time().is_empty());
    }
}

#[test]
fn chart_pane_keyboard_navigation_and_priority() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let (_feed, _ctx) = setup_chart_pane_with_bars(&mut probe, 20, 0);
        let mut pin = probe.pin_mut();

        pin.as_mut().focus_canvas();
        assert!(!pin.crosshair_visible());

        // Right arrow selects adjacent bar
        pin.as_mut().select_adjacent_bar(1);
        chart::process_events();
        assert!(pin.crosshair_visible());
        let first_idx = pin.active_bar_index();

        // Advance to next bar
        pin.as_mut().select_adjacent_bar(1);
        chart::process_events();
        let next_idx = pin.active_bar_index();
        assert_eq!(next_idx, first_idx + 1);

        // Clamping to visible edge (e.g. repeated Left moves to first bar and clamps)
        for _ in 0..30 {
            pin.as_mut().select_adjacent_bar(-1);
        }
        chart::process_events();
        assert_eq!(pin.active_bar_index(), 0);

        // Escape clears selection
        pin.as_mut().clear_selection();
        chart::process_events();
        assert!(!pin.crosshair_visible());

        // Printable key still opens target prompt with priority
        pin.as_mut().set_target_prompt_draft("V");
        assert!(pin.target_prompt_preview().contains("V"));
    }
}

#[test]
fn chart_pane_marker_tooltip_coexists_with_crosshair() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let (feed, _ctx) = setup_chart_pane_with_bars(&mut probe, 20, 5);
        chart::feed_rebuild_overlays(feed, 0, 20, 80.0, 120.0, 640.0, 360.0);
        chart::process_events();

        let mut pin = probe.pin_mut();
        pin.as_mut().hover_canvas(100.0, 150.0);
        assert!(pin.crosshair_visible());
        assert!(!pin.readout_open().is_empty());
    }
}

const KEY_P: i32 = 0x50;
const KEY_5: i32 = 0x35;
const SHIFT: i32 = 0x0200_0000;
const CONTROL: i32 = 0x0400_0000;
const KEYPAD: i32 = 0x2000_0000;

#[test]
fn chart_pane_real_key_events_open_prompt_with_shift_and_keypad() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        assert!(pin.as_mut().send_canvas_key(KEY_P, SHIFT, "P"));
        assert!(pin.target_prompt_preview().starts_with("P "));
    }
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        assert!(pin.as_mut().send_canvas_key(KEY_5, KEYPAD, "5"));
    }
}

#[test]
fn chart_pane_shortcut_modifiers_never_open_prompt() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _ctx = setup_chart_pane_with_context(&mut probe);
        let mut pin = probe.pin_mut();
        pin.as_mut().focus_canvas();
        assert!(!pin.as_mut().send_canvas_key(KEY_P, CONTROL, "p"));
        assert!(!pin.as_mut().send_canvas_key(KEY_P, CONTROL | SHIFT, "P"));
        assert!(!pin.target_prompt_open());
    }
}

const T0: i64 = 1_789_725_600_000;

/// Bar `i` has unique prices that need three decimals: O 10+i, H 10.5+i, L 9.25+i,
/// C 10.125+i.
unsafe fn setup_chart_pane_with_known_bars(
    probe: &mut cxx::UniquePtr<chart::ChartPaneProbe>,
    bar_count: i64,
) -> *mut chart::BarFeed {
    let feed = chart::make_test_feed();
    chart::feed_set_symbol(feed, "PETR4");
    chart::feed_set_timeframe(feed, "1m", 60_000);
    for i in 0..bar_count {
        push_known_bar(feed, i, false);
    }
    let mut pin = probe.pin_mut();
    pin.as_mut().set_size(640.0, 360.0);
    pin.as_mut().set_feed(feed);
    chart::process_events();
    feed
}

unsafe fn push_known_bar(feed: *mut chart::BarFeed, i: i64, forming: bool) {
    let base = i as f64;
    bar_feed::apply_test_bar(
        feed as *mut bar_feed::ffi::BarFeed,
        T0 + i * 60_000,
        10.0 + base,
        10.5 + base,
        9.25 + base,
        10.125 + base,
        forming,
    );
    chart::process_events();
}

#[test]
fn chart_pane_readout_shows_exact_bar_after_pan_and_zoom() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _feed = setup_chart_pane_with_known_bars(&mut probe, 30);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(10);
        chart::process_events();

        pin.as_mut().pan_bars(-5);
        pin.as_mut().hover_canvas(1.0, 100.0);
        assert_eq!(pin.active_bar_index(), 15);
        assert_eq!(
            pin.readout_time(),
            bar_feed::format_bar_time(T0 + 15 * 60_000)
        );
        assert_eq!(pin.readout_open(), "25.000");
        assert_eq!(pin.readout_high(), "25.500");
        assert_eq!(pin.readout_low(), "24.250");
        assert_eq!(pin.readout_close(), "25.125");

        // Zooming in anchored at the left edge keeps bar 15 under the pointer.
        pin.as_mut().zoom_at(0.0, 1);
        pin.as_mut().hover_canvas(1.0, 100.0);
        assert_eq!(pin.active_bar_index(), 15);
        assert_eq!(pin.readout_close(), "25.125");
        assert_eq!(pin.navigation_mode(), "inspecting");
    }
}

#[test]
fn chart_pane_readout_item_shows_pointer_price() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _feed = setup_chart_pane_with_known_bars(&mut probe, 20);
        let mut pin = probe.pin_mut();
        pin.as_mut().hover_canvas(100.0, 150.0);
        let pointer = pin.readout_pointer_price();
        assert!(!pointer.is_empty());
        // Uses the feed's three decimals, not floating-point noise.
        assert_eq!(pointer.split('.').nth(1).map(str::len), Some(3));
        assert_eq!(pin.readout_item_pointer_price(), pointer);
    }
}

#[test]
fn chart_pane_forming_ticks_widen_price_axis() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let feed = setup_chart_pane_with_known_bars(&mut probe, 10);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        chart::process_events();
        assert!(pin.high_price() < 25.0);

        let time = T0 + 10 * 60_000;
        bar_feed::apply_test_bar(
            feed as *mut bar_feed::ffi::BarFeed,
            time,
            19.0,
            30.0,
            18.0,
            29.0,
            true,
        );
        chart::process_events();
        assert!(pin.high_price() >= 30.0);

        bar_feed::apply_test_bar(
            feed as *mut bar_feed::ffi::BarFeed,
            time,
            19.0,
            40.0,
            18.0,
            39.0,
            true,
        );
        chart::process_events();
        assert!(pin.high_price() >= 40.0);
        assert_eq!(pin.navigation_mode(), "following");
    }
}

#[test]
fn chart_pane_keyboard_selection_survives_new_bars() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let feed = setup_chart_pane_with_known_bars(&mut probe, 20);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(10);
        chart::process_events();
        pin.as_mut().focus_canvas();
        pin.as_mut().select_adjacent_bar(-1);
        assert_eq!(pin.active_bar_index(), 19);

        push_known_bar(feed, 20, false);
        assert_eq!(pin.active_bar_index(), 19);
        assert_eq!(pin.readout_close(), "29.125");
    }
}

#[test]
fn chart_pane_target_change_returns_to_following_live() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let feed = setup_chart_pane_with_known_bars(&mut probe, 30);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(10);
        chart::process_events();
        pin.as_mut().pan_bars(-5);
        assert_eq!(pin.navigation_mode(), "inspecting");

        chart::feed_set_symbol(feed, "VALE3");
        chart::process_events();
        assert_eq!(pin.navigation_mode(), "following");
    }
}

#[test]
fn chart_pane_draws_bars_after_retarget_without_resize() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let feed = setup_chart_pane_with_known_bars(&mut probe, 20);
        assert!(probe.result().vertex_count > 0);

        let generation = chart::feed_target_generation(feed) + 1;
        chart::feed_retarget(feed, "VALE3", "1m", generation);
        chart::process_events();
        for i in 0..20 {
            push_known_bar(feed, i, false);
        }
        // Same pane size: nothing resizes the chart item after the feed swaps its series.
        assert!(probe.result().vertex_count > 0);
    }
}

#[test]
fn chart_pane_real_wheel_events_zoom_around_the_pointer() {
    let _guard = setup();
    unsafe {
        let mut probe = chart::make_chart_pane_probe();
        let _feed = setup_chart_pane_with_known_bars(&mut probe, 100);
        let mut pin = probe.pin_mut();
        pin.as_mut().set_bars_visible(20);
        chart::process_events();
        assert_eq!((pin.first_bar(), pin.last_bar()), (80, 100));

        // One notch up at the plot's left edge zooms in and keeps the leftmost bar.
        pin.as_mut().send_wheel(1.0, 100.0, 120);
        assert_eq!((pin.first_bar(), pin.last_bar()), (80, 96));
        assert_eq!(pin.navigation_mode(), "inspecting");

        // Touchpad-sized deltas accumulate: three 40-unit events make one step out.
        for _ in 0..3 {
            pin.as_mut().send_wheel(1.0, 100.0, -40);
        }
        assert_eq!(pin.last_bar() - pin.first_bar(), 20);
    }
}
