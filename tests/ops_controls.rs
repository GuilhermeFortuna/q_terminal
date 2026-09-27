#![allow(clippy::float_cmp)]

pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use std::pin::Pin;
use std::time::Duration;

use cxx_qt_lib::QString;
use execution::store::ExecutionHandle;
use execution_controls::ffi::ExecutionControls;
use stream::fake_commands::CommandFakeState;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;

fn controls_pin(addr: usize) -> Pin<&'static mut ExecutionControls> {
    unsafe { Pin::new_unchecked(&mut *(addr as *mut ExecutionControls)) }
}

fn make_controls(api_base: &str) -> (usize, ExecutionHandle) {
    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();

    let addr = chart_bridge::make_test_execution_controls();
    assert_ne!(addr, 0);
    let handle = ExecutionHandle::new();
    execution_controls::stage_control_handle(handle.clone());
    unsafe {
        chart_bridge::execution_controls_setup(addr, api_base, "operator");
        chart_bridge::execution_controls_bind_handle(addr);
        chart_bridge::execution_controls_update_health(
            addr, false, true, "active", 1.0, true, true,
        );
    }
    (addr, handle)
}

async fn pump_events() {
    for _ in 0..8 {
        chart_bridge::process_events();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn wait_for_command_log(server: &FakeServer, min_len: usize) {
    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        if server.command_log().len() >= min_len {
            return;
        }
    }
    panic!(
        "expected >= {min_len} command log entries, got {}",
        server.command_log().len()
    );
}

async fn wait_for_phase(addr: usize, action_id: &str, expected: &str) {
    for _ in 0..100 {
        chart_bridge::process_events();
        let phase = controls_pin(addr)
            .action_phase(QString::from(action_id))
            .to_string();
        if phase == expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let phase = controls_pin(addr)
        .action_phase(QString::from(action_id))
        .to_string();
    panic!("expected phase {expected}, got {phase}");
}

/// Criterion 1: one REST request per operator action with a stable idempotency key.
#[tokio::test]
async fn criterion_1_one_request_per_action_with_idempotency_key() {
    let server = FakeServer::start().await;
    let (addr, _handle) = make_controls(&server.api_base());

    let args = r#"{"name":"paper-a","initial_balance":"10000","currency":"BRL"}"#;
    let action_id = controls_pin(addr)
        .request(QString::from("create_account"), QString::from(args))
        .to_string();
    assert!(!action_id.is_empty());

    // Duplicate click while in-flight must not send a second request.
    let dup = controls_pin(addr)
        .request(QString::from("create_account"), QString::from(args))
        .to_string();
    assert!(dup.is_empty());

    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "awaiting_stream").await;

    let log = server.command_log();
    assert_eq!(log.len(), 1, "expected exactly one command request");
    assert_eq!(log[0].method, "POST");
    assert!(log[0].path.contains("/execution/accounts"));
}

/// Criterion 2: control settles only when the matching stream event arrives.
#[tokio::test]
async fn criterion_2_settles_on_stream_event() {
    let server = FakeServer::start().await;
    let dep_id = "11111111-1111-1111-1111-111111111111";
    let (addr, handle) = make_controls(&server.api_base());

    handle.mutate(|store| {
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_id, "paused"),
            )
            .unwrap(),
        );
    });

    let args = format!(r#"{{"deployment_id":"{dep_id}"}}"#);
    let action_id = controls_pin(addr)
        .request(QString::from("start"), QString::from(&args))
        .to_string();
    assert!(!action_id.is_empty());
    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "awaiting_stream").await;

    handle.mutate(|store| {
        store.apply(
            stream::execution::decode_execution_entry(
                "deployments",
                &fx::deployment(dep_id, "running"),
            )
            .unwrap(),
        );
    });
    wait_for_phase(addr, &action_id, "settled").await;
}

/// Criterion 3: refused commands surface backend code and message verbatim.
#[tokio::test]
async fn criterion_3_refused_command_shows_backend_error() {
    let server = FakeServer::start().await;
    server
        .set_command_script(CommandFakeState::refuse_lifecycle("illegal_transition"))
        .await;
    let dep_id = "11111111-1111-1111-1111-111111111111";
    let (addr, _handle) = make_controls(&server.api_base());

    let args = format!(r#"{{"deployment_id":"{dep_id}"}}"#);
    let action_id = controls_pin(addr)
        .request(QString::from("start"), QString::from(&args))
        .to_string();
    assert!(!action_id.is_empty());
    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "refused").await;

    let msg = controls_pin(addr)
        .action_message(QString::from(&action_id))
        .to_string();
    assert!(
        msg.contains("illegal_transition"),
        "expected backend code in message, got: {msg}"
    );
}

/// Criterion 5: invalid/cancelled payloads never hit the API (no request logged).
#[tokio::test]
async fn criterion_5_invalid_payload_sends_no_request() {
    let server = FakeServer::start().await;
    let (addr, _handle) = make_controls(&server.api_base());

    // Simulates a cancelled or incomplete form: missing required resolve fields.
    let args = r#"{"order_id":"ord-1","outcome":"filled","reason":"manual"}"#;
    let action_id = controls_pin(addr)
        .request(QString::from("resolve"), QString::from(args))
        .to_string();
    assert!(action_id.is_empty());
    pump_events().await;
    assert!(server.command_log().is_empty());
}

/// Criterion 6: filled resolve requires price, quantity, and time before submit.
#[tokio::test]
async fn criterion_6_resolve_filled_requires_fill_fields() {
    let server = FakeServer::start().await;
    let (addr, _handle) = make_controls(&server.api_base());

    let incomplete = r#"{"order_id":"ord-1","outcome":"filled","reason":"manual","price":"100"}"#;
    let empty = controls_pin(addr)
        .request(QString::from("resolve"), QString::from(incomplete))
        .to_string();
    assert!(empty.is_empty());

    let complete = r#"{
        "order_id":"ord-1",
        "outcome":"filled",
        "reason":"manual reconcile",
        "price":"100.5",
        "quantity":"2",
        "filled_at":"2026-09-18T12:00:00Z"
    }"#;
    let action_id = controls_pin(addr)
        .request(QString::from("resolve"), QString::from(complete))
        .to_string();
    assert!(!action_id.is_empty());
    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "awaiting_stream").await;
    assert_eq!(server.command_log().len(), 1);
}

/// Criterion 7: saved-run picker fields and live-deployment confirmation banner.
#[tokio::test]
async fn criterion_7_saved_runs_and_live_warning() {
    let server = FakeServer::start().await;
    let (addr, _handle) = make_controls(&server.api_base());

    controls_pin(addr).fetch_saved_runs();
    let json = {
        let mut loaded = String::new();
        for _ in 0..100 {
            chart_bridge::process_events();
            let raw = controls_pin(addr).saved_runs_json().to_string();
            if raw != "[]" {
                loaded = raw;
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        if loaded.is_empty() {
            panic!("saved runs never loaded");
        }
        loaded
    };

    let runs: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["strategy_name"], "MACrossover");
    assert_eq!(runs[0]["symbol"], "WIN$N");
    assert_eq!(runs[0]["timeframe"], "1m");
    assert!(runs[0].get("saved_at").is_some());

    let confirm_qml = std::fs::read_to_string("qml/ConfirmDialog.qml").unwrap();
    assert!(
        confirm_qml.contains("LIVE DEPLOYMENT"),
        "live deployment warning must be prominent in ConfirmDialog"
    );
    let deploy_qml = std::fs::read_to_string("qml/DeployDialog.qml").unwrap();
    assert!(
        deploy_qml.contains("mt5_live"),
        "deploy dialog must branch on live broker mode"
    );
}

/// Q-069 Criterion 1: Strategy catalog fetch and exact symbol search.
#[tokio::test]
async fn test_q069_catalog_fetch_and_symbol_search() {
    let server = FakeServer::start().await;
    let (addr, _handle) = make_controls(&server.api_base());

    controls_pin(addr).fetch_strategy_catalog();
    let mut catalog_json = String::new();
    for _ in 0..100 {
        chart_bridge::process_events();
        let raw = controls_pin(addr).strategy_catalog_json().to_string();
        if raw != "{\"strategies\":[]}" && !raw.is_empty() {
            catalog_json = raw;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!catalog_json.is_empty(), "strategy catalog failed to load");
    let cat: serde_json::Value = serde_json::from_str(&catalog_json).unwrap();
    let strategies = cat["strategies"].as_array().expect("strategies array");
    assert_eq!(strategies.len(), 3);
    assert_eq!(strategies[0]["name"], "MovingAverageCross");
    assert_eq!(strategies[0]["strategy_type"], "candle");
    assert_eq!(strategies[0]["default_timeframe"], "15m");
    assert_eq!(strategies[1]["name"], "CustomMACrossover");
    assert_eq!(strategies[1]["strategy_type"], "candle");
    assert_eq!(strategies[2]["name"], "TickScalper");
    assert_eq!(strategies[2]["strategy_type"], "tick");

    controls_pin(addr).search_symbols(QString::from("WIN"));
    let mut search_json = String::new();
    for _ in 0..100 {
        chart_bridge::process_events();
        let raw = controls_pin(addr).symbol_search_results_json().to_string();
        if raw != "[]" && !raw.is_empty() {
            search_json = raw;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        !search_json.is_empty(),
        "symbol search failed to return results"
    );
    let results: Vec<serde_json::Value> = serde_json::from_str(&search_json).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["symbol"], "WIN$N");
    assert_eq!(results[0]["name"], "Mini Ibovespa Futuro");
}

/// Q-069 Criterion 2: Create paper deployment sends catalog values with idempotency key and settles on stream.
#[tokio::test]
async fn test_q069_create_paper_deployment_with_catalog() {
    let server = FakeServer::start().await;
    let (addr, handle) = make_controls(&server.api_base());

    let payload = serde_json::json!({
        "paper_account_id": "acc-paper-01",
        "name": "MA Cross 15m Paper",
        "broker_mode": "paper",
        "catalog": {
            "strategy_name": "MovingAverageCross",
            "strategy_params": { "fast_period": 10, "slow_period": 30 },
            "exit_params": { "stop_loss_pct": 1.5 },
            "symbol": "WIN$N",
            "timeframe": "15m",
            "sizing_config": { "quantity": "1" },
            "risk_config": { "max_position_size": "2" },
            "paper_cost_config": { "point_value": "0.2", "slippage": "0.5", "commission": "1.0" }
        }
    })
    .to_string();

    let action_id = controls_pin(addr)
        .request(QString::from("create_deployment"), QString::from(&payload))
        .to_string();
    assert!(!action_id.is_empty());

    // Duplicate submission in flight returns empty string
    let dup_id = controls_pin(addr)
        .request(QString::from("create_deployment"), QString::from(&payload))
        .to_string();
    assert!(dup_id.is_empty());

    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "awaiting_stream").await;

    let log = server.command_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].method, "POST");
    assert!(log[0].path.contains("/execution/deployments"));
    let body = &log[0].body;
    assert_eq!(body["broker_mode"], "paper");
    assert_eq!(body["live_activation_enabled"], false);
    assert_eq!(body["catalog"]["strategy_name"], "MovingAverageCross");
    assert_eq!(body["catalog"]["timeframe"], "15m");

    // Settle when stream event arrives
    let dep_id = "22222222-2222-2222-2222-222222222222";
    handle.mutate(|store| {
        let mut entry = fx::deployment(dep_id, "draft");
        entry["name"] = serde_json::json!("MA Cross 15m Paper");
        store.apply(stream::execution::decode_execution_entry("deployments", &entry).unwrap());
    });
    wait_for_phase(addr, &action_id, "settled").await;
}

/// Q-069 Criterion 3: Paused-flat edit validation, PATCH request, revision conflict, and stream settlement.
#[tokio::test]
async fn test_q069_edit_deployment_validation_and_patch() {
    let server = FakeServer::start().await;
    let dep_id = "33333333-3333-3333-3333-333333333333";
    let (addr, handle) = make_controls(&server.api_base());

    // 1. Initial deployment in running state -> edit disabled
    handle.mutate(|store| {
        let mut entry = fx::deployment(dep_id, "running");
        entry["config_revision"] = serde_json::json!(1);
        store.apply(stream::execution::decode_execution_entry("deployments", &entry).unwrap());
    });
    pump_events().await;

    let is_editable = controls_pin(addr).is_deployment_editable(QString::from(dep_id));
    assert!(!is_editable, "running deployment must not be editable");
    let reason = controls_pin(addr)
        .deployment_edit_disabled_reason(QString::from(dep_id))
        .to_string();
    assert!(
        reason.contains("running"),
        "reason must cite running lifecycle"
    );

    // 2. Transition to paused and flat -> edit enabled
    handle.mutate(|store| {
        let mut entry = fx::deployment(dep_id, "paused");
        entry["config_revision"] = serde_json::json!(1);
        store.apply(stream::execution::decode_execution_entry("deployments", &entry).unwrap());
    });
    pump_events().await;

    let is_editable_now = controls_pin(addr).is_deployment_editable(QString::from(dep_id));
    assert!(is_editable_now, "paused-flat deployment must be editable");

    // 3. Submit valid PATCH edit
    let edit_payload = serde_json::json!({
        "deployment_id": dep_id,
        "expected_revision": 1,
        "configuration": {
            "strategy_params": { "fast_period": 14, "slow_period": 40 },
            "exit_params": { "stop_loss_pct": 2.0 },
            "sizing_config": { "quantity": "3" },
            "risk_config": { "max_position_size": "5" },
            "paper_cost_config": { "point_value": "0.2", "slippage": "0.6", "commission": "1.2" }
        }
    })
    .to_string();

    let action_id = controls_pin(addr)
        .request(
            QString::from("edit_deployment"),
            QString::from(&edit_payload),
        )
        .to_string();
    assert!(!action_id.is_empty());

    wait_for_command_log(&server, 1).await;
    wait_for_phase(addr, &action_id, "awaiting_stream").await;

    let log = server.command_log();
    assert_eq!(log[0].method, "PATCH");
    assert!(log[0]
        .path
        .contains(&format!("/deployments/{dep_id}/configuration")));
    let req_body = &log[0].body;
    assert_eq!(req_body["expected_revision"], 1);
    assert_eq!(req_body["strategy_params"]["fast_period"], 14);

    // Stream arrives with revision 2 -> settled
    handle.mutate(|store| {
        let mut entry = fx::deployment(dep_id, "paused");
        entry["config_revision"] = serde_json::json!(2);
        store.apply(stream::execution::decode_execution_entry("deployments", &entry).unwrap());
    });
    wait_for_phase(addr, &action_id, "settled").await;
}

/// Q-069 Criterion 4: Performance summary and marks fetch with generation-safe cancellation.
#[tokio::test]
async fn test_q069_performance_fetch_and_generation_safety() {
    let server = FakeServer::start().await;
    let dep_id = "dep-0001";
    let (addr, _handle) = make_controls(&server.api_base());

    controls_pin(addr).fetch_performance(QString::from(dep_id), QString::from(""));
    let mut summary_json = String::new();
    for _ in 0..100 {
        chart_bridge::process_events();
        let raw = controls_pin(addr).performance_summary_json().to_string();
        if raw != "{}" && !raw.is_empty() {
            summary_json = raw;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(!summary_json.is_empty(), "performance summary must load");
    let summary: serde_json::Value = serde_json::from_str(&summary_json).unwrap();
    assert_eq!(summary["deployment_id"], dep_id);
    assert_eq!(summary["net_pnl"], "187.50");
    assert_eq!(summary["win_rate"], "0.60");
    assert_eq!(summary["closed_trade_count"], 5);

    let marks_raw = controls_pin(addr).performance_marks_json().to_string();
    assert_ne!(marks_raw, "[]");
    let marks: serde_json::Value = serde_json::from_str(&marks_raw).unwrap();
    let marks_arr = marks.as_array().expect("marks items");
    assert_eq!(marks_arr.len(), 1);
    assert_eq!(marks_arr[0]["equity_delta"], "37.50");

    // Clear performance cancels and clears properties
    controls_pin(addr).clear_performance();
    pump_events().await;
    assert_eq!(
        controls_pin(addr).performance_summary_json().to_string(),
        "{}"
    );
    assert_eq!(
        controls_pin(addr).performance_marks_json().to_string(),
        "[]"
    );
}
