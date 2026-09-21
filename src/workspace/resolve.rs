//! Workspace resolution against the available display set (Q-053).

use crate::display::identity::{resolve_binding, DisplayDescription, ResolutionRule};

use super::schema::{WorkspaceFile, WorkspaceWindow};

/// Adapts a workspace when the bound displays are not all present.
pub fn resolve_for_displays(
    file: &WorkspaceFile,
    displays: &[DisplayDescription],
) -> (WorkspaceFile, Vec<String>) {
    let mut messages = Vec::new();
    if displays.len() <= 1 && file.windows.len() > 1 {
        messages
            .push("single display available; merged multi-window workspace into one window".into());
        return (merge_workspace_windows(file), messages);
    }

    let mut windows = Vec::new();
    for w in &file.windows {
        let (_, report) = resolve_binding(&w.display, displays);
        if report.rule == ResolutionRule::Unresolved {
            messages.push(format!(
                "window '{}' display unresolved; using primary",
                w.composition
            ));
        } else {
            messages.push(format!(
                "window '{}': {} ({})",
                w.composition,
                report.message,
                report.rule_name()
            ));
        }
        windows.push(w.clone());
    }
    (
        WorkspaceFile {
            schema_version: file.schema_version,
            name: file.name.clone(),
            windows,
            selection: file.selection.clone(),
        },
        messages,
    )
}

fn merge_workspace_windows(file: &WorkspaceFile) -> WorkspaceFile {
    use crate::shell::layout::{Layout, Node, Orientation};

    let mut layout = Layout::default();
    let specs: Vec<(String, Node)> = file
        .windows
        .iter()
        .map(|w| (w.composition.clone(), w.root.clone()))
        .collect();
    let ids = layout.restore_workspace(&specs);
    if let Some(target) = ids.first() {
        let _ = layout.merge_into(target);
    }
    let merged = layout.windows().first().cloned();
    let display = file
        .windows
        .first()
        .map(|w| w.display.clone())
        .unwrap_or_else(|| crate::display::identity::fixtures::asus_vg328().fingerprint);
    let geometry = file
        .windows
        .first()
        .map(|w| w.geometry.clone())
        .unwrap_or_else(|| crate::display::placement::GeometryIntent {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        });
    WorkspaceFile {
        schema_version: file.schema_version,
        name: file.name.clone(),
        windows: vec![WorkspaceWindow {
            composition: merged
                .as_ref()
                .map(|w| w.composition.clone())
                .unwrap_or_else(|| "merged".into()),
            root: merged.map(|w| w.root).unwrap_or_else(|| Node::Split {
                orientation: Orientation::Vertical,
                children: file.windows.iter().map(|w| w.root.clone()).collect(),
                weights: vec![1.0 / file.windows.len() as f64; file.windows.len()],
            }),
            display,
            geometry,
            detached: false,
        }],
        selection: file.selection.clone(),
    }
}

trait RuleName {
    fn rule_name(&self) -> &'static str;
}

impl RuleName for crate::display::identity::ResolutionReport {
    fn rule_name(&self) -> &'static str {
        match self.rule {
            ResolutionRule::Exact => "exact",
            ResolutionRule::StableMinusGeometry => "stable_minus_geometry",
            ResolutionRule::GeometryAndRole => "geometry_and_role",
            ResolutionRule::Primary => "primary",
            ResolutionRule::Unresolved => "unresolved",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::identity::fixtures::*;
    use crate::workspace::schema::default_trading;

    #[test]
    fn single_display_merges_trading_workspace() {
        let file = default_trading();
        let (resolved, msgs) = resolve_for_displays(&file, &[asus_vg328()]);
        assert_eq!(resolved.windows.len(), 1);
        let panels: Vec<&str> = resolved.windows[0].root.panels();
        assert!(panels.contains(&"chart"));
        assert!(panels.contains(&"deployments"));
        assert!(!msgs.is_empty());
    }
}
