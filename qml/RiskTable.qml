pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: Theme.surfaceBase

    required property ExecutionModels executionModels

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size0

        // Table Header
        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size28
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size12
                anchors.rightMargin: Spacing.size12
                spacing: Spacing.size8

                Text { text: "TIME"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size140 }
                Text { text: "REJECTION CODE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size140 }
                Text { text: "MESSAGE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size200 }
                Text { text: "CONTEXT"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.fillWidth: true }
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
                height: Spacing.size28
                color: (index % 2 === 0) ? Theme.surfaceBase : Theme.surfaceRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Spacing.size12
                    anchors.rightMargin: Spacing.size12
                    spacing: Spacing.size8

                    Text {
                        text: Format.formatIsoTime(rowRect.created_at)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size140
                    }

                    Text {
                        text: rowRect.rejection_code
                        color: Semantic.foreground(Semantic.negative)
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size140
                    }

                    Text {
                        text: rowRect.message
                        color: Theme.negativeSoft
                        font.pixelSize: Theme.typeBodySmall
                        elide: Text.ElideRight
                        Layout.preferredWidth: Spacing.size200
                    }

                    Text {
                        text: rowRect.context
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabel
                        font.family: Theme.numericFontFamily
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
