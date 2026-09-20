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
                Text { text: "ACTION"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size80 }
                Text { text: "OUTCOME"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size100 }
                Text { text: "REQUESTED QTY"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size120 }
                Text { text: "REASON"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.fillWidth: true }
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
                required property int index
                required property string id
                required property string signal_action
                required property string outcome
                required property string requested_quantity
                required property string reason
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
                        text: rowRect.signal_action.toUpperCase()
                        color: Semantic.foreground(Semantic.decisionAction(rowRect.signal_action))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size80
                    }

                    Text {
                        text: rowRect.outcome
                        color: Semantic.foreground(Semantic.decisionOutcome(rowRect.outcome))
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size100
                    }

                    Text {
                        text: rowRect.requested_quantity !== "" ? rowRect.requested_quantity : "--"
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size120
                    }

                    Text {
                        text: rowRect.reason
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
