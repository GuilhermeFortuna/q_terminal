//! Placement intent vs placement authority (Q-053).

use serde::{Deserialize, Serialize};

use super::identity::{
    clamp_geometry_to_display, resolve_binding, scale_geometry, DisplayDescription,
    ResolutionReport, ScreenFingerprint, ScreenGeometry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementMode {
    /// X11, XWayland or offscreen: the terminal sets geometry directly.
    Direct,
    /// Native Wayland: compositor rules and window identity only.
    Compositor,
    /// No placement authority (headless report, unknown platform).
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryIntent {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<ScreenGeometry> for GeometryIntent {
    fn from(g: ScreenGeometry) -> Self {
        Self {
            x: g.x,
            y: g.y,
            width: g.width,
            height: g.height,
        }
    }
}

impl From<GeometryIntent> for ScreenGeometry {
    fn from(g: GeometryIntent) -> Self {
        ScreenGeometry {
            x: g.x,
            y: g.y,
            width: g.width,
            height: g.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowPlacementIntent {
    pub window_key: String,
    pub composition: String,
    pub display_binding: ScreenFingerprint,
    pub geometry: GeometryIntent,
    pub resolution: ResolutionReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementReport {
    pub mode: PlacementMode,
    pub windows: Vec<WindowPlacementIntent>,
    pub message: String,
}

pub struct PlacementStrategy {
    pub mode: PlacementMode,
}

impl PlacementStrategy {
    pub fn detect_from_env() -> Self {
        let platform = std::env::var("QT_QPA_PLATFORM")
            .unwrap_or_default()
            .to_lowercase();
        let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok()
            && !platform.contains("xcb")
            && !platform.contains("offscreen");
        let mode = if on_wayland {
            PlacementMode::Compositor
        } else {
            PlacementMode::Direct
        };
        Self { mode }
    }

    pub fn for_tests(mode: PlacementMode) -> Self {
        Self { mode }
    }

    /// Resolves display bindings and prepares geometry for each window.
    pub fn prepare(
        &self,
        workspace_name: &str,
        windows: &[super::compositor::CompositorWindow],
        displays: &[DisplayDescription],
    ) -> PlacementReport {
        let mut prepared = Vec::new();
        for w in windows {
            let (display, resolution) = resolve_binding(&w.display_binding, displays);
            let saved_geom = ScreenGeometry::from(w.geometry.clone());
            let actual_geom = if let Some(d) = display {
                if resolution.rule == super::identity::ResolutionRule::StableMinusGeometry {
                    scale_geometry(
                        &saved_geom,
                        &w.display_binding.geometry,
                        &d.fingerprint.geometry,
                    )
                } else {
                    clamp_geometry_to_display(&saved_geom, &d.fingerprint.geometry)
                }
            } else {
                saved_geom
            };
            prepared.push(WindowPlacementIntent {
                window_key: w.window_key.clone(),
                composition: w.composition.clone(),
                display_binding: w.display_binding.clone(),
                geometry: GeometryIntent::from(actual_geom),
                resolution,
            });
        }
        let message = match self.mode {
            PlacementMode::Direct => {
                "windows placed directly by the terminal".to_string()
            }
            PlacementMode::Compositor => format!(
                "compositor places windows for workspace '{workspace_name}'; export rules to install them"
            ),
            PlacementMode::None => "no placement authority on this platform".to_string(),
        };
        PlacementReport {
            mode: self.mode,
            windows: prepared,
            message,
        }
    }

    pub fn claims_direct_restore(&self) -> bool {
        self.mode == PlacementMode::Direct
    }
}

#[cfg(test)]
mod tests {
    use super::super::compositor::CompositorWindow;
    use super::super::identity::fixtures::*;
    use super::*;

    #[test]
    fn offscreen_uses_direct_mode() {
        let strategy = PlacementStrategy::for_tests(PlacementMode::Direct);
        assert!(strategy.claims_direct_restore());
    }

    #[test]
    fn wayland_does_not_claim_direct_restore() {
        let strategy = PlacementStrategy::for_tests(PlacementMode::Compositor);
        assert!(!strategy.claims_direct_restore());
    }

    #[test]
    fn prepare_scales_geometry_on_resolution_change() {
        let strategy = PlacementStrategy::for_tests(PlacementMode::Direct);
        let binding = asus_vg328().fingerprint;
        let windows = vec![CompositorWindow {
            window_key: "market".into(),
            composition: "market".into(),
            title: "q_terminal · market".into(),
            app_id: "q_terminal-trading-market".into(),
            display_binding: binding,
            geometry: GeometryIntent {
                x: 0,
                y: 0,
                width: 1280,
                height: 800,
            },
        }];
        let report = strategy.prepare("Trading", &windows, &[asus_different_resolution()]);
        assert_eq!(
            report.windows[0].resolution.rule,
            super::super::identity::ResolutionRule::StableMinusGeometry
        );
        assert!(report.windows[0].geometry.width > 1280);
    }
}
