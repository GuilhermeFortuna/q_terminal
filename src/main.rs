pub mod bridge;

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
    0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless-report") {
        std::process::exit(headless_report());
    }
    std::process::exit(run_windowed());
}
