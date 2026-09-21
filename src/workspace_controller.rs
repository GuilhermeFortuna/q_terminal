//! QML face of workspace persistence and display placement (Q-053).

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::display::compositor::{
    export_hyprland_rules, window_app_id, window_title, CompositorWindow,
};
use crate::display::identity::DisplayDescription;
use crate::display::placement::{PlacementMode, PlacementStrategy};
use crate::shell::layout::Node;
use crate::workspace::resolve::resolve_for_displays;
use crate::workspace::schema::{WorkspaceFile, WorkspaceSelection, WorkspaceWindow};
use crate::workspace::store::WorkspaceStore;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("placement_cxx.h");
        fn placement_mode_name() -> QString;
        fn displays_json() -> QString;
        fn apply_window_placement(
            title: &QString,
            app_id: &QString,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            direct: bool,
        );
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, active_workspace)]
        #[qproperty(QString, placement_mode)]
        #[qproperty(QString, workspace_list)]
        #[qproperty(QString, reports_json)]
        #[qproperty(QString, pending_restore_json)]
        #[qproperty(QString, pending_placement_json)]
        #[qproperty(QString, pending_selection_json)]
        #[qproperty(i32, revision)]
        type WorkspaceController = super::WorkspaceControllerRust;

        #[qinvokable]
        fn save(
            self: Pin<&mut WorkspaceController>,
            shell_windows_json: QString,
            selection_json: QString,
            name: QString,
        );
        #[qinvokable]
        fn switch_to(self: Pin<&mut WorkspaceController>, name: QString);
        #[qinvokable]
        fn duplicate(self: Pin<&mut WorkspaceController>, name: QString, as_name: QString);
        #[qinvokable]
        fn remove(self: Pin<&mut WorkspaceController>, name: QString);
        #[qinvokable]
        fn export_compositor_rules(self: &WorkspaceController) -> QString;
        #[qinvokable]
        fn restore_last(
            self: Pin<&mut WorkspaceController>,
            shell_windows_json: QString,
        ) -> QString;
        #[qinvokable]
        fn capture_current(
            self: &WorkspaceController,
            shell_windows_json: QString,
            selection_json: QString,
        ) -> QString;
        #[qinvokable]
        fn apply_placement(
            self: &WorkspaceController,
            workspace_name: QString,
            windows_json: QString,
        );
        #[qinvokable]
        fn refresh(self: Pin<&mut WorkspaceController>);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RestoreSpec {
    composition: String,
    root: Node,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CapturedWindow {
    composition: String,
    root: Node,
    display: crate::display::identity::ScreenFingerprint,
    geometry: crate::display::placement::GeometryIntent,
    detached: bool,
}

pub struct WorkspaceControllerRust {
    pub active_workspace: QString,
    pub placement_mode: QString,
    pub workspace_list: QString,
    pub reports_json: QString,
    pub pending_restore_json: QString,
    pub pending_placement_json: QString,
    pub pending_selection_json: QString,
    pub revision: i32,
    store: WorkspaceStore,
    strategy: PlacementStrategy,
    pending_windows: Vec<WorkspaceWindow>,
}

impl Default for WorkspaceControllerRust {
    fn default() -> Self {
        let strategy = PlacementStrategy::detect_from_env();
        let store = WorkspaceStore::open_default();
        let mode = mode_string(strategy.mode);
        Self {
            active_workspace: QString::default(),
            placement_mode: QString::from(mode),
            workspace_list: QString::default(),
            reports_json: QString::from("[]"),
            pending_restore_json: QString::from("[]"),
            pending_placement_json: QString::from("[]"),
            pending_selection_json: QString::from("{}"),
            revision: 0,
            store,
            strategy,
            pending_windows: Vec::new(),
        }
    }
}

fn mode_string(mode: PlacementMode) -> &'static str {
    match mode {
        PlacementMode::Direct => "direct",
        PlacementMode::Compositor => "compositor",
        PlacementMode::None => "none",
    }
}

impl ffi::WorkspaceController {
    fn bump(mut self: std::pin::Pin<&mut Self>) {
        let rev = self.rust().revision + 1;
        self.as_mut().set_revision(rev);
    }

    fn publish_lists(mut self: std::pin::Pin<&mut Self>) {
        let names = self.rust().store.list();
        let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".into());
        self.as_mut().set_workspace_list(QString::from(json));
        let reports = self
            .rust()
            .store
            .reports()
            .iter()
            .map(|r| serde_json::json!({"name": r.name, "message": r.message}))
            .collect::<Vec<_>>();
        self.as_mut().set_reports_json(QString::from(
            serde_json::to_string(&reports).unwrap_or_else(|_| "[]".into()),
        ));
        self.bump();
    }

    fn displays() -> Vec<DisplayDescription> {
        let json = ffi::displays_json().to_string();
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub fn refresh(mut self: std::pin::Pin<&mut Self>) {
        let mode = ffi::placement_mode_name().to_string();
        if !mode.is_empty() {
            self.as_mut().set_placement_mode(QString::from(mode));
        }
        self.as_mut().publish_lists();
    }

    pub fn restore_last(
        mut self: std::pin::Pin<&mut Self>,
        _shell_windows_json: QString,
    ) -> QString {
        let (file, _) = self.as_mut().rust_mut().store.load_last_or_default();
        self.as_mut().apply_resolved_workspace(file);
        self.as_mut().publish_lists();
        self.restore_specs_json()
    }

    fn apply_resolved_workspace(mut self: std::pin::Pin<&mut Self>, file: WorkspaceFile) {
        let displays = Self::displays();
        let (resolved, messages) = resolve_for_displays(&file, &displays);
        for msg in messages {
            eprintln!("workspace: {msg}");
        }
        self.as_mut()
            .set_active_workspace(QString::from(resolved.name.clone()));
        self.as_mut()
            .rust_mut()
            .pending_windows
            .clone_from(&resolved.windows);
        let restore_json = self.restore_specs_json();
        self.as_mut().set_pending_restore_json(restore_json);
        self.as_mut().set_pending_placement_json(QString::from(
            serde_json::to_string(&resolved.windows).unwrap_or_else(|_| "[]".into()),
        ));
        self.as_mut().set_pending_selection_json(QString::from(
            serde_json::to_string(&resolved.selection).unwrap_or_else(|_| "{}".into()),
        ));
        let _ = self.rust().store.set_last_used(&resolved.name);
    }

    fn restore_specs_json(&self) -> QString {
        let specs: Vec<RestoreSpec> = self
            .rust()
            .pending_windows
            .iter()
            .map(|w| RestoreSpec {
                composition: w.composition.clone(),
                root: w.root.clone(),
            })
            .collect();
        QString::from(serde_json::to_string(&specs).unwrap_or_else(|_| "[]".into()))
    }

    pub fn apply_placement(&self, workspace_name: QString, windows_json: QString) {
        let windows: Vec<WorkspaceWindow> =
            serde_json::from_str(&windows_json.to_string()).unwrap_or_default();
        let compositor: Vec<CompositorWindow> = windows
            .iter()
            .map(|w| CompositorWindow {
                window_key: w.composition.clone(),
                composition: w.composition.clone(),
                title: window_title(&workspace_name.to_string(), &w.composition),
                app_id: window_app_id(&workspace_name.to_string(), &w.composition),
                display_binding: w.display.clone(),
                geometry: w.geometry.clone(),
            })
            .collect();
        let report = self.rust().strategy.prepare(
            &workspace_name.to_string(),
            &compositor,
            &Self::displays(),
        );
        let direct = report.mode == PlacementMode::Direct;
        for (w, prepared) in windows.iter().zip(report.windows.iter()) {
            let title = window_title(&workspace_name.to_string(), &w.composition);
            let app_id = window_app_id(&workspace_name.to_string(), &w.composition);
            ffi::apply_window_placement(
                &QString::from(title),
                &QString::from(app_id),
                prepared.geometry.x,
                prepared.geometry.y,
                prepared.geometry.width,
                prepared.geometry.height,
                direct,
            );
        }
    }

    pub fn capture_current(&self, shell_windows_json: QString, selection_json: QString) -> QString {
        let name = self.rust().active_workspace.to_string();
        let windows: Vec<crate::shell::layout::Window> =
            serde_json::from_str(&shell_windows_json.to_string()).unwrap_or_default();
        let selection: WorkspaceSelection =
            serde_json::from_str(&selection_json.to_string()).unwrap_or_default();
        let displays = Self::displays();
        let captured: Vec<CapturedWindow> = windows
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let display = displays
                    .get(i)
                    .or_else(|| displays.first())
                    .map(|d| d.fingerprint.clone())
                    .unwrap_or_else(|| {
                        crate::display::identity::fixtures::asus_vg328().fingerprint
                    });
                CapturedWindow {
                    composition: w.composition.clone(),
                    root: w.root.clone(),
                    display,
                    geometry: crate::display::placement::GeometryIntent {
                        x: 0,
                        y: 0,
                        width: 1280,
                        height: 800,
                    },
                    detached: selection.detached.contains_key(&w.id),
                }
            })
            .collect();
        let file = WorkspaceFile {
            schema_version: crate::workspace::schema::CURRENT_SCHEMA_VERSION,
            name,
            windows: captured
                .into_iter()
                .map(|c| WorkspaceWindow {
                    composition: c.composition,
                    root: c.root,
                    display: c.display,
                    geometry: c.geometry,
                    detached: c.detached,
                })
                .collect(),
            selection,
        };
        QString::from(serde_json::to_string(&file).unwrap_or_else(|_| "{}".into()))
    }

    pub fn save(
        mut self: std::pin::Pin<&mut Self>,
        shell_windows_json: QString,
        selection_json: QString,
        name: QString,
    ) {
        let payload = self.capture_current(shell_windows_json, selection_json);
        if let Ok(mut file) = serde_json::from_str::<WorkspaceFile>(&payload.to_string()) {
            file.name = name.to_string();
            if let Err(e) = self.rust().store.save(&file) {
                eprintln!("workspace save: {e}");
            } else {
                self.as_mut().set_active_workspace(name);
            }
        }
        self.as_mut().publish_lists();
    }

    pub fn switch_to(mut self: std::pin::Pin<&mut Self>, name: QString) {
        let name_str = name.to_string();
        match self.as_mut().rust_mut().store.load(&name_str) {
            Ok(file) => {
                self.as_mut().apply_resolved_workspace(file);
            }
            Err(e) => eprintln!("workspace switch: {e}"),
        }
        self.as_mut().publish_lists();
    }

    pub fn duplicate(mut self: std::pin::Pin<&mut Self>, name: QString, as_name: QString) {
        if let Err(e) = self
            .as_mut()
            .rust_mut()
            .store
            .duplicate(&name.to_string(), &as_name.to_string())
        {
            eprintln!("workspace duplicate: {e}");
        }
        self.as_mut().publish_lists();
    }

    pub fn remove(mut self: std::pin::Pin<&mut Self>, name: QString) {
        if let Err(e) = self.as_mut().rust_mut().store.remove(&name.to_string()) {
            eprintln!("workspace remove: {e}");
        }
        self.as_mut().publish_lists();
    }

    pub fn export_compositor_rules(&self) -> QString {
        let name = self.rust().active_workspace.to_string();
        let windows = self.rust().pending_windows.clone();
        let compositor: Vec<CompositorWindow> = windows
            .iter()
            .map(|w| CompositorWindow {
                window_key: w.composition.clone(),
                composition: w.composition.clone(),
                title: window_title(&name, &w.composition),
                app_id: window_app_id(&name, &w.composition),
                display_binding: w.display.clone(),
                geometry: w.geometry.clone(),
            })
            .collect();
        let path = self.rust().store.workspaces_dir().join(format!(
            "{}.hyprland.conf",
            name.replace(' ', "-").to_lowercase()
        ));
        let text = export_hyprland_rules(&name, &compositor);
        if let Err(e) = std::fs::write(&path, &text) {
            eprintln!("workspace export: {e}");
            return QString::default();
        }
        QString::from(path.display().to_string())
    }
}
