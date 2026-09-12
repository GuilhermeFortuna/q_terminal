import qml
import QtQuick

Window {
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
            font.pixelSize: 16
        }
        Text {
            text: "Core: " + appInfo.core_version
            font.pixelSize: 16
        }
        Text {
            text: "Contracts: " + appInfo.contracts_rev
            font.pixelSize: 16
        }
        Text {
            text: "Render Backend: " + appInfo.render_backend
            font.pixelSize: 16
        }
    }
}
