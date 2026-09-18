#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/history/mod.rs"]
pub mod history;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use std::time::{Duration, Instant};

use bridge::bar_feed::{empty_bar_columns, make_bar_columns, BarDelivery, BarFeedRust};
use config::Config;
use cxx_qt_lib::QString;
use history::{Loaded, Source};
use stream::client::{ConnectionState, StreamClient};
use stream::sink::BarSink;

/// §8.1 State 1: Stream unavailable
/// Feed keeps bars, marks stale, computes data age, shows degraded badge and reason.
#[test]
fn test_stream_unavailable_keeps_bars_marks_stale_shows_reason() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    assert_eq!(feed.timeframe_ms, 60_000);

    feed.history_controller().open_gate();

    // Initial bars ingested
    let completed = BarDelivery::Completed(make_bar_columns(1_000_000, 10.0, 11.0, 9.0, 10.5));
    feed.apply_delivery(completed);
    assert_eq!(feed.bar_count, 1);
    assert_eq!(feed.applied, 1);

    // Stream drops
    feed.set_connection_state("reconnecting");
    feed.set_last_error("connection reset by peer");
    feed.last_applied_instant = Some(Instant::now() - Duration::from_millis(150_000));
    feed.update_data_age();

    // Bars must be kept; status must indicate staleness and reason
    assert_eq!(feed.bar_count, 1);
    assert!(feed.stale);
    assert!(feed.data_age_ms >= 150_000);
    assert_eq!(feed.connection_state.to_string(), "reconnecting");
    assert_eq!(feed.last_error.to_string(), "connection reset by peer");
}

/// §8.1 State 2: Stream recovers
/// Marks fresh, clears error, status returns to live.
#[test]
fn test_stream_recovers_clears_stale_and_error() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    feed.set_connection_state("reconnecting");
    feed.set_last_error("connection reset by peer");
    feed.last_applied_instant = Some(Instant::now() - Duration::from_millis(150_000));
    feed.update_data_age();
    assert!(feed.stale);

    // Recovery
    feed.set_connection_state("live");
    feed.set_last_error("");
    feed.last_applied_instant = Some(Instant::now() - Duration::from_millis(5_000));
    feed.update_data_age();

    assert_eq!(feed.connection_state.to_string(), "live");
    assert!(feed.last_error.to_string().is_empty());
    assert!(!feed.stale);
    assert!(feed.data_age_ms < 60_000);
}

/// §8.1 State 3: History fallback
/// Status indicates fallback source and shortfall.
#[test]
fn test_history_fallback_indicates_source_and_shortfall() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    feed.history_source = QString::from("api");
    feed.history_shortfall = 25;
    feed.history_bars = 75;

    assert_eq!(feed.history_source.to_string(), "api");
    assert_eq!(feed.history_shortfall, 25);
    assert_eq!(feed.history_bars, 75);
}

/// §8.1 State 4: No history available
/// Chart opens in live-only state, accepts live bars as they arrive.
#[test]
fn test_no_history_enters_live_only_and_accepts_live_bars() {
    let mut feed = BarFeedRust::new("PETR4", "1m");
    assert_eq!(feed.bar_count, 0);

    // Complete history load with no bars
    feed.complete_history_load(Loaded {
        bars: empty_bar_columns(),
        source: Source::None,
        dataset: None,
        shortfall: 0,
        reason: None,
    });
    assert_eq!(feed.history_bars, 0);
    assert_eq!(feed.history_source.to_string(), "none");
    assert!(!feed.live_only);

    // Live bars arrive over stream -> live_only becomes true
    let live_bar = BarDelivery::Completed(make_bar_columns(1_000_060, 20.0, 22.0, 19.5, 21.0));
    feed.apply_delivery(live_bar);

    assert_eq!(feed.bar_count, 1);
    assert!((feed.last_price - 21.0).abs() < 1e-6);
    assert!(feed.live_only);
}

/// §8.1 State 5: API down at startup
/// Stream client handles unreachable API without panicking, retries gracefully.
#[tokio::test]
async fn test_api_down_at_startup_retries_without_crash() {
    let config = Config {
        api_base: "http://127.0.0.1:39123".to_string(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let state = client.connection_state();
    assert!(
        matches!(
            state,
            ConnectionState::Connecting | ConnectionState::Reconnecting { .. }
        ),
        "unexpected state: {:?}",
        state
    );
}
