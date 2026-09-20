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
    required property OpsStatus opsStatus
    required property ExecutionControls executionControls

    property string lastActionId: ""

    function deploymentField(field) {
        return root.executionModels.field_for_selected_deployment(field);
    }

    function requestLifecycle(action, confirm) {
        var depId = root.executionModels.selected_deployment_id;
        if (depId === "") return;
        var payload = { deployment_id: depId };
        var id = root.executionControls.request(action, JSON.stringify(payload));
        root.lastActionId = id;
    }

    ConfirmDialog {
        id: flattenDialog
        actionTitle: "Flatten position"
        consequenceText: {
            var sym = root.deploymentField("symbol");
            var tf = root.deploymentField("timeframe");
            var name = root.deploymentField("name");
            var qty = root.deploymentField("pos_quantity");
            return "Flatten " + sym + " " + tf + " · " + name + " — sends a market order through the edge to close " + (qty || "0") + " contracts";
        }
        liveWarning: root.deploymentField("broker_mode") === "mt5_live"
        onConfirmed: requestLifecycle("flatten", true)
    }

    ConfirmDialog {
        id: stopDialog
        actionTitle: "Stop deployment"
        consequenceText: "Stops the deployment; open positions remain until flattened."
        liveWarning: root.deploymentField("broker_mode") === "mt5_live"
        onConfirmed: requestLifecycle("stop", true)
    }

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
        spacing: Spacing.size0

        // Top Header: Selected Deployment info and Net Position Card
        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size72
            color: Theme.surfaceElevated
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16
                spacing: Spacing.size16

                // Deployment identity, lifecycle, pending action, last bar
                ColumnLayout {
                    spacing: Spacing.size4
                    Layout.alignment: Qt.AlignVCenter
                    Layout.fillWidth: true

                    Text {
                        text: {
                            var name = root.executionModels.field_for_selected_deployment("name");
                            var id = root.executionModels.selected_deployment_id;
                            if (id === "") return "DEPLOYMENT: None selected";
                            return "DEPLOYMENT: " + (name !== "" ? name : id);
                        }
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodyLarge
                        font.bold: true
                    }

                    RowLayout {
                        visible: root.executionModels.selected_deployment_id !== ""
                        spacing: Spacing.size8

                        Rectangle {
                            height: Spacing.size18
                            implicitWidth: lifeDetailText.implicitWidth + 8
                            radius: Spacing.size3
                            color: Semantic.background(Semantic.lifecycle(root.executionModels.field_for_selected_deployment("lifecycle")))

                            Text {
                                id: lifeDetailText
                                anchors.centerIn: parent
                                text: root.executionModels.field_for_selected_deployment("lifecycle").toUpperCase()
                                color: Semantic.foreground(Semantic.lifecycle(root.executionModels.field_for_selected_deployment("lifecycle")))
                                font.pixelSize: Theme.typeLabelSmall
                                font.bold: true
                            }
                        }

                        Text {
                            visible: root.executionModels.field_for_selected_deployment("pending_action") !== ""
                            text: "Desired: " + root.executionModels.field_for_selected_deployment("pending_action")
                            color: Theme.warningStrong
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }

                        Text {
                            visible: root.executionModels.field_for_selected_deployment("last_bar_close_time") !== ""
                            text: "Last bar: " + Format.formatIsoTime(root.executionModels.field_for_selected_deployment("last_bar_close_time"))
                            color: Theme.textMuted
                            font.pixelSize: Theme.typeLabel
                        }
                    }

                    Text {
                        text: "Click a deployment on the left to inspect"
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabel
                        visible: root.executionModels.selected_deployment_id === ""
                    }
                }

                // Lifecycle action bar
                RowLayout {
                    visible: root.executionModels.selected_deployment_id !== ""
                    spacing: Spacing.size6
                    Layout.alignment: Qt.AlignVCenter

                    Repeater {
                        model: [
                            { label: "Start", kind: "start", confirm: false },
                            { label: "Pause", kind: "pause", confirm: false },
                            { label: "Stop", kind: "stop", confirm: true },
                            { label: "Flatten", kind: "flatten", confirm: true }
                        ]

                        delegate: Rectangle {
                            required property var modelData
                            height: Spacing.size24
                            implicitWidth: actText.implicitWidth + 16
                            radius: Spacing.size4
                            opacity: root.executionControls.is_command_enabled(modelData.kind) ? 1.0 : 0.4
                            color: actMouse.containsMouse ? Theme.accentStrong : Theme.accentPressed
                            border.color: Theme.accent

                            Text {
                                id: actText
                                anchors.centerIn: parent
                                text: modelData.label
                                color: Theme.textOnAccent
                                font.pixelSize: Theme.typeLabel
                                font.bold: true
                            }

                            MouseArea {
                                id: actMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                enabled: root.executionControls.is_command_enabled(modelData.kind)
                                onClicked: {
                                    if (modelData.kind === "flatten") {
                                        flattenDialog.open();
                                    } else if (modelData.kind === "stop") {
                                        stopDialog.open();
                                    } else {
                                        requestLifecycle(modelData.kind, false);
                                    }
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                // Position Marks & PnL Summary (read-only from backend /positions poller)
                Rectangle {
                    height: Spacing.size48
                    implicitWidth: posRow.implicitWidth + 20
                    radius: Spacing.size4
                    color: Theme.surfaceBase
                    border.color: Theme.borderDefault
                    border.width: Spacing.size1
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: posRow
                        anchors.centerIn: parent
                        spacing: Spacing.size16

                        // Position Side & Qty
                        ColumnLayout {
                            spacing: Spacing.size1
                            Text { text: "POSITION"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: {
                                    if (!pos || !pos.side || pos.side === "flat") return "FLAT";
                                    return pos.side.toUpperCase() + " " + (pos.quantity || "--");
                                }
                                color: Semantic.foreground(Semantic.position(pos && pos.side ? pos.side : "flat"))
                                font.pixelSize: Theme.typeBodySmall
                                font.bold: true
                            }
                        }

                        // Avg Entry
                        ColumnLayout {
                            spacing: Spacing.size1
                            Text { text: "AVG ENTRY"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.average_entry_price) ? pos.average_entry_price : "--"
                                color: Theme.textPrimary
                                font.pixelSize: Theme.typeBodySmall
                                font.bold: true
                            }
                        }

                        // Mark Price
                        ColumnLayout {
                            spacing: Spacing.size1
                            Text { text: "MARK PRICE"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.current_mark_price) ? pos.current_mark_price : "--"
                                color: Theme.accent
                                font.pixelSize: Theme.typeBodySmall
                                font.bold: true
                            }
                        }

                        // Unrealized PnL
                        ColumnLayout {
                            spacing: Spacing.size1
                            Text { text: "UNREALIZED PnL"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                            Text {
                                property var pos: root.getSelectedPosition()
                                text: (pos && pos.unrealized_pnl) ? pos.unrealized_pnl : "--"
                                color: Semantic.foreground(Semantic.signedString(pos && pos.unrealized_pnl ? pos.unrealized_pnl : ""))
                                font.pixelSize: Theme.typeBodySmall
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
            height: Spacing.size32
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size8
                spacing: Spacing.size4

                Repeater {
                    model: ["Orders", "Fills", "Decisions", "Risk Events", "Account & Ledger"]

                    delegate: Rectangle {
                        id: tabButton
                        required property int index
                        required property string modelData
                        readonly property bool isActive: root.currentTabIndex === tabButton.index

                        height: Spacing.size28
                        implicitWidth: tabText.implicitWidth + 24
                        radius: Spacing.size4
                        color: tabButton.isActive ? Theme.surfaceSelected : (tabMouse.containsMouse ? Theme.surfaceElevated : Theme.transparent)
                        border.color: tabButton.isActive ? Theme.accent : Theme.transparent
                        border.width: Spacing.size1
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
                            color: tabButton.isActive ? Theme.accent : (tabMouse.containsMouse ? Theme.textStrong : Theme.textSecondary)
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: tabButton.isActive
                        }
                    }
                }
            }
        }

        // Account Details sub-bar (visible only on "Account & Ledger" tab)
        Rectangle {
            Layout.fillWidth: true
            height: (root.currentTabIndex === 4) ? 92 : 0
            visible: root.currentTabIndex === 4
            color: Theme.surfaceRaised
            border.color: Theme.surfaceSelected
            border.width: Spacing.size1
            clip: true

            ColumnLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16
                anchors.topMargin: Spacing.size8
                anchors.bottomMargin: Spacing.size8
                spacing: Spacing.size8

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size12

                    Text {
                        text: "ACCOUNTS:"
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                        Layout.alignment: Qt.AlignVCenter
                    }

                    ListView {
                        id: accountsList
                        Layout.fillWidth: true
                        Layout.preferredHeight: Spacing.size28
                        orientation: ListView.Horizontal
                        clip: true
                        model: root.executionModels.accounts
                        spacing: Spacing.size8

                        delegate: Rectangle {
                            id: accChip
                            required property string id
                            required property string name
                            required property string currency
                            required property string cash_balance

                            readonly property bool isSelected: root.executionModels.selected_account_id === accChip.id

                            height: Spacing.size28
                            implicitWidth: accRow.implicitWidth + 16
                            radius: Spacing.size4
                            color: accChip.isSelected ? Theme.surfaceSelected : (accMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceBase)
                            border.color: accChip.isSelected ? Theme.accent : Theme.borderDefault
                            border.width: Spacing.size1
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
                                spacing: Spacing.size8

                                Text {
                                    text: accChip.name !== "" ? accChip.name : accChip.id
                                    color: accChip.isSelected ? Theme.accent : Theme.textStrong
                                    font.pixelSize: Theme.typeBodySmall
                                    font.bold: true
                                }

                                Text {
                                    text: accChip.cash_balance + " " + accChip.currency
                                    color: Theme.textSecondary
                                    font.pixelSize: Theme.typeLabel
                                }
                            }
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size12
                    visible: root.executionModels.selected_account_id !== ""

                    Repeater {
                        model: [
                            { label: "Cash balance", field: "cash_balance", color: Theme.textStrong },
                            { label: "Equity (cash)", field: "cash_balance", color: Theme.textStrong },
                            { label: "Session P&L vs initial", field: "session_pnl", color: Theme.textPrimary }
                        ]

                        delegate: Rectangle {
                            required property var modelData
                            Layout.fillWidth: true
                            height: Spacing.size40
                            radius: Spacing.size4
                            color: Theme.surfaceBase
                            border.color: Theme.borderDefault
                            border.width: Spacing.size1

                            ColumnLayout {
                                anchors.fill: parent
                                anchors.margins: Spacing.size8
                                spacing: Spacing.size2

                                Text {
                                    text: modelData.label.toUpperCase()
                                    color: Theme.textMuted
                                    font.pixelSize: Theme.typeLabelSmall
                                    font.bold: true
                                }

                                Text {
                                    text: {
                                        var value = root.executionModels.field_for_selected_account(modelData.field);
                                        if (value === "") return "--";
                                        var currency = root.executionModels.field_for_selected_account("currency");
                                        return value + (currency !== "" ? " " + currency : "");
                                    }
                                    color: modelData.field === "session_pnl" ? Semantic.foreground(Semantic.signedString(root.executionModels.field_for_selected_account("session_pnl"))) : modelData.color
                                    font.pixelSize: Theme.typeBodySmall
                                    font.bold: true
                                }
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
            height: Spacing.size32
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size12
                anchors.rightMargin: Spacing.size12

                Text {
                    text: "Newest rows streamed live · revision " + root.executionModels.revision
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabel
                    Layout.alignment: Qt.AlignVCenter
                }

                Item { Layout.fillWidth: true }

                Rectangle {
                    id: loadOlderBtn
                    height: Spacing.size22
                    implicitWidth: loadOlderText.implicitWidth + 16
                    radius: Spacing.size3
                    color: loadOlderMouse.containsMouse ? Theme.accentStrong : Theme.accentPressed
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
                        color: Theme.textOnAccent
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                }
            }
        }
    }
}
