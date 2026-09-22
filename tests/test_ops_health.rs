#![allow(clippy::float_cmp)]

pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use execution::health::HealthPoller;
use ops_status::OpsStatusRust;
use stream::fake_server::FakeServer;

/// Acceptance Criterion 5 & Step 4:
/// The poller issues one health request and one positions request every two seconds,
/// and they are the only periodic requests.
#[tokio::test]
async fn test_poller_cadence_against_fake_server() {
    let server = FakeServer::start().await;
    let poller = Arc::new(HealthPoller::new(&server.api_base()));

    let update_count = Arc::new(AtomicUsize::new(0));
    let cnt = update_count.clone();

    poller.start(move |_| {
        cnt.fetch_add(1, Ordering::SeqCst);
    });

    // Wait 2.2 seconds (initial poll + 1st tick at 2s = 2 polls)
    tokio::time::sleep(Duration::from_millis(2200)).await;
    poller.stop();

    let h_calls = server.health_calls();
    let p_calls = server.positions_calls();

    // Verify both health and positions endpoints were called at each tick
    assert!(h_calls >= 2, "expected >= 2 health calls, got {h_calls}");
    assert_eq!(
        h_calls, p_calls,
        "health calls ({h_calls}) must equal positions calls ({p_calls})"
    );

    // Verify no unexpected endpoints were hit periodically
    // (FakeServer tracks all REST calls in rest_calls)
    assert_eq!(
        server.rest_calls(),
        h_calls + p_calls,
        "only health and positions should be called periodically"
    );
}

/// Acceptance Criterion 5:
/// Multi-tick cadence test: verifies health and positions requests are executed in lockstep
/// and remain the only periodic requests across 30 ticks.
#[tokio::test]
async fn test_poller_cadence_multi_tick_lockstep() {
    let server = FakeServer::start().await;
    let poller = Arc::new(HealthPoller::with_interval(
        &server.api_base(),
        Duration::from_millis(15),
    ));

    let update_count = Arc::new(AtomicUsize::new(0));
    let cnt = update_count.clone();

    poller.start(move |_| {
        cnt.fetch_add(1, Ordering::SeqCst);
    });

    // Wait for at least 30 updates
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(15)).await;
        if update_count.load(Ordering::SeqCst) >= 30 {
            break;
        }
    }
    poller.stop();

    let h_calls = server.health_calls();
    let p_calls = server.positions_calls();

    assert!(
        h_calls >= 30,
        "expected at least 30 health calls, got {h_calls}"
    );
    assert_eq!(
        h_calls, p_calls,
        "health calls ({h_calls}) must equal positions calls ({p_calls})"
    );
    assert_eq!(
        server.rest_calls(),
        h_calls + p_calls,
        "only health and positions should be called periodically"
    );
}

/// Acceptance Criterion 4:
/// §8.1 State: Postgres down
/// 503 response from health marks API degraded and postgres unavailable.
#[tokio::test]
async fn test_degraded_state_postgres_down() {
    let server = FakeServer::start().await;
    server.set_health_503(true);

    let client = reqwest::Client::new();
    let update = HealthPoller::poll_step(&client, &server.api_base()).await;

    assert!(update.api_degraded, "api_degraded should be true on 503");
    assert!(
        !update.postgres_available,
        "postgres_available should be false on 503"
    );

    let mut ops = OpsStatusRust::default();
    assert!(ops.postgres_available);
    ops.mark_postgres_down();

    assert_eq!(ops.api_status.to_string(), "degraded");
    assert!(!ops.postgres_available);
}

/// Acceptance Criterion 4:
/// §8.1 State: API offline
/// Unreachable API marks API offline, worker status unknown, postgres unavailable.
#[tokio::test]
async fn test_degraded_state_api_offline() {
    let client = reqwest::Client::new();
    // Port 1 is not accepting connections
    let update = HealthPoller::poll_step(&client, "http://127.0.0.1:1").await;

    assert!(update.api_offline, "api_offline should be true");
    assert!(
        !update.postgres_available,
        "postgres_available should be false"
    );

    let mut ops = OpsStatusRust::default();
    ops.mark_api_offline();

    assert_eq!(ops.api_status.to_string(), "offline");
    assert_eq!(ops.worker_status.to_string(), "unknown");
    assert!(!ops.postgres_available);
}

/// Acceptance Criterion 4:
/// §8.1 State: Execution worker down
/// Health response reports worker_status: "down" and stale heartbeat age.
#[test]
fn test_degraded_state_worker_down_and_stale() {
    let mut ops = OpsStatusRust::default();

    let health_json = serde_json::json!({
        "api_status": "ok",
        "checked_at": "2026-09-19T03:00:00Z",
        "worker_status": "down",
        "worker_heartbeat_age_s": 45.2,
        "kill_switch_enabled": false,
        "live_capability_locked": true,
        "market_data_status": "streaming",
        "unknown_order_count": 0,
        "edge": {
            "reachable": true,
            "mt5_connected": true,
            "terminal_build": 4150
        }
    })
    .to_string();

    ops.apply_health_str(&health_json);

    assert_eq!(ops.api_status.to_string(), "ok");
    assert_eq!(ops.worker_status.to_string(), "down");
    assert_eq!(ops.worker_heartbeat_age_s, 45.2);
    assert!(ops.edge_reachable);
    assert!(ops.edge_mt5_connected);
}

#[test]
fn missing_worker_heartbeat_is_explicitly_unknown() {
    let mut ops = OpsStatusRust::default();
    ops.apply_health_str(
        &serde_json::json!({
            "api_status": "ok",
            "worker_status": "active",
            "edge": { "reachable": true, "mt5_connected": true, "terminal_build": 4150 }
        })
        .to_string(),
    );

    assert!(
        !ops.worker_heartbeat_known,
        "a missing heartbeat must not be represented as a fresh 0.0s heartbeat"
    );
}

/// Acceptance Criterion 4:
/// §8.1 State: MT5 edge disconnected
/// Health response reports edge unreachable or mt5_connected false, quotes stale.
#[test]
fn test_degraded_state_edge_or_terminal_down() {
    let mut ops = OpsStatusRust::default();

    let health_json = serde_json::json!({
        "api_status": "ok",
        "checked_at": "2026-09-19T03:00:00Z",
        "worker_status": "active",
        "worker_heartbeat_age_s": 0.4,
        "kill_switch_enabled": false,
        "live_capability_locked": false,
        "market_data_status": "stale",
        "unknown_order_count": 0,
        "edge": {
            "reachable": true,
            "mt5_connected": false,
            "terminal_build": 4150
        }
    })
    .to_string();

    ops.apply_health_str(&health_json);

    assert_eq!(ops.api_status.to_string(), "ok");
    assert_eq!(ops.worker_status.to_string(), "active");
    assert!(ops.edge_reachable);
    assert!(!ops.edge_mt5_connected);
    assert_eq!(ops.market_data_status.to_string(), "stale");
}

/// Acceptance Criterion 4:
/// §8.1 State: Stream disconnected with last state kept and stream age.
#[test]
fn test_degraded_state_stream_disconnected_with_age() {
    let mut ops = OpsStatusRust::default();
    ops.set_stream_info("DISCONNECTED", 15.5);

    assert_eq!(ops.stream_state.to_string(), "DISCONNECTED");
    assert_eq!(ops.stream_age_s, 15.5);

    // Reconnecting state
    ops.set_stream_info("RECONNECTING", 16.0);
    assert_eq!(ops.stream_state.to_string(), "RECONNECTING");
    assert_eq!(ops.stream_age_s, 16.0);
}

/// Acceptance Criterion 4:
/// Position marks and unrealized PnL JSON applied to OpsStatus.
#[test]
fn test_positions_marks_and_pnl_applied() {
    let mut ops = OpsStatusRust::default();

    let pos_json = serde_json::json!([
        {
            "deployment_id": "dep-1",
            "symbol": "PETR4",
            "quantity": "100",
            "side": "buy",
            "average_entry_price": "25.50",
            "mark_price": "26.00",
            "unrealized_pnl": "50.00"
        }
    ])
    .to_string();

    ops.apply_positions_str(&pos_json);
    assert_eq!(ops.positions_pnl_json.to_string(), pos_json);
}
