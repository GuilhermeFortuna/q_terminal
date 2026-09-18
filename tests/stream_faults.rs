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
async fn test_expired_history_range_resnapshots() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
        .await;

    // Configure server to return 410 on history
    server.set_history_expired(true);

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

    // Now send gap: seq 15
    server
        .send_bar("bars.completed", 15, "PETR4", "1m", dummy_bar(180, 10.4))
        .await;

    // History fails with 410 -> triggers resnapshot
    for _ in 0..50 {
        if client.counters().resnapshots >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        client.counters().resnapshots >= 1,
        "expired history range must trigger resnapshot"
    );

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_epoch_changed_resnapshots_topic() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
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

    // Send epoch changed for bars.completed
    server.send_epoch_changed("bars.completed", "epoch-2").await;

    for _ in 0..50 {
        if client.counters().resnapshots >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        client.counters().resnapshots >= 1,
        "epoch changed must trigger resnapshot"
    );

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_lagging_and_cursor_expired_resnapshots() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
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

    // Send lagging
    server.send_lagging("bars.completed", 20).await;
    for _ in 0..50 {
        if client.counters().resnapshots >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(client.counters().resnapshots >= 1);

    // Send cursor expired
    server.send_cursor_expired("bars.forming").await;
    for _ in 0..50 {
        if client.counters().resnapshots >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(client.counters().resnapshots >= 2);

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_connect_503_reports_unavailable_and_retries() {
    let server = FakeServer::start().await;
    server.set_deny_503(true);

    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
    };

    let sink = BarSink::new();
    let client = StreamClient::start(config, sink.clone());

    // Wait for client to attempt connection and report Unavailable
    let mut saw_unavailable = false;
    for _ in 0..50 {
        let state = client.connection_state();
        if state == ConnectionState::Unavailable
            || matches!(state, ConnectionState::Reconnecting { .. })
        {
            saw_unavailable = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        saw_unavailable,
        "client should report Unavailable or Reconnecting on 503"
    );

    client.shutdown();
    server.shutdown().await;
}

#[tokio::test]
async fn test_mid_stream_close_reconnects_and_recovers() {
    let server = FakeServer::start().await;
    server
        .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
        .await;
    server
        .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
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
    assert_eq!(client.connection_state(), ConnectionState::Live);

    // Close client connection from server side
    server.close_client().await;

    // Client should reconnect and become Live again!
    let mut reconnected = false;
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if client.connection_state() == ConnectionState::Live {
            reconnected = true;
            break;
        }
    }

    assert!(
        reconnected,
        "client should reconnect and transition back to Live after mid-stream close"
    );

    client.shutdown();
    server.shutdown().await;
}
