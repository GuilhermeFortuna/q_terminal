pragma ComponentBehavior: Bound
import QtQuick
import qml

Window {
    id: window
    visible: true
    width: 1280
    height: 800
    title: "q_terminal"
    color: "#131722"

    property var feed: null
    property var executionModels: null
    property var opsStatus: null

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

    readonly property var activeFeed: window.feed ? window.feed : defaultFeed
    readonly property var activeExecutionModels: window.executionModels ? window.executionModels : defaultExecutionModels
    readonly property var activeOpsStatus: window.opsStatus ? window.opsStatus : defaultOpsStatus

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
        feed: window.activeFeed
    }
}
