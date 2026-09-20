use q_terminal::{chart_bridge, config, headless_execution_report, headless_report};

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
    if args.iter().any(|arg| arg == "--gallery") {
        #[cfg(feature = "gallery")]
        std::process::exit(q_terminal::gallery::run());
        #[cfg(not(feature = "gallery"))]
        {
            eprintln!("--gallery requires the gallery feature");
            std::process::exit(2);
        }
    }
    if args.iter().any(|arg| arg == "--gallery-shot") {
        #[cfg(feature = "gallery")]
        std::process::exit(q_terminal::gallery::capture(std::path::Path::new(
            "gallery-shots",
        )));
        #[cfg(not(feature = "gallery"))]
        {
            eprintln!("--gallery-shot requires the gallery feature");
            std::process::exit(2);
        }
    }
    if args.iter().any(|arg| arg == "--bench-frames") {
        let visible_buckets = read_arg_i32(&args, "--buckets", 2000);
        let bar_count = read_arg_i32(&args, "--bars", 500_000);
        let duration_ms = read_arg_i32(&args, "--duration-ms", 300_000);
        let execution_rows = read_arg_i32(&args, "--execution-rows", 0);
        let markers = read_arg_i32(&args, "--markers", 0);
        let overlays = read_arg_i32(&args, "--overlays", 0);
        std::process::exit(q_terminal::run_bench_frames(
            visible_buckets,
            bar_count,
            duration_ms,
            execution_rows,
            markers,
            overlays,
        ));
    }
    let auto_close_ms = read_arg_i32(&args, "--auto-close-ms", 0);
    std::process::exit(q_terminal::startup::run_slice_opts(
        config::Config::load(),
        false,
        auto_close_ms as u64,
        0,
        0,
        0,
    ));
}

fn read_arg_i32(args: &[String], flag: &str, default: i32) -> i32 {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
