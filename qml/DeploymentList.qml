pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    color: "#131722"
    border.color: "#1e293b"
    border.width: 1

    required property ExecutionModels executionModels
    required property ExecutionControls executionControls
    required property OpsStatus opsStatus

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
        spacing: 0

        // List Header
        Rectangle {
            Layout.fillWidth: true
            height: 36
            color: "#1e222d"
            border.color: "#334155"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12

                Text {
                    text: "DEPLOYMENTS"
                    color: "#94a3b8"
                    font.pixelSize: 11
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }

                Rectangle {
                    height: 22
                    implicitWidth: acctBtnText.implicitWidth + 12
                    radius: 3
                    color: acctMouse.containsMouse ? "#243044" : "#1e293b"
                    border.color: "#334155"
                    opacity: root.executionControls.is_command_enabled("create_account") ? 1.0 : 0.4

                    Text {
                        id: acctBtnText
                        anchors.centerIn: parent
                        text: "+ Account"
                        color: "#38bdf8"
                        font.pixelSize: 10
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
                    height: 22
                    implicitWidth: newBtnText.implicitWidth + 12
                    radius: 3
                    color: newMouse.containsMouse ? "#1d4ed8" : "#1e3a8a"
                    border.color: "#2563eb"
                    opacity: root.executionControls.is_command_enabled("create_deployment") ? 1.0 : 0.4

                    Text {
                        id: newBtnText
                        anchors.centerIn: parent
                        text: "+ New"
                        color: "#ffffff"
                        font.pixelSize: 10
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
                    color: "#64748b"
                    font.pixelSize: 11
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
            spacing: 2

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

                readonly property bool isSelected: root.executionModels.selected_deployment_id === card.id
                readonly property bool isLive: card.broker_mode.toLowerCase() === "live"

                width: deploymentsView.width
                height: 84
                color: card.isSelected ? "#1e293b" : (mouseArea.containsMouse ? "#182030" : "#131722")
                border.color: card.isSelected ? "#38bdf8" : "#1e293b"
                border.width: card.isSelected ? 1 : 1

                // Left status indicator bar
                Rectangle {
                    width: 4
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    color: {
                        if (card.lifecycle === "running") return "#10b981";
                        if (card.lifecycle === "paused") return "#f59e0b";
                        if (card.lifecycle === "stopped") return "#ef4444";
                        return "#64748b";
                    }
                }

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        root.executionModels.select_deployment(card.id);
                    }
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 10
                    anchors.topMargin: 8
                    anchors.bottomMargin: 8
                    spacing: 4

                    // Line 1: Name and Badges
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        Text {
                            text: card.name !== "" ? card.name : card.id
                            color: "#f8fafc"
                            font.pixelSize: 13
                            font.bold: true
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }

                        // Unknown orders warning badge
                        Rectangle {
                            height: 18
                            implicitWidth: unkOrderText.implicitWidth + 8
                            radius: 3
                            visible: Number(card.unknown_order_count) > 0
                            color: "#7f1d1d"
                            border.color: "#ef4444"

                            Text {
                                id: unkOrderText
                                anchors.centerIn: parent
                                text: "!" + card.unknown_order_count + " unk"
                                color: "#fca5a5"
                                font.pixelSize: 9
                                font.bold: true
                            }
                        }

                        // Broker Mode Badge: Live vs Paper
                        Rectangle {
                            height: 18
                            implicitWidth: modeText.implicitWidth + 8
                            radius: 3
                            color: card.isLive ? "#991b1b" : "#1e293b"
                            border.color: card.isLive ? "#f87171" : "#475569"

                            Text {
                                id: modeText
                                anchors.centerIn: parent
                                text: card.broker_mode.toUpperCase()
                                color: card.isLive ? "#fef2f2" : "#94a3b8"
                                font.pixelSize: 9
                                font.bold: true
                            }
                        }

                        // Lifecycle badge
                        Rectangle {
                            height: 18
                            implicitWidth: lifeText.implicitWidth + 8
                            radius: 3
                            color: {
                                if (card.lifecycle === "running") return "#064e3b";
                                if (card.lifecycle === "paused") return "#451a03";
                                if (card.lifecycle === "stopped") return "#450a0a";
                                return "#1e293b";
                            }

                            Text {
                                id: lifeText
                                anchors.centerIn: parent
                                text: card.lifecycle.toUpperCase()
                                color: {
                                    if (card.lifecycle === "running") return "#34d399";
                                    if (card.lifecycle === "paused") return "#fbbf24";
                                    if (card.lifecycle === "stopped") return "#f87171";
                                    return "#94a3b8";
                                }
                                font.pixelSize: 9
                                font.bold: true
                            }
                        }
                    }

                    // Line 2: Symbol, Timeframe, Pending Action
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: card.symbol + " · " + card.timeframe
                            color: "#94a3b8"
                            font.pixelSize: 11
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            visible: card.pending_action !== ""
                            text: "Action: " + card.pending_action
                            color: "#f59e0b"
                            font.pixelSize: 10
                            font.bold: true
                        }
                    }

                    // Line 3: Net Position & Entry Price, Last Evaluated Bar
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        Text {
                            text: {
                                if (card.pos_side === "" || card.pos_side.toLowerCase() === "flat") {
                                    return "Position: FLAT";
                                }
                                return card.pos_side.toUpperCase() + " " + card.pos_quantity + " @ " + card.pos_entry_price;
                            }
                            color: {
                                if (card.pos_side.toLowerCase() === "long") return "#34d399";
                                if (card.pos_side.toLowerCase() === "short") return "#f87171";
                                return "#64748b";
                            }
                            font.pixelSize: 11
                            font.bold: card.pos_side.toLowerCase() !== "flat" && card.pos_side !== ""
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            text: card.last_bar_close_time !== "" ? Format.formatShortTime(card.last_bar_close_time) : "--"
                            color: "#64748b"
                            font.pixelSize: 10
                        }
                    }
                }
            }
        }
    }
}
