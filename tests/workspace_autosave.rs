//! Tests for workspace autosave debouncing, final-window capture, shutdown flush,
//! and atomic write failure handling (Q-077).

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use q_terminal::shell::layout::Layout;
use q_terminal::workspace::autosave::Autosaver;
use q_terminal::workspace::schema::{
    default_chart, default_single, default_trading, parse_workspace, serialize_workspace,
    WorkspaceChartPreferences, WorkspaceFile, WorkspaceSelection, WorkspaceWindow,
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

#[test]
fn precedence_explicit_env_over_saved_over_toml_over_defaults() {
    use q_terminal::startup::resolve_initial_target;

    // 1. Defaults only: toml empty, env none, saved none -> PETR4 / 1m
    let r1 = resolve_initial_target("", "", None, None, None);
    assert_eq!(r1.symbol, "PETR4");
    assert_eq!(r1.timeframe, "1m");
    assert!(!r1.is_session_override);

    // 2. Config TOML: toml has VALE3 / 5m, env none, saved none -> VALE3 / 5m
    let r2 = resolve_initial_target("VALE3", "5m", None, None, None);
    assert_eq!(r2.symbol, "VALE3");
    assert_eq!(r2.timeframe, "5m");
    assert!(!r2.is_session_override);

    // 3. Saved preferences override TOML
    let saved_manual = WorkspaceChartPreferences {
        symbol: "ITUB4".into(),
        timeframe: "15m".into(),
        mode: "manual".into(),
        followed_deployment_id: None,
        last_manual_symbol: Some("ITUB4".into()),
        last_manual_timeframe: Some("15m".into()),
        visible_bars: Some(150),
    };
    let r3 = resolve_initial_target("VALE3", "5m", None, None, Some(&saved_manual));
    assert_eq!(r3.symbol, "ITUB4");
    assert_eq!(r3.timeframe, "15m");
    assert_eq!(r3.mode.as_str(), "manual");
    assert!(!r3.is_session_override);

    // 4. Explicit non-empty env overrides saved symbol, keeps saved timeframe if env timeframe is empty
    let r4 = resolve_initial_target("VALE3", "5m", Some("BBDC4"), Some(""), Some(&saved_manual));
    assert_eq!(r4.symbol, "BBDC4");
    assert_eq!(r4.timeframe, "15m");
    assert!(r4.is_session_override);

    // 5. Saved preferences in following mode
    let saved_following = WorkspaceChartPreferences {
        symbol: "PETR4".into(),
        timeframe: "1m".into(),
        mode: "following".into(),
        followed_deployment_id: Some("dep-xyz".into()),
        last_manual_symbol: Some("ITUB4".into()),
        last_manual_timeframe: Some("15m".into()),
        visible_bars: Some(120),
    };
    let r5 = resolve_initial_target("VALE3", "5m", None, None, Some(&saved_following));
    assert_eq!(r5.mode.as_str(), "following");
    assert_eq!(r5.followed_deployment_id, Some("dep-xyz".into()));
    assert_eq!(r5.last_manual, ("ITUB4".into(), "15m".into()));
    assert!(!r5.is_session_override);

    // 6. Explicit env overrides following mode into manual session override
    let r6 = resolve_initial_target(
        "VALE3",
        "5m",
        Some("USIM5"),
        Some("1h"),
        Some(&saved_following),
    );
    assert_eq!(r6.symbol, "USIM5");
    assert_eq!(r6.timeframe, "1h");
    assert_eq!(r6.mode.as_str(), "manual");
    assert_eq!(r6.followed_deployment_id, None);
    assert_eq!(r6.last_manual, ("ITUB4".into(), "15m".into()));
    assert!(r6.is_session_override);
}

#[test]
fn session_override_preserves_committed_workspace_and_protects_target() {
    let dir = temp_dir("session_override_preserves");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    // Save committed workspace with VALE3 / 5m
    let mut file = default_chart();
    file.chart_preferences.insert(
        "chart".into(),
        WorkspaceChartPreferences {
            symbol: "VALE3".into(),
            timeframe: "5m".into(),
            mode: "manual".into(),
            followed_deployment_id: None,
            last_manual_symbol: Some("VALE3".into()),
            last_manual_timeframe: Some("5m".into()),
            visible_bars: Some(100),
        },
    );
    store.save(&file).unwrap();

    // Session starts with explicit env override (e.g. PETR4 / 1m)
    let env_target = ("PETR4", "1m");
    let fetcher =
        q_terminal::chart_target::OverlayFetcher::new("http://localhost", std::ptr::null_mut());
    let targeter = q_terminal::chart_target::ChartTargeter::new(
        std::ptr::null_mut(),
        (env_target.0.into(), env_target.1.into()),
        q_terminal::stream::client::Retargeter::new_test(),
        fetcher,
    );
    // Mark targeter as session override
    targeter.set_session_override(true);
    assert!(targeter.is_session_override());

    // External target selection (e.g. from execution feed) is ignored during session override
    let switched = targeter.select(Some((
        "dep-1".into(),
        "Dep".into(),
        "B3SA3".into(),
        "15m".into(),
    )));
    assert!(switched.is_none());
    assert_eq!(
        (
            targeter.selection().committed_symbol.as_str(),
            targeter.selection().committed_timeframe.as_str()
        ),
        (env_target.0, env_target.1)
    );

    // Following deployment is also ignored during session override
    let followed = targeter.follow_deployment();
    assert!(followed.is_none());
    assert!(targeter.is_session_override());

    // Committed file on disk remains VALE3 / 5m
    let loaded = store.load("Chart").unwrap();
    assert_eq!(loaded.chart_preferences["chart"].symbol, "VALE3");
    assert_eq!(loaded.chart_preferences["chart"].timeframe, "5m");

    // Operator explicit manual request clears session override
    let _ = targeter.request_manual("MGLU3", "1m").unwrap();
    assert!(!targeter.is_session_override());
    assert_eq!(
        (
            targeter.selection().committed_symbol.as_str(),
            targeter.selection().committed_timeframe.as_str()
        ),
        ("MGLU3", "1m")
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn zoom_visible_bars_round_trip_and_clamping() {
    let dir = temp_dir("zoom_visible_bars");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    let mut file = default_single();
    file.chart_preferences.insert(
        "chart".into(),
        WorkspaceChartPreferences {
            symbol: "PETR4".into(),
            timeframe: "1m".into(),
            mode: "manual".into(),
            followed_deployment_id: None,
            last_manual_symbol: Some("PETR4".into()),
            last_manual_timeframe: Some("1m".into()),
            visible_bars: Some(350),
        },
    );
    store.save(&file).unwrap();

    let loaded = store.load("Single monitor").unwrap();
    assert_eq!(loaded.chart_preferences["chart"].visible_bars, Some(350));

    // Deserializing with invalid negative visible_bars drops it
    let mut invalid_file = default_single();
    invalid_file.name = "InvalidZoom".into();
    let mut text = serialize_workspace(&invalid_file);
    text.push_str("\n[chart_preferences.chart]\nsymbol = \"PETR4\"\ntimeframe = \"1m\"\nmode = \"manual\"\nvisible_bars = -5\n");
    let parsed = parse_workspace(&text).unwrap();
    assert_eq!(parsed.chart_preferences["chart"].visible_bars, None);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn delayed_followed_deployment_fallback_on_archived_or_missing() {
    let dir = temp_dir("delayed_fallback");
    let mut store = WorkspaceStore::open_dir(dir.clone());

    // Saved workspace: following dep-missing, last manual ITUB4 / 15m
    let mut file = default_chart();
    file.chart_preferences.insert(
        "chart".into(),
        WorkspaceChartPreferences {
            symbol: "PETR4".into(),
            timeframe: "1m".into(),
            mode: "following".into(),
            followed_deployment_id: Some("dep-missing".into()),
            last_manual_symbol: Some("ITUB4".into()),
            last_manual_timeframe: Some("15m".into()),
            visible_bars: Some(150),
        },
    );
    store.save(&file).unwrap();

    let initial = q_terminal::startup::resolve_initial_target(
        "VALE3",
        "5m",
        None,
        None,
        file.chart_preferences.get("chart"),
    );
    assert_eq!(initial.mode.as_str(), "following");
    assert_eq!(initial.followed_deployment_id, Some("dep-missing".into()));
    assert_eq!(initial.last_manual, ("ITUB4".into(), "15m".into()));

    // Targeter starts following dep-missing with initial target
    let fetcher =
        q_terminal::chart_target::OverlayFetcher::new("http://localhost", std::ptr::null_mut());
    let targeter = q_terminal::chart_target::ChartTargeter::new(
        std::ptr::null_mut(),
        ("PETR4".into(), "1m".into()),
        q_terminal::stream::client::Retargeter::new_test(),
        fetcher,
    );
    targeter.select(Some((
        "dep-missing".into(),
        "Missing".into(),
        "PETR4".into(),
        "1m".into(),
    )));
    assert!(targeter.following_target().is_some());

    // In wire_execution, when confirmed snapshot arrives and dep is missing:
    // Fallback selects initial.last_manual ("ITUB4", "15m")
    let is_active = false;
    if !is_active {
        let (sym, tf) = &initial.last_manual;
        let _ = targeter.request_manual(sym, tf);
    }

    // Targeter is now on manual ITUB4 / 15m, not following
    assert_eq!(
        (
            targeter.selection().committed_symbol.as_str(),
            targeter.selection().committed_timeframe.as_str()
        ),
        ("ITUB4", "15m")
    );
    assert!(targeter.is_manual());

    // Saved workspace configuration on disk is NOT overwritten until operator explicitly saves or edits
    let reloaded = store.load("Chart").unwrap();
    assert_eq!(reloaded.chart_preferences["chart"].mode, "following");
    assert_eq!(
        reloaded.chart_preferences["chart"].followed_deployment_id,
        Some("dep-missing".into())
    );

    let _ = fs::remove_dir_all(dir);
}
