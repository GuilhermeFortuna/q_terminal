import QtQuick
import qml

PanelFrame {
    id: root
    kind: "status"
    commandTarget: header
    OpsHeader {
        id: header
        objectName: "opsHeader"
        anchors.fill: parent
        opsStatus: root.shell.activeOpsStatus
        executionControls: root.shell.activeExecutionControls
        executionModels: root.shell.activeExecutionModels
    }
}
