import qml
import QtQuick
import QtQuick.Controls

ApplicationWindow {
    id: window
    visible: true
    width: 640
    height: 480
    title: "q_terminal"

    AppInfo {
        id: appInfo
        objectName: "appInfo"
    }

    Column {
        anchors.centerIn: parent
        spacing: 12

        Text {
            text: "Application: " + appInfo.app_version
        }
        Text {
            text: "Core: " + appInfo.core_version
        }
        Text {
            text: "Contracts: " + appInfo.contracts_rev
        }
        Text {
            text: "Render Backend: " + appInfo.render_backend
        }
    }
}
