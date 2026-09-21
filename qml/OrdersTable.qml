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
                Text { text: "ORDER ID"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size90 }
                Text { text: "INTENT ID"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size110 }
                Text { text: "SIDE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size60 }
                Text { text: "TYPE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size70 }
                Text { text: "QTY"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size80 }
                Text { text: "STATUS"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size80 }
                Text { text: "RECONCILIATION"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.fillWidth: true }
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
                required property int index
                required property string id
                required property string intent_id
                required property string side
                required property string order_type
                required property string quantity
                required property string status
                required property string reconciliation_state
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
                        text: Format.truncateId(rowRect.id, 8)
                        color: Theme.accent
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size90
                    }

                    Text {
                        text: Format.truncateId(rowRect.intent_id, 10)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size110
                    }

                    Text {
                        text: rowRect.side.toUpperCase()
                        color: Semantic.foreground(Semantic.side(rowRect.side))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size60
                    }

                    Text {
                        text: rowRect.order_type.toUpperCase()
                        color: Theme.textPrimary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size70
                    }

                    Text {
                        text: rowRect.quantity
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size80
                    }

                    Text {
                        text: rowRect.status.toUpperCase()
                        color: Semantic.foreground(Semantic.orderStatus(rowRect.status))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size80
                    }

                    Text {
                        text: rowRect.reconciliation_state
                        color: Semantic.foreground(Semantic.reconciliation(rowRect.reconciliation_state))
                        font.pixelSize: Theme.typeBodySmall
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
