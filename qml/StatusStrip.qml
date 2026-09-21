pragma ComponentBehavior: Bound
import QtQuick
import qml

Rectangle {
    id: root

    property var feed: null
    color: Theme.surfaceElevated

    function formatDataAge(ms) {
        if (!ms || ms <= 0) {
            return "";
        }
        var sec = Math.floor(ms / 1000);
        if (sec < 60) {
            return sec + "s";
        }
        var min = Math.floor(sec / 60);
        var remSec = sec % 60;
        return min + "m " + remSec + "s";
    }

    readonly property string connectionState: root.feed ? root.feed.connection_state : "disconnected"
    readonly property string lastError: root.feed ? root.feed.last_error : ""
    readonly property bool isStale: root.feed ? root.feed.stale : false
    readonly property int dataAgeMs: root.feed ? root.feed.data_age_ms : 0
    readonly property string dataAgeText: formatDataAge(dataAgeMs)
    readonly property string historySource: root.feed ? root.feed.history_source : ""
    readonly property int historyShortfall: root.feed ? root.feed.history_shortfall : 0
    readonly property bool isLiveOnly: root.feed ? root.feed.live_only : false

    Row {
        id: leftItems
        anchors.left: parent.left
        anchors.leftMargin: Spacing.size8
        anchors.verticalCenter: parent.verticalCenter
        spacing: Spacing.size12

        Row {
            spacing: Spacing.size6
            anchors.verticalCenter: parent.verticalCenter

            Rectangle {
                width: Spacing.size8
                height: Spacing.size8
                radius: Spacing.size4
                anchors.verticalCenter: parent.verticalCenter
                color: Semantic.foreground(Semantic.health(root.connectionState))
            }

            Text {
                id: stateText
                objectName: "connectionStateText"
                text: root.connectionState.toUpperCase()
                color: Theme.textPrimary
                font.pixelSize: Theme.typeBodySmall
                font.bold: true
            }
        }

        Rectangle {
            id: staleBadge
            objectName: "staleBadge"
            visible: root.isStale
            color: Theme.critical
            radius: Spacing.size3
            height: Spacing.size16
            width: staleText.width + 8
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: staleText
                objectName: "staleText"
                anchors.centerIn: parent
                text: "STALE (" + root.dataAgeText + ")"
                color: Theme.textOnAccent
                font.pixelSize: Theme.typeLabel
                font.bold: true
            }
        }

        Rectangle {
            id: liveOnlyBadge
            objectName: "liveOnlyBadge"
            visible: root.isLiveOnly
            color: Theme.accentLegacy
            radius: Spacing.size3
            height: Spacing.size16
            width: liveOnlyText.width + 8
            anchors.verticalCenter: parent.verticalCenter

            Text {
                id: liveOnlyText
                objectName: "liveOnlyText"
                anchors.centerIn: parent
                text: "LIVE ONLY"
                color: Theme.textOnAccent
                font.pixelSize: Theme.typeLabel
                font.bold: true
            }
        }

        Text {
            id: historyText
            objectName: "historyText"
            visible: root.historySource !== "" && root.historySource !== "none"
            text: {
                var txt = "History: " + root.historySource.toUpperCase();
                if (root.historyShortfall > 0) {
                    txt += " (shortfall: " + root.historyShortfall + " bars)";
                }
                return txt;
            }
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            id: errorText
            objectName: "errorText"
            visible: root.lastError !== ""
            text: root.lastError
            color: Theme.critical
            font.pixelSize: Theme.typeBodySmall
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideRight
        }
    }

    Row {
        id: rightItems
        anchors.right: parent.right
        anchors.rightMargin: Spacing.size8
        anchors.verticalCenter: parent.verticalCenter
        spacing: Spacing.size12

        Text {
            text: "Applied: " + (root.feed ? root.feed.applied : 0)
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
        }
        Text {
            text: "Dropped: " + (root.feed ? root.feed.dropped : 0)
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
        }
        Text {
            text: "Gaps: " + (root.feed ? root.feed.gaps_closed : 0)
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
        }
        Text {
            text: "REST: " + (root.feed ? root.feed.rest_calls : 0)
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
        }
    }
}
