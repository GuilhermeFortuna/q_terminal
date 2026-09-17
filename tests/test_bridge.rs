#[path = "../src/bridge.rs"]
mod bridge;
#[path = "../src/config.rs"]
#[allow(dead_code)]
mod config;
#[path = "../src/history/mod.rs"]
mod history;

#[test]
fn test_app_info_core_version_matches_core_info() {
    let app_info = bridge::AppInfoRust::default();
    assert!(!app_info.core_version.to_string().is_empty());
    assert_eq!(app_info.core_version.to_string(), "2026.9.16");
}
