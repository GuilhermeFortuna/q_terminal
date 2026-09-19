//! `--headless-report --execution` against the fake server.

#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

#[path = "../src/bridge.rs"]
#[allow(dead_code)]
pub mod bridge;
#[path = "../src/config.rs"]
#[allow(dead_code)]
pub mod config;
#[path = "../src/execution/mod.rs"]
#[allow(dead_code)]
pub mod execution;
#[path = "../src/history/mod.rs"]
#[allow(dead_code)]
pub mod history;
#[path = "../src/stream/mod.rs"]
#[allow(dead_code)]
pub mod stream;

use std::process::Command;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;

const DEP: &str = "11111111-1111-1111-1111-111111111111";

fn run_report(api_base: String) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_q_terminal"))
        .args(["--headless-report", "--execution"])
        .env("Q_TERMINAL_API_BASE", api_base)
        .env("Q_TERMINAL_SYMBOL", "PETR4")
        .env("Q_TERMINAL_TIMEFRAME", "1m")
        .env("Q_TERMINAL_REPORT_TIMEOUT_MS", "1500")
        .output()
        .expect("run q_terminal")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_prints_the_snapshot_counts_and_applied_sequences() {
    let server = FakeServer::start().await;
    server
        .exec_publish_silent("deployments", fx::deployment(DEP, "running"))
        .await;
    server
        .exec_publish_silent("orders", fx::order("o1", DEP, "filled"))
        .await;
    server
        .exec_publish_silent("orders", fx::order("o2", DEP, "submitted"))
        .await;
    server
        .exec_publish_silent("ledger", fx::ledger("l1", "999.90"))
        .await;

    let base = server.api_base();
    let out = tokio::task::spawn_blocking(move || run_report(base))
        .await
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for want in [
        "confirmed: true",
        "deployments: 1",
        "accounts: 1",
        "orders: 2",
        "kill_switch: off",
        "seq orders: 2 (epoch-1)",
        "seq deployments: 1 (epoch-1)",
    ] {
        assert!(
            stdout.lines().any(|l| l == want),
            "missing `{want}` in:\n{stdout}"
        );
    }
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_fails_when_the_snapshot_is_unavailable() {
    let server = FakeServer::start().await;
    server.set_snapshot_503(true);
    let base = server.api_base();
    let out = tokio::task::spawn_blocking(move || run_report(base))
        .await
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("confirmed: false"));
    server.shutdown().await;
}
