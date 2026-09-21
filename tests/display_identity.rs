//! Q-053 display fingerprints, rescue, merge and compositor rules.

use q_terminal::display::compositor::{
    export_hyprland_rules, window_app_id, window_title, CompositorWindow,
};
use q_terminal::display::identity::fixtures::*;
use q_terminal::display::identity::{
    geometry_outside_all, rescue_after_removal, resolve_binding, ResolutionRule, ScreenGeometry,
};
use q_terminal::display::placement::{PlacementMode, PlacementStrategy};
use q_terminal::shell::layout::Layout;
use q_terminal::workspace::resolve::resolve_for_displays;
use q_terminal::workspace::schema::default_trading;

#[test]
fn fingerprints_resolve_by_expected_rules() {
    let saved = asus_vg328().fingerprint;
    let (_, exact) = resolve_binding(&saved, &two_monitor_desk());
    assert_eq!(exact.rule, ResolutionRule::Exact);

    let saved = asus_vg328().fingerprint;
    let (_, stable) = resolve_binding(&saved, &[asus_different_resolution()]);
    assert_eq!(stable.rule, ResolutionRule::StableMinusGeometry);

    let saved = renamed_asus().fingerprint;
    let (_, renamed) = resolve_binding(&saved, &[asus_vg328()]);
    assert_eq!(renamed.rule, ResolutionRule::StableMinusGeometry);

    let saved = twin_samsung_no_serial().fingerprint;
    let (resolved, twin) =
        resolve_binding(&saved, &[samsung_odyssey_g5(), twin_samsung_no_serial()]);
    assert_eq!(twin.rule, ResolutionRule::Exact);
    assert_eq!(resolved.unwrap().fingerprint.name, "DP-1");
}

#[test]
fn removed_display_rescues_window_inside_bounds() {
    let binding = samsung_odyssey_g5().fingerprint;
    let geom = ScreenGeometry {
        x: 2000,
        y: 50,
        width: 1280,
        height: 800,
    };
    let remaining = vec![asus_vg328()];
    let (_, report, clamped) = rescue_after_removal(&binding, &geom, &remaining);
    assert_ne!(report.rule, ResolutionRule::Unresolved);
    assert!(!geometry_outside_all(&clamped, &remaining));
}

#[test]
fn single_display_merge_keeps_every_panel() {
    let file = default_trading();
    let (resolved, _) = resolve_for_displays(&file, &[asus_vg328()]);
    assert_eq!(resolved.windows.len(), 1);
    let panels = resolved.windows[0].root.panels();
    assert!(panels.contains(&"chart"));
    assert!(panels.contains(&"deployments"));
    assert!(panels.contains(&"detail"));

    let mut layout = Layout::default();
    let specs: Vec<(String, q_terminal::shell::layout::Node)> = file
        .windows
        .iter()
        .map(|w| (w.composition.clone(), w.root.clone()))
        .collect();
    layout.restore_workspace(&specs);
    layout.merge_all_windows().unwrap();
    let target = layout.windows()[0].id.clone();
    layout.split_all(&target).unwrap();
    assert_eq!(layout.windows().len(), 2);
}

#[test]
fn wayland_does_not_claim_direct_placement() {
    let strategy = PlacementStrategy::for_tests(PlacementMode::Compositor);
    assert!(!strategy.claims_direct_restore());
}

#[test]
fn hyprland_rules_match_trading_fixture() {
    let windows = vec![
        CompositorWindow {
            window_key: "market".into(),
            composition: "market".into(),
            title: window_title("Trading", "market"),
            app_id: window_app_id("Trading", "market"),
            display_binding: asus_vg328().fingerprint,
            geometry: q_terminal::display::placement::GeometryIntent {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        },
        CompositorWindow {
            window_key: "operations".into(),
            composition: "operations".into(),
            title: window_title("Trading", "operations"),
            app_id: window_app_id("Trading", "operations"),
            display_binding: samsung_odyssey_g5().fingerprint,
            geometry: q_terminal::display::placement::GeometryIntent {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
            },
        },
    ];
    let rules = export_hyprland_rules("Trading", &windows);
    assert!(rules.contains("q_terminal-trading-market"));
    assert!(rules.contains("monitor HDMI-A-1"));
    assert!(rules.contains("monitor DP-3"));
}
