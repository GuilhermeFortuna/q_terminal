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
                Text { text: "TYPE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size100 }
                Text { text: "AMOUNT"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size110 }
                Text { text: "BALANCE AFTER"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.preferredWidth: Spacing.size120 }
                Text { text: "DESCRIPTION"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel; font.bold: true; Layout.fillWidth: true }
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
                        text: rowRect.entry_type.toUpperCase()
                        color: Semantic.foreground(Semantic.ledgerEntry(rowRect.entry_type))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size100
                    }

                    Text {
                        text: rowRect.amount
                        color: Semantic.foreground(Semantic.signedString(rowRect.amount))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                        Layout.preferredWidth: Spacing.size110
                    }

                    Text {
                        text: rowRect.balance_after
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        Layout.preferredWidth: Spacing.size120
                    }

                    Text {
                        text: rowRect.description
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
