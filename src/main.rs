pub mod bridge;
pub mod chart_bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

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

/// Loads qml/Main.qml into a QQmlApplicationEngine and runs the event loop.
pub fn run_windowed() -> i32 {
    run_windowed_with_bench(false, 2000, 500_000, 0)
}

pub fn run_bench_frames(visible_buckets: i32, bar_count: i32, duration_ms: i32) -> i32 {
    run_windowed_with_bench(true, visible_buckets, bar_count, duration_ms)
}

fn run_windowed_with_bench(
    bench: bool,
    visible_buckets: i32,
    bar_count: i32,
    duration_ms: i32,
) -> i32 {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    let uri = QString::from("target/cxxqt/qml_modules");
    engine.as_mut().unwrap().add_import_path(&uri);

    let qml_file = QString::from("qml/Main.qml");
    let qml_url = QUrl::from_local_file(&qml_file);
    engine.as_mut().unwrap().load(&qml_url);

    bridge::ffi::setup_window(engine.as_mut().unwrap());

    if bench {
        bridge::ffi::run_frame_bench(
            engine.as_mut().unwrap(),
            visible_buckets,
            bar_count,
            duration_ms,
        );
    }

    app.as_mut().unwrap().exec()
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
    std::process::exit(run_windowed());
}

fn read_arg_i32(args: &[String], flag: &str, default: i32) -> i32 {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
