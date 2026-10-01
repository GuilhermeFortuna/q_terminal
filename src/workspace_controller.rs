#![allow(clippy::too_many_arguments)]

//! QML face of workspace persistence and display placement (Q-053).

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::display::compositor::{
    export_hyprland_rules, window_app_id, window_title, CompositorWindow,
};
use crate::display::identity::DisplayDescription;
use crate::display::placement::{PlacementMode, PlacementStrategy};
use crate::shell::layout::Node;
use crate::workspace::autosave::Autosaver;
use crate::workspace::resolve::resolve_for_displays;
use crate::workspace::schema::{
    WorkspaceChartPreferences, WorkspaceFile, WorkspaceSelection, WorkspaceWindow,
};
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
        #[qproperty(QString, pending_study_sets_json)]
        #[qproperty(QString, pending_chart_preferences_json)]
        #[qproperty(bool, is_dirty)]
        #[qproperty(bool, is_restoring)]
        #[qproperty(QString, save_error)]
        #[qproperty(i32, revision)]
        type WorkspaceController = super::WorkspaceControllerRust;

        #[qinvokable]
        fn save(
            self: Pin<&mut WorkspaceController>,
            shell_windows_json: QString,
            selection_json: QString,
            study_sets_json: QString,
            chart_preferences_json: QString,
            name: QString,
        );
        #[qinvokable]
        fn flush_pending(
            self: Pin<&mut WorkspaceController>,
            shell_windows_json: QString,
            selection_json: QString,
            study_sets_json: QString,
            chart_preferences_json: QString,
        ) -> bool;
        #[qinvokable]
        fn mark_dirty(self: Pin<&mut WorkspaceController>);
        #[qinvokable]
        fn suppress_autosave(self: Pin<&mut WorkspaceController>);
        #[qinvokable]
        fn resume_autosave(self: Pin<&mut WorkspaceController>);
        #[qinvokable]
        fn capture_chart_preferences(
            self: &WorkspaceController,
            panel_id: QString,
            symbol: QString,
            timeframe: QString,
            mode: QString,
            followed_deployment_id: QString,
            last_manual_symbol: QString,
            last_manual_timeframe: QString,
            visible_bars: i32,
        ) -> QString;
        #[qinvokable]
        fn restore_chart_preferences(self: &WorkspaceController, panel_id: QString) -> QString;
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
            study_sets_json: QString,
            chart_preferences_json: QString,
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
    pub pending_study_sets_json: QString,
    pub pending_chart_preferences_json: QString,
    pub is_dirty: bool,
    pub is_restoring: bool,
    pub save_error: QString,
    pub revision: i32,
    store: WorkspaceStore,
    strategy: PlacementStrategy,
    pending_windows: Vec<WorkspaceWindow>,
    autosaver: Autosaver,
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
            pending_study_sets_json: QString::from("{}"),
            pending_chart_preferences_json: QString::from("{}"),
            is_dirty: false,
            is_restoring: false,
            save_error: QString::default(),
            revision: 0,
            store,
            strategy,
            pending_windows: Vec::new(),
            autosaver: Autosaver::default(),
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
        self.as_mut().suppress_autosave();
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
        self.as_mut().set_pending_study_sets_json(QString::from(
            serde_json::to_string(&resolved.study_sets).unwrap_or_else(|_| "{}".into()),
        ));
        self.as_mut()
            .set_pending_chart_preferences_json(QString::from(
                serde_json::to_string(&resolved.chart_preferences).unwrap_or_else(|_| "{}".into()),
            ));
        let _ = self.rust().store.set_last_used(&resolved.name);
    }

    pub fn suppress_autosave(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().autosaver.suppress();
        self.as_mut().set_is_restoring(true);
    }

    pub fn resume_autosave(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().autosaver.resume();
        self.as_mut().set_is_restoring(false);
    }

    pub fn mark_dirty(mut self: std::pin::Pin<&mut Self>) {
        if self.rust().autosaver.is_suppressed() {
            return;
        }
        self.as_mut()
            .rust_mut()
            .autosaver
            .mark_dirty(std::time::Instant::now());
        self.as_mut().set_is_dirty(true);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn capture_chart_preferences(
        &self,
        panel_id: QString,
        symbol: QString,
        timeframe: QString,
        mode: QString,
        followed_deployment_id: QString,
        last_manual_symbol: QString,
        last_manual_timeframe: QString,
        visible_bars: i32,
    ) -> QString {
        let key = if panel_id.to_string().is_empty() {
            "chart".to_string()
        } else {
            panel_id.to_string()
        };
        let dep_id = followed_deployment_id.to_string();
        let man_sym = last_manual_symbol.to_string();
        let man_tf = last_manual_timeframe.to_string();
        let prefs = WorkspaceChartPreferences {
            symbol: symbol.to_string(),
            timeframe: timeframe.to_string(),
            mode: mode.to_string(),
            followed_deployment_id: if dep_id.is_empty() {
                None
            } else {
                Some(dep_id)
            },
            last_manual_symbol: if man_sym.is_empty() {
                None
            } else {
                Some(man_sym)
            },
            last_manual_timeframe: if man_tf.is_empty() {
                None
            } else {
                Some(man_tf)
            },
            visible_bars: if visible_bars > 0 {
                Some(visible_bars)
            } else {
                Some(120)
            },
        };
        let mut map = std::collections::HashMap::new();
        map.insert(key, prefs);
        QString::from(serde_json::to_string(&map).unwrap_or_else(|_| "{}".into()))
    }

    pub fn restore_chart_preferences(&self, panel_id: QString) -> QString {
        let key = if panel_id.to_string().is_empty() {
            "chart"
        } else {
            &panel_id.to_string()
        };
        let json = self.rust().pending_chart_preferences_json.to_string();
        if let Ok(map) = serde_json::from_str::<
            std::collections::HashMap<String, WorkspaceChartPreferences>,
        >(&json)
        {
            if let Some(prefs) = map.get(key) {
                return QString::from(serde_json::to_string(prefs).unwrap_or_else(|_| "{}".into()));
            }
        }
        QString::from("{}")
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

    pub fn capture_current(
        &self,
        shell_windows_json: QString,
        selection_json: QString,
        study_sets_json: QString,
        chart_preferences_json: QString,
    ) -> QString {
        let name = self.rust().active_workspace.to_string();
        let windows: Vec<crate::shell::layout::Window> =
            serde_json::from_str(&shell_windows_json.to_string()).unwrap_or_default();
        let selection: WorkspaceSelection =
            serde_json::from_str(&selection_json.to_string()).unwrap_or_default();
        let study_sets: std::collections::HashMap<
            String,
            Vec<crate::workspace::schema::WorkspaceStudy>,
        > = serde_json::from_str(&study_sets_json.to_string()).unwrap_or_default();
        let mut chart_preferences: std::collections::HashMap<
            String,
            crate::workspace::schema::WorkspaceChartPreferences,
        > = serde_json::from_str(&chart_preferences_json.to_string()).unwrap_or_default();
        if chart_preferences.is_empty() {
            chart_preferences =
                serde_json::from_str(&self.rust().pending_chart_preferences_json.to_string())
                    .unwrap_or_default();
        }
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
        let final_windows: Vec<WorkspaceWindow> =
            if captured.is_empty() && !self.rust().pending_windows.is_empty() {
                self.rust().pending_windows.clone()
            } else {
                captured
                    .into_iter()
                    .map(|c| WorkspaceWindow {
                        composition: c.composition,
                        root: c.root,
                        display: c.display,
                        geometry: c.geometry,
                        detached: c.detached,
                    })
                    .collect()
            };
        let file = WorkspaceFile {
            schema_version: crate::workspace::schema::CURRENT_SCHEMA_VERSION,
            name,
            windows: final_windows,
            selection,
            study_sets,
            chart_preferences,
        };
        QString::from(serde_json::to_string(&file).unwrap_or_else(|_| "{}".into()))
    }

    pub fn save(
        mut self: std::pin::Pin<&mut Self>,
        shell_windows_json: QString,
        selection_json: QString,
        study_sets_json: QString,
        chart_preferences_json: QString,
        name: QString,
    ) {
        let payload = self.capture_current(
            shell_windows_json,
            selection_json,
            study_sets_json,
            chart_preferences_json,
        );
        if let Ok(mut file) = serde_json::from_str::<WorkspaceFile>(&payload.to_string()) {
            file.name = name.to_string();
            if file.windows.is_empty() {
                eprintln!("workspace save: refusing to overwrite with empty window layout");
                return;
            }
            if let Err(e) = self.rust().store.save(&file) {
                let err_msg = e.to_string();
                eprintln!("workspace save: {err_msg}");
                self.as_mut().set_save_error(QString::from(err_msg));
            } else {
                let rev = self.rust().autosaver.dirty_revision();
                self.as_mut().rust_mut().autosaver.on_save_success(rev);
                self.as_mut().set_is_dirty(false);
                self.as_mut().set_save_error(QString::default());
                self.as_mut().set_active_workspace(name);
                self.as_mut()
                    .rust_mut()
                    .pending_windows
                    .clone_from(&file.windows);
            }
        }
        self.as_mut().publish_lists();
    }

    pub fn flush_pending(
        mut self: std::pin::Pin<&mut Self>,
        shell_windows_json: QString,
        selection_json: QString,
        study_sets_json: QString,
        chart_preferences_json: QString,
    ) -> bool {
        let name = self.rust().active_workspace.to_string();
        if name.is_empty() {
            return false;
        }
        if !self.rust().autosaver.is_dirty() {
            return true;
        }
        let rev = match self.as_mut().rust_mut().autosaver.prepare_flush() {
            Some(r) => r,
            None => return true,
        };
        let payload = self.capture_current(
            shell_windows_json,
            selection_json,
            study_sets_json,
            chart_preferences_json,
        );
        if let Ok(mut file) = serde_json::from_str::<WorkspaceFile>(&payload.to_string()) {
            file.name = name;
            if file.windows.is_empty() {
                eprintln!(
                    "workspace flush_pending: refusing to overwrite with empty window layout"
                );
                return false;
            }
            match self.rust().store.save(&file) {
                Ok(()) => {
                    self.as_mut().rust_mut().autosaver.on_save_success(rev);
                    let still_dirty = self.rust().autosaver.is_dirty();
                    self.as_mut().set_is_dirty(still_dirty);
                    self.as_mut().set_save_error(QString::default());
                    self.as_mut()
                        .rust_mut()
                        .pending_windows
                        .clone_from(&file.windows);
                    self.as_mut().publish_lists();
                    true
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    eprintln!("workspace flush_pending failed: {err_msg}");
                    self.as_mut()
                        .rust_mut()
                        .autosaver
                        .on_save_failure(err_msg.clone());
                    self.as_mut().set_is_dirty(true);
                    self.as_mut().set_save_error(QString::from(err_msg));
                    false
                }
            }
        } else {
            false
        }
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
