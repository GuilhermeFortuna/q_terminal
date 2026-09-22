//! Q-052 criteria 1–7: several top-level windows over one set of stores, headlessly.
//!
//! The QML shell root is driven through `shell_eval`, which evaluates an expression with the
//! root as scope, so `controller`, `windowObjects()` and `panelItem(id)` are the same names
//! the shell's own code uses. Every test starts a full slice against the fake server.
#![allow(clippy::await_holding_lock)]

pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, shell, shell_controller, startup, stream,
};

use std::time::Duration;

use config::Config;
use startup::SliceContext;
use stream::fake_exec as fx;
use stream::fake_server::FakeServer;

const DEP: &str = "11111111-1111-1111-1111-111111111111";
const DEP2: &str = "33333333-3333-3333-3333-333333333333";

fn ev(ctx: &mut SliceContext, js: &str) -> String {
    chart_bridge::shell_eval(ctx.engine.as_mut().unwrap(), js)
}

fn ev_num(ctx: &mut SliceContext, js: &str) -> i64 {
    let out = ev(ctx, js);
    out.parse()
        .unwrap_or_else(|_| panic!("`{js}` gave `{out}`, not a number"))
}

async fn pump(ms: u64) {
    let deadline = std::time::Instant::now() + Duration::from_millis(ms);
    while std::time::Instant::now() < deadline {
        chart_bridge::process_events();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    chart_bridge::process_events();
}

async fn until(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..300 {
        chart_bridge::process_events();
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
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
    until("stream subscription", || {
        server.ws_subscribes() >= 1 && server.exec_snapshot_calls() >= 1
    })
    .await;
    pump(300).await;
    (server, ctx)
}

fn windows(ctx: &mut SliceContext) -> i64 {
    let shell = ev_num(ctx, "controller.window_count");
    let objects = ev_num(ctx, "windowObjects().length");
    let visible = chart_bridge::shell_visible_window_count() as i64;
    assert_eq!(shell, objects, "controller and window objects agree");
    assert_eq!(shell, visible, "every window is visible");
    shell
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Counts {
    ws: usize,
    subscribes: usize,
    snapshots: usize,
}

fn counts(server: &FakeServer) -> Counts {
    Counts {
        ws: server.ws_connections(),
        subscribes: server.ws_subscribes(),
        snapshots: server.exec_snapshot_calls(),
    }
}

async fn health_polls_over(server: &FakeServer, ms: u64) -> usize {
    let before = server.health_calls();
    pump(ms).await;
    server.health_calls() - before
}

#[tokio::test(flavor = "current_thread")]
async fn one_two_and_four_windows_share_one_subscription() {
    let (server, mut ctx) = start().await;

    assert_eq!(windows(&mut ctx), 1);
    let base = counts(&server);
    assert_eq!(
        base,
        Counts {
            ws: 1,
            subscribes: 1,
            snapshots: 1
        }
    );
    let one_window_polls = health_polls_over(&server, 4600).await;
    assert!(one_window_polls >= 1, "the poller runs");

    ev(&mut ctx, "openWindow('market')");
    ev(&mut ctx, "openWindow('operations')");
    pump(200).await;
    assert_eq!(windows(&mut ctx), 2);
    assert_eq!(counts(&server), base);

    ev(&mut ctx, "controller.float_panel('tape')");
    ev(&mut ctx, "controller.float_panel('dom')");
    pump(200).await;
    assert_eq!(windows(&mut ctx), 4);
    assert_eq!(counts(&server), base);
    let four_window_polls = health_polls_over(&server, 4600).await;
    assert!(
        four_window_polls.abs_diff(one_window_polls) <= 1,
        "one poll cadence at any window count: {one_window_polls} vs {four_window_polls}"
    );

    // Closing a window and reopening it changes nothing on the wire.
    let floating = ev(&mut ctx, "controller.panel_window('dom')");
    ev(&mut ctx, &format!("windowObject('{floating}').close()"));
    pump(200).await;
    assert_eq!(windows(&mut ctx), 3);
    ev(&mut ctx, "openWindow('market')");
    pump(200).await;
    assert_eq!(counts(&server), base);
    assert_eq!(server.ws_connections(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn an_event_is_visible_in_every_window_at_the_same_revision() {
    let (server, mut ctx) = start().await;
    ev(&mut ctx, "openWindow('market')");
    ev(&mut ctx, "openWindow('operations')");
    pump(200).await;
    assert_eq!(windows(&mut ctx), 2);

    let before = ev_num(&mut ctx, "activeExecutionModels.revision");
    server
        .exec_publish("deployments", fx::deployment(DEP, "running"))
        .await;
    until("the store applied the event", || {
        ev_num(&mut ctx, "activeExecutionModels.revision") > before
    })
    .await;
    ev(
        &mut ctx,
        &format!("selectDeployment(windowObjects()[0].windowId, '{DEP}')"),
    );

    let same_store = ev(
        &mut ctx,
        "windowObjects().every(function (w) { return w.shell.activeExecutionModels === activeExecutionModels; })",
    );
    assert_eq!(same_store, "true", "no window holds a store of its own");

    let per_window = |ctx: &mut SliceContext| {
        ev(
            ctx,
            "windowObjects().map(function (w) { return w.shell.activeExecutionModels.revision + ':' + w.shell.activeExecutionModels.field_for_selected_deployment('lifecycle'); }).join(',')",
        )
    };
    let seen = per_window(&mut ctx);
    let views: Vec<&str> = seen.split(',').collect();
    assert_eq!(views.len(), 2);
    assert_eq!(views[0], views[1], "both windows read one revision: {seen}");
    assert!(views[0].ends_with(":running"), "{seen}");

    // A lifecycle change reaches both windows in the same turn.
    server
        .exec_publish("deployments", fx::deployment(DEP, "paused"))
        .await;
    until("paused reaches the store", || {
        ev(
            &mut ctx,
            "activeExecutionModels.field_for_selected_deployment('lifecycle')",
        ) == "paused"
    })
    .await;
    let seen = per_window(&mut ctx);
    let views: Vec<&str> = seen.split(',').collect();
    assert_eq!(views[0], views[1], "{seen}");
    assert!(views[0].ends_with(":paused"), "{seen}");
}

#[tokio::test(flavor = "current_thread")]
async fn closing_a_window_keeps_the_rest_working_and_the_last_one_exits() {
    let (server, mut ctx) = start().await;
    ev(&mut ctx, "openWindow('market')");
    let ops = ev(&mut ctx, "openWindow('operations')");
    pump(200).await;
    assert_eq!(windows(&mut ctx), 2);
    let panels = ev_num(&mut ctx, "JSON.parse(controller.panel_ids()).length");
    assert!(chart_bridge::shell_quit_on_last_window_closed());

    // Close the market window: it is not the last, so the application stays.
    let market = ev(&mut ctx, "controller.panel_window('chart')");
    assert_ne!(market, ops);
    ev(&mut ctx, &format!("windowObject('{market}').close()"));
    pump(300).await;
    assert_eq!(windows(&mut ctx), 1);
    assert_eq!(
        ev_num(&mut ctx, "JSON.parse(controller.panel_ids()).length"),
        panels,
        "no panel was lost with the window"
    );
    let stranded = ev(
        &mut ctx,
        &format!(
            "JSON.parse(controller.panel_ids()).filter(function (p) {{ return panelItem(p).windowId !== '{ops}'; }}).join(',')"
        ),
    );
    assert_eq!(
        stranded, "",
        "every panel is reachable in the remaining window"
    );

    // Its stores are intact and still live.
    let before = ev_num(&mut ctx, "activeExecutionModels.revision");
    server
        .exec_publish("deployments", fx::deployment(DEP, "running"))
        .await;
    until("the remaining window still gets events", || {
        ev_num(&mut ctx, "activeExecutionModels.revision") > before
    })
    .await;
    assert_eq!(server.ws_connections(), 1);

    // Closing the last window ends the application.
    ev(&mut ctx, &format!("windowObject('{ops}').close()"));
    pump(300).await;
    assert_eq!(chart_bridge::shell_visible_window_count(), 0);
    assert_eq!(ev_num(&mut ctx, "controller.window_count"), 0);
}

/// The whole application, not a harness: with every window closed the event loop returns.
#[test]
fn closing_every_window_ends_the_process() {
    let home = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_q_terminal"))
        .args(["--auto-close-ms", "1500"])
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("QT_QPA_PLATFORM", "offscreen")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY")
        .env_remove("Q_TERMINAL_API_BASE")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn q_terminal");
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "exited with {status}");
            return;
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("the process kept running after its last window closed");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_moved_panel_keeps_its_identity_and_state() {
    let (server, mut ctx) = start().await;
    for i in 0..14 {
        server
            .exec_publish(
                "deployments",
                fx::deployment(&format!("aaaaaaaa-0000-0000-0000-{i:012}"), "running"),
            )
            .await;
    }
    ev(&mut ctx, "openWindow('market')");
    let ops = ev(&mut ctx, "openWindow('operations')");
    pump(300).await;
    let market = ev(&mut ctx, "controller.panel_window('chart')");
    until("deployments arrived", || {
        ev(&mut ctx, "activeExecutionModels.deployments.count") == "14"
            || ev_num(&mut ctx, "activeExecutionModels.revision") > 0
    })
    .await;
    pump(300).await;

    // State that lives in the panels, not in the shared stores.
    ev(&mut ctx, "panelItem('detail').detail.currentTabIndex = 3");
    ev(
        &mut ctx,
        "panelItem('deployments').commandTarget.scrollY = 200",
    );
    let identity = |ctx: &mut SliceContext| {
        ev(
            ctx,
            "[panelItem('detail'), panelItem('deployments')].map(String).join('|')",
        )
    };
    let before = identity(&mut ctx);
    assert_eq!(
        ev(&mut ctx, "panelItem('detail').windowId"),
        ops,
        "starts in the operations window"
    );

    ev(
        &mut ctx,
        &format!("controller.move_panel('detail', '{market}')"),
    );
    ev(
        &mut ctx,
        &format!("controller.move_panel('deployments', '{market}')"),
    );
    pump(300).await;

    assert_eq!(identity(&mut ctx), before, "the same objects, not copies");
    assert_eq!(ev(&mut ctx, "panelItem('detail').panelId"), "detail");
    assert_eq!(ev(&mut ctx, "panelItem('detail').windowId"), market);
    assert_eq!(ev(&mut ctx, "panelItem('deployments').windowId"), market);
    assert_eq!(
        ev(&mut ctx, "panelItem('detail').detail.currentTabIndex"),
        "3",
        "active tab kept"
    );
    assert_eq!(
        ev(
            &mut ctx,
            "Math.round(panelItem('deployments').commandTarget.scrollY)"
        ),
        "200",
        "scroll position kept"
    );
    assert_eq!(
        ev(&mut ctx, "controller.panel_window('detail')"),
        market,
        "the layout agrees"
    );
    assert_eq!(
        server.ws_connections(),
        1,
        "the subscription did not change"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_detached_window_holds_its_own_selection() {
    let (server, mut ctx) = start().await;
    server
        .exec_publish("deployments", fx::deployment(DEP, "running"))
        .await;
    server
        .exec_publish("deployments", fx::deployment(DEP2, "paused"))
        .await;
    let w1 = ev(&mut ctx, "windowObjects()[0].windowId");
    let w2 = ev(&mut ctx, "openWindow('market')");
    pump(300).await;

    ev(&mut ctx, &format!("selectDeployment('{w1}', '{DEP}')"));
    assert_eq!(
        ev(&mut ctx, &format!("controller.effective_selection('{w2}')")),
        DEP
    );

    ev(&mut ctx, &format!("controller.detach('{w2}')"));
    ev(&mut ctx, &format!("selectDeployment('{w2}', '{DEP2}')"));
    assert_eq!(
        ev(&mut ctx, &format!("controller.effective_selection('{w2}')")),
        DEP2
    );
    assert_eq!(
        ev(&mut ctx, "controller.global_selection()"),
        DEP,
        "global untouched"
    );
    assert_eq!(
        ev(&mut ctx, "activeExecutionModels.selected_deployment_id"),
        DEP,
        "the shared models were not retargeted by the detached window"
    );
    assert_eq!(
        ev(&mut ctx, &format!("controller.effective_selection('{w1}')")),
        DEP
    );
    assert_eq!(
        ev(&mut ctx, &format!("windowObject('{w2}').detached")),
        "true",
        "the window says it is detached"
    );
    assert_eq!(
        ev(&mut ctx, &format!("windowObject('{w1}').detached")),
        "false"
    );

    ev(&mut ctx, &format!("controller.attach('{w2}')"));
    assert_eq!(
        ev(&mut ctx, &format!("controller.effective_selection('{w2}')")),
        DEP
    );
    assert_eq!(
        ev(&mut ctx, &format!("windowObject('{w2}').detached")),
        "false"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn commands_are_one_registry_and_enablement_is_global() {
    let (server, mut ctx) = start().await;
    ev(&mut ctx, "openWindow('operations')");
    pump(200).await;

    // Enumerable, labelled, with unique shortcuts.
    let rows = ev(&mut ctx, "controller.commands_json()");
    let rows: serde_json::Value = serde_json::from_str(&rows).unwrap();
    let rows = rows.as_array().unwrap();
    assert!(rows.len() >= 12);
    let mut shortcuts: Vec<&str> = rows
        .iter()
        .map(|r| r["shortcut"].as_str().unwrap())
        .collect();
    assert!(rows
        .iter()
        .all(|r| !r["label"].as_str().unwrap().is_empty()));
    let n = shortcuts.len();
    shortcuts.sort_by_key(|s| s.to_lowercase());
    shortcuts.dedup_by_key(|s| s.to_lowercase());
    assert_eq!(shortcuts.len(), n, "no two commands share a shortcut");

    // Healthy backend: the command is enabled, from every window.
    until("health applied", || {
        ev(
            &mut ctx,
            "activeExecutionControls.shell_command_enabled('deployment.create', false)",
        ) == "true"
    })
    .await;
    let every_window = |ctx: &mut SliceContext, id: &str| {
        ev(
            ctx,
            &format!("windowObjects().map(function (w) {{ return w.shell.commandEnabled('{id}'); }}).join(',')"),
        )
    };
    assert_eq!(every_window(&mut ctx, "deployment.create"), "true,true");

    // Engaging the kill switch disables it in every window at once.
    let mut health = serde_json::json!({
        "api_status": "ok", "checked_at": "2026-09-19T00:00:00Z", "deployments": [],
        "edge": {"checked_at": "2026-09-19T00:00:00Z", "mt5_connected": true,
                 "reachable": true, "terminal_build": 4150},
        "kill_switch_enabled": true, "live_capability_locked": false,
        "market_data_status": "streaming", "unknown_order_count": 0,
        "worker_heartbeat_age_s": 0.5, "worker_started_at": "2026-09-19T00:00:00Z",
        "worker_status": "active"
    });
    server.set_health_json(&health.to_string()).await;
    until("kill switch reaches the shell", || {
        ev(&mut ctx, "activeOpsStatus.kill_switch_enabled") == "true"
    })
    .await;
    assert_eq!(every_window(&mut ctx, "deployment.create"), "false,false");
    assert_eq!(every_window(&mut ctx, "kill-switch.set"), "false,false");
    assert_eq!(every_window(&mut ctx, "kill-switch.clear"), "true,true");

    health["kill_switch_enabled"] = false.into();
    server.set_health_json(&health.to_string()).await;
    until("kill switch clears", || {
        ev(&mut ctx, "activeOpsStatus.kill_switch_enabled") == "false"
    })
    .await;
    assert_eq!(every_window(&mut ctx, "deployment.create"), "true,true");
}

#[tokio::test(flavor = "current_thread")]
async fn merging_keeps_every_panel_and_splitting_restores_the_arrangement() {
    let (_server, mut ctx) = start().await;
    ev(&mut ctx, "openWindow('market')");
    let ops = ev(&mut ctx, "openWindow('operations')");
    pump(300).await;
    assert_eq!(windows(&mut ctx), 2);

    ev(&mut ctx, "panelItem('detail').detail.currentTabIndex = 2");
    let layout_before = ev(&mut ctx, "controller.layout_json()");
    let identities = |ctx: &mut SliceContext| {
        ev(
            ctx,
            "JSON.parse(controller.panel_ids()).sort().map(function (p) { return String(panelItem(p)); }).join('|')",
        )
    };
    let panels_before = identities(&mut ctx);

    ev(&mut ctx, &format!("controller.merge_into('{ops}')"));
    pump(300).await;
    assert_eq!(windows(&mut ctx), 1, "one window holds everything");
    assert_eq!(
        identities(&mut ctx),
        panels_before,
        "same panels, same objects"
    );
    assert_eq!(
        ev(&mut ctx, "panelItem('detail').detail.currentTabIndex"),
        "2",
        "state survives the merge"
    );
    let reachable = ev(
        &mut ctx,
        &format!(
            "JSON.parse(controller.panel_ids()).every(function (p) {{ return panelItem(p).windowId === '{ops}'; }})"
        ),
    );
    assert_eq!(
        reachable, "true",
        "every panel is reachable in the one window"
    );

    ev(&mut ctx, &format!("controller.split_all('{ops}')"));
    pump(300).await;
    assert_eq!(windows(&mut ctx), 2);
    assert_eq!(ev(&mut ctx, "controller.layout_json()"), layout_before);
    assert_eq!(identities(&mut ctx), panels_before);
    assert_eq!(
        ev(&mut ctx, "panelItem('detail').detail.currentTabIndex"),
        "2",
        "state survives the split"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn merged_workstation_prioritizes_chart_and_restores_legacy_window_tree() {
    let (_server, mut ctx) = start().await;

    let layout: serde_json::Value = serde_json::from_str(&ev(
        &mut ctx,
        "controller.window_layout(windowObjects()[0].windowId)",
    ))
    .unwrap();
    let root = &layout["root"];
    assert_eq!(root["type"], "split");
    assert_eq!(root["weights"], serde_json::json!([0.12, 0.84, 0.04]));

    let workbench = &root["children"][1];
    assert_eq!(workbench["weights"], serde_json::json!([0.22, 0.78]));
    let right_pane = &workbench["children"][1];
    assert_eq!(right_pane["weights"], serde_json::json!([0.68, 0.32]));
    assert_eq!(
        right_pane["children"][0]["panels"],
        serde_json::json!(["chart"]),
        "the chart owns the majority of the right pane"
    );
    assert_eq!(
        right_pane["children"][1]["panels"],
        serde_json::json!(["detail"]),
        "execution detail remains a separately resizable pane"
    );

    let legacy = serde_json::json!([{
        "id": "legacy",
        "composition": "merged",
        "root": {
            "type": "split",
            "orientation": "vertical",
            "weights": [0.14, 0.82, 0.04],
            "children": [
                { "type": "tabs", "panels": ["status"], "active": 0 },
                { "type": "split", "orientation": "horizontal", "weights": [0.26, 0.74], "children": [
                    { "type": "tabs", "panels": ["deployments"], "active": 0 },
                    { "type": "split", "orientation": "vertical", "weights": [0.42, 0.58], "children": [
                        { "type": "tabs", "panels": ["chart"], "active": 0 },
                        { "type": "tabs", "panels": ["detail"], "active": 0 }
                    ]}
                ]},
                { "type": "tabs", "panels": ["instrument"], "active": 0 }
            ]
        }
    }]);
    ev(
        &mut ctx,
        &format!(
            "controller.restore_workspace('{}')",
            legacy.to_string().replace('\'', "\\\\'")
        ),
    );
    pump(200).await;
    assert_eq!(windows(&mut ctx), 1);
    assert_eq!(
        ev(
            &mut ctx,
            "JSON.parse(controller.panel_ids()).sort().join(',')"
        ),
        "chart,deployments,detail,instrument,status",
        "a saved pre-Q-058 tree restores every panel"
    );
}

/// Looks at the shell: writes every window to `target/shell-shots/` for a human (or an
/// agent with an image tool) to read. It asserts only that the images exist.
#[tokio::test(flavor = "current_thread")]
async fn windows_render_to_images() {
    let (server, mut ctx) = start().await;
    for (i, name) in ["momentum", "reversion", "breakout"].iter().enumerate() {
        let id = format!("bbbbbbbb-0000-0000-0000-{i:012}");
        let mut dep = fx::deployment(&id, if i == 1 { "paused" } else { "running" });
        dep["name"] = (*name).into();
        server.exec_publish("deployments", dep).await;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/shell-shots");
    std::fs::create_dir_all(&dir).unwrap();
    for f in std::fs::read_dir(&dir).unwrap().flatten() {
        let _ = std::fs::remove_file(f.path());
    }
    let dir_str = dir.to_str().unwrap().to_string();

    pump(600).await;
    assert_eq!(chart_bridge::shell_grab_windows(&dir_str), 1, "merged");
    ev(&mut ctx, "openWindow('market')");
    ev(&mut ctx, "openWindow('operations')");
    pump(600).await;
    assert_eq!(chart_bridge::shell_grab_windows(&dir_str), 2, "two windows");
    let w = ev(&mut ctx, "windowObjects()[1].windowId");
    ev(&mut ctx, &format!("controller.detach('{w}')"));
    pump(300).await;
    // The detached window is drawn again over its own image, banner included.
    assert_eq!(chart_bridge::shell_grab_windows(&dir_str), 2);
    assert!(dir.read_dir().unwrap().count() >= 2);
}
