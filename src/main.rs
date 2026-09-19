#[rustfmt::skip]
#[path = "../contracts/stream.rs"]
pub mod contracts_stream;

pub mod bridge;
pub mod chart_bridge;
pub mod config;
pub mod execution;
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
