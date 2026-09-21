#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

pub mod bridge;
pub mod chart_bridge;
pub mod chart_target;
pub mod config;
pub mod display;
pub mod execution;
#[cfg(feature = "gallery")]
pub mod gallery;
pub mod history;
pub mod ops_session;
pub mod semantic;
pub mod shell;
pub mod startup;
pub mod stream;
pub mod workspace;

pub use bridge::execution_controls;
pub use bridge::execution_models;
pub use bridge::ops_status;
pub use bridge::shell_controller;
pub use bridge::workspace_controller;

/// Prints app version, core version, contracts rev and render backend, then exits 0.
/// Opens no window; the path CI and an agent session take.
pub fn headless_report() -> i32 {
    let app_info = bridge::AppInfoRust::default();
    println!("{}", app_info.app_version);
    println!("{}", app_info.core_version);
    println!("{}", app_info.contracts_rev);
    println!("{}", app_info.render_backend);
    0
}

/// Connects, lets the execution store converge on the backend's snapshot, prints its
/// counts and applied sequences, and exits 0. Exits 1 if it cannot be confirmed.
pub fn headless_execution_report() -> i32 {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("configuration error: {e}");
            return 1;
        }
    };
    let handle = execution::store::ExecutionHandle::new();
    let client = stream::client::StreamClient::start_with(
        config,
        stream::client::Sinks {
            bars: stream::sink::BarSink::new(),
            execution: Some(handle.clone()),
        },
    );
    let timeout_ms: u64 = std::env::var("Q_TERMINAL_REPORT_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    while !handle.read(|s| s.is_confirmed()) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    let confirmed = handle.read(|s| s.is_confirmed());
    for line in handle.read(|s| s.report_lines()) {
        println!("{line}");
    }
    let last_error = client.last_error();
    client.shutdown();
    if !confirmed {
        eprintln!("execution state not confirmed: {last_error}");
        return 1;
    }
    0
}

/// Loads qml/Main.qml and runs the slice.
pub fn run_windowed() -> i32 {
    startup::run_slice(config::Config::load())
}

pub fn run_bench_frames(
    visible_buckets: i32,
    bar_count: i32,
    duration_ms: i32,
    execution_rows: i32,
    markers: i32,
    overlays: i32,
) -> i32 {
    let _ = (visible_buckets, bar_count);
    startup::run_slice_opts(
        config::Config::load(),
        true,
        duration_ms as u64,
        execution_rows,
        markers,
        overlays,
    )
}
