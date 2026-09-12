#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, app_version)]
        #[qproperty(QString, core_version)]
        #[qproperty(QString, contracts_rev)]
        #[qproperty(QString, render_backend)]
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
