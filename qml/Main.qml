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
    property var executionModels: null
    property var opsStatus: null
    property var executionControls: null

    BarFeed {
        id: defaultFeed
        objectName: "barFeed"
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

    readonly property var activeFeed: shell.feed ? shell.feed : defaultFeed
    readonly property var activeExecutionModels: shell.executionModels ? shell.executionModels : defaultExecutionModels
    readonly property var activeOpsStatus: shell.opsStatus ? shell.opsStatus : defaultOpsStatus
    readonly property var activeExecutionControls: shell.executionControls ? shell.executionControls : defaultExecutionControls
    readonly property var controller: shellController

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

    // Called from a window's closing signal. The window object itself is destroyed a turn
    // later, never from inside its own signal.
    function windowClosing(id) {
        shellController.close_window(id);
    }

    // A selection made in a window. Attached windows share the global selection, so the one
    // shared execution model retargets; a detached window keeps its selection to itself.
    function selectDeployment(windowId, deploymentId) {
        if (shellController.select_deployment(windowId, deploymentId)) {
            shell.activeExecutionModels.select_deployment(deploymentId);
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
        }
    }

    // Runs a command on the panel that owns its dialog, and brings that window forward.
    function forward(panelId, command) {
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
            "tape": placeholderPanel,
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
        }
    }

    property Instantiator windows: Instantiator {
        model: shell.windowModel
        delegate: ShellWindow {
            shell: shell
        }
    }

    Component.onCompleted: {
        shellController.open_window("merged");
        shell.syncWindows();
    }
}
