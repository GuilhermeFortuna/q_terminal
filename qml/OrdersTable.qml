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
                Text { text: "ORDER ID"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 90 }
                Text { text: "INTENT ID"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 110 }
                Text { text: "SIDE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 60 }
                Text { text: "TYPE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 70 }
                Text { text: "QTY"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 80 }
                Text { text: "STATUS"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 80 }
                Text { text: "RECONCILIATION"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.fillWidth: true }
            }
        }

        // Table Rows ListView
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.orders
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: rowRect
                required property string id
                required property string intent_id
                required property string side
                required property string order_type
                required property string quantity
                required property string status
                required property string reconciliation_state
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
                        text: Format.truncateId(rowRect.id, 8)
                        color: "#38bdf8"
                        font.pixelSize: 11
                        Layout.preferredWidth: 90
                    }

                    Text {
                        text: Format.truncateId(rowRect.intent_id, 10)
                        color: "#94a3b8"
                        font.pixelSize: 11
                        Layout.preferredWidth: 110
                    }

                    Text {
                        text: rowRect.side.toUpperCase()
                        color: (rowRect.side.toLowerCase() === "buy") ? "#34d399" : "#f87171"
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 60
                    }

                    Text {
                        text: rowRect.order_type.toUpperCase()
                        color: "#cbd5e1"
                        font.pixelSize: 11
                        Layout.preferredWidth: 70
                    }

                    Text {
                        text: rowRect.quantity
                        color: "#f8fafc"
                        font.pixelSize: 11
                        Layout.preferredWidth: 80
                    }

                    Text {
                        text: rowRect.status.toUpperCase()
                        color: {
                            if (rowRect.status === "filled") return "#34d399";
                            if (rowRect.status === "pending") return "#fbbf24";
                            if (rowRect.status === "rejected" || rowRect.status === "cancelled") return "#f87171";
                            return "#94a3b8";
                        }
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 80
                    }

                    Text {
                        text: rowRect.reconciliation_state
                        color: (rowRect.reconciliation_state === "reconciled" || rowRect.reconciliation_state === "none") ? "#64748b" : "#f59e0b"
                        font.pixelSize: 11
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
