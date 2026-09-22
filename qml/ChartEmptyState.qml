pragma ComponentBehavior: Bound
import QtQuick
import qml

Rectangle {
    id: root

    property var feed: null
    property var context: null
    color: Theme.surfaceBase

    readonly property string connectionState: root.feed ? root.feed.connection_state : "disconnected"
    readonly property string lastError: root.feed ? root.feed.last_error : ""
    readonly property bool isLiveOnly: root.feed ? root.feed.live_only : false
    readonly property bool historyLoading: root.feed ? root.feed.history_loading : false
    readonly property string conditionLabel: root.context ? root.context.condition_label : ""
    readonly property string symbolLine: root.context ? root.context.symbol_line : ""
    readonly property string sourceLabel: root.context ? root.context.source_label : ""

    Column {
        anchors.centerIn: parent
        spacing: Spacing.size8
        width: Math.min(parent.width - Spacing.size16 * 2, Spacing.dialogWidth)

        Text {
            id: identityLine
            objectName: "emptyIdentityText"
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            color: Theme.textSecondary
            font.pixelSize: Theme.typeBodySmall
            elide: Text.ElideRight
            maximumLineCount: 1
            visible: root.symbolLine !== ""
            text: root.symbolLine + (root.sourceLabel !== "" ? (" · " + root.sourceLabel) : "")
        }

        Text {
            id: messageText
            objectName: "emptyMessageText"
            anchors.horizontalCenter: parent.horizontalCenter
            color: Theme.textPrimary
            font.pixelSize: Theme.typeTitle
            font.bold: true
            text: {
                if (root.conditionLabel !== "") {
                    return root.conditionLabel;
                }
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
