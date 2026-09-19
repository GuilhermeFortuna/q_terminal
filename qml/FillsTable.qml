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
                Text { text: "FILL ID"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 90 }
                Text { text: "ORDER ID"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 90 }
                Text { text: "SIDE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 60 }
                Text { text: "PRICE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 90 }
                Text { text: "QTY"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 80 }
                Text { text: "FEE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 70 }
                Text { text: "SLIPPAGE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.fillWidth: true }
            }
        }

        // Table Rows ListView
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.fills
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: rowRect
                required property string id
                required property string order_id
                required property string side
                required property string price
                required property string quantity
                required property string fee
                required property string slippage
                required property string filled_at

                width: listView.width
                height: 28
                color: (index % 2 === 0) ? "#131722" : "#161c28"

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    spacing: 8

                    Text {
                        text: Format.formatIsoTime(rowRect.filled_at)
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
                        text: Format.truncateId(rowRect.order_id, 8)
                        color: "#94a3b8"
                        font.pixelSize: 11
                        Layout.preferredWidth: 90
                    }

                    Text {
                        text: rowRect.side.toUpperCase()
                        color: (rowRect.side.toLowerCase() === "buy") ? "#34d399" : "#f87171"
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 60
                    }

                    Text {
                        text: rowRect.price
                        color: "#f8fafc"
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 90
                    }

                    Text {
                        text: rowRect.quantity
                        color: "#f8fafc"
                        font.pixelSize: 11
                        Layout.preferredWidth: 80
                    }

                    Text {
                        text: rowRect.fee !== "" ? rowRect.fee : "--"
                        color: "#94a3b8"
                        font.pixelSize: 11
                        Layout.preferredWidth: 70
                    }

                    Text {
                        text: rowRect.slippage !== "" ? rowRect.slippage : "--"
                        color: "#64748b"
                        font.pixelSize: 11
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
