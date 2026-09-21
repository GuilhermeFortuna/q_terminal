//! QML face of the multi-window shell (Q-052). It owns the layout, the selection context and
//! the command registry as plain Rust structure; QML renders them. It never owns the stores:
//! those are declared once at the shell root and shared by every window.

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;

use crate::shell::commands::CommandRegistry;
use crate::shell::context::SelectionContext;
use crate::shell::layout::{Layout, Window};
use crate::shell::panels;
use crate::workspace::schema::WorkspaceSelection;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// Bumped on every structural change (windows opened, closed, panels moved).
        #[qproperty(i32, revision)]
        #[qproperty(i32, window_count)]
        /// JSON array of the open window ids, in creation order.
        #[qproperty(QString, window_ids)]
        /// Bumped whenever any window's selection or detach state changes.
        #[qproperty(i32, selection_revision)]
        type ShellController = super::ShellControllerRust;

        #[qinvokable]
        fn open_window(self: Pin<&mut ShellController>, composition: QString) -> QString;
        #[qinvokable]
        fn close_window(self: Pin<&mut ShellController>, id: QString);
        #[qinvokable]
        fn move_panel(self: Pin<&mut ShellController>, panel: QString, to_window: QString);
        #[qinvokable]
        fn float_panel(self: Pin<&mut ShellController>, panel: QString) -> QString;
        #[qinvokable]
        fn dock_back(self: Pin<&mut ShellController>, panel: QString);
        #[qinvokable]
        fn merge_into(self: Pin<&mut ShellController>, window: QString);
        #[qinvokable]
        fn split_out(self: Pin<&mut ShellController>, panel: QString) -> QString;
        #[qinvokable]
        fn split_all(self: Pin<&mut ShellController>, window: QString);
        #[qinvokable]
        fn set_active(self: Pin<&mut ShellController>, window: QString, panel: QString);
        #[qinvokable]
        fn set_weights(
            self: Pin<&mut ShellController>,
            window: QString,
            path_json: QString,
            weights_json: QString,
        );

        #[qinvokable]
        fn window_layout(self: &ShellController, id: QString) -> QString;
        #[qinvokable]
        fn layout_json(self: &ShellController) -> QString;
        #[qinvokable]
        fn panel_window(self: &ShellController, panel: QString) -> QString;
        #[qinvokable]
        fn panel_ids(self: &ShellController) -> QString;
        #[qinvokable]
        fn panel_registry(self: &ShellController) -> QString;
        #[qinvokable]
        fn is_merged(self: &ShellController, window: QString) -> bool;

        #[qinvokable]
        fn select_deployment(
            self: Pin<&mut ShellController>,
            window: QString,
            deployment: QString,
        ) -> bool;
        #[qinvokable]
        fn effective_selection(self: &ShellController, window: QString) -> QString;
        #[qinvokable]
        fn global_selection(self: &ShellController) -> QString;
        #[qinvokable]
        fn is_detached(self: &ShellController, window: QString) -> bool;
        #[qinvokable]
        fn detach(self: Pin<&mut ShellController>, window: QString);
        #[qinvokable]
        fn attach(self: Pin<&mut ShellController>, window: QString);

        #[qinvokable]
        fn commands_json(self: &ShellController) -> QString;

        #[qinvokable]
        fn restore_workspace(self: Pin<&mut ShellController>, windows_json: QString) -> QString;
        #[qinvokable]
        fn capture_windows(self: &ShellController) -> QString;
        #[qinvokable]
        fn restore_selection(self: Pin<&mut ShellController>, selection_json: QString);
        #[qinvokable]
        fn capture_selection(self: &ShellController) -> QString;
        #[qinvokable]
        fn clear_windows(self: Pin<&mut ShellController>);
    }
}

pub struct ShellControllerRust {
    pub revision: i32,
    pub window_count: i32,
    pub window_ids: QString,
    pub selection_revision: i32,
    layout: Layout,
    context: SelectionContext,
}

impl Default for ShellControllerRust {
    fn default() -> Self {
        // Fails loudly at startup on a shortcut collision instead of picking a winner.
        let _ = CommandRegistry::shared();
        Self {
            revision: 0,
            window_count: 0,
            window_ids: QString::from("[]"),
            selection_revision: 0,
            layout: Layout::default(),
            context: SelectionContext::default(),
        }
    }
}

impl ffi::ShellController {
    fn publish(mut self: std::pin::Pin<&mut Self>) {
        let ids: Vec<String> = self
            .rust()
            .layout
            .windows()
            .iter()
            .map(|w| w.id.clone())
            .collect();
        let count = ids.len() as i32;
        let json = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into());
        let rev = self.rust().revision + 1;
        self.as_mut().set_window_ids(QString::from(json));
        self.as_mut().set_window_count(count);
        self.as_mut().set_revision(rev);
    }

    fn bump_selection(mut self: std::pin::Pin<&mut Self>) {
        let rev = self.rust().selection_revision + 1;
        self.as_mut().set_selection_revision(rev);
    }

    pub fn open_window(mut self: std::pin::Pin<&mut Self>, composition: QString) -> QString {
        let result = self
            .as_mut()
            .rust_mut()
            .layout
            .open(&composition.to_string());
        match result {
            Ok(id) => {
                self.publish();
                QString::from(id)
            }
            Err(e) => {
                eprintln!("shell: {e}");
                QString::default()
            }
        }
    }

    pub fn close_window(mut self: std::pin::Pin<&mut Self>, id: QString) {
        let id = id.to_string();
        if self.as_mut().rust_mut().layout.close(&id).is_ok() {
            self.as_mut().rust_mut().context.forget(&id);
            self.as_mut().publish();
            self.bump_selection();
        }
    }

    pub fn move_panel(mut self: std::pin::Pin<&mut Self>, panel: QString, to_window: QString) {
        let r = self
            .as_mut()
            .rust_mut()
            .layout
            .move_panel(&panel.to_string(), &to_window.to_string());
        if r.is_ok() {
            self.publish();
        }
    }

    pub fn float_panel(mut self: std::pin::Pin<&mut Self>, panel: QString) -> QString {
        let r = self.as_mut().rust_mut().layout.float(&panel.to_string());
        match r {
            Ok(id) => {
                self.publish();
                QString::from(id)
            }
            Err(_) => QString::default(),
        }
    }

    pub fn dock_back(mut self: std::pin::Pin<&mut Self>, panel: QString) {
        if self
            .as_mut()
            .rust_mut()
            .layout
            .dock_back(&panel.to_string())
            .is_ok()
        {
            self.publish();
        }
    }

    pub fn merge_into(mut self: std::pin::Pin<&mut Self>, window: QString) {
        if self
            .as_mut()
            .rust_mut()
            .layout
            .merge_into(&window.to_string())
            .is_ok()
        {
            self.publish();
        }
    }

    pub fn split_out(mut self: std::pin::Pin<&mut Self>, panel: QString) -> QString {
        let r = self
            .as_mut()
            .rust_mut()
            .layout
            .split_out(&panel.to_string());
        match r {
            Ok(id) => {
                self.publish();
                QString::from(id)
            }
            Err(_) => QString::default(),
        }
    }

    pub fn split_all(mut self: std::pin::Pin<&mut Self>, window: QString) {
        if self
            .as_mut()
            .rust_mut()
            .layout
            .split_all(&window.to_string())
            .is_ok()
        {
            self.publish();
        }
    }

    /// Like pane sizes, the active tab is recorded without a rebuild: the tab bar already
    /// shows it.
    pub fn set_active(mut self: std::pin::Pin<&mut Self>, window: QString, panel: QString) {
        let _ = self
            .as_mut()
            .rust_mut()
            .layout
            .set_active(&window.to_string(), &panel.to_string());
    }

    /// Pane sizes are recorded without bumping `revision`: the panes already show them, and a
    /// rebuild while dragging would drop the drag.
    pub fn set_weights(
        mut self: std::pin::Pin<&mut Self>,
        window: QString,
        path_json: QString,
        weights_json: QString,
    ) {
        let path: Vec<usize> = serde_json::from_str(&path_json.to_string()).unwrap_or_default();
        let weights: Vec<f64> = serde_json::from_str(&weights_json.to_string()).unwrap_or_default();
        let _ = self
            .as_mut()
            .rust_mut()
            .layout
            .set_weights(&window.to_string(), &path, &weights);
    }

    pub fn window_layout(&self, id: QString) -> QString {
        QString::from(self.rust().layout.window_json(&id.to_string()))
    }

    pub fn layout_json(&self) -> QString {
        QString::from(self.rust().layout.to_json())
    }

    pub fn panel_window(&self, panel: QString) -> QString {
        QString::from(
            self.rust()
                .layout
                .window_of(&panel.to_string())
                .unwrap_or_default(),
        )
    }

    pub fn panel_ids(&self) -> QString {
        let ids: Vec<&str> = self.rust().layout.all_panels();
        QString::from(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into()))
    }

    pub fn panel_registry(&self) -> QString {
        QString::from(panels::registry_json())
    }

    pub fn is_merged(&self, window: QString) -> bool {
        self.rust().layout.is_merged(&window.to_string())
    }

    /// Returns whether the *global* selection changed, i.e. whether the shared execution
    /// models must be retargeted. A detached window's selection stays its own.
    pub fn select_deployment(
        mut self: std::pin::Pin<&mut Self>,
        window: QString,
        deployment: QString,
    ) -> bool {
        let changed = self
            .as_mut()
            .rust_mut()
            .context
            .select(&window.to_string(), &deployment.to_string());
        self.bump_selection();
        changed
    }

    pub fn effective_selection(&self, window: QString) -> QString {
        QString::from(self.rust().context.effective(&window.to_string()))
    }

    pub fn global_selection(&self) -> QString {
        QString::from(self.rust().context.global())
    }

    pub fn is_detached(&self, window: QString) -> bool {
        self.rust().context.is_detached(&window.to_string())
    }

    pub fn detach(mut self: std::pin::Pin<&mut Self>, window: QString) {
        self.as_mut().rust_mut().context.detach(&window.to_string());
        self.bump_selection();
    }

    pub fn attach(mut self: std::pin::Pin<&mut Self>, window: QString) {
        self.as_mut().rust_mut().context.attach(&window.to_string());
        self.bump_selection();
    }

    pub fn commands_json(&self) -> QString {
        QString::from(CommandRegistry::shared().to_json())
    }

    pub fn restore_workspace(mut self: std::pin::Pin<&mut Self>, windows_json: QString) -> QString {
        #[derive(serde::Deserialize)]
        struct Spec {
            composition: String,
            root: crate::shell::layout::Node,
        }
        let specs: Vec<Spec> = serde_json::from_str(&windows_json.to_string()).unwrap_or_default();
        let specs: Vec<(String, crate::shell::layout::Node)> =
            specs.into_iter().map(|s| (s.composition, s.root)).collect();
        let ids = self.as_mut().rust_mut().layout.restore_workspace(&specs);
        self.as_mut().publish();
        self.bump_selection();
        QString::from(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into()))
    }

    pub fn capture_windows(&self) -> QString {
        let windows: Vec<Window> = self.rust().layout.windows().to_vec();
        QString::from(serde_json::to_string(&windows).unwrap_or_else(|_| "[]".into()))
    }

    pub fn restore_selection(mut self: std::pin::Pin<&mut Self>, selection_json: QString) {
        if let Ok(sel) = serde_json::from_str::<WorkspaceSelection>(&selection_json.to_string()) {
            self.as_mut()
                .rust_mut()
                .context
                .restore(&sel.global, &sel.detached);
            self.bump_selection();
        }
    }

    pub fn capture_selection(&self) -> QString {
        let sel = WorkspaceSelection {
            global: self.rust().context.global().to_string(),
            detached: self.rust().context.detached_map().clone(),
        };
        QString::from(serde_json::to_string(&sel).unwrap_or_else(|_| "{}".into()))
    }

    pub fn clear_windows(mut self: std::pin::Pin<&mut Self>) {
        self.as_mut().rust_mut().layout.clear();
        self.as_mut().publish();
        self.bump_selection();
    }
}
