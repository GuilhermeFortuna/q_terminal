#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
pub mod bridge;
#[path = "../src/config.rs"]
pub mod config;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use config::Config;
use std::time::Duration;
use stream::client::{ConnectionState, StreamClient};
use stream::fake_server::FakeServer;
use stream::sink::BarSink;
use stream::topic_state::BarColumns;

fn dummy_bar(t: i64, close: f64) -> BarColumns {
    BarColumns::single(t, 10.0, 11.0, 9.0, close, 100.0)
}

#[tokio::test]
async fn test_connect_snapshot_live_happy_path() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 100, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 101, dummy_bar(120, 10.5))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink.clone());

    // Wait until Live
    let mut live = false;
    for _ in 0..50 {
        if client.connection_state() == ConnectionState::Live {
            live = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(live, "client should transition to Live state");

    // Deliver a live completed bar and forming bar
    server
        .send_bar("bars.completed", 101, "PETR4", "1m", dummy_bar(180, 11.0))
        .await;
    server
        .send_bar("bars.forming", 102, "PETR4", "1m", dummy_bar(240, 11.5))
        .await;

    // Wait for deliveries to reach sink
    for _ in 0..50 {
        if client.counters().applied >= 4 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let deliveries = sink.drain();
    assert!(!deliveries.is_empty(), "sink should have received bars");
    assert!(client.counters().applied >= 4);

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_completed_gap_closed_from_history() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
        .await;

    // Server has gap: missing seq 11 in stream, but available in REST history
    server
        .add_history_entry("bars.completed", 11, dummy_bar(120, 10.2))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink.clone());

    // Wait until Live
    for _ in 0..50 {
        if client.connection_state() == ConnectionState::Live {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Now send seq 12, creating a gap since last seq was 10
    server
        .send_bar("bars.completed", 12, "PETR4", "1m", dummy_bar(180, 10.4))
        .await;

    // Client should fetch history for seq 11, close gap, and apply 11 then 12
    for _ in 0..50 {
        if client.counters().gaps_closed >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        client.counters().gaps_closed >= 1,
        "gap should have been closed from history"
    );

    let deliveries = sink.drain();
    assert!(
        deliveries.len() >= 3,
        "should apply snapshot + history 11 + entry 12"
    );

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_rejected_topic_leaves_other_working() {
    let server = FakeServer::start().await;
    server.set_reject_forming(true);
    server
        .set_snapshot("bars.completed", 5, dummy_bar(60, 10.0))
        .await;

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink.clone());

    // Wait until Live or error recorded
    for _ in 0..50 {
        if !client.last_error().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(client.last_error(), "unsupported_timeframe");

    // bars.completed should continue working!
    server
        .send_bar("bars.completed", 6, "PETR4", "1m", dummy_bar(120, 10.1))
        .await;

    for _ in 0..50 {
        if client.counters().applied >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        client.counters().applied >= 2,
        "completed bars should still be applied even if forming is rejected"
    );

    client.shutdown();
    server.shutdown().await;
}
