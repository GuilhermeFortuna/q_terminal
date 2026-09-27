pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: Theme.surfaceBase

    required property ExecutionModels executionModels

    property string selFillId: ""
    property string selOrderId: ""
    property string selFilledAt: ""
    property string selSide: ""
    property string selPrice: ""
    property string selQty: ""
    property string selFee: ""
    property string selSlippage: ""
    property string selQuoteBid: ""
    property string selQuoteAsk: ""
    property string selQuoteTime: ""

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
                required property string quote_bid
                required property string quote_ask
                required property string quote_timestamp

                readonly property bool isSelected: listView.currentIndex === rowRect.index

                width: listView.width
                height: Spacing.size28
                color: isSelected ? Theme.surfaceSelected : ((index % 2 === 0) ? Theme.surfaceBase : Theme.surfaceRaised)

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        listView.currentIndex = rowRect.index;
                        root.selFillId = rowRect.id;
                        root.selOrderId = rowRect.order_id;
                        root.selFilledAt = rowRect.filled_at;
                        root.selSide = rowRect.side;
                        root.selPrice = rowRect.price;
                        root.selQty = rowRect.quantity;
                        root.selFee = rowRect.fee;
                        root.selSlippage = rowRect.slippage;
                        root.selQuoteBid = rowRect.quote_bid;
                        root.selQuoteAsk = rowRect.quote_ask;
                        root.selQuoteTime = rowRect.quote_timestamp;
                    }
                }

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
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeBodySmall
                        Layout.fillWidth: true
                    }
                }
            }
        }

        // Wireshark-style dense Audit Detail Inspector pane for Fills
        Rectangle {
            Layout.fillWidth: true
            height: root.selFillId !== "" ? Spacing.size72 : Spacing.none
            visible: root.selFillId !== ""
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Spacing.size8
                spacing: Spacing.size4

                RowLayout {
                    spacing: Spacing.size12
                    Text { text: "FILL AUDIT"; color: Theme.accent; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                    Text {
                        text: "FILL ID: " + root.selFillId
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                    Text {
                        text: "ORDER ID: " + root.selOrderId
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                    Text {
                        text: "TIME: " + Format.formatIsoTime(root.selFilledAt)
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        text: "EXECUTION: " + root.selSide.toUpperCase() + " " + root.selQty + " @ " + root.selPrice
                        color: Semantic.foreground(Semantic.side(root.selSide))
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Text {
                        text: "FEE: " + (root.selFee !== "" ? root.selFee : "0.00")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                    Text {
                        text: "SLIPPAGE: " + (root.selSlippage !== "" ? root.selSlippage : "0.0")
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                RowLayout {
                    spacing: Spacing.size16
                    Text {
                        visible: root.selQuoteBid !== "" || root.selQuoteAsk !== ""
                        text: "MARKET QUOTE AT FILL: Bid " + (root.selQuoteBid || "--") + " | Ask " + (root.selQuoteAsk || "--") + (root.selQuoteTime !== "" ? (" (" + Format.formatIsoTime(root.selQuoteTime) + ")") : "")
                        color: Theme.accent
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }
            }
        }
    }
}
