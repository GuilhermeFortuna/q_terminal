pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: "#131722"

    required property ExecutionModels executionModels

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Table Header
        Rectangle {
            Layout.fillWidth: true
            height: 28
            color: "#182030"
            border.color: "#334155"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                spacing: 8

                Text { text: "TIME"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 140 }
                Text { text: "ACTION"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 80 }
                Text { text: "OUTCOME"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 100 }
                Text { text: "REQUESTED QTY"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 120 }
                Text { text: "REASON"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.fillWidth: true }
            }
        }

        // Table Rows ListView
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.decisions
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: rowRect
                required property string id
                required property string signal_action
                required property string outcome
                required property string requested_quantity
                required property string reason
                required property string created_at

                width: listView.width
                height: 28
                color: (index % 2 === 0) ? "#131722" : "#161c28"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 8

                    Text {
                        text: Format.formatIsoTime(rowRect.created_at)
                        color: "#94a3b8"
                        font.pixelSize: 11
                        Layout.preferredWidth: 140
                    }

                    Text {
                        text: rowRect.signal_action.toUpperCase()
                        color: {
                            var a = rowRect.signal_action.toLowerCase();
                            if (a === "buy" || a === "open_long") return "#34d399";
                            if (a === "sell" || a === "open_short") return "#f87171";
                            if (a === "hold") return "#94a3b8";
                            return "#cbd5e1";
                        }
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 80
                    }

                    Text {
                        text: rowRect.outcome
                        color: {
                            var o = rowRect.outcome.toLowerCase();
                            if (o === "executed" || o === "submitted") return "#34d399";
                            if (o === "rejected" || o === "blocked") return "#f87171";
                            return "#fbbf24";
                        }
                        font.pixelSize: 11
                        Layout.preferredWidth: 100
                    }

                    Text {
                        text: rowRect.requested_quantity !== "" ? rowRect.requested_quantity : "--"
                        color: "#f8fafc"
                        font.pixelSize: 11
                        Layout.preferredWidth: 120
                    }

                    Text {
                        text: rowRect.reason
                        color: "#94a3b8"
                        font.pixelSize: 11
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
