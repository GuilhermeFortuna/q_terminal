pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: Theme.surfaceBase

    required property ExecutionModels executionModels

    property string selDecisionId: ""
    property string selCreatedAt: ""
    property string selBarCloseTime: ""
    property string selSignalAction: ""
    property string selOutcome: ""
    property string selRequestedQuantity: ""
    property string selReason: ""
    property string selConfigRevision: ""

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
                required property string bar_close_time
                required property string config_revision
                required property string created_at

                readonly property bool isSelected: listView.currentIndex === rowRect.index

                width: listView.width
                height: Spacing.size28
                color: isSelected ? Theme.surfaceSelected : ((index % 2 === 0) ? Theme.surfaceBase : Theme.surfaceRaised)

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        listView.currentIndex = rowRect.index;
                        root.selDecisionId = rowRect.id;
                        root.selCreatedAt = rowRect.created_at;
                        root.selBarCloseTime = rowRect.bar_close_time;
                        root.selSignalAction = rowRect.signal_action;
                        root.selOutcome = rowRect.outcome;
                        root.selRequestedQuantity = rowRect.requested_quantity;
                        root.selReason = rowRect.reason;
                        root.selConfigRevision = rowRect.config_revision;
                    }
                }

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

        // Wireshark-style dense Audit Detail Inspector pane for Decisions
        Rectangle {
            Layout.fillWidth: true
            height: root.selDecisionId !== "" ? Spacing.size72 : Spacing.none
            visible: root.selDecisionId !== ""
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Spacing.size8
                spacing: Spacing.size4

                RowLayout {
                    spacing: Spacing.size12
                    Text { text: "DECISION AUDIT"; color: Theme.accent; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                    Text {
                        text: "DECISION ID: " + root.selDecisionId
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                    Text {
                        visible: root.selConfigRevision !== ""
                        text: "REV " + root.selConfigRevision
                        color: Theme.accent
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Text {
                        text: "BAR: " + (root.selBarCloseTime !== "" ? Format.formatIsoTime(root.selBarCloseTime) : "--")
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        text: "SIGNAL: " + root.selSignalAction.toUpperCase() + " → OUTCOME: " + root.selOutcome.toUpperCase() + " (Qty: " + (root.selRequestedQuantity || "--") + ")"
                        color: Semantic.foreground(Semantic.decisionOutcome(root.selOutcome))
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Text {
                        text: "CREATED: " + Format.formatIsoTime(root.selCreatedAt)
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        visible: root.selReason !== ""
                        text: "REASON: " + root.selReason
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
