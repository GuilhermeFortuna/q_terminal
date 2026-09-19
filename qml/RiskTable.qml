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
                Text { text: "REJECTION CODE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 140 }
                Text { text: "MESSAGE"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.preferredWidth: 200 }
                Text { text: "CONTEXT"; color: "#64748b"; font.pixelSize: 10; font.bold: true; Layout.fillWidth: true }
            }
        }

        // Table Rows ListView
        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.risk_events
            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: rowRect
                required property int index
                required property string id
                required property string rejection_code
                required property string message
                required property string context
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
                        text: rowRect.rejection_code
                        color: "#f87171"
                        font.pixelSize: 11
                        font.bold: true
                        Layout.preferredWidth: 140
                    }

                    Text {
                        text: rowRect.message
                        color: "#fca5a5"
                        font.pixelSize: 11
                        elide: Text.ElideRight
                        Layout.preferredWidth: 200
                    }

                    Text {
                        text: rowRect.context
                        color: "#64748b"
                        font.pixelSize: 10
                        font.family: "monospace"
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
