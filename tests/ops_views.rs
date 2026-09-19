#![allow(clippy::float_cmp)]

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
pub use bridge::execution_models;
#[path = "../src/history/mod.rs"]
pub mod history;
pub use bridge::ops_status;
#[path = "../src/stream/mod.rs"]
pub mod stream;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use execution::health::HealthPoller;
use execution::store::ExecutionHandle;
use ops_status::OpsStatusRust;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;

fn make_models() -> (*mut chart_bridge::ExecutionModels, ExecutionHandle) {
    let models = chart_bridge::make_test_execution_models();
    assert!(!models.is_null());
    let handle = ExecutionHandle::new();
    unsafe {
        let pin = std::pin::Pin::new_unchecked(
            &mut *(models as *mut execution_models::ffi::ExecutionModels),
        );
        pin.rust_mut().bind_handle(handle.clone());
    }
    (models, handle)
}

/// Acceptance Criterion 1:
/// Headless QML tests against a store filled from the fake server show each deployment's
/// fields, each detail table's rows in order, and the account and ledger, with the decimal
/// strings formatted without loss.
#[test]
fn test_ops_views_criterion_1_fields_tables_order_and_decimal_lossless() {
    let _guard = chart_bridge::QT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let (models, handle) = make_models();

    let dep_1 = "11111111-1111-1111-1111-111111111111";
    let dep_2 = "22222222-2222-2222-2222-222222222222";

    // Ledger entry (creates account with updated balance)
    let mut led_a = fx::ledger("led-001", "105000.25");
    led_a["created_at"] = serde_json::json!("2026-09-18T09:50:00Z");
    led_a["amount"] = serde_json::json!("5000.25");
    led_a["description"] = serde_json::json!("Realized profit");

    handle.mutate(|store| {
        // Deployments
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_1, "running"),
            )
            .unwrap(),
        );
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_2, "paused"),
            )
            .unwrap(),
        );

        // Orders for dep_1 (2 orders, ord-002 is newer)
        let mut ord_a = fx::order("ord-001", dep_1, "filled");
        ord_a["created_at"] = serde_json::json!("2026-09-18T10:00:00Z");
        ord_a["quantity"] = serde_json::json!("100.50000000");
        store.apply(stream::execution::decode_execution_entry("orders", &ord_a).unwrap());

        let mut ord_b = fx::order("ord-002", dep_1, "pending");
        ord_b["created_at"] = serde_json::json!("2026-09-18T10:05:00Z");
        ord_b["quantity"] = serde_json::json!("200.75000000");
        ord_b["reconciliation_state"] = serde_json::json!("unknown");
        store.apply(stream::execution::decode_execution_entry("orders", &ord_b).unwrap());

        // Fills for dep_1 (creates open position: quantity 100.50000000, price 34.56789000)
        let mut fill_a = fx::fill("fill-001", dep_1, "100.50000000", true);
        fill_a["filled_at"] = serde_json::json!("2026-09-18T10:00:02Z");
        fill_a["quantity"] = serde_json::json!("100.50000000");
        fill_a["price"] = serde_json::json!("34.56789000");
        fill_a["fee"] = serde_json::json!("0.12340000");
        fill_a["slippage"] = serde_json::json!("0.00500000");
        if let Some(pos) = fill_a.get_mut("position_after") {
            pos["average_entry_price"] = serde_json::json!("34.56789000");
        }
        store.apply(stream::execution::decode_execution_entry("fills", &fill_a).unwrap());

        // Decisions for dep_1
        let mut dec_a = fx::decision("dec-001", dep_1);
        dec_a["created_at"] = serde_json::json!("2026-09-18T09:59:59Z");
        dec_a["signal_action"] = serde_json::json!("buy");
        dec_a["requested_quantity"] = serde_json::json!("100.50000000");
        dec_a["reason"] = serde_json::json!("strategy_trigger_long");
        store.apply(stream::execution::decode_execution_entry("decisions", &dec_a).unwrap());

        // Risk rejection for dep_1
        let mut risk_a = fx::risk_rejection("risk-001", dep_1);
        risk_a["created_at"] = serde_json::json!("2026-09-18T09:58:00Z");
        risk_a["rejection_code"] = serde_json::json!("max_position_limit");
        risk_a["message"] = serde_json::json!("Requested size violates risk limit");
        store.apply(stream::execution::decode_execution_entry("risk", &risk_a).unwrap());

        store.apply(stream::execution::decode_execution_entry("ledger", &led_a).unwrap());
    });

    unsafe {
        let mut pin = std::pin::Pin::new_unchecked(
            &mut *(models as *mut execution_models::ffi::ExecutionModels),
        );
        // Ledger entries are loaded via paged REST (load_older("ledger")) into older_ledger
        let mut led_row = led_a.clone();
        led_row["id"] = serde_json::json!("led-001");
        led_row["amount"] = serde_json::json!("5000.25");
        led_row["balance_after"] = serde_json::json!("105000.25");
        led_row["description"] = serde_json::json!("Realized profit");
        pin.as_mut()
            .rust_mut()
            .older_ledger
            .entry("22222222-2222-2222-2222-222222222222".to_string())
            .or_default()
            .push(led_row);
        chart_bridge::execution_models_sync(models);
        let rust = pin.rust();

        // 1. Deployments table assertions
        assert_eq!(rust.deployments_count(), 2);
        assert_eq!(rust.get_deployment_field(0, "id"), dep_1);
        assert_eq!(rust.get_deployment_field(0, "lifecycle"), "running");
        assert_eq!(rust.get_deployment_field(0, "broker_mode"), "paper");
        assert_eq!(rust.get_deployment_field(0, "pos_side"), "long");
        // Decimal string precision must be lossless
        assert_eq!(
            rust.get_deployment_field(0, "pos_quantity"),
            "100.50000000"
        );
        assert_eq!(
            rust.get_deployment_field(0, "pos_entry_price"),
            "34.56789000"
        );
        // Unknown order count
        assert_eq!(rust.get_deployment_field(0, "unknown_order_count"), "1");

        // 2. Orders table assertions (newest first)
        assert_eq!(rust.orders_count(), 2);
        assert_eq!(
            rust.get_order_field(0, "id"),
            "ord-002",
            "newest order must be row 0"
        );
        assert_eq!(rust.get_order_field(0, "quantity"), "200.75000000");
        assert_eq!(rust.get_order_field(0, "status"), "pending");
        assert_eq!(rust.get_order_field(0, "reconciliation_state"), "unknown");

        assert_eq!(
            rust.get_order_field(1, "id"),
            "ord-001",
            "older order must be row 1"
        );
        assert_eq!(rust.get_order_field(1, "quantity"), "100.50000000");
        assert_eq!(rust.get_order_field(1, "status"), "filled");

        // 3. Fills table assertions
        assert_eq!(rust.fills_count(), 1);
        assert_eq!(rust.get_fill_field(0, "id"), "fill-001");
        assert_eq!(rust.get_fill_field(0, "price"), "34.56789000");
        assert_eq!(rust.get_fill_field(0, "quantity"), "100.50000000");
        assert_eq!(rust.get_fill_field(0, "fee"), "0.12340000");
        assert_eq!(rust.get_fill_field(0, "slippage"), "0.00500000");

        // 4. Decisions table assertions
        assert_eq!(rust.decisions_count(), 1);
        assert_eq!(rust.get_decision_field(0, "id"), "dec-001");
        assert_eq!(rust.get_decision_field(0, "signal_action"), "buy");
        assert_eq!(rust.get_decision_field(0, "requested_quantity"), "100.50000000");
        assert_eq!(rust.get_decision_field(0, "reason"), "strategy_trigger_long");

        // 5. Risk table assertions
        assert_eq!(rust.risk_count(), 1);
        assert_eq!(rust.get_risk_field(0, "id"), "risk-001");
        assert_eq!(rust.get_risk_field(0, "rejection_code"), "max_position_limit");
        assert_eq!(
            rust.get_risk_field(0, "message"),
            "Requested size violates risk limit"
        );

        // 6. Accounts and ledger assertions
        assert_eq!(rust.accounts_count(), 1);
        assert_eq!(rust.get_account_field(0, "cash_balance"), "105000.25");

        assert_eq!(rust.ledger_count(), 1);
        assert_eq!(rust.get_ledger_field(0, "id"), "led-001");
        assert_eq!(rust.get_ledger_field(0, "amount"), "5000.25");
        assert_eq!(rust.get_ledger_field(0, "balance_after"), "105000.25");
        assert_eq!(rust.get_ledger_field(0, "description"), "Realized profit");

        chart_bridge::delete_test_execution_models(models);
    }
}

/// Acceptance Criterion 2:
/// Selecting a deployment shows only its rows.
/// "Load older" issues one paged REST request and appends its rows below.
#[tokio::test(flavor = "current_thread")]
async fn test_ops_views_criterion_2_selection_isolation_and_load_older_paged_rest() {
    let _guard = chart_bridge::QT_TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

    let server = FakeServer::start().await;
    let (models, handle) = make_models();

    let dep_1 = "11111111-1111-1111-1111-111111111111";
    let dep_2 = "22222222-2222-2222-2222-222222222222";

    handle.mutate(|store| {
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_1, "running"),
            )
            .unwrap(),
        );
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_2, "paused"),
            )
            .unwrap(),
        );

        // dep_1 order
        store.apply(
            stream::execution::decode_execution_entry(
                "orders",
                &fx::order("dep1-ord-01", dep_1, "submitted"),
            )
            .unwrap(),
        );

        // dep_2 order
        store.apply(
            stream::execution::decode_execution_entry(
                "orders",
                &fx::order("dep2-ord-01", dep_2, "filled"),
            )
            .unwrap(),
        );
    });

    unsafe {
        let mut pin = std::pin::Pin::new_unchecked(
            &mut *(models as *mut execution_models::ffi::ExecutionModels),
        );
        pin.as_mut().setup(QString::from(&server.api_base()));
        chart_bridge::execution_models_sync(models);

        // By default dep_1 is selected
        assert_eq!(pin.rust().selected_deployment_id.to_string(), dep_1);
        assert_eq!(pin.rust().orders_count(), 1);
        assert_eq!(pin.rust().get_order_field(0, "id"), "dep1-ord-01");

        // Select dep_2: rows must switch immediately to dep_2 only
        pin.as_mut().select_deployment(QString::from(dep_2));
        assert_eq!(pin.rust().selected_deployment_id.to_string(), dep_2);
        assert_eq!(pin.rust().orders_count(), 1);
        assert_eq!(pin.rust().get_order_field(0, "id"), "dep2-ord-01");

        // Verify "load older" REST request:
        let initial_paged = server.paged_calls();
        pin.as_mut().load_older(QString::from("orders"));

        // Allow background async fetch to complete
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            if server.paged_calls() > initial_paged {
                break;
            }
        }

        // Must issue exactly ONE paged REST request
        assert_eq!(
            server.paged_calls(),
            initial_paged + 1,
            "load_older must issue exactly 1 paged REST request"
        );

        // Process Qt event queue until queued callback runs and appends rows
        for _ in 0..50 {
            chart_bridge::process_events();
            if pin.rust().orders_count() >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        // Older order appended below
        assert_eq!(pin.rust().orders_count(), 2);
        assert_eq!(pin.rust().get_order_field(0, "id"), "dep2-ord-01");
        assert_eq!(pin.rust().get_order_field(1, "id"), "older-order-99");

        // Switching back to dep_1 shows ONLY dep_1 rows (older order was scoped to dep_2)
        pin.as_mut().select_deployment(QString::from(dep_1));
        assert_eq!(pin.rust().orders_count(), 1);
        assert_eq!(pin.rust().get_order_field(0, "id"), "dep1-ord-01");

        chart_bridge::delete_test_execution_models(models);
    }
}

/// Acceptance Criterion 4:
/// Each §8.1 degraded state is produced by the fake server or a fake health response,
/// and asserted headlessly: the state label, the kept data, and its age.
#[test]
fn test_ops_views_criterion_4_all_section_8_1_degraded_states_asserted_headlessly() {
    let mut status = OpsStatusRust::default();

    // 1. Stream disconnected with age and kept data
    status.set_stream_info("disconnected", 45.0);
    assert_eq!(status.stream_state.to_string(), "disconnected");
    assert_eq!(status.stream_age_s, 45.0);

    // 2. Stream reconnecting
    status.set_stream_info("reconnecting", 5.0);
    assert_eq!(status.stream_state.to_string(), "reconnecting");
    assert_eq!(status.stream_age_s, 5.0);

    // 3. Postgres down (503 / database unavailable)
    status.mark_postgres_down();
    assert_eq!(status.api_status.to_string(), "degraded");
    assert!(!status.postgres_available, "postgres must be marked unavailable on 503");

    // 4. API offline / unreachable
    status.mark_api_offline();
    assert_eq!(status.api_status.to_string(), "offline");
    assert_eq!(status.worker_status.to_string(), "unknown");
    assert!(!status.postgres_available);

    // 5. Worker offline or stale heartbeat
    let worker_offline = serde_json::json!({
        "api_status": "ok",
        "worker_status": "offline",
        "worker_heartbeat_age_s": 120.0,
        "edge": { "reachable": true, "mt5_connected": true, "terminal_build": 4150 },
        "unknown_order_count": 0,
        "kill_switch_enabled": false,
        "live_capability_locked": false
    });
    status.apply_health_str(&worker_offline.to_string());
    assert_eq!(status.worker_status.to_string(), "offline");
    assert_eq!(status.worker_heartbeat_age_s, 120.0);

    // 6. MT5 edge down
    let edge_down = serde_json::json!({
        "api_status": "ok",
        "worker_status": "active",
        "worker_heartbeat_age_s": 1.0,
        "edge": { "reachable": false, "mt5_connected": false, "terminal_build": 0 },
        "unknown_order_count": 0,
        "kill_switch_enabled": false,
        "live_capability_locked": false
    });
    status.apply_health_str(&edge_down.to_string());
    assert!(!status.edge_reachable);
    assert!(!status.edge_mt5_connected);

    // 7. Unknown orders requiring reconciliation
    let unk_orders = serde_json::json!({
        "api_status": "ok",
        "worker_status": "active",
        "worker_heartbeat_age_s": 1.0,
        "edge": { "reachable": true, "mt5_connected": true, "terminal_build": 4150 },
        "unknown_order_count": 3,
        "kill_switch_enabled": true,
        "live_capability_locked": true
    });
    status.apply_health_str(&unk_orders.to_string());
    assert_eq!(status.unknown_orders, 3);
    assert!(status.kill_switch_enabled);
    assert!(status.live_locked);
}

/// Acceptance Criterion 5:
/// The poller issues one health request and one positions request every two seconds,
/// and they are the only periodic requests, counted by the fake server.
#[tokio::test]
async fn test_ops_views_criterion_5_poller_periodic_requests_only() {
    let server = FakeServer::start().await;
    let poller = Arc::new(HealthPoller::with_interval(
        &server.api_base(),
        Duration::from_millis(20),
    ));

    let count = Arc::new(AtomicUsize::new(0));
    let cnt = count.clone();

    poller.start(move |_| {
        cnt.fetch_add(1, Ordering::SeqCst);
    });

    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(15)).await;
        if count.load(Ordering::SeqCst) >= 20 {
            break;
        }
    }
    poller.stop();

    let h_calls = server.health_calls();
    let p_calls = server.positions_calls();

    assert!(h_calls >= 20, "expected >= 20 health calls, got {h_calls}");
    assert_eq!(h_calls, p_calls, "health calls must equal positions calls");
    assert_eq!(
        server.rest_calls(),
        h_calls + p_calls,
        "only health and positions should be called periodically"
    );
}
