pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

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
    assert_eq!(lines[1], "2026.9.24");
    // q_core v2026.09.24 bakes in the q_contracts rev it was built against, which is
    // older than this repo's CONTRACTS_REV (Q-046 moved it to pick up the Q-039
    // execution payloads; nothing under schema/api/arrow or schema/lake changed).
    // Restore `include_str!("../CONTRACTS_REV").trim()` once q_core re-pins to >= 4e87497.
    assert_eq!(lines[2], "998a50570905524bfb9af0465a725b170f2970df");
    assert_eq!(lines[3], "headless");
}
