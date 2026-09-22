//! Q-055: chart identity strip and freshness labels.

pub use q_terminal::{
    bridge, chart_bridge, chart_context, chart_target, config, contracts_stream, execution,
    execution_controls, execution_models, history, ops_session, ops_status, startup, stream,
};

use chart_context::{
    compute_condition, format_last_bar_time, source_label, symbol_line, ChartContextRust,
};

#[test]
fn format_last_bar_time_unavailable_for_zero() {
    assert_eq!(format_last_bar_time(0), "Last bar unavailable");
    assert_eq!(format_last_bar_time(-1), "Last bar unavailable");
}

#[test]
fn format_last_bar_time_includes_utc() {
    let label = format_last_bar_time(1_728_000_000);
    assert!(label.ends_with("UTC"));
    assert!(label.contains("2024"));
}

#[test]
fn connected_socket_with_old_bar_is_stale_not_live() {
    let mut feed = bridge::bar_feed::BarFeedRust::new("PETR4", "1m");
    feed.set_connection_state("live");
    feed.set_data_age_ms(120_000);
    feed.last_time = 1_728_000_000;
    feed.bar_count = 10;
    let (label, role) = compute_condition(&feed);
    assert_eq!(label, "Stale 2m 0s");
    assert_eq!(role, "stale");
}

#[test]
fn feed_without_timestamp_never_reports_live() {
    let mut feed = bridge::bar_feed::BarFeedRust::new("PETR4", "1m");
    feed.set_connection_state("live");
    feed.data_age_ms = -1;
    feed.last_time = 0;
    let (label, _) = compute_condition(&feed);
    assert_ne!(label, "Live");
    assert_eq!(format_last_bar_time(feed.last_time), "Last bar unavailable");
}

#[test]
fn source_label_configured_and_following() {
    let (configured, tip) = source_label(None);
    assert_eq!(configured, "Configured");
    assert!(tip.is_empty());
    let (following, tip) = source_label(Some("momentum-alpha"));
    assert_eq!(following, "Following: momentum-alpha");
    assert_eq!(tip, "momentum-alpha");
}

#[test]
fn symbol_line_switching_uses_pending_target() {
    let line = symbol_line("PETR4", "1m", true, "VALE3", "5m");
    assert_eq!(line, "Switching to VALE3 · 5m");
    let settled = symbol_line("VALE3", "5m", false, "VALE3", "5m");
    assert_eq!(settled, "VALE3 · 5m · Candles");
}

#[test]
fn chart_context_transitions_configured_to_following_and_back() {
    let mut ctx = ChartContextRust::default();
    ctx.set_configured("PETR4", "1m");
    ctx.on_target("alpha", "VALE3", "5m", true);
    assert!(ctx.is_switching);
    let (source, _) = source_label(ctx.following_name());
    assert_eq!(source, "Following: alpha");

    ctx.on_target("", "PETR4", "1m", false);
    let (source, _) = source_label(ctx.following_name());
    assert_eq!(source, "Configured");
}

#[test]
fn chart_identity_probe_shows_configured_and_following_targets() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();

    unsafe {
        let feed = chart_bridge::make_test_feed();
        chart_bridge::feed_set_symbol(feed, "PETR4");
        chart_bridge::feed_set_timeframe(feed, "1m", 60_000);
        chart_bridge::feed_set_connection_state(feed, "live");
        chart_bridge::feed_set_data_age_ms(feed, 0);
        chart_bridge::feed_set_bar_count(feed, 5);

        let ctx = chart_bridge::make_test_chart_context() as *mut chart_context::ffi::ChartContext;
        let feed_ffi = feed as *mut bridge::bar_feed::ffi::BarFeed;
        chart_context::bind_feed(ctx, feed_ffi);
        chart_context::set_configured(ctx, "PETR4", "1m");
        chart_context::sync_ptr(ctx);

        let mut probe = chart_bridge::make_chart_identity_probe();
        let mut pin = probe.pin_mut();
        pin.as_mut()
            .set_context(ctx as *mut chart_bridge::chart::ChartContext);
        pin.as_mut().set_feed(feed);
        let configured = pin.result();
        assert_eq!(configured.symbol_line, "PETR4 · 1m · Candles");
        assert_eq!(configured.source_label, "Configured");
        assert_eq!(configured.condition_label, "Loading");

        chart_context::notify_target(
            ctx,
            Some(("momentum-alpha", "VALE3", "5m", true)),
            ("PETR4", "1m"),
        );
        chart_context::notify_retarget(ctx, "VALE3", "5m");
        chart_bridge::feed_set_symbol(feed, "VALE3");
        chart_bridge::feed_set_timeframe(feed, "5m", 300_000);
        chart_bridge::feed_set_bar_count(feed, 0);
        chart_context::sync_ptr(ctx);
        pin.as_mut().set_feed(feed);
        chart_bridge::process_events();
        let following = pin.result();
        assert_eq!(following.source_label, "Following: momentum-alpha");
        assert!(following.is_switching);
    }
}
