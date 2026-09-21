//! Compositor placement rule export (Hyprland first, Q-053).

use serde::{Deserialize, Serialize};

use super::identity::ScreenFingerprint;
use super::placement::GeometryIntent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositorWindow {
    pub window_key: String,
    pub composition: String,
    pub title: String,
    pub app_id: String,
    pub display_binding: ScreenFingerprint,
    pub geometry: GeometryIntent,
}

/// Builds a Hyprland window rule block for the saved workspace.
pub fn export_hyprland_rules(workspace_name: &str, windows: &[CompositorWindow]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# q_terminal workspace: {workspace_name}\n"));
    out.push_str("# Install under ~/.config/hypr/windowrules.conf or merge into hyprland.conf\n\n");
    for w in windows {
        out.push_str(&format!(
            "windowrulev2 = float, class:^{}$\n",
            escape_regex(&w.app_id)
        ));
        out.push_str(&format!(
            "windowrulev2 = size {} {}, class:^{}$\n",
            w.geometry.width,
            w.geometry.height,
            escape_regex(&w.app_id)
        ));
        out.push_str(&format!(
            "windowrulev2 = move {} {}, class:^{}$\n",
            w.geometry.x,
            w.geometry.y,
            escape_regex(&w.app_id)
        ));
        out.push_str(&format!(
            "windowrulev2 = monitor {}, class:^{}$\n",
            escape_regex(&w.display_binding.name),
            escape_regex(&w.app_id)
        ));
        out.push_str(&format!(
            "windowrulev2 = title:^{}$, class:^{}$\n",
            escape_regex(&w.title),
            escape_regex(&w.app_id)
        ));
        out.push('\n');
    }
    out
}

fn escape_regex(s: &str) -> String {
    s.replace('\\', "\\\\")
}

/// Stable per-window identity derived from workspace and composition.
pub fn window_app_id(workspace_name: &str, composition: &str) -> String {
    let ws = slug(workspace_name);
    let comp = slug(composition);
    format!("q_terminal-{ws}-{comp}")
}

pub fn window_title(workspace_name: &str, composition: &str) -> String {
    format!("q_terminal · {} · {}", workspace_name, composition)
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::super::identity::fixtures::*;
    use super::*;

    #[test]
    fn hyprland_rules_match_fixture() {
        let windows = vec![
            CompositorWindow {
                window_key: "market".into(),
                composition: "market".into(),
                title: window_title("Trading", "market"),
                app_id: window_app_id("Trading", "market"),
                display_binding: asus_vg328().fingerprint,
                geometry: GeometryIntent {
                    x: 0,
                    y: 0,
                    width: 1280,
                    height: 800,
                },
            },
            CompositorWindow {
                window_key: "operations".into(),
                composition: "operations".into(),
                title: window_title("Trading", "operations"),
                app_id: window_app_id("Trading", "operations"),
                display_binding: samsung_odyssey_g5().fingerprint,
                geometry: GeometryIntent {
                    x: 1920,
                    y: 0,
                    width: 1280,
                    height: 800,
                },
            },
        ];
        let rules = export_hyprland_rules("Trading", &windows);
        assert!(rules.contains("q_terminal-trading-market"));
        assert!(rules.contains("q_terminal-trading-operations"));
        assert!(rules.contains("monitor HDMI-A-1"));
        assert!(rules.contains("monitor DP-3"));
        assert!(rules.contains("size 1280 800"));
        assert!(rules.contains("move 1920 0"));
    }
}
