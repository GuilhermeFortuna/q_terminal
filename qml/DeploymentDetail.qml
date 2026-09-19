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
    required property OpsStatus opsStatus

    property int currentTabIndex: 0
    readonly property var tabNames: ["orders", "fills", "decisions", "risk", "ledger"]

    function getSelectedPosition() {
        var depId = root.executionModels.selected_deployment_id;
        if (!depId || depId === "") return null;
        try {
            var raw = root.opsStatus.positions_pnl_json;
            if (!raw || raw === "") return null;
            var arr = JSON.parse(raw);
            if (Array.isArray(arr)) {
                for (var i = 0; i < arr.length; ++i) {
                    if (arr[i].deployment_id === depId) {
                        return arr[i];
                    }
                }
            }
        } catch (e) {}
        return null;
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Top Header: Selected Deployment info and Net Position Card
        Rectangle {
            Layout.fillWidth: true
            height: 64
            color: "#1e222d"
            border.color: "#334155"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 16

                // Deployment Name / ID
                ColumnLayout {
                    spacing: 2
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        text: "DEPLOYMENT: " + (root.executionModels.selected_deployment_id !== "" ? root.executionModels.selected_deployment_id : "None selected")
                        color: "#f8fafc"
                        font.pixelSize: 13
                        font.bold: true
                    }

                    Text {
                        text: "Click a deployment on the left to inspect"
                        color: "#64748b"
                        font.pixelSize: 10
                        visible: root.executionModels.selected_deployment_id === ""
                    }
                }

                Item { Layout.fillWidth: true }

                // Position Marks & PnL Summary (read-only from backend /positions poller)
                Rectangle {
                    height: 48
                    implicitWidth: posRow.implicitWidth + 20
                    radius: 4
                    color: "#131722"
                    border.color: "#334155"
                    border.width: 1
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: posRow
                        anchors.centerIn: parent
                        spacing: 16

                        // Position Side & Qty
                        ColumnLayout {
                            spacing: 1
                            Text { text: "POSITION"; color: "#64748b"; font.pixelSize: 9; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: {
                                    if (!pos || !pos.side || pos.side === "flat") return "FLAT";
                                    return pos.side.toUpperCase() + " " + (pos.quantity || "--");
                                }
                                color: {
                                    var pos = root.getSelectedPosition();
                                    if (!pos || !pos.side || pos.side === "flat") return "#94a3b8";
                                    return (pos.side.toLowerCase() === "long") ? "#34d399" : "#f87171";
                                }
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        // Avg Entry
                        ColumnLayout {
                            spacing: 1
                            Text { text: "AVG ENTRY"; color: "#64748b"; font.pixelSize: 9; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.average_entry_price) ? pos.average_entry_price : "--"
                                color: "#cbd5e1"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        // Mark Price
                        ColumnLayout {
                            spacing: 1
                            Text { text: "MARK PRICE"; color: "#64748b"; font.pixelSize: 9; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.current_mark_price) ? pos.current_mark_price : "--"
                                color: "#38bdf8"
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }

                        // Unrealized PnL
                        ColumnLayout {
                            spacing: 1
                            Text { text: "UNREALIZED PnL"; color: "#64748b"; font.pixelSize: 9; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.unrealized_pnl) ? pos.unrealized_pnl : "--"
                                color: {
                                    var pos = root.getSelectedPosition();
                                    if (!pos || !pos.unrealized_pnl) return "#94a3b8";
                                    var s = String(pos.unrealized_pnl);
                                    if (s.startsWith("-")) return "#f87171";
                                    if (s !== "0" && s !== "0.00") return "#34d399";
                                    return "#f8fafc";
                                }
                                font.pixelSize: 11
                                font.bold: true
                            }
                        }
                    }
                }
            }
        }

        // Tab Navigation Bar
        Rectangle {
            Layout.fillWidth: true
            height: 32
            color: "#182030"
            border.color: "#334155"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 8
                spacing: 4

                Repeater {
                    model: ["Orders", "Fills", "Decisions", "Risk Events", "Account & Ledger"]

                    delegate: Rectangle {
                        id: tabButton
                        required property int index
                        required property string modelData
                        readonly property bool isActive: root.currentTabIndex === tabButton.index

                        height: 28
                        implicitWidth: tabText.implicitWidth + 24
                        radius: 4
                        color: tabButton.isActive ? "#1e293b" : (tabMouse.containsMouse ? "#1e222d" : "transparent")
                        border.color: tabButton.isActive ? "#38bdf8" : "transparent"
                        border.width: 1
                        Layout.alignment: Qt.AlignVCenter

                        MouseArea {
                            id: tabMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                root.currentTabIndex = tabButton.index;
                            }
                        }

                        Text {
                            id: tabText
                            anchors.centerIn: parent
                            text: tabButton.modelData
                            color: tabButton.isActive ? "#38bdf8" : (tabMouse.containsMouse ? "#f8fafc" : "#94a3b8")
                            font.pixelSize: 11
                            font.bold: tabButton.isActive
                        }
                    }
                }
            }
        }

        // Account Details sub-bar (visible only on "Account & Ledger" tab)
        Rectangle {
            Layout.fillWidth: true
            height: (root.currentTabIndex === 4) ? 44 : 0
            visible: root.currentTabIndex === 4
            color: "#161c28"
            border.color: "#1e293b"
            border.width: 1
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 12

                Text {
                    text: "ACCOUNTS:"
                    color: "#64748b"
                    font.pixelSize: 10
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }

                ListView {
                    id: accountsList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    orientation: ListView.Horizontal
                    clip: true
                    model: root.executionModels.accounts
                    spacing: 8

                    delegate: Rectangle {
                        id: accChip
                        required property string id
                        required property string name
                        required property string currency
                        required property string cash_balance

                        readonly property bool isSelected: root.executionModels.selected_account_id === accChip.id

                        height: 28
                        implicitWidth: accRow.implicitWidth + 16
                        radius: 4
                        color: accChip.isSelected ? "#1e293b" : (accMouse.containsMouse ? "#243044" : "#131722")
                        border.color: accChip.isSelected ? "#38bdf8" : "#334155"
                        border.width: 1
                        anchors.verticalCenter: parent.verticalCenter

                        MouseArea {
                            id: accMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                root.executionModels.select_account(accChip.id);
                            }
                        }

                        RowLayout {
                            id: accRow
                            anchors.centerIn: parent
                            spacing: 8

                            Text {
                                text: accChip.name !== "" ? accChip.name : accChip.id
                                color: accChip.isSelected ? "#38bdf8" : "#f8fafc"
                                font.pixelSize: 11
                                font.bold: true
                            }

                            Text {
                                text: accChip.cash_balance + " " + accChip.currency
                                color: "#94a3b8"
                                font.pixelSize: 10
                            }
                        }
                    }
                }
            }
        }

        // Active Tab View Content
        StackLayout {
            id: contentStack
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: root.currentTabIndex

            OrdersTable {
                executionModels: root.executionModels
            }

            FillsTable {
                executionModels: root.executionModels
            }

            DecisionsTable {
                executionModels: root.executionModels
            }

            RiskTable {
                executionModels: root.executionModels
            }

            LedgerTable {
                executionModels: root.executionModels
            }
        }

        // Table Footer: "Load older" button for paging
        Rectangle {
            Layout.fillWidth: true
            height: 32
            color: "#182030"
            border.color: "#334155"
            border.width: 1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12

                Text {
                    text: "Newest rows streamed live · revision " + root.executionModels.revision
                    color: "#64748b"
                    font.pixelSize: 10
                    Layout.alignment: Qt.AlignVCenter
                }

                Item { Layout.fillWidth: true }

                Rectangle {
                    id: loadOlderBtn
                    height: 22
                    implicitWidth: loadOlderText.implicitWidth + 16
                    radius: 3
                    color: loadOlderMouse.containsMouse ? "#2563eb" : "#1d4ed8"
                    Layout.alignment: Qt.AlignVCenter

                    MouseArea {
                        id: loadOlderMouse
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            var tabName = root.tabNames[root.currentTabIndex];
                            root.executionModels.load_older(tabName);
                        }
                    }

                    Text {
                        id: loadOlderText
                        anchors.centerIn: parent
                        text: "Load older " + root.tabNames[root.currentTabIndex]
                        color: "#ffffff"
                        font.pixelSize: 10
                        font.bold: true
                    }
                }
            }
        }
    }
}
