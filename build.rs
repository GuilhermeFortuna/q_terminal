use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("qml").qml_file("qml/Main.qml"))
        .qt_module("Quick")
        .file("src/bridge.rs")
        .file("src/bar_feed.rs")
        .cpp_file("src/render_backend.cpp")
        .build();
}
