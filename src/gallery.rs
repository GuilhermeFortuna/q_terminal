use std::path::Path;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

use crate::{bridge, chart_bridge};

fn gallery_engine() -> cxx::UniquePtr<QQmlApplicationEngine> {
    chart_bridge::ensure_application();
    let mut engine = QQmlApplicationEngine::new();
    engine
        .as_mut()
        .unwrap()
        .add_import_path(&QString::from("target/cxxqt/qml_modules"));
    engine
        .as_mut()
        .unwrap()
        .load(&QUrl::from_local_file(&QString::from(
            "qml/gallery/Gallery.qml",
        )));
    engine
}

pub fn run() -> i32 {
    let _engine = gallery_engine();
    chart_bridge::exec_application()
}

pub fn capture(output_dir: &Path) -> i32 {
    let mut engine = gallery_engine();
    if !bridge::ffi::verify_design_fonts() {
        eprintln!("vendored Inter/JetBrains Mono verification failed");
        return 1;
    }
    if let Err(error) = std::fs::create_dir_all(output_dir) {
        eprintln!("create gallery output directory: {error}");
        return 1;
    }
    for (name, width, height) in [
        ("gallery-1920x1080.png", 1920, 1080),
        ("gallery-2560x1440.png", 2560, 1440),
    ] {
        let path = output_dir.join(name);
        let path = QString::from(path.to_string_lossy().as_ref());
        if !bridge::ffi::capture_window(engine.as_mut().unwrap(), &path, width, height) {
            eprintln!("failed to capture {name}");
            return 1;
        }
    }
    0
}
