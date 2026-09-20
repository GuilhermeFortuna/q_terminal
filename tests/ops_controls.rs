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
