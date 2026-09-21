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
                Text { text: "FILL ID"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size90 }
                Text { text: "ORDER ID"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size90 }
                Text { text: "SIDE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size60 }
                Text { text: "PRICE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size90 }
                Text { text: "QTY"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size80 }
                Text { text: "FEE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size70 }
                Text { text: "SLIPPAGE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.fillWidth: true }
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
                required property int index
                required property string id
                required property string order_id
                required property string side
                required property string price
                required property string quantity
                required property string fee
                required property string slippage
                required property string filled_at

                width: listView.width
                height: Spacing.size28
                color: (index % 2 === 0) ? Theme.surfaceBase : Theme.surfaceRaised

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Spacing.size12
                    anchors.rightMargin: Spacing.size12
                    spacing: Spacing.size8

                    Text {
                        text: Format.formatIsoTime(rowRect.filled_at)
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
                        text: Format.truncateId(rowRect.order_id, 8)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size90
                    }

                    Text {
                        text: rowRect.side.toUpperCase()
                        color: Semantic.foreground(Semantic.side(rowRect.side))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size60
                    }

                    Text {
                        text: rowRect.price
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size90
                    }

                    Text {
                        text: rowRect.quantity
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size80
                    }

                    Text {
                        text: rowRect.fee !== "" ? rowRect.fee : "--"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size70
                    }

                    Text {
                        text: rowRect.slippage !== "" ? rowRect.slippage : "--"
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeBodySmall
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
