//! Q-053 workspace round-trip, corrupt files and switch cadence.

pub use q_terminal::{
    bridge, chart_bridge, config, execution, shell, shell_controller, startup, stream, workspace,
};

use std::time::Duration;

use config::Config;
use startup::SliceContext;
use stream::fake_server::FakeServer;
use workspace::schema::{default_trading, parse_workspace, serialize_workspace};
use workspace::store::WorkspaceStore;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "q_terminal_ws_test_{}_{}",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn workspace_round_trips_through_toml() {
    let file = default_trading();
    let text = serialize_workspace(&file);
    let loaded = parse_workspace(&text).unwrap();
    assert_eq!(loaded, file);
    assert_eq!(loaded.windows.len(), 2);
    assert_eq!(loaded.windows[0].composition, "market");
    assert_eq!(loaded.windows[1].composition, "operations");
}

#[test]
fn future_version_and_malformed_files_are_reported() {
    let err = parse_workspace("schema_version = 99\nname = \"x\"\nwindows = []\nselection = { global = \"\", detached = {} }\n")
        .unwrap_err();
    assert!(err.to_string().contains("newer"));
    let err = parse_workspace("[[[not toml").unwrap_err();
    assert!(err.to_string().contains("parse"));
}

#[test]
fn layout_restore_preserves_arrangement() {
    use shell::layout::{Layout, Node};

    let file = default_trading();
    let mut layout = Layout::default();
    let specs: Vec<(String, Node)> = file
        .windows
        .iter()
        .map(|w| (w.composition.clone(), w.root.clone()))
        .collect();
    layout.restore_workspace(&specs);
    assert_eq!(layout.windows().len(), 2);
    assert_eq!(layout.windows()[0].root, file.windows[0].root);
    assert_eq!(layout.windows()[1].root, file.windows[1].root);
}

fn ev(ctx: &mut SliceContext, js: &str) -> String {
    chart_bridge::shell_eval(ctx.engine.as_mut().unwrap(), js)
}

async fn pump(ms: u64) {
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    while std::time::Instant::now() < deadline {
        chart_bridge::process_events();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    chart_bridge::process_events();
}

async fn start() -> (FakeServer, SliceContext) {
    let server = FakeServer::start().await;
    let config = Config {
        api_base: server.api_base(),
        symbol: "PETR4".to_string(),
        timeframe: "1m".to_string(),
        operator: "operator".to_string(),
    };
    let ctx = startup::setup_slice(&Ok(config));
    for _ in 0..300 {
        chart_bridge::process_events();
        if server.ws_subscribes() >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    pump(400).await;
    (server, ctx)
}

#[tokio::test(flavor = "current_thread")]
async fn switching_workspaces_does_not_resubscribe() {
    let (server, mut ctx) = start().await;
    let base_ws = server.ws_subscribes();
    let base_snap = server.exec_snapshot_calls();

    for _ in 0..10 {
        ev(&mut ctx, "switchWorkspace('Single monitor')");
        pump(150).await;
        ev(&mut ctx, "switchWorkspace('Trading')");
        pump(150).await;
    }

    assert_eq!(server.ws_subscribes(), base_ws);
    assert_eq!(server.exec_snapshot_calls(), base_snap);
    assert_eq!(server.ws_connections(), 1);
}

#[test]
fn defaults_are_created_once_and_not_overwritten() {
    let dir = temp_dir("defaults_once");
    let store = WorkspaceStore::open_dir(dir.clone());
    let trading_path = store.path_for("Trading");
    let first = std::fs::read_to_string(&trading_path).unwrap();
    let mut inner = parse_workspace(&first).unwrap();
    inner.name = "Trading edited".into();
    std::fs::write(&trading_path, serialize_workspace(&inner)).unwrap();

    let mut store2 = WorkspaceStore::open_dir(dir.clone());
    let second = std::fs::read_to_string(&trading_path).unwrap();
    assert!(second.contains("Trading edited"));
    let single = store2.load("Single monitor").unwrap();
    assert_eq!(single.windows[0].composition, "merged");
    let _ = std::fs::remove_dir_all(dir);
}
