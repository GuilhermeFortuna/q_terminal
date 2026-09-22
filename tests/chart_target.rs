//! Q-049 criterion 1: selecting a deployment retargets the chart, and no bar of the
//! previous symbol is drawn after the switch, including a switch in the middle of a load
//! or a burst of stream frames.
#![allow(clippy::await_holding_lock)]

pub use q_terminal::{
    bridge, chart_bridge, chart_context, chart_target, config, contracts_stream, execution,
    execution_controls, execution_models, history, ops_session, ops_status, startup, stream,
};

use std::time::Duration;

use bridge::bar_feed::{make_bar_columns, BarDelivery, BarFeedRust};
use config::Config;
use stream::fake_server::FakeServer;
use stream::topic_state::BarColumns;

fn bar(t: i64, close: f64) -> BarColumns {
    BarColumns::single(t, close - 0.5, close + 0.5, close - 1.0, close, 100.0)
}

fn feed_times(feed: *mut chart_bridge::BarFeed) -> Vec<i64> {
    unsafe {
        (0..chart_bridge::feed_bar_times_len(feed))
            .map(|i| chart_bridge::feed_bar_time_at(feed, i))
            .collect()
    }
}

fn symbol(feed: *mut chart_bridge::BarFeed) -> String {
    unsafe { chart_bridge::feed_symbol(feed) }
}

async fn until(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..200 {
        chart_bridge::process_events();
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn stale_generation_is_dropped_and_counted() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    feed.history_controller().open_gate();
    assert!(feed.apply_stamped(
        0,
        BarDelivery::Completed(make_bar_columns(60, 1.0, 2.0, 0.5, 1.5))
    ));
    assert_eq!(feed.series.bar_count, 1);

    feed.reset_for_target("VALE3", "1m", 1);
    assert_eq!(feed.series.bar_count, 0);
    assert!(feed.bar_times.is_empty());
    feed.history_controller().open_gate();

    // Bars and history results of the previous target arrive late.
    assert!(!feed.apply_stamped(
        0,
        BarDelivery::Completed(make_bar_columns(120, 1.0, 2.0, 0.5, 1.5))
    ));
    let loaded = history::Loaded {
        bars: bridge::bar_feed::empty_bar_columns(),
        source: history::Source::None,
        dataset: None,
        shortfall: 0,
        reason: None,
    };
    assert!(!feed.complete_history_load_for(0, loaded));
    assert_eq!(feed.stale_dropped, 2);
    assert_eq!(feed.series.bar_count, 0);

    assert!(feed.apply_stamped(
        1,
        BarDelivery::Completed(make_bar_columns(180, 1.0, 2.0, 0.5, 1.5))
    ));
    assert_eq!(feed.bar_times, vec![180]);
}

#[tokio::test(flavor = "current_thread")]
async fn switch_shows_only_the_new_symbol_even_mid_burst() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    // Each symbol has its own keyed snapshot; the unkeyed one is the last sent.
    server
        .send_bar("bars.completed", 300, "VALE3", "1m", bar(9_000, 50.0))
        .await;
    server
        .send_bar("bars.completed", 100, "PETR4", "1m", bar(1_000, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 100, bar(1_060, 10.2))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().expect("targeter").clone();

    until("PETR4 bars", || feed_times(feed).contains(&1_000)).await;
    let gen_before = unsafe { chart_bridge::feed_target_generation(feed) };

    // Switch while PETR4 frames are still arriving.
    for i in 0..20 {
        server
            .send_bar(
                "bars.completed",
                101 + i,
                "PETR4",
                "1m",
                bar(2_000 + i * 60, 11.0),
            )
            .await;
    }
    let target = targeter
        .select(Some((
            "dep-2".into(),
            "dep-2".into(),
            "VALE3".into(),
            "1m".into(),
        )))
        .expect("target changed");
    assert_eq!(target.generation as i64, gen_before + 1);
    assert_eq!(symbol(feed), "VALE3");
    assert!(feed_times(feed).is_empty(), "previous bars dropped at once");
    for i in 20..40 {
        server
            .send_bar(
                "bars.completed",
                101 + i,
                "PETR4",
                "1m",
                bar(2_000 + i * 60, 11.0),
            )
            .await;
    }

    until("VALE3 bars", || feed_times(feed).contains(&9_000)).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    chart_bridge::process_events();
    let times = feed_times(feed);
    assert!(
        times.iter().all(|t| *t >= 9_000),
        "no PETR4 bar after the switch: {times:?}"
    );

    // Same target again changes nothing; None goes back to the configured symbol.
    assert!(targeter
        .select(Some((
            "dep-2".into(),
            "dep-2".into(),
            "VALE3".into(),
            "1m".into()
        )))
        .is_none());
    assert!(targeter.select(None).is_some());
    assert_eq!(symbol(feed), "PETR4");
    until("PETR4 back", || !feed_times(feed).is_empty()).await;
    assert!(feed_times(feed).iter().all(|t| *t < 9_000));
}

#[test]
fn disconnect_keeps_bars_and_reports_disconnected_condition() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    feed.history_controller().open_gate();
    assert!(feed.apply_stamped(
        0,
        BarDelivery::Completed(make_bar_columns(60, 1.0, 2.0, 0.5, 1.5))
    ));
    feed.set_connection_state("unavailable");
    feed.last_time = 1_728_000_000;
    let (label, role) = chart_context::compute_condition(&feed);
    assert_eq!(
        feed.series.bar_count, 1,
        "last rendered candles stay visible"
    );
    assert_eq!(label, "Disconnected");
    assert_eq!(role, "critical");
}

#[tokio::test(flavor = "current_thread")]
async fn following_to_manual_switches_mode_and_retargets() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    server
        .send_bar("bars.completed", 100, "PETR4", "1m", bar(1_000, 10.0))
        .await;
    server
        .send_bar("bars.completed", 200, "VALE3", "1m", bar(2_000, 20.0))
        .await;
    server
        .send_bar("bars.completed", 300, "ITUB4", "5m", bar(3_000, 30.0))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().expect("targeter").clone();

    until("initial PETR4", || feed_times(feed).contains(&1_000)).await;
    assert_eq!(targeter.mode(), chart_target::ChartMode::Following);
    assert!(targeter.is_following());

    // Switch to deployment while following
    let target = targeter
        .select(Some((
            "dep-1".into(),
            "dep-alpha".into(),
            "VALE3".into(),
            "1m".into(),
        )))
        .expect("target changed");
    assert_eq!(target.symbol, "VALE3");
    assert_eq!(targeter.mode(), chart_target::ChartMode::Following);

    until("VALE3 bars", || feed_times(feed).contains(&2_000)).await;

    // Operator requests manual target: switches to Manual mode and retargets
    let manual_target = targeter
        .request_manual("ITUB4", "5m")
        .expect("valid target")
        .expect("target changed");
    assert_eq!(manual_target.symbol, "ITUB4");
    assert_eq!(manual_target.timeframe, "5m");
    assert_eq!(targeter.mode(), chart_target::ChartMode::Manual);
    assert!(targeter.is_manual());
    assert_eq!(symbol(feed), "ITUB4");
    assert!(
        feed_times(feed).is_empty(),
        "previous bars dropped immediately"
    );
    assert_eq!(unsafe { chart_bridge::feed_marker_count(feed) }, 0);

    until("ITUB4 bars", || feed_times(feed).contains(&3_000)).await;
}

#[tokio::test(flavor = "current_thread")]
async fn deployment_change_during_manual_does_not_retarget() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    server
        .send_bar("bars.completed", 100, "PETR4", "1m", bar(1_000, 10.0))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().expect("targeter").clone();

    until("initial PETR4", || feed_times(feed).contains(&1_000)).await;

    // Enter manual mode
    targeter
        .request_manual("VALE3", "5m")
        .expect("valid target")
        .expect("target changed");
    assert_eq!(targeter.mode(), chart_target::ChartMode::Manual);
    assert_eq!(symbol(feed), "VALE3");
    let gen_before = targeter.generation();

    // Deployment selection changes while in manual mode
    let res = targeter.select(Some((
        "dep-2".into(),
        "dep-beta".into(),
        "BOVA11".into(),
        "15m".into(),
    )));
    assert!(res.is_none(), "manual mode suppresses deployment retarget");
    assert_eq!(targeter.mode(), chart_target::ChartMode::Manual);
    assert_eq!(symbol(feed), "VALE3", "chart remains on manual symbol");
    assert_eq!(
        targeter.generation(),
        gen_before,
        "generation does not advance"
    );
    assert_eq!(
        targeter.following_target(),
        Some((
            "dep-2".into(),
            "dep-beta".into(),
            "BOVA11".into(),
            "15m".into()
        )),
        "remembers the currently selected deployment"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn manual_to_following_restores_selected_deployment() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    server
        .send_bar("bars.completed", 100, "PETR4", "1m", bar(1_000, 10.0))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().expect("targeter").clone();

    until("initial PETR4", || feed_times(feed).contains(&1_000)).await;

    // Enter manual mode
    targeter
        .request_manual("VALE3", "5m")
        .expect("valid target");

    // Select deployment while in manual mode
    targeter.select(Some((
        "dep-2".into(),
        "dep-beta".into(),
        "BOVA11".into(),
        "15m".into(),
    )));
    assert_eq!(symbol(feed), "VALE3");

    // Return to following mode: immediately retargets to current global deployment
    let target = targeter
        .follow_deployment()
        .expect("retargets to deployment");
    assert_eq!(targeter.mode(), chart_target::ChartMode::Following);
    assert_eq!(target.symbol, "BOVA11");
    assert_eq!(target.timeframe, "15m");
    assert_eq!(symbol(feed), "BOVA11");

    // When deployment is unselected, returns to configured pair
    let target_none = targeter.select(None).expect("falls back to configured");
    assert_eq!(target_none.symbol, "PETR4");
    assert_eq!(target_none.timeframe, "1m");
    assert_eq!(symbol(feed), "PETR4");
    until("PETR4 back", || !feed_times(feed).is_empty()).await;
}

#[tokio::test(flavor = "current_thread")]
async fn rapid_target_changes_drop_interleaved_stale_bars() {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    chart_bridge::register_chart_types();

    let server = FakeServer::start().await;
    server
        .send_bar("bars.completed", 100, "PETR4", "1m", bar(1_000, 10.0))
        .await;
    server
        .send_bar("bars.completed", 200, "SYM1", "1m", bar(2_000, 20.0))
        .await;
    server
        .send_bar("bars.completed", 300, "SYM2", "5m", bar(3_000, 30.0))
        .await;
    server
        .send_bar("bars.completed", 400, "SYM3", "15m", bar(4_000, 40.0))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    let feed = ctx.feed_ptr;
    let targeter = ctx.targeter.as_ref().expect("targeter").clone();

    until("initial PETR4", || feed_times(feed).contains(&1_000)).await;

    // Rapid target changes
    let t1 = targeter.request_manual("SYM1", "1m").unwrap().unwrap();
    let t2 = targeter.request_manual("SYM2", "5m").unwrap().unwrap();
    let t3 = targeter.request_manual("SYM3", "15m").unwrap().unwrap();

    assert_eq!(t1.generation + 1, t2.generation);
    assert_eq!(t2.generation + 1, t3.generation);
    assert_eq!(symbol(feed), "SYM3");

    until("SYM3 bars", || feed_times(feed).contains(&4_000)).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    chart_bridge::process_events();

    let times = feed_times(feed);
    assert!(times.contains(&4_000));
    assert!(!times.contains(&2_000), "stale SYM1 bars dropped");
    assert!(!times.contains(&3_000), "stale SYM2 bars dropped");
}

#[test]
fn delayed_old_generation_history_result_is_dropped() {
    let mut feed = BarFeedRust::new("SYM1", "1m");
    feed.history_controller().open_gate();

    // Settled on Gen 1
    feed.reset_for_target("SYM1", "1m", 1);
    assert_eq!(feed.series.bar_count, 0);

    // Retarget to Gen 2
    feed.reset_for_target("SYM2", "5m", 2);
    assert_eq!(feed.series.bar_count, 0);

    // Delayed Gen 1 history arrives
    let gen1_loaded = history::Loaded {
        bars: bridge::bar_feed::make_bar_columns(100, 10.0, 11.0, 9.0, 10.5),
        source: history::Source::Lake,
        dataset: None,
        shortfall: 0,
        reason: None,
    };
    assert!(!feed.complete_history_load_for(1, gen1_loaded));
    assert_eq!(feed.stale_dropped, 1);
    assert_eq!(feed.series.bar_count, 0, "Gen 1 bars not added to Gen 2");

    // Gen 2 history arrives
    let gen2_loaded = history::Loaded {
        bars: bridge::bar_feed::make_bar_columns(200, 20.0, 21.0, 19.0, 20.5),
        source: history::Source::Lake,
        dataset: None,
        shortfall: 0,
        reason: None,
    };
    assert!(feed.complete_history_load_for(2, gen2_loaded));
    assert_eq!(feed.series.bar_count, 1);
    assert_eq!(feed.bar_times, vec![200]);
}

#[test]
fn invalid_target_input_is_rejected_with_attempted_pair_named() {
    let err_empty = chart_target::validate_target("", "1m").unwrap_err();
    assert!(err_empty.contains(" · 1m"));
    assert!(err_empty.contains("empty"));

    let err_sym = chart_target::validate_target("BAD SYM", "1m").unwrap_err();
    assert!(err_sym.contains("'BAD SYM · 1m'"));
    assert!(err_sym.contains("invalid symbol characters"));

    let err_tf = chart_target::validate_target("PETR4", "99xyz").unwrap_err();
    assert!(err_tf.contains("'PETR4 · 99xyz'"));
    assert!(err_tf.contains("unsupported timeframe format"));

    let valid = chart_target::validate_target("petr4", " 15m ").unwrap();
    assert_eq!(valid, ("PETR4".to_string(), "15m".to_string()));
}
