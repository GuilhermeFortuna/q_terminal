use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn find_q_qt_include_dir(build_dir: &std::path::Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(build_dir).expect("read target build directory") {
        let path = entry.expect("read build directory entry").path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if name.starts_with("q-qt-") {
            let include = path.join("out").join("cxxqtbuild").join("include");
            if include.is_dir() {
                return Some(include);
            }
        }
    }
    None
}

fn q_qt_include_dir(build_dir: &Path) -> PathBuf {
    if let Some(include) = find_q_qt_include_dir(build_dir) {
        return include;
    }

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = std::process::Command::new(cargo)
        .args(["build", "--package", "q-qt"])
        .status()
        .expect("failed to pre-build q-qt for include headers");
    assert!(status.success(), "q-qt pre-build failed");

    find_q_qt_include_dir(build_dir).expect("q-qt cxxqtbuild include directory not found")
}

fn link_q_qt_headers(build_dir: &Path) {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let link_path = manifest_dir.join("cpp/q-qt");
    let q_qt_headers = q_qt_include_dir(build_dir).join("q-qt");

    if link_path.exists() {
        std::fs::remove_dir_all(&link_path).expect("remove stale q-qt header symlink");
    }
    std::os::unix::fs::symlink(&q_qt_headers, &link_path).expect("symlink q-qt headers into cpp/");
    println!("cargo:rerun-if-changed={}", q_qt_headers.display());
}

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let build_dir = out_dir.ancestors().nth(2).expect("target build directory");
    link_q_qt_headers(build_dir);

    CxxQtBuilder::new_qml_module(QmlModule::new("qml").qml_file("qml/Main.qml"))
        .qt_module("Quick")
        .include_dir("cpp")
        .file("src/bridge.rs")
        .file("src/chart_bridge.rs")
        .cpp_file("src/render_backend.cpp")
        .cpp_file("cpp/bar_chart_node.cpp")
        .cpp_file("cpp/bar_chart_item.h")
        .cpp_file("cpp/bar_chart_item.cpp")
        .cpp_file("cpp/bar_chart_probe.cpp")
        .cpp_file("cpp/chart_cxx.cpp")
        .cpp_file("cpp/frame_bench.cpp")
        .build();

    merge_chart_qmltypes();
}

fn merge_chart_qmltypes() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let plugin_qmltypes = out_dir.join("qt-build-utils/qml_modules/qml/plugin.qmltypes");
    let supplement_path = manifest_dir.join("qml/chart_types.qmltypes");

    let base = std::fs::read_to_string(&plugin_qmltypes).expect("read plugin.qmltypes");
    let supplement_body =
        std::fs::read_to_string(&supplement_path).expect("read chart_types.qmltypes");
    let module_close = base
        .rfind('}')
        .expect("module closing brace in plugin.qmltypes");
    let merged = format!(
        "{}\n{}\n}}",
        base[..module_close].trim_end(),
        supplement_body.trim_end()
    );
    std::fs::write(&plugin_qmltypes, &merged).expect("write merged plugin.qmltypes");

    let installed_qmltypes = manifest_dir
        .join("target")
        .join("cxxqt/qml_modules/qml/plugin.qmltypes");
    if let Some(parent) = installed_qmltypes.parent() {
        std::fs::create_dir_all(parent).expect("create cxxqt qml module directory");
    }
    std::fs::write(&installed_qmltypes, &merged).expect("write installed plugin.qmltypes");
    println!("cargo:rerun-if-changed={}", supplement_path.display());
}
