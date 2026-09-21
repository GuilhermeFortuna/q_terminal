//! Panel registry (Q-052): every panel has a stable string identity, a title, an icon and a
//! rule for whether it may appear more than once. Q-053 persists these identifiers, so they
//! are chosen here and never derived from object identity.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelSpec {
    /// Stable kind identifier; the id of the first (or only) instance.
    pub kind: &'static str,
    pub title: &'static str,
    /// Icon name resolved by `qml/theme/Icons.qml`.
    pub icon: &'static str,
    /// Whether a second instance (`kind#2`, ...) may exist.
    pub multiple: bool,
    /// Empty surface reserved for a phase 5 panel.
    pub placeholder: bool,
}

pub const PANELS: [PanelSpec; 8] = [
    PanelSpec {
        kind: "status",
        title: "Status",
        icon: "pulse",
        multiple: false,
        placeholder: false,
    },
    PanelSpec {
        kind: "deployments",
        title: "Deployments",
        icon: "list",
        multiple: false,
        placeholder: false,
    },
    PanelSpec {
        kind: "detail",
        title: "Detail",
        icon: "table",
        multiple: false,
        placeholder: false,
    },
    PanelSpec {
        kind: "chart",
        title: "Chart",
        icon: "chart",
        // One feed carries one viewport, so a second chart would fight over its geometry.
        multiple: false,
        placeholder: false,
    },
    PanelSpec {
        kind: "instrument",
        title: "Instrument",
        icon: "info",
        multiple: false,
        placeholder: false,
    },
    PanelSpec {
        kind: "tape",
        title: "Tape",
        icon: "list",
        multiple: true,
        placeholder: true,
    },
    PanelSpec {
        kind: "dom",
        title: "DOM",
        icon: "table",
        multiple: true,
        placeholder: true,
    },
    PanelSpec {
        kind: "footprint",
        title: "Footprint",
        icon: "chart",
        multiple: true,
        placeholder: true,
    },
];

pub fn spec(kind: &str) -> Option<&'static PanelSpec> {
    PANELS.iter().find(|p| p.kind == kind)
}

/// The kind of a panel id: `chart#2` is a `chart`.
pub fn kind_of(panel_id: &str) -> &str {
    panel_id.split('#').next().unwrap_or(panel_id)
}

/// Every id the registry can name: known kinds, with an optional `#n` instance suffix on
/// kinds that allow several.
pub fn is_valid_id(panel_id: &str) -> bool {
    let Some(spec) = spec(kind_of(panel_id)) else {
        return false;
    };
    match panel_id.split_once('#') {
        None => true,
        Some((_, n)) => spec.multiple && n.parse::<u32>().map(|n| n >= 2).unwrap_or(false),
    }
}

/// The next free instance id of `kind` given the ids already in use, or `None` if the kind
/// is unknown or is a singleton that already exists.
pub fn next_instance_id<'a>(
    kind: &str,
    existing: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let spec = spec(kind)?;
    let taken: Vec<&str> = existing.into_iter().collect();
    if !taken.contains(&kind) {
        return Some(kind.to_string());
    }
    if !spec.multiple {
        return None;
    }
    (2u32..)
        .map(|n| format!("{kind}#{n}"))
        .find(|id| !taken.contains(&id.as_str()))
}

/// Registry as data for QML: `{id: {title, icon, multiple, placeholder}}` per kind.
pub fn registry_json() -> String {
    let map: BTreeMap<&str, serde_json::Value> = PANELS
        .iter()
        .map(|p| {
            (
                p.kind,
                serde_json::json!({
                    "title": p.title,
                    "icon": p.icon,
                    "multiple": p.multiple,
                    "placeholder": p.placeholder,
                }),
            )
        })
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_unique_and_titled() {
        let mut kinds: Vec<_> = PANELS.iter().map(|p| p.kind).collect();
        kinds.sort();
        kinds.dedup();
        assert_eq!(kinds.len(), PANELS.len());
        assert!(PANELS
            .iter()
            .all(|p| !p.title.is_empty() && !p.icon.is_empty()));
    }

    #[test]
    fn singleton_has_one_instance_only() {
        assert_eq!(next_instance_id("detail", []), Some("detail".into()));
        assert_eq!(next_instance_id("detail", ["detail"]), None);
    }

    #[test]
    fn multiple_kinds_number_their_instances() {
        assert_eq!(next_instance_id("tape", ["tape"]), Some("tape#2".into()));
        assert_eq!(
            next_instance_id("tape", ["tape", "tape#2"]),
            Some("tape#3".into())
        );
    }

    #[test]
    fn ids_validate_against_the_registry() {
        assert!(is_valid_id("chart"));
        assert!(is_valid_id("tape#2"));
        assert!(!is_valid_id("tape#1"));
        assert!(!is_valid_id("chart#2"));
        assert!(!is_valid_id("detail#2"));
        assert!(!is_valid_id("nope"));
        assert_eq!(kind_of("tape#4"), "tape");
    }
}
