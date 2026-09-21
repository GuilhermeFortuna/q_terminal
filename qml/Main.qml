pragma ComponentBehavior: Bound
import QtQuick
import qml

Window {
    id: window
    visible: true
    width: Spacing.size1280
    height: Spacing.size800
    title: "q_terminal"
    color: Theme.surfaceBase

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

    readonly property var activeFeed: window.feed ? window.feed : defaultFeed
    readonly property var activeExecutionModels: window.executionModels ? window.executionModels : defaultExecutionModels
    readonly property var activeOpsStatus: window.opsStatus ? window.opsStatus : defaultOpsStatus
    readonly property var activeExecutionControls: window.executionControls ? window.executionControls : defaultExecutionControls

    AppInfo {
        id: appInfo
        objectName: "appInfo"
    }

    OpsWorkspace {
        id: opsWorkspace
        objectName: "opsWorkspace"
        anchors.fill: parent
        executionModels: window.activeExecutionModels
        opsStatus: window.activeOpsStatus
        executionControls: window.activeExecutionControls
        feed: window.activeFeed
    }
}
