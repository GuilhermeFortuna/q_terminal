//! Q-065: the tick-generation and time-formatting helpers in `qml/ChartAxis.js` are pure
//! JS with no Qt Quick item involved, so they are exercised headlessly with Qt's own test
//! runner (`qmltestrunner`) against `qml/tests/tst_ChartAxis.qml`, rather than duplicated as
//! Rust logic. This keeps `cargo test` (the `make test` / `make check` entrypoint) as the
//! single gate without adding a second CI command.

use std::env;
use std::path::PathBuf;
use std::process::Command;

// Prefers the project's own vendored Qt (matching `QMLLINT`'s lookup in the Makefile) over
// any system Qt on PATH: a mismatched system Qt version can silently fail to launch under
// `QT_QPA_PLATFORM=offscreen` even though it's a working qmltestrunner in isolation.
fn find_qmltestrunner() -> Option<PathBuf> {
    if let Ok(path) = env::var("QMLTESTRUNNER") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let home = env::var("HOME").unwrap_or_default();
    let downloaded = PathBuf::from(home).join(".local/share/qt_minimal_download");
    if downloaded.is_dir() {
        if let Some(found) = walk_for_binary(&downloaded, "qmltestrunner")
            .into_iter()
            .next()
        {
            return Some(found);
        }
    }
    for dir in env::var("PATH").unwrap_or_default().split(':') {
        let candidate = PathBuf::from(dir).join("qmltestrunner");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn walk_for_binary(root: &std::path::Path, name: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn chart_axis_js_headless_suite_passes() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_qml = manifest_dir.join("qml/tests/tst_ChartAxis.qml");
    assert!(test_qml.is_file(), "missing {}", test_qml.display());

    let Some(runner) = find_qmltestrunner() else {
        panic!(
            "qmltestrunner not found on PATH, $QMLTESTRUNNER, or ~/.local/share/qt_minimal_download; \
             install qt6-declarative-dev-tools or set QMLTESTRUNNER"
        );
    };

    let output = Command::new(&runner)
        .arg("-input")
        .arg(&test_qml)
        .env("QT_QPA_PLATFORM", "offscreen")
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", runner.display()));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "qmltestrunner reported failures:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
