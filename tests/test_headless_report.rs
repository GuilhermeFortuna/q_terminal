#[path = "../src/bridge.rs"]
#[allow(dead_code)]
mod bridge;
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;
#[path = "../src/history/mod.rs"]
#[allow(dead_code)]
mod history;

use std::process::Command;

#[test]
fn test_headless_report() {
    let binary = env!("CARGO_BIN_EXE_q_terminal");
    let output = Command::new(binary)
        .arg("--headless-report")
        .output()
        .expect("Failed to execute q_terminal");

    assert!(output.status.success(), "Process did not exit successfully");

    let stdout = String::from_utf8(output.stdout).expect("Stdout is not valid UTF-8");
    let lines: Vec<&str> = stdout.trim().lines().collect();

    assert_eq!(lines.len(), 4, "Expected 4 lines of output, got: {stdout}");
    assert_eq!(lines[0], env!("CARGO_PKG_VERSION"));
    assert_eq!(lines[1], "2026.9.16");
    assert_eq!(lines[2], include_str!("../CONTRACTS_REV").trim());
    assert_eq!(lines[3], "headless");
}
