use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn find_q_qt_include_dir(build_dir: &std::path::Path) -> Option<PathBuf> {
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(build_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with("q-qt-") {
                let include = path.join("out").join("cxxqtbuild").join("include");
                if include.is_dir() {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    candidates.push((mtime, include));
                }
            }
        }
    }
    candidates.sort_by_key(|a| std::cmp::Reverse(a.0));
    candidates.into_iter().map(|(_, p)| p).next()
}

fn q_qt_include_dir(build_dir: &Path) -> PathBuf {
    find_q_qt_include_dir(build_dir).unwrap_or_else(|| {
        panic!(
            "q-qt cxxqtbuild include directory not found in target build directory ({build_dir:?}); ensure q-qt is listed in [build-dependencies]"
        );
    })
}

fn link_q_qt_headers(build_dir: &Path) {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let link_path = manifest_dir.join("cpp/q-qt");
    let q_qt_headers = q_qt_include_dir(build_dir).join("q-qt");

    if link_path.symlink_metadata().is_ok() {
        let current = std::fs::read_link(&link_path).ok();
        if current.as_ref() == Some(&q_qt_headers) {
            println!("cargo:rerun-if-changed={}", q_qt_headers.display());
            return;
        }
        std::fs::remove_file(&link_path)
            .or_else(|_| std::fs::remove_dir_all(&link_path))
            .expect("remove stale q-qt header symlink");
    }
    match std::os::unix::fs::symlink(&q_qt_headers, &link_path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            let current = std::fs::read_link(&link_path).expect("read concurrent q-qt symlink");
            assert_eq!(current, q_qt_headers, "stale q-qt header symlink");
        }
        Err(err) => panic!("symlink q-qt headers into cpp/: {err}"),
    }
    println!("cargo:rerun-if-changed={}", q_qt_headers.display());
}

fn main() {
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        if let Ok(canonical) = std::fs::canonicalize(&out_dir) {
            std::env::set_var("OUT_DIR", canonical);
        }
    }

    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let build_dir = out_dir.ancestors().nth(2).expect("target build directory");
    link_q_qt_headers(build_dir);

    CxxQtBuilder::new_qml_module(
        QmlModule::new("qml")
            .qml_file("qml/Main.qml")
            .qml_file("qml/Viewport.qml")
            .qml_file("qml/ChartPane.qml")
            .qml_file("qml/EmptyState.qml")
            .qml_file("qml/StatusStrip.qml")
            .qml_file("qml/OpsHeader.qml")
            .qml_file("qml/DeploymentList.qml")
            .qml_file("qml/OrdersTable.qml")
            .qml_file("qml/FillsTable.qml")
            .qml_file("qml/DecisionsTable.qml")
            .qml_file("qml/RiskTable.qml")
            .qml_file("qml/LedgerTable.qml")
            .qml_file("qml/DeploymentDetail.qml")
            .qml_file("qml/OpsWorkspace.qml")
            .qml_file("qml/ConfirmDialog.qml")
            .qml_file("qml/AccountDialog.qml")
            .qml_file("qml/DeployDialog.qml")
            .qml_file("qml/ResolveDialog.qml"),
    )
    .qrc_resources(
        qt_build_utils::QResources::new()
            .resource(
                qt_build_utils::QResource::new()
                    .prefix("/qt/qml/qml/qml")
                    .file(qt_build_utils::QResourceFile::new("qml/Format.js").alias("Format.js")),
            )
            .resource(
                qt_build_utils::QResource::new()
                    .prefix("/qt/qml/qml")
                    .file(qt_build_utils::QResourceFile::new("qml/Format.js").alias("Format.js")),
            ),
    )
    // The default is the entire crate root. That makes Cargo watch generated
    // files below target/ and causes every successful build to invalidate the
    // next one. Only cpp/ contains headers exported by this crate.
    .crate_include_root(Some("cpp".to_owned()))
    .qt_module("Quick")
    .include_dir("src")
    .file("src/bridge.rs")
    .file("src/chart_bridge.rs")
    .file("src/bar_feed.rs")
    .file("src/execution_models.rs")
    .file("src/execution_controls.rs")
    .file("src/ops_status.rs")
    .cpp_file("src/render_backend.cpp")
    .cpp_file("cpp/bar_chart_node.cpp")
    .cpp_file(manifest_dir.join("cpp/bar_chart_item.h"))
    .cpp_file("cpp/bar_chart_item.cpp")
    .cpp_file(manifest_dir.join("cpp/overlay_chart_item.h"))
    .cpp_file("cpp/overlay_chart_item.cpp")
    .cpp_file("cpp/bar_chart_probe.cpp")
    .cpp_file("cpp/chart_cxx.cpp")
    .cpp_file("cpp/execution_models_cxx.cpp")
    .cpp_file("cpp/frame_bench.cpp")
    .cpp_file(manifest_dir.join("cpp/table_model.h"))
    .cpp_file("cpp/table_model.cpp")
    .cpp_file("cpp/table_bridge.cpp")
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
