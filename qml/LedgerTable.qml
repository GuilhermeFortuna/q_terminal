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
                Text { text: "TYPE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 100 }
                Text { text: "AMOUNT"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 110 }
                Text { text: "BALANCE AFTER"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 120 }
                Text { text: "DESCRIPTION"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.fillWidth: true }
            }
        }

        // Table Rows ListView
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.ledger
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: rowRect
                required property int index
                required property string id
                required property string entry_type
                required property string amount
                required property string balance_after
                required property string description
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
                        text: rowRect.entry_type.toUpperCase()
                        color: {
                            var t = rowRect.entry_type.toLowerCase();
                            if (t === "deposit" || t === "trade_pnl_positive") return "#34d399";
                            if (t === "withdrawal" || t === "fee" || t === "trade_pnl_negative") return "#f87171";
                            return "#cbd5e1";
                        }
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 100
                    }

                    Text {
                        text: rowRect.amount
                        color: {
                            if (rowRect.amount.startsWith("-")) return "#f87171";
                            if (rowRect.amount !== "0" && rowRect.amount !== "0.00") return "#34d399";
                            return "#f8fafc";
                        }
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 110
                    }

                    Text {
                        text: rowRect.balance_after
                        color: "#f8fafc"
                        font.pixelSize: 11
                        Layout.preferredWidth: 120
                    }

                    Text {
                        text: rowRect.description
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
