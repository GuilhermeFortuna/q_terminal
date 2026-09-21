//! Stable display fingerprints and binding resolution (Q-053).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenGeometry {
    pub fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.width && py < self.y + self.height
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

/// Attributes kept separately so resolution can degrade when geometry changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenFingerprint {
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub serial: Option<String>,
    pub geometry: ScreenGeometry,
    pub device_pixel_ratio: f64,
    /// When true, this display was primary at save time.
    pub primary: bool,
}

impl ScreenFingerprint {
    pub fn confidence(&self) -> &'static str {
        if self.serial.as_ref().is_some_and(|s| !s.is_empty()) {
            "high"
        } else if !self.manufacturer.is_empty() || !self.model.is_empty() {
            "medium"
        } else {
            "low"
        }
    }

    /// Manufacturer, model and serial — not connector name or geometry.
    fn stable_key(&self) -> (String, String, Option<String>) {
        (
            self.manufacturer.clone(),
            self.model.clone(),
            self.serial.clone(),
        )
    }

    pub fn exact_match(&self, other: &ScreenFingerprint) -> bool {
        self.name == other.name
            && self.stable_key() == other.stable_key()
            && self.geometry == other.geometry
            && (self.device_pixel_ratio - other.device_pixel_ratio).abs() < f64::EPSILON
    }

    pub fn stable_minus_geometry_match(&self, other: &ScreenFingerprint) -> bool {
        self.stable_key() == other.stable_key()
    }
}

/// A display as seen by the platform or a test fixture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayDescription {
    pub fingerprint: ScreenFingerprint,
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionRule {
    Exact,
    StableMinusGeometry,
    GeometryAndRole,
    Primary,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionReport {
    pub rule: ResolutionRule,
    pub confidence: String,
    pub display_name: String,
    pub message: String,
}

impl ResolutionReport {
    fn resolved(rule: ResolutionRule, display: &DisplayDescription, message: &str) -> Self {
        Self {
            rule,
            confidence: display.fingerprint.confidence().to_string(),
            display_name: display.fingerprint.name.clone(),
            message: message.to_string(),
        }
    }
}

/// Resolves a saved binding against the displays currently available.
pub fn resolve_binding(
    binding: &ScreenFingerprint,
    available: &[DisplayDescription],
) -> (Option<DisplayDescription>, ResolutionReport) {
    if available.is_empty() {
        return (
            None,
            ResolutionReport {
                rule: ResolutionRule::Unresolved,
                confidence: binding.confidence().to_string(),
                display_name: binding.name.clone(),
                message: "no displays available".into(),
            },
        );
    }

    for d in available {
        if binding.exact_match(&d.fingerprint) {
            return (
                Some(d.clone()),
                ResolutionReport::resolved(ResolutionRule::Exact, d, "exact fingerprint match"),
            );
        }
    }

    for d in available {
        if binding.stable_minus_geometry_match(&d.fingerprint) {
            return (
                Some(d.clone()),
                ResolutionReport::resolved(
                    ResolutionRule::StableMinusGeometry,
                    d,
                    "same monitor at different resolution",
                ),
            );
        }
    }

    // Geometry-and-role heuristic: match by similar size and saved primary flag.
    let mut best: Option<(usize, i64)> = None;
    for (i, d) in available.iter().enumerate() {
        let dx = (binding.geometry.width - d.fingerprint.geometry.width).abs() as i64;
        let dy = (binding.geometry.height - d.fingerprint.geometry.height).abs() as i64;
        let size_penalty = dx + dy;
        let role_bonus = if binding.primary && d.primary {
            -10_000
        } else {
            0
        };
        let score = size_penalty + role_bonus;
        if best.map(|(_, s)| score < s).unwrap_or(true) {
            best = Some((i, score));
        }
    }
    if let Some((i, _)) = best {
        let d = &available[i];
        return (
            Some(d.clone()),
            ResolutionReport::resolved(
                ResolutionRule::GeometryAndRole,
                d,
                "geometry and role heuristic",
            ),
        );
    }

    let primary = available
        .iter()
        .find(|d| d.primary)
        .or_else(|| available.first())
        .cloned();
    if let Some(d) = primary {
        return (
            Some(d.clone()),
            ResolutionReport::resolved(ResolutionRule::Primary, &d, "primary display fallback"),
        );
    }

    (
        None,
        ResolutionReport {
            rule: ResolutionRule::Unresolved,
            confidence: binding.confidence().to_string(),
            display_name: binding.name.clone(),
            message: "could not resolve display binding".into(),
        },
    )
}

/// Moves a window whose display vanished to the best remaining display.
pub fn rescue_after_removal(
    binding: &ScreenFingerprint,
    geometry: &ScreenGeometry,
    remaining: &[DisplayDescription],
) -> (DisplayDescription, ResolutionReport, ScreenGeometry) {
    let (display, report) = resolve_binding(binding, remaining);
    let display = display.unwrap_or_else(|| {
        remaining
            .iter()
            .find(|d| d.primary)
            .or_else(|| remaining.first())
            .cloned()
            .expect("remaining is non-empty")
    });
    let geom = clamp_geometry_to_display(geometry, &display.fingerprint.geometry);
    let report = if report.rule == ResolutionRule::Unresolved {
        ResolutionReport::resolved(
            ResolutionRule::Primary,
            &display,
            "display removed; rescued to available display",
        )
    } else {
        report
    };
    (display, report, geom)
}

/// Keeps geometry inside a display and never returns coordinates outside every display.
pub fn clamp_geometry_to_display(
    intent: &ScreenGeometry,
    display: &ScreenGeometry,
) -> ScreenGeometry {
    let width = intent.width.min(display.width).max(1);
    let height = intent.height.min(display.height).max(1);
    let max_x = display.x + display.width - width;
    let max_y = display.y + display.height - height;
    let x = intent.x.clamp(display.x, max_x.max(display.x));
    let y = intent.y.clamp(display.y, max_y.max(display.y));
    ScreenGeometry {
        x,
        y,
        width,
        height,
    }
}

/// Scales saved geometry proportionally when the bound display's resolution changed.
pub fn scale_geometry(
    intent: &ScreenGeometry,
    saved_display: &ScreenGeometry,
    actual_display: &ScreenGeometry,
) -> ScreenGeometry {
    if saved_display.width <= 0 || saved_display.height <= 0 {
        return clamp_geometry_to_display(intent, actual_display);
    }
    let sx = actual_display.width as f64 / saved_display.width as f64;
    let sy = actual_display.height as f64 / saved_display.height as f64;
    let scaled = ScreenGeometry {
        x: actual_display.x + ((intent.x - saved_display.x) as f64 * sx).round() as i32,
        y: actual_display.y + ((intent.y - saved_display.y) as f64 * sy).round() as i32,
        width: (intent.width as f64 * sx).round() as i32,
        height: (intent.height as f64 * sy).round() as i32,
    };
    clamp_geometry_to_display(&scaled, actual_display)
}

pub fn geometry_outside_all(intent: &ScreenGeometry, displays: &[DisplayDescription]) -> bool {
    if displays.is_empty() {
        return true;
    }
    let (cx, cy) = intent.center();
    !displays
        .iter()
        .any(|d| d.fingerprint.geometry.contains_point(cx, cy))
}

/// Test fixtures including the workstation's two real monitors.
pub mod fixtures {
    use super::*;

    pub fn asus_vg328() -> DisplayDescription {
        DisplayDescription {
            fingerprint: ScreenFingerprint {
                name: "HDMI-A-1".into(),
                manufacturer: "ASUS".into(),
                model: "VG328".into(),
                serial: None,
                geometry: ScreenGeometry {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                device_pixel_ratio: 1.0,
                primary: true,
            },
            primary: true,
        }
    }

    pub fn samsung_odyssey_g5() -> DisplayDescription {
        DisplayDescription {
            fingerprint: ScreenFingerprint {
                name: "DP-3".into(),
                manufacturer: "SAM".into(),
                model: "Odyssey G5".into(),
                serial: Some("HX5Y903636".into()),
                geometry: ScreenGeometry {
                    x: 1920,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
                device_pixel_ratio: 1.0,
                primary: false,
            },
            primary: false,
        }
    }

    pub fn two_monitor_desk() -> Vec<DisplayDescription> {
        vec![asus_vg328(), samsung_odyssey_g5()]
    }

    pub fn renamed_asus() -> DisplayDescription {
        let mut d = asus_vg328();
        d.fingerprint.name = "HDMI-A-2".into();
        d
    }

    pub fn asus_different_resolution() -> DisplayDescription {
        let mut d = asus_vg328();
        d.fingerprint.geometry = ScreenGeometry {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
        };
        d
    }

    pub fn twin_samsung_no_serial() -> DisplayDescription {
        DisplayDescription {
            fingerprint: ScreenFingerprint {
                name: "DP-1".into(),
                manufacturer: "SAM".into(),
                model: "Odyssey G5".into(),
                serial: None,
                geometry: ScreenGeometry {
                    x: 4480,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
                device_pixel_ratio: 1.0,
                primary: false,
            },
            primary: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn exact_match_survives_output_reorder() {
        let saved = samsung_odyssey_g5().fingerprint;
        let available = vec![samsung_odyssey_g5(), asus_vg328()];
        let (_, report) = resolve_binding(&saved, &available);
        assert_eq!(report.rule, ResolutionRule::Exact);
        assert_eq!(report.display_name, "DP-3");
    }

    #[test]
    fn stable_minus_geometry_matches_resolution_change() {
        let saved = asus_vg328().fingerprint;
        let available = vec![asus_different_resolution()];
        let (_, report) = resolve_binding(&saved, &available);
        assert_eq!(report.rule, ResolutionRule::StableMinusGeometry);
    }

    #[test]
    fn no_serial_still_identifies_with_lower_confidence() {
        let saved = asus_vg328().fingerprint;
        assert_eq!(saved.confidence(), "medium");
        let available = vec![asus_vg328()];
        let (_, report) = resolve_binding(&saved, &available);
        assert_eq!(report.rule, ResolutionRule::Exact);
        assert_eq!(report.confidence, "medium");
    }

    #[test]
    fn identical_models_without_serial_resolve_by_connector_name() {
        let saved = twin_samsung_no_serial().fingerprint;
        let available = vec![samsung_odyssey_g5(), twin_samsung_no_serial()];
        let (resolved, report) = resolve_binding(&saved, &available);
        assert_eq!(report.rule, ResolutionRule::Exact);
        assert_eq!(resolved.unwrap().fingerprint.name, "DP-1");
        assert_eq!(report.confidence, "medium");
    }

    #[test]
    fn removed_display_rescues_window() {
        let binding = samsung_odyssey_g5().fingerprint;
        let geom = ScreenGeometry {
            x: 2000,
            y: 100,
            width: 1280,
            height: 800,
        };
        let remaining = vec![asus_vg328()];
        let (_, report, clamped) = rescue_after_removal(&binding, &geom, &remaining);
        assert_ne!(report.rule, ResolutionRule::Unresolved);
        assert!(!geometry_outside_all(&clamped, &remaining));
    }

    #[test]
    fn clamp_never_leaves_display_bounds() {
        let display = ScreenGeometry {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let intent = ScreenGeometry {
            x: -100,
            y: 900,
            width: 3000,
            height: 2000,
        };
        let out = clamp_geometry_to_display(&intent, &display);
        assert!(out.x >= 0);
        assert!(out.y >= 0);
        assert!(out.x + out.width <= 1920);
        assert!(out.y + out.height <= 1080);
    }
}
