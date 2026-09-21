import QtQuick
import qml

PanelFrame {
    id: root
    kind: "deployments"
    commandTarget: list

    // What this window's list shows: the global selection, or its own once detached.
    readonly property string selectedId: {
        void root.shell.controller.selection_revision;
        return root.shell.controller.is_detached(root.windowId) ? root.shell.controller.effective_selection(root.windowId) : root.shell.activeExecutionModels.selected_deployment_id;
    }

    DeploymentList {
        id: list
        objectName: "deploymentList"
        anchors.fill: parent
        executionModels: root.shell.activeExecutionModels
        executionControls: root.shell.activeExecutionControls
        opsStatus: root.shell.activeOpsStatus
        selectedId: root.selectedId
        routesSelection: true
        onSelectRequested: function (id) {
            root.shell.selectDeployment(root.windowId, id);
        }
    }
}
