#[path = "bar_feed.rs"]
#[allow(clippy::float_cmp)]
pub mod bar_feed;
#[path = "execution_controls.rs"]
pub mod execution_controls;
#[path = "execution_models.rs"]
pub mod execution_models;
#[path = "ops_status.rs"]
pub mod ops_status;

#[cxx_qt::bridge]
pub mod ffi {
    #[allow(dead_code)]
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qqmlapplicationengine.h");
        type QQmlApplicationEngine = cxx_qt_lib::QQmlApplicationEngine;

        include!("render_backend.h");
        fn setup_window(engine: Pin<&mut QQmlApplicationEngine>);
        fn query_graphics_api(engine: Pin<&mut QQmlApplicationEngine>) -> QString;

        include!("frame_bench.h");
        fn run_frame_bench(
            engine: Pin<&mut QQmlApplicationEngine>,
            visible_buckets: i32,
            bar_count: i32,
            duration_ms: i32,
            execution_rows: i32,
            markers: i32,
            overlays: i32,
        );
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, app_version)] // this repository's version
        #[qproperty(QString, core_version)] // from q_core::CoreInfo
        #[qproperty(QString, contracts_rev)] // from CONTRACTS_REV
        #[qproperty(QString, render_backend)] // QSGRendererInterface graphics API in use
        type AppInfo = super::AppInfoRust;
    }
}

use cxx_qt_lib::QString;

pub struct AppInfoRust {
    pub app_version: QString,
    pub core_version: QString,
    pub contracts_rev: QString,
    pub render_backend: QString,
}

pub fn read_core_info() -> (QString, QString) {
    #[allow(dead_code)]
    struct CoreInfoMirror {
        version: QString,
        contracts_rev: QString,
    }
    let rust = q_qt::CoreInfoRust::default();
    let mirror: CoreInfoMirror = unsafe { std::mem::transmute(rust) };
    (mirror.version, mirror.contracts_rev)
}

impl AppInfoRust {
    pub fn new(render_backend: &str) -> Self {
        let (core_ver, contracts) = read_core_info();
        Self {
            app_version: QString::from(env!("CARGO_PKG_VERSION")),
            core_version: core_ver,
            contracts_rev: contracts,
            render_backend: QString::from(render_backend),
        }
    }
}

impl Default for AppInfoRust {
    fn default() -> Self {
        Self::new("headless")
    }
}
