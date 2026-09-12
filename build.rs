use cxx_qt_build::CxxQtBuilder;

fn main() {
    CxxQtBuilder::new()
        .qt_module("Quick")
        .file("src/bridge.rs")
        .build();
}
