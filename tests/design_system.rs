use std::collections::HashSet;
use std::fs;
use std::path::Path;

const COMPONENTS: &[&str] = &[
    "Panel",
    "SectionHeader",
    "Toolbar",
    "AppButton",
    "IconButton",
    "StatusBadge",
    "ConnectionIndicator",
    "Metric",
    "DataTable",
    "DataTableHeader",
    "DataTableRow",
    "DataTableCell",
    "TabBar",
    "SplitPane",
    "EmptyState",
    "AppDialog",
    "AppTextField",
    "AppComboBox",
];

const STATES: &[&str] = &[
    "rest", "hover", "pressed", "focused", "selected", "disabled", "degraded", "stale",
];

#[test]
fn every_declared_component_exists_and_is_in_the_catalog() {
    let catalog = fs::read_to_string("qml/components/ComponentCatalog.js").unwrap();
    let qmldir = fs::read_to_string("qml/components/qmldir").unwrap();
    for component in COMPONENTS {
        assert!(
            Path::new(&format!("qml/components/{component}.qml")).is_file(),
            "missing component file: {component}"
        );
        assert!(
            catalog.contains(&format!("name: \"{component}\"")),
            "missing catalog entry: {component}"
        );
        assert!(
            qmldir.contains(&format!("{component} 1.0 {component}.qml")),
            "component is not registered: {component}"
        );
    }
}

#[test]
fn component_catalog_declares_every_interaction_state() {
    let catalog = fs::read_to_string("qml/components/ComponentCatalog.js").unwrap();
    for state in STATES {
        assert!(
            catalog.contains(&format!("\"{state}\"")),
            "catalog never declares state: {state}"
        );
    }
    let state_tokens = fs::read_to_string("qml/components/ControlState.qml").unwrap();
    let mut treatments = HashSet::new();
    for state in &STATES[..6] {
        let marker = format!("case \"{state}\": return \"");
        let start = state_tokens
            .find(&marker)
            .unwrap_or_else(|| panic!("missing treatment for state: {state}"))
            + marker.len();
        let end = state_tokens[start..].find('"').unwrap() + start;
        treatments.insert(&state_tokens[start..end]);
    }
    assert_eq!(
        treatments.len(),
        6,
        "rest/hover/pressed/focused/selected/disabled treatments must be distinct"
    );
}

#[test]
fn gallery_is_driven_by_the_same_catalog() {
    let gallery = fs::read_to_string("qml/gallery/Gallery.qml").unwrap();
    assert!(gallery.contains("ComponentCatalog.entries"));
    let entry = fs::read_to_string("qml/gallery/GalleryEntry.qml").unwrap();
    assert!(entry.contains("ComponentPreview"));

    let cargo = fs::read_to_string("Cargo.toml").unwrap();
    assert!(cargo.contains("gallery = []"));
    let build = fs::read_to_string("build.rs").unwrap();
    assert!(build.contains("CARGO_FEATURE_GALLERY"));
}

#[test]
fn vendored_font_and_icon_assets_are_present() {
    for path in [
        "assets/fonts/Inter-Variable.ttf",
        "assets/fonts/JetBrainsMono-Variable.ttf",
        "assets/icons/chevron-down.svg",
        "assets/icons/triangle-alert.svg",
    ] {
        let metadata = fs::metadata(path).unwrap_or_else(|_| panic!("missing asset: {path}"));
        assert!(metadata.len() > 100, "empty asset: {path}");
    }
}
