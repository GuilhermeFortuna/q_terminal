import QtQuick
import qml

PanelFrame {
    id: root
    kind: "detail"
    commandTarget: detail
    readonly property alias detail: detail
    DeploymentDetail {
        id: detail
        objectName: "deploymentDetail"
        anchors.fill: parent
        executionModels: root.shell.activeExecutionModels
        opsStatus: root.shell.activeOpsStatus
        executionControls: root.shell.activeExecutionControls
    }
}
