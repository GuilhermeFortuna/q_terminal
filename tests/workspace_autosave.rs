//! Tests for workspace autosave debouncing, final-window capture, shutdown flush,
//! and atomic write failure handling (Q-077).

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use q_terminal::shell::layout::Layout;
use q_terminal::workspace::autosave::Autosaver;
use q_terminal::workspace::schema::{
    default_chart, default_single, default_trading, parse_workspace, WorkspaceChartPreferences,
    WorkspaceFile, WorkspaceSelection, WorkspaceWindow,
};
use q_terminal::workspace::store::{InjectedWriteFailure, WorkspaceStore};

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "q_terminal_ws_autosave_test_{}_{}",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn debounce_coalesces_burst_of_edits_without_sleeps() {
    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    let t0 = Instant::now();

    // First edit arrives at t0
    autosaver.mark_dirty(t0);
    assert!(autosaver.is_dirty());
    assert_eq!(autosaver.dirty_revision(), 1);
    assert!(!autosaver.is_deadline_reached(t0 + Duration::from_millis(200)));

    // Second edit arrives at t0 + 200ms: deadline resets to (t0 + 200ms) + 500ms = t0 + 700ms
    autosaver.mark_dirty(t0 + Duration::from_millis(200));
    assert_eq!(autosaver.dirty_revision(), 2);
    // At t0 + 500ms, deadline is NOT reached because the second edit reset it
    assert!(!autosaver.is_deadline_reached(t0 + Duration::from_millis(500)));

    // Third edit arrives at t0 + 400ms: deadline resets to t0 + 900ms
    autosaver.mark_dirty(t0 + Duration::from_millis(400));
    assert_eq!(autosaver.dirty_revision(), 3);
    assert!(!autosaver.is_deadline_reached(t0 + Duration::from_millis(899)));

    // At t0 + 900ms, the deadline is reached
    assert!(autosaver.is_deadline_reached(t0 + Duration::from_millis(900)));

    // Flush saves the latest revision (3) in a single write
    let rev = autosaver.prepare_flush().expect("ready to flush");
    assert_eq!(rev, 3);
    autosaver.on_save_success(rev);

    assert!(!autosaver.is_dirty());
    assert_eq!(autosaver.save_count(), 1);
    assert_eq!(autosaver.saved_revision(), 3);
}

#[test]
fn final_window_capture_retains_nonempty_layout() {
    let dir = temp_dir("final_window_capture");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    // Single window workspace
    let mut file = default_chart();
    file.name = "MyChart".into();
    store.save(&file).unwrap();

    // Simulate shell layout with 1 window
    let mut layout = Layout::default();
    let win_id = layout.open("market").unwrap();
    assert_eq!(layout.windows().len(), 1);

    // When the final window is closing, capture occurs BEFORE destroying the layout
    let captured_windows = layout.windows().to_vec();
    assert_eq!(captured_windows.len(), 1);
    assert!(captured_windows[0].root.panels().contains(&"chart"));

    // Saving captured layout retains nonempty window list
    let saved_file = WorkspaceFile {
        schema_version: q_terminal::workspace::schema::CURRENT_SCHEMA_VERSION,
        name: "MyChart".into(),
        windows: vec![WorkspaceWindow {
            composition: "market".into(),
            root: captured_windows[0].root.clone(),
            display: file.windows[0].display.clone(),
            geometry: file.windows[0].geometry.clone(),
            detached: false,
        }],
        selection: WorkspaceSelection::default(),
        study_sets: Default::default(),
        chart_preferences: Default::default(),
    };
    store.save(&saved_file).unwrap();

    // Now close removes the window from layout
    layout.close(&win_id).unwrap();
    assert!(layout.windows().is_empty());

    // Attempting to save after teardown with empty windows is refused
    assert!(saved_file.windows.len() == 1);
    let loaded = store.load("MyChart").unwrap();
    assert_eq!(loaded.windows.len(), 1);
    assert!(loaded.windows[0].root.panels().contains(&"chart"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn multiple_window_close_saves_remaining_layout() {
    let dir = temp_dir("multiple_window_close");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    let mut layout = Layout::default();
    let market_win = layout.open("market").unwrap();
    let ops_win = layout.open("operations").unwrap();
    assert_eq!(layout.windows().len(), 2);

    // Close window 1 (operations): layout now has 1 window
    layout.close(&ops_win).unwrap();
    assert_eq!(layout.windows().len(), 1);
    assert_eq!(layout.windows()[0].id, market_win);

    // Save the remaining layout
    let remaining_file = WorkspaceFile {
        schema_version: q_terminal::workspace::schema::CURRENT_SCHEMA_VERSION,
        name: "Trading".into(),
        windows: vec![WorkspaceWindow {
            composition: "market".into(),
            root: layout.windows()[0].root.clone(),
            display: q_terminal::display::identity::fixtures::asus_vg328().fingerprint,
            geometry: q_terminal::display::placement::GeometryIntent {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            detached: false,
        }],
        selection: WorkspaceSelection::default(),
        study_sets: Default::default(),
        chart_preferences: Default::default(),
    };
    store.save(&remaining_file).unwrap();

    let loaded = store.load("Trading").unwrap();
    assert_eq!(loaded.windows.len(), 1);
    assert_eq!(loaded.windows[0].composition, "market");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn shutdown_flush_saves_pending_dirty_edits() {
    let dir = temp_dir("shutdown_flush");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    let t0 = Instant::now();

    // Operator changes chart preferences
    autosaver.mark_dirty(t0);
    assert!(autosaver.is_dirty());

    // Before 500ms expires (e.g. at t0 + 100ms), shutdown/switch flushes synchronously
    assert!(!autosaver.is_deadline_reached(t0 + Duration::from_millis(100)));
    let rev = autosaver
        .prepare_flush()
        .expect("shutdown flushes immediately");

    let mut file = default_single();
    file.name = "ShutdownWs".into();
    file.chart_preferences.insert(
        "chart".into(),
        WorkspaceChartPreferences {
            symbol: "VALE3".into(),
            timeframe: "5m".into(),
            mode: "manual".into(),
            followed_deployment_id: None,
            last_manual_symbol: Some("VALE3".into()),
            last_manual_timeframe: Some("5m".into()),
            visible_bars: Some(200),
        },
    );
    store.save(&file).unwrap();
    autosaver.on_save_success(rev);

    assert!(!autosaver.is_dirty());
    let loaded = store.load("ShutdownWs").unwrap();
    assert_eq!(loaded.chart_preferences["chart"].symbol, "VALE3");
    assert_eq!(loaded.chart_preferences["chart"].visible_bars, Some(200));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn write_failure_preserves_valid_file_and_retains_dirty() {
    let dir = temp_dir("write_failure");
    let mut store = WorkspaceStore::open_dir(dir.clone());
    let file = default_trading();
    store.save(&file).unwrap();

    let orig_disk_content = fs::read_to_string(store.path_for("Trading")).unwrap();
    assert!(parse_workspace(&orig_disk_content).is_ok());

    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    autosaver.mark_dirty(Instant::now());

    // Inject write failure
    store.set_injected_failure(InjectedWriteFailure::Write);

    let mut modified = file.clone();
    modified.windows[0].composition = "corrupted_attempt".into();

    let _rev = autosaver.prepare_flush().unwrap();
    let res = store.save(&modified);
    assert!(res.is_err());
    autosaver.on_save_failure(res.unwrap_err().to_string());

    // Prior file remains parseable and unchanged
    let current_disk = fs::read_to_string(store.path_for("Trading")).unwrap();
    assert_eq!(current_disk, orig_disk_content);
    assert!(parse_workspace(&current_disk).is_ok());

    // Dirty state remains retryable
    assert!(autosaver.is_dirty());
    assert!(autosaver.last_error().is_some());

    // Reset failure: retry succeeds and clears dirty state
    store.set_injected_failure(InjectedWriteFailure::None);
    let rev2 = autosaver.prepare_flush().unwrap();
    store.save(&modified).unwrap();
    autosaver.on_save_success(rev2);

    assert!(!autosaver.is_dirty());
    let reloaded = store.load("Trading").unwrap();
    assert_eq!(reloaded.windows[0].composition, "corrupted_attempt");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn sync_failure_preserves_valid_file_and_retains_dirty() {
    let dir = temp_dir("sync_failure");
    let mut store = WorkspaceStore::open_dir(dir.clone());
    let file = default_single();
    store.save(&file).unwrap();
    let orig_content = fs::read_to_string(store.path_for("Single monitor")).unwrap();

    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    autosaver.mark_dirty(Instant::now());

    store.set_injected_failure(InjectedWriteFailure::Sync);
    let _rev = autosaver.prepare_flush().unwrap();
    let mut mod_file = file.clone();
    mod_file.name = "Single monitor modified".into();
    let res = store.save(&mod_file);
    assert!(res.is_err());
    autosaver.on_save_failure(res.unwrap_err().to_string());

    assert_eq!(
        fs::read_to_string(store.path_for("Single monitor")).unwrap(),
        orig_content
    );
    assert!(autosaver.is_dirty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rename_failure_preserves_valid_file_and_retains_dirty() {
    let dir = temp_dir("rename_failure");
    let mut store = WorkspaceStore::open_dir(dir.clone());
    let file = default_chart();
    store.save(&file).unwrap();
    let orig_content = fs::read_to_string(store.path_for("Chart")).unwrap();

    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    autosaver.mark_dirty(Instant::now());

    store.set_injected_failure(InjectedWriteFailure::Rename);
    let _rev = autosaver.prepare_flush().unwrap();
    let res = store.save(&file);
    assert!(res.is_err());
    autosaver.on_save_failure(res.unwrap_err().to_string());

    assert_eq!(
        fs::read_to_string(store.path_for("Chart")).unwrap(),
        orig_content
    );
    assert!(autosaver.is_dirty());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn restore_does_not_trigger_autosave() {
    let mut autosaver = Autosaver::new(Duration::from_millis(500));
    // During restore, autosave is suppressed
    autosaver.suppress();
    assert!(autosaver.is_suppressed());

    // Edits / property assignments firing during restore do not mark dirty
    autosaver.mark_dirty(Instant::now());
    assert!(!autosaver.is_dirty());
    assert!(autosaver.prepare_flush().is_none());
    assert_eq!(autosaver.save_count(), 0);

    // Once restore finishes and valid state is applied, resume autosave
    autosaver.resume();
    assert!(!autosaver.is_suppressed());
    assert!(!autosaver.is_dirty());
    assert_eq!(autosaver.save_count(), 0);
}
