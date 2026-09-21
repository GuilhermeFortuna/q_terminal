pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: Theme.surfaceBase
    border.color: Theme.surfaceSelected
    border.width: Spacing.size1

    required property ExecutionModels executionModels
    required property ExecutionControls executionControls
    required property OpsStatus opsStatus

    // The selection this list shows. A window detached from the global selection binds its
    // own here and, with routesSelection, hands a click to the shell instead of selecting
    // in the shared models itself (Q-052).
    property string selectedId: root.executionModels.selected_deployment_id
    property bool routesSelection: false
    signal selectRequested(string id)
    // Scroll position; the panel keeps it when it moves between windows.
    property alias scrollY: deploymentsView.contentY

    function trigger(command) {
        if (command === "account.create") {
            accountDialog.open();
        } else if (command === "deployment.create") {
            deployDialog.open();
        }
    }

    AccountDialog {
        id: accountDialog
        onCreateRequested: function(name, balance, currency) {
            var payload = { name: name, initial_balance: balance, currency: currency };
            executionControls.request("create_account", JSON.stringify(payload));
        }
    }

    DeployDialog {
        id: deployDialog
        executionControls: root.executionControls
        accountId: root.executionModels.selected_account_id
        onDeployRequested: function(payload) {
            executionControls.request("create_deployment", JSON.stringify(payload));
        }
        onLiveDeployConfirmed: function(payload) {
            liveDeployConfirm.payload = payload;
            liveDeployConfirm.open();
        }
    }

    ConfirmDialog {
        id: liveDeployConfirm
        property var payload: ({})
        actionTitle: "Deploy LIVE deployment"
        consequenceText: "Creates a live MT5 deployment that can place real broker orders when gates permit."
        liveWarning: true
        onConfirmed: {
            executionControls.request("create_deployment", JSON.stringify(liveDeployConfirm.payload));
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size0

        // List Header
        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size36
            color: Theme.surfaceElevated
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size12
                anchors.rightMargin: Spacing.size12

                Text {
                    text: "DEPLOYMENTS"
                    color: Theme.textSecondary
                    font.pixelSize: Theme.typeBodySmall
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }

                Rectangle {
                    height: Spacing.size22
                    implicitWidth: acctBtnText.implicitWidth + 12
                    radius: Spacing.size3
                    color: acctMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceSelected
                    border.color: Theme.borderDefault
                    opacity: root.executionControls.is_command_enabled("create_account") ? 1.0 : 0.4

                    Text {
                        id: acctBtnText
                        anchors.centerIn: parent
                        text: "+ Account"
                        color: Theme.accent
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }

                    MouseArea {
                        id: acctMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: root.executionControls.is_command_enabled("create_account")
                        onClicked: accountDialog.open()
                    }
                }

                Rectangle {
                    height: Spacing.size22
                    implicitWidth: newBtnText.implicitWidth + 12
                    radius: Spacing.size3
                    color: newMouse.containsMouse ? Theme.accentPressed : Theme.accentDark
                    border.color: Theme.accentStrong
                    opacity: root.executionControls.is_command_enabled("create_deployment") ? 1.0 : 0.4

                    Text {
                        id: newBtnText
                        anchors.centerIn: parent
                        text: "+ New"
                        color: Theme.textOnAccent
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }

                    MouseArea {
                        id: newMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        enabled: root.executionControls.is_command_enabled("create_deployment")
                        onClicked: deployDialog.open()
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                Text {
                    text: (deploymentsView.count !== undefined ? deploymentsView.count : 0) + " active"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeBodySmall
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Deployments ListView
        ListView {
            id: deploymentsView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.executionModels.deployments
            spacing: Spacing.size2

            boundsBehavior: Flickable.StopAtBounds

            delegate: Rectangle {
                id: card
                required property string id
                required property string name
                required property string symbol
                required property string timeframe
                required property string broker_mode
                required property string lifecycle
                required property string pending_action
                required property var unknown_order_count
                required property string pos_side
                required property string pos_quantity
                required property string pos_entry_price
                required property string pos_unrealized_pnl
                required property string pos_mark_price
                required property string last_bar_close_time

                readonly property bool isSelected: root.selectedId === card.id
                readonly property bool isLive: card.broker_mode.toLowerCase() === "live"

                width: deploymentsView.width
                height: Spacing.size84
                color: card.isSelected ? Theme.surfaceSelected : (mouseArea.containsMouse ? Theme.surfaceOverlay : Theme.surfaceBase)
                border.color: card.isSelected ? Theme.accent : Theme.surfaceSelected
                border.width: card.isSelected ? 1 : 1

                // Left status indicator bar
                Rectangle {
                    width: Spacing.size4
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    color: Semantic.foreground(Semantic.lifecycle(card.lifecycle))
                }

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        if (root.routesSelection) {
                            root.selectRequested(card.id);
                        } else {
                            root.executionModels.select_deployment(card.id);
                        }
                    }
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Spacing.size12
                    anchors.rightMargin: Spacing.size10
                    anchors.topMargin: Spacing.size8
                    anchors.bottomMargin: Spacing.size8
                    spacing: Spacing.size4

                    // Line 1: Name and Badges
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size6

                        Text {
                            text: card.name !== "" ? card.name : card.id
                            color: Theme.textStrong
                            font.pixelSize: Theme.typeBodyLarge
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        // Unknown orders warning badge
                        Rectangle {
                            height: Spacing.size18
                            implicitWidth: unkOrderText.implicitWidth + 8
                            radius: Spacing.size3
                            visible: Number(card.unknown_order_count) > 0
                            color: Theme.criticalSurface
                            border.color: Theme.negativeStrong

                            Text {
                                id: unkOrderText
                                anchors.centerIn: parent
                                text: "!" + card.unknown_order_count + " unk"
                                color: Theme.negativeSoft
                                font.pixelSize: Theme.typeLabelSmall
                                font.bold: true
                            }
                        }

                        // Broker Mode Badge: Live vs Paper
                        Rectangle {
                            height: Spacing.size18
                            implicitWidth: modeText.implicitWidth + 8
                            radius: Spacing.size3
                            color: Semantic.background(Semantic.brokerMode(card.broker_mode))
                            border.color: Semantic.border(Semantic.brokerMode(card.broker_mode))

                            Text {
                                id: modeText
                                anchors.centerIn: parent
                                text: card.broker_mode.toUpperCase()
                                color: Semantic.foreground(Semantic.brokerMode(card.broker_mode))
                                font.pixelSize: Theme.typeLabelSmall
                                font.bold: true
                            }
                        }

                        // Lifecycle badge
                        Rectangle {
                            height: Spacing.size18
                            implicitWidth: lifeText.implicitWidth + 8
                            radius: Spacing.size3
                            color: Semantic.background(Semantic.lifecycle(card.lifecycle))

                            Text {
                                id: lifeText
                                anchors.centerIn: parent
                                text: card.lifecycle.toUpperCase()
                                color: Semantic.foreground(Semantic.lifecycle(card.lifecycle))
                                font.pixelSize: Theme.typeLabelSmall
                                font.bold: true
                            }
                        }
                    }

                    // Line 2: Symbol, Timeframe, Pending Action
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size8

                        Text {
                            text: card.symbol + " · " + card.timeframe
                            color: Theme.textSecondary
                            font.pixelSize: Theme.typeBodySmall
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            visible: card.pending_action !== ""
                            text: "Action: " + card.pending_action
                            color: Theme.warningStrong
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }
                    }

                    // Line 3: Net Position & Entry Price, Last Evaluated Bar
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size8

                        Text {
                            text: {
                                if (card.pos_side === "" || card.pos_side.toLowerCase() === "flat") {
                                    return "Position: FLAT";
                                }
                                return card.pos_side.toUpperCase() + " " + card.pos_quantity + " @ " + card.pos_entry_price;
                            }
                            color: Semantic.foreground(Semantic.position(card.pos_side))
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: card.pos_side.toLowerCase() !== "flat" && card.pos_side !== ""
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: card.last_bar_close_time !== "" ? Format.formatShortTime(card.last_bar_close_time) : "--"
                            color: Theme.textMuted
                            font.pixelSize: Theme.typeLabel
                        }
                    }
                }
            }
        }
    }
}
