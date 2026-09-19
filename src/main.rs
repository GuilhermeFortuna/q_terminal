#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

pub mod bridge;
pub mod chart_bridge;
pub mod config;
pub mod execution;
pub mod execution_models;
pub mod history;
pub mod startup;
pub mod stream;

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
    // Let events already in flight land, then read.
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

pub fn run_bench_frames(visible_buckets: i32, bar_count: i32, duration_ms: i32) -> i32 {
    let _ = (visible_buckets, bar_count);
    startup::run_slice_opts(config::Config::load(), true, duration_ms as u64)
}

fn main() {
    cxx_qt::init_crate!(q_qt);
    cxx_qt::init_crate!(cxx_qt_lib);
    cxx_qt::init_crate!(cxx_qt);

    chart_bridge::register_chart_types();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless-report") {
        if args.iter().any(|arg| arg == "--execution") {
            std::process::exit(headless_execution_report());
        }
        std::process::exit(headless_report());
    }
    if args.iter().any(|arg| arg == "--bench-frames") {
        let visible_buckets = read_arg_i32(&args, "--buckets", 2000);
        let bar_count = read_arg_i32(&args, "--bars", 500_000);
        let duration_ms = read_arg_i32(&args, "--duration-ms", 300_000);
        std::process::exit(run_bench_frames(visible_buckets, bar_count, duration_ms));
    }
    let auto_close_ms = read_arg_i32(&args, "--auto-close-ms", 0);
    std::process::exit(startup::run_slice_opts(
        config::Config::load(),
        false,
        auto_close_ms as u64,
    ));
}

fn read_arg_i32(args: &[String], flag: &str, default: i32) -> i32 {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
