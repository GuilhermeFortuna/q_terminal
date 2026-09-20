pragma ComponentBehavior: Bound
import QtQuick
import qml

Rectangle {
    id: root

    property var feed: null
    color: Theme.surfaceBase

    readonly property string connectionState: root.feed ? root.feed.connection_state : "disconnected"
    readonly property string lastError: root.feed ? root.feed.last_error : ""
    readonly property bool isLiveOnly: root.feed ? root.feed.live_only : false
    readonly property bool historyLoading: root.feed ? root.feed.history_loading : false

    Column {
        anchors.centerIn: parent
        spacing: Spacing.size8

        Text {
            id: messageText
            objectName: "emptyMessageText"
            anchors.horizontalCenter: parent.horizontalCenter
            color: Theme.textPrimary
            font.pixelSize: Theme.typeTitle
            font.bold: true
            text: {
                var s = root.connectionState.toLowerCase();
                if (s === "retrying" || (s === "connecting" && root.lastError !== "")) {
                    return "API unreachable, retrying...";
                }
                if (root.historyLoading) {
                    return "Loading history...";
                }
                if (root.isLiveOnly) {
                    return "Waiting for live bars (live only)...";
                }
                return "No bars available";
            }
        }

        Text {
            id: reasonText
            objectName: "emptyReasonText"
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.lastError !== ""
            color: Theme.critical
            font.pixelSize: Theme.typeBody
            text: root.lastError
        }
    }
}
