pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: Theme.surfaceBase

    required property ExecutionModels executionModels

    property string selOrderId: ""
    property string selIntentId: ""
    property string selIntentTime: ""
    property string selDispatchTime: ""
    property string selDecisionId: ""
    property string selStatus: ""
    property string selRejection: ""
    property string selReconciliation: ""
    property string selSide: ""
    property string selQty: ""
    property string selCreatedAt: ""

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
                required property string rejection_reason
                required property string dispatch_attempted_at
                required property string intent_committed_at
                required property string decision_id
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
                        root.selOrderId = rowRect.id;
                        root.selIntentId = rowRect.intent_id;
                        root.selIntentTime = rowRect.intent_committed_at;
                        root.selDispatchTime = rowRect.dispatch_attempted_at;
                        root.selDecisionId = rowRect.decision_id;
                        root.selStatus = rowRect.status;
                        root.selRejection = rowRect.rejection_reason;
                        root.selReconciliation = rowRect.reconciliation_state;
                        root.selSide = rowRect.side;
                        root.selQty = rowRect.quantity;
                        root.selCreatedAt = rowRect.created_at;
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

        // Wireshark-style dense Audit Detail Inspector pane
        Rectangle {
            Layout.fillWidth: true
            height: root.selOrderId !== "" ? Spacing.size72 : Spacing.none
            visible: root.selOrderId !== ""
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Spacing.size8
                spacing: Spacing.size4

                RowLayout {
                    spacing: Spacing.size12
                    Text { text: "AUDIT TRAIL"; color: Theme.accent; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                    Text {
                        text: "ORDER: " + root.selOrderId
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                    Text {
                        text: "STATUS: " + root.selStatus.toUpperCase()
                        color: Semantic.foreground(Semantic.orderStatus(root.selStatus))
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Text {
                        visible: root.selReconciliation !== ""
                        text: "RECON: " + root.selReconciliation
                        color: Semantic.foreground(Semantic.reconciliation(root.selReconciliation))
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        text: "DECISION: " + (root.selDecisionId !== "" ? root.selDecisionId : "--")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                    Text {
                        text: "→ INTENT: " + (root.selIntentId !== "" ? root.selIntentId : "--") + (root.selIntentTime !== "" ? (" (" + Format.formatIsoTime(root.selIntentTime) + ")") : "")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                    Text {
                        text: "→ DISPATCH: " + (root.selDispatchTime !== "" ? Format.formatIsoTime(root.selDispatchTime) : "Not attempted")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        visible: root.selRejection !== ""
                        text: "REJECTION REASON: " + root.selRejection
                        color: Theme.critical
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Text {
                        text: "CREATED: " + Format.formatIsoTime(root.selCreatedAt) + " | SIDE: " + root.selSide.toUpperCase() + " | QTY: " + root.selQty
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }
            }
        }
    }
}
