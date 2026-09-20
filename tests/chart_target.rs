//! Q-049 criterion 1: selecting a deployment retargets the chart, and no bar of the
//! previous symbol is drawn after the switch, including a switch in the middle of a load
//! or a burst of stream frames.
#![allow(clippy::await_holding_lock)]

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/chart_bridge.rs"]
pub mod chart_bridge;
#[path = "../src/chart_target.rs"]
pub mod chart_target;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/execution/mod.rs"]
pub mod execution;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/startup.rs"]
pub mod startup;
#[path = "../src/stream/mod.rs"]
pub mod stream;

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
        .select(Some(("VALE3".into(), "1m".into())))
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
        .select(Some(("VALE3".into(), "1m".into())))
        .is_none());
    assert!(targeter.select(None).is_some());
    assert_eq!(symbol(feed), "PETR4");
    until("PETR4 back", || !feed_times(feed).is_empty()).await;
    assert!(feed_times(feed).iter().all(|t| *t < 9_000));
}
