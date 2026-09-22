use std::path::{Path, PathBuf};

use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

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

    let mut qml_module = QmlModule::new("qml")
        .qml_file("qml/Main.qml")
        .qml_file("qml/Viewport.qml")
        .qml_file("qml/ChartPane.qml")
        .qml_file("qml/ChartEmptyState.qml")
        .qml_file("qml/StatusStrip.qml")
        .qml_file("qml/OpsHeader.qml")
        .qml_file("qml/DeploymentList.qml")
        .qml_file("qml/OrdersTable.qml")
        .qml_file("qml/FillsTable.qml")
        .qml_file("qml/DecisionsTable.qml")
        .qml_file("qml/RiskTable.qml")
        .qml_file("qml/LedgerTable.qml")
        .qml_file("qml/DeploymentDetail.qml")
        .qml_file("qml/shell/PanelFrame.qml")
        .qml_file("qml/shell/PanelHost.qml")
        .qml_file("qml/shell/CommandPalette.qml")
        .qml_file("qml/shell/ShellWindow.qml")
        .qml_file("qml/shell/WorkspaceMenu.qml")
        .qml_file("qml/shell/PlacementNotice.qml")
        .qml_file("qml/panels/StatusPanel.qml")
        .qml_file("qml/panels/DeploymentsPanel.qml")
        .qml_file("qml/panels/DetailPanel.qml")
        .qml_file("qml/panels/ChartPanel.qml")
        .qml_file("qml/panels/InstrumentPanel.qml")
        .qml_file("qml/panels/PlaceholderPanel.qml")
        .qml_file("qml/ConfirmDialog.qml")
        .qml_file("qml/AccountDialog.qml")
        .qml_file("qml/DeployDialog.qml")
        .qml_file("qml/ResolveDialog.qml")
        .qml_file(QmlFile::from("qml/theme/Palette.qml").singleton(true))
        .qml_file(QmlFile::from("qml/theme/Typography.qml").singleton(true))
        .qml_file(QmlFile::from("qml/theme/Spacing.qml").singleton(true))
        .qml_file(QmlFile::from("qml/theme/Icons.qml").singleton(true))
        .qml_file(QmlFile::from("qml/theme/Semantic.qml").singleton(true))
        .qml_file(QmlFile::from("qml/theme/Theme.qml").singleton(true))
        .qml_file(QmlFile::from("qml/components/ControlState.qml").singleton(true))
        .qml_file("qml/components/Panel.qml")
        .qml_file("qml/components/SectionHeader.qml")
        .qml_file("qml/components/Toolbar.qml")
        .qml_file("qml/components/AppButton.qml")
        .qml_file("qml/components/IconButton.qml")
        .qml_file("qml/components/StatusBadge.qml")
        .qml_file("qml/components/ConnectionIndicator.qml")
        .qml_file("qml/components/Metric.qml")
        .qml_file("qml/components/DataTable.qml")
        .qml_file("qml/components/DataTableHeader.qml")
        .qml_file("qml/components/DataTableRow.qml")
        .qml_file("qml/components/DataTableCell.qml")
        .qml_file("qml/components/TabBar.qml")
        .qml_file("qml/components/SplitPane.qml")
        .qml_file("qml/components/EmptyState.qml")
        .qml_file("qml/components/AppDialog.qml")
        .qml_file("qml/components/AppTextField.qml")
        .qml_file("qml/components/AppComboBox.qml")
        .qml_file("qml/components/ChartIdentity.qml")
        .qml_file("qml/components/ChartTargetPicker.qml")
        .qml_file("qml/style/Button.qml")
        .qml_file("qml/style/TextField.qml")
        .qml_file("qml/style/ComboBox.qml")
        .qml_file("qml/style/TabButton.qml");
    if std::env::var_os("CARGO_FEATURE_GALLERY").is_some() {
        qml_module = qml_module
            .qml_file("qml/gallery/Gallery.qml")
            .qml_file("qml/gallery/GalleryEntry.qml")
            .qml_file("qml/gallery/ComponentPreview.qml");
    }

    CxxQtBuilder::new_qml_module(qml_module)
        .qrc_resources(
            qt_build_utils::QResources::new()
                .resource(
                    qt_build_utils::QResource::new()
                        .prefix("/qt/qml/qml/qml")
                        .file(
                            qt_build_utils::QResourceFile::new("qml/Format.js").alias("Format.js"),
                        ),
                )
                .resource(
                    qt_build_utils::QResource::new()
                        .prefix("/qt/qml/qml/components")
                        .file(
                            qt_build_utils::QResourceFile::new(
                                "qml/components/ComponentCatalog.js",
                            )
                            .alias("ComponentCatalog.js"),
                        ),
                )
                .resource(
                    qt_build_utils::QResource::new().prefix("/qt/qml/qml").file(
                        qt_build_utils::QResourceFile::new("qml/Format.js").alias("Format.js"),
                    ),
                )
                .resource(
                    qt_build_utils::QResource::new()
                        .prefix("/assets/fonts")
                        .file(
                            qt_build_utils::QResourceFile::new("assets/fonts/Inter-Variable.ttf")
                                .alias("Inter-Variable.ttf"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new(
                                "assets/fonts/JetBrainsMono-Variable.ttf",
                            )
                            .alias("JetBrainsMono-Variable.ttf"),
                        ),
                )
                .resource(
                    qt_build_utils::QResource::new()
                        .prefix("/assets/icons")
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/chevron-down.svg")
                                .alias("chevron-down.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/plus.svg")
                                .alias("plus.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/refresh-cw.svg")
                                .alias("refresh-cw.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/power.svg")
                                .alias("power.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/triangle-alert.svg")
                                .alias("triangle-alert.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/circle-x.svg")
                                .alias("circle-x.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/circle-check-big.svg")
                                .alias("circle-check-big.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/info.svg")
                                .alias("info.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/search.svg")
                                .alias("search.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/settings.svg")
                                .alias("settings.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/x.svg").alias("x.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/activity.svg")
                                .alias("activity.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/list.svg")
                                .alias("list.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/table.svg")
                                .alias("table.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new(
                                "assets/icons/chart-candlestick.svg",
                            )
                            .alias("chart-candlestick.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new("assets/icons/external-link.svg")
                                .alias("external-link.svg"),
                        )
                        .file(
                            qt_build_utils::QResourceFile::new(
                                "assets/icons/arrow-down-to-line.svg",
                            )
                            .alias("arrow-down-to-line.svg"),
                        ),
                ),
        )
        // The default is the entire crate root. That makes Cargo watch generated
        // files below target/ and causes every successful build to invalidate the
        // next one. Only cpp/ contains headers exported by this crate.
        .crate_include_root(Some("cpp".to_owned()))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .include_dir("src")
        .file("src/bridge.rs")
        .file("src/chart_bridge.rs")
        .file("src/bar_feed.rs")
        .file("src/chart_context.rs")
        .file("src/execution_models.rs")
        .file("src/execution_controls.rs")
        .file("src/ops_status.rs")
        .file("src/shell_controller.rs")
        .file("src/workspace_controller.rs")
        .cpp_file("cpp/placement_cxx.cpp")
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
