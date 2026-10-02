pragma ComponentBehavior: Bound
import QtQuick
import QtQml.Models
import qml

// The shell root (Q-052). It is not a window and draws nothing: it declares the stores once, owns the
// persistent panel items, and opens as many top-level windows as the arrangement asks for.
// Every window reads these same stores, so there is one stream subscription, one execution
// store and one command path however many windows exist.
Item {
    id: shell

    // Test seams: a harness may hand in fakes instead of the default stores.
    property var feed: null
    property var chartContext: null
    property var executionModels: null
    property var opsStatus: null
    property var executionControls: null

    BarFeed {
        id: defaultFeed
        objectName: "barFeed"
    }

    ChartContext {
        id: defaultChartContext
        objectName: "chartContext"
    }

    ExecutionModels {
        id: defaultExecutionModels
        objectName: "executionModels"
    }

    OpsStatus {
        id: defaultOpsStatus
        objectName: "opsStatus"
    }

    ExecutionControls {
        id: defaultExecutionControls
        objectName: "executionControls"
    }

    AppInfo {
        id: appInfo
        objectName: "appInfo"
    }

    ShellController {
        id: shellController
        objectName: "shellController"
    }

    WorkspaceController {
        id: workspaceController
        objectName: "workspaceController"
    }

    readonly property var activeFeed: shell.feed ? shell.feed : defaultFeed
    readonly property var activeChartContext: shell.chartContext ? shell.chartContext : defaultChartContext
    readonly property var activeExecutionModels: shell.executionModels ? shell.executionModels : defaultExecutionModels
    readonly property var activeOpsStatus: shell.opsStatus ? shell.opsStatus : defaultOpsStatus
    readonly property var activeExecutionControls: shell.executionControls ? shell.executionControls : defaultExecutionControls
    readonly property var controller: shellController
    readonly property var workspace: workspaceController

    readonly property var commands: JSON.parse(shellController.commands_json())
    readonly property var registry: JSON.parse(shellController.panel_registry())

    // Persistent panel items by id, created on first use and never destroyed while the
    // shell lives. Windows borrow them; see PanelHost.
    property var panelItems: ({})

    function panelSpec(id) {
        return shell.registry[id.split("#")[0]];
    }

    function iconFor(name) {
        return Icons[name] !== undefined ? Icons[name] : Icons.info;
    }

    function commandsForPanel(kind) {
        return shell.commands.filter(function (c) {
            return c.scope === "panel" && c.panel === kind;
        });
    }

    function panelItem(id) {
        var existing = shell.panelItems[id];
        if (existing) {
            return existing;
        }
        var kind = id.split("#")[0];
        var component = shell.panelComponents[kind];
        var item = component.createObject(shell, {
            shell: shell,
            panelId: id
        });
        shell.panelItems[id] = item;
        return item;
    }

    function windowObjects() {
        var out = [];
        for (var i = 0; i < windows.count; ++i) {
            out.push(windows.objectAt(i));
        }
        return out;
    }

    function windowObject(id) {
        return shell.windowObjects().find(function (w) {
            return w && w.windowId === id;
        });
    }

    function openWindow(composition) {
        var id = shellController.open_window(composition);
        shell.syncWindows();
        return id;
    }

    Timer {
        id: autosaveTimer
        interval: 500
        repeat: false
        onTriggered: shell.flushPending()
    }

    function scheduleAutosave() {
        if (workspaceController.is_restoring) {
            return;
        }
        workspaceController.mark_dirty();
        autosaveTimer.restart();
    }

    function captureChartPreferences() {
        var chartPrefs = {};
        if (activeChartContext) {
            var visibleBars = 120;
            var chart = shell.panelItem("chart");
            if (chart && chart.chartPane && chart.chartPane.viewport) {
                var vc = chart.chartPane.viewport.visibleCount();
                if (vc > 0) {
                    visibleBars = vc;
                } else if (chart.chartPane.viewport.visibleBars > 0) {
                    visibleBars = chart.chartPane.viewport.visibleBars;
                }
            }
            var sym = activeChartContext.active_symbol || "PETR4";
            var tf = activeChartContext.active_timeframe || "1m";
            if (activeChartContext.is_session_override && activeChartContext.last_manual_symbol) {
                sym = activeChartContext.last_manual_symbol;
                if (activeChartContext.last_manual_timeframe) {
                    tf = activeChartContext.last_manual_timeframe;
                }
            }
            chartPrefs.chart = {
                symbol: sym,
                timeframe: tf,
                mode: activeChartContext.chart_mode || "following",
                followed_deployment_id: (activeChartContext.chart_mode === "following" ? (activeChartContext.followed_deployment_id || "") : ""),
                last_manual_symbol: activeChartContext.last_manual_symbol || "",
                last_manual_timeframe: activeChartContext.last_manual_timeframe || "",
                visible_bars: visibleBars
            };
        }
        return chartPrefs;
    }

    function flushPending() {
        autosaveTimer.stop();
        var studySets = {};
        if (activeFeed) {
            try {
                studySets.chart = JSON.parse(activeFeed.study_list_json()).studies || [];
            } catch (e) {
                studySets.chart = [];
            }
        }
        var chartPrefs = captureChartPreferences();
        workspaceController.flush_pending(
            shellController.capture_windows(),
            shellController.capture_selection(),
            JSON.stringify(studySets),
            JSON.stringify(chartPrefs)
        );
    }

    function saveWorkspace(name) {
        autosaveTimer.stop();
        var studySets = {};
        if (activeFeed) {
            try {
                studySets.chart = JSON.parse(activeFeed.study_list_json()).studies || [];
            } catch (e) {
                studySets.chart = [];
            }
        }
        var chartPrefs = captureChartPreferences();
        workspaceController.save(
            shellController.capture_windows(),
            shellController.capture_selection(),
            JSON.stringify(studySets),
            JSON.stringify(chartPrefs),
            name
        );
    }

    function applyWorkspacePlacement() {
        workspaceController.apply_placement(
            workspaceController.active_workspace,
            workspaceController.pending_placement_json
        );
    }

    function restoreWorkspaceLayout() {
        shellController.restore_workspace(workspaceController.pending_restore_json);
        shellController.restore_selection(workspaceController.pending_selection_json);
        try {
            var studies = JSON.parse(workspaceController.pending_study_sets_json || "{}");
            if (activeFeed) activeFeed.restore_studies_json(JSON.stringify(studies.chart || []));
        } catch (e) {
            console.warn("Workspace studies could not be restored:", e);
        }
        try {
            var prefs = JSON.parse(workspaceController.pending_chart_preferences_json || "{}");
            if (prefs.chart) {
                var cp = prefs.chart;
                if (activeChartContext) {
                    if (cp.last_manual_symbol && cp.last_manual_timeframe) {
                        activeChartContext.set_last_manual(cp.last_manual_symbol, cp.last_manual_timeframe);
                    }
                    if (cp.followed_deployment_id) {
                        activeChartContext.set_followed_deployment(cp.followed_deployment_id);
                    }
                    if (cp.mode === "manual") {
                        activeChartContext.request_target(cp.symbol || "PETR4", cp.timeframe || "1m");
                    } else {
                        activeChartContext.follow_deployment();
                    }
                }
                var chart = shell.panelItem("chart");
                if (chart && chart.chartPane && chart.chartPane.viewport && cp.visible_bars > 0) {
                    chart.chartPane.viewport.visibleBars = cp.visible_bars;
                    chart.chartPane.viewport.updateViewport();
                }
            }
        } catch (e) {
            console.warn("Workspace chart preferences could not be restored:", e);
        }
        shell.syncWindows();
        Qt.callLater(shell.applyWorkspacePlacement);
        Qt.callLater(function() {
            workspaceController.resume_autosave();
        });
    }

    function switchWorkspace(name) {
        shell.flushPending();
        workspaceController.switch_to(name);
        shell.restoreWorkspaceLayout();
    }

    // Called from a window's closing signal. The window object itself is destroyed a turn
    // later, never from inside its own signal.
    function windowClosing(id) {
        var winObjs = shell.windowObjects();
        if (winObjs.length <= 1) {
            shell.flushPending();
        }
        shellController.close_window(id);
        if (winObjs.length > 1) {
            shell.scheduleAutosave();
        }
    }

    // A selection made in a window. Attached windows share the global selection, so the one
    // shared execution model retargets; a detached window keeps its selection to itself.
    function selectDeployment(windowId, deploymentId) {
        if (shellController.select_deployment(windowId, deploymentId)) {
            shell.activeExecutionModels.select_deployment(deploymentId);
            shell.scheduleAutosave();
        }
    }

    // Enablement comes from global state alone, so every window gets the same answer.
    function commandEnabled(id) {
        return shell.activeExecutionControls.shell_command_enabled(id, shell.activeOpsStatus.kill_switch_enabled);
    }

    function commandReason(id) {
        return shell.activeExecutionControls.shell_command_reason(id, shell.activeOpsStatus.kill_switch_enabled);
    }

    function runCommand(id, fromWindow) {
        switch (id) {
        case "palette.open":
            fromWindow.openPalette();
            break;
        case "window.open-market":
            shell.openWindow("market");
            break;
        case "window.open-operations":
            shell.openWindow("operations");
            break;
        case "window.merge-all":
            shellController.merge_into(fromWindow.windowId);
            break;
        case "window.split-all":
            shellController.split_all(fromWindow.windowId);
            break;
        case "window.close":
            fromWindow.close();
            break;
        case "selection.toggle-detach":
            fromWindow.toggleDetach();
            break;
        case "kill-switch.set":
        case "kill-switch.clear":
            shell.forward("status", id);
            break;
        case "account.create":
        case "deployment.create":
            shell.forward("deployments", id);
            break;
        case "deployment.flatten":
            shell.forward("detail", id);
            break;
        case "chart.focus-symbol":
            shell.forward("chart", id);
            break;
        case "chart.follow-deployment":
            shell.activeChartContext.follow_deployment();
            break;
        case "operations.toggle":
            var wasOps = shellController.is_operations_visible();
            shellController.toggle_operations(fromWindow ? fromWindow.windowId : "");
            shell.syncWindows();
            shell.scheduleAutosave();
            if (wasOps && !shellController.is_operations_visible()) {
                shell.focusChart();
            }
            break;
        case "tape.toggle":
            var wasTape = shellController.is_tape_visible();
            shellController.toggle_tape(fromWindow ? fromWindow.windowId : "");
            shell.syncWindows();
            shell.scheduleAutosave();
            if (wasTape && !shellController.is_tape_visible()) {
                shell.focusChart();
            }
            break;
        }
    }

    function focusChart() {
        var chart = shell.panelItem("chart");
        if (chart && typeof chart.focusCanvas === "function") {
            chart.focusCanvas();
        }
    }

    // Runs a command on the panel that owns its dialog, and brings that window forward.
    function forward(panelId, command) {
        if (panelId === "status" || panelId === "deployments" || panelId === "detail") {
            if (!shellController.is_operations_visible()) {
                var wins = shell.windowObjects();
                var targetWin = wins.length > 0 ? wins[0].windowId : "";
                shellController.ensure_operations_visible(targetWin);
                shell.syncWindows();
            }
        }
        var panel = shell.panelItem(panelId);
        var w = shell.windowObject(shellController.panel_window(panelId));
        if (w) {
            w.raise();
            w.requestActivate();
        }
        panel.trigger(command);
    }

    readonly property var panelComponents: ({
            "status": statusPanel,
            "deployments": deploymentsPanel,
            "detail": detailPanel,
            "chart": chartPanel,
            "instrument": instrumentPanel,
            "tape": tapePanel,
            "dom": placeholderPanel,
            "footprint": placeholderPanel
        })

    Component {
        id: statusPanel
        StatusPanel {}
    }
    Component {
        id: deploymentsPanel
        DeploymentsPanel {}
    }
    Component {
        id: detailPanel
        DetailPanel {}
    }
    Component {
        id: chartPanel
        ChartPanel {}
    }
    Component {
        id: instrumentPanel
        InstrumentPanel {}
    }
    Component {
        id: tapePanel
        TapePanel {}
    }
    Component {
        id: placeholderPanel
        PlaceholderPanel {}
    }

    // Window objects follow the controller's window list one to one. A model diff rather
    // than an array model, so opening one window never rebuilds the others.
    property ListModel windowModel: ListModel {}

    function syncWindows() {
        var ids = JSON.parse(shellController.window_ids);
        for (var i = windowModel.count - 1; i >= 0; --i) {
            if (ids.indexOf(windowModel.get(i).windowId) < 0) {
                // A window the arrangement dropped disappears at once, not when the
                // engine gets round to deleting it.
                var gone = windows.objectAt(i);
                if (gone) {
                    gone.visible = false;
                }
                windowModel.remove(i);
            }
        }
        for (var id of ids) {
            var known = false;
            for (var j = 0; j < windowModel.count; ++j) {
                known = known || windowModel.get(j).windowId === id;
            }
            if (!known) {
                windowModel.append({
                    windowId: id
                });
            }
        }
    }

    // Any structural change (a window opened, closed, floated, merged) resyncs the window
    // objects. One turn later, so a window is never destroyed from inside its own signal.
    Connections {
        target: shellController
        function onWindow_idsChanged() {
            Qt.callLater(shell.syncWindows);
            shell.scheduleAutosave();
        }
    }

    Connections {
        target: shell.activeChartContext
        function onActive_symbolChanged() {
            shell.scheduleAutosave();
        }
        function onActive_timeframeChanged() {
            shell.scheduleAutosave();
        }
        function onChart_modeChanged() {
            shell.scheduleAutosave();
        }
    }

    Connections {
        target: shell.activeFeed
        function onOverlay_revisionChanged() {
            shell.scheduleAutosave();
        }
    }

    property Instantiator windows: Instantiator {
        model: shell.windowModel
        delegate: ShellWindow {
            shell: shell
        }
    }

    Component.onCompleted: {
        workspaceController.refresh();
        workspaceController.restore_last("");
        shell.restoreWorkspaceLayout();
    }
}
