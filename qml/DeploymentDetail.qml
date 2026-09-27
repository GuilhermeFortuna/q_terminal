pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
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
    readonly property bool hasSelection: root.executionModels.selected_deployment_id !== ""

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

    EditDeploymentDialog {
        id: editDialog
        executionControls: root.executionControls
        deploymentId: root.executionModels.selected_deployment_id
    }

    property string _lastBarFetched: ""

    Connections {
        target: root.executionModels
        function onSelected_deployment_idChanged() {
            var depId = root.executionModels.selected_deployment_id;
            if (depId !== "") {
                root.executionControls.fetch_performance(depId, "");
            } else {
                root.executionControls.clear_performance();
            }
        }
        function onRevisionChanged() {
            var depId = root.executionModels.selected_deployment_id;
            var barTime = root.deploymentField("last_bar_close_time");
            if (depId !== "" && barTime !== "" && barTime !== root._lastBarFetched) {
                root._lastBarFetched = barTime;
                root.executionControls.fetch_performance(depId, barTime);
            }
        }
    }

    Component.onCompleted: {
        var depId = root.executionModels.selected_deployment_id;
        if (depId !== "") {
            root.executionControls.fetch_performance(depId, "");
        }
    }

    property int currentTabIndex: 0

    function trigger(command) {
        if (command === "deployment.flatten") {
            flattenDialog.open();
        } else if (command === "detail.next-tab") {
            root.currentTabIndex = (root.currentTabIndex + 1) % 5;
        } else if (command === "detail.previous-tab") {
            root.currentTabIndex = (root.currentTabIndex + 4) % 5;
        }
    }
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

    function getPerformanceSummary() {
        try {
            var raw = root.executionControls.performance_summary_json;
            if (!raw || raw === "" || raw === "{}") return null;
            return JSON.parse(raw);
        } catch (e) {
            return null;
        }
    }

    function getPerformanceMarks() {
        try {
            var raw = root.executionControls.performance_marks_json;
            if (!raw || raw === "" || raw === "[]") return [];
            var parsed = JSON.parse(raw);
            if (Array.isArray(parsed)) return parsed;
            if (parsed && Array.isArray(parsed.items)) return parsed.items;
            return [];
        } catch (e) {
            return [];
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size0

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: !root.hasSelection

            ColumnLayout {
                anchors.centerIn: parent
                width: parent.width - Theme.spaceXxl * 2
                spacing: Theme.spaceSm

                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "No deployment selected"
                    color: Theme.textStrong
                    font.pixelSize: Theme.typeBodyLarge
                    font.bold: true
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    text: "Select a deployment from the list to inspect execution details."
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabel
                }
            }
        }

        // Top Header: Selected Deployment info and Net Position Card
        Rectangle {
            Layout.fillWidth: true
            height: root.hasSelection ? Spacing.size72 : Spacing.none
            visible: root.hasSelection
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
                            implicitWidth: lifeDetailText.implicitWidth + Spacing.size8
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

                    Rectangle {
                        property bool canEdit: root.executionControls.is_deployment_editable(root.executionModels.selected_deployment_id)
                        property string disabledReason: root.executionControls.deployment_edit_disabled_reason(root.executionModels.selected_deployment_id)
                        height: Spacing.size24
                        implicitWidth: editBtnText.implicitWidth + Spacing.size16
                        radius: Spacing.size4
                        opacity: canEdit ? 1.0 : 0.4
                        color: editMouse.containsMouse && canEdit ? Theme.surfaceSelected : Theme.surfaceBase
                        border.color: canEdit ? Theme.accent : Theme.borderDefault

                        Text {
                            id: editBtnText
                            anchors.centerIn: parent
                            text: "Edit Config"
                            color: parent.canEdit ? Theme.textPrimary : Theme.textMuted
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }

                        ToolTip.visible: editMouse.containsMouse && !canEdit && disabledReason !== ""
                        ToolTip.text: disabledReason

                        MouseArea {
                            id: editMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: parent.canEdit
                            cursorShape: parent.canEdit ? Qt.PointingHandCursor : Qt.ArrowCursor
                            onClicked: editDialog.open()
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                // Position Marks & PnL Summary (read-only from backend /positions poller)
                Rectangle {
                    height: Spacing.size48
                    implicitWidth: posRow.implicitWidth + Spacing.size20
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

        // Performance & Bar Equity Delta Strip
        Rectangle {
            Layout.fillWidth: true
            height: root.hasSelection ? Spacing.size40 : Spacing.none
            visible: root.hasSelection
            color: Theme.surfaceBase
            border.color: Theme.borderDefault
            border.width: Spacing.size1

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16
                spacing: Spacing.size20

                // Performance label & Mark status badge
                RowLayout {
                    spacing: Spacing.size8
                    Text {
                        text: "PERFORMANCE"
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                        font.bold: true
                    }
                    Rectangle {
                        property var perf: root.getPerformanceSummary()
                        property string status: perf && perf.mark_status ? perf.mark_status : (root.deploymentField("last_bar_close_time") !== "" ? "pending" : "none")
                        height: Spacing.size16
                        implicitWidth: markStatusText.implicitWidth + Spacing.size8
                        radius: Spacing.size2
                        color: status === "marked" ? Theme.positiveSurface : (status === "pending" ? Theme.warningSurface : Theme.surfaceSelected)

                        Text {
                            id: markStatusText
                            anchors.centerIn: parent
                            text: {
                                if (parent.status === "marked") return "MARK COMMIT";
                                if (parent.status === "pending") return "MARK PENDING";
                                if (parent.status === "unavailable") return "MARK UNAVAILABLE";
                                return "NO MARKS";
                            }
                            color: {
                                if (parent.status === "marked") return Theme.positiveStrong;
                                if (parent.status === "pending") return Theme.warningStrong;
                                return Theme.textMuted;
                            }
                            font.pixelSize: Theme.typeLabelSmall
                            font.bold: true
                        }
                    }
                }

                // Net PnL
                RowLayout {
                    spacing: Spacing.size4
                    Text { text: "Net:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text {
                        property var perf: root.getPerformanceSummary()
                        text: (perf && perf.net_pnl !== undefined && perf.net_pnl !== null) ? perf.net_pnl : "--"
                        color: Semantic.foreground(Semantic.signedString(text))
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                }

                // Realized PnL
                RowLayout {
                    spacing: Spacing.size4
                    Text { text: "Realized:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text {
                        property var perf: root.getPerformanceSummary()
                        text: (perf && perf.realized_pnl !== undefined && perf.realized_pnl !== null) ? perf.realized_pnl : "--"
                        color: Semantic.foreground(Semantic.signedString(text))
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                }

                // Fees
                RowLayout {
                    spacing: Spacing.size4
                    Text { text: "Fees:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text {
                        property var perf: root.getPerformanceSummary()
                        text: (perf && perf.fees !== undefined && perf.fees !== null) ? perf.fees : "--"
                        color: Theme.textSecondary
                        font.pixelSize: Theme.typeLabel
                    }
                }

                // Win Rate & Trades
                RowLayout {
                    spacing: Spacing.size4
                    Text { text: "Win Rate:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text {
                        property var perf: root.getPerformanceSummary()
                        text: {
                            if (!perf || perf.win_rate === undefined || perf.win_rate === null) return "--";
                            var wr = parseFloat(perf.win_rate);
                            if (isNaN(wr)) return perf.win_rate;
                            return (wr * 100).toFixed(0) + "%";
                        }
                        color: Theme.textPrimary
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                    Text {
                        property var perf: root.getPerformanceSummary()
                        text: {
                            if (!perf || perf.closed_trade_count === undefined || perf.closed_trade_count === null) return "";
                            return "(" + perf.closed_trade_count + " " + (perf.closed_trade_count === 1 ? "trade" : "trades") + ")";
                        }
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                    }
                }

                // Separator
                Rectangle {
                    width: Spacing.size1
                    height: Spacing.size16
                    color: Theme.borderDefault
                }

                // Equity Delta History (recent bars)
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size6
                    Text {
                        text: "Equity Δ:"
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabelSmall
                    }

                    Item {
                        Layout.fillWidth: true
                        height: Spacing.size20
                        clip: true

                        Row {
                            spacing: Spacing.size4
                            anchors.verticalCenter: parent.verticalCenter

                            Repeater {
                                model: root.getPerformanceMarks()

                                delegate: Rectangle {
                                    required property var modelData
                                    height: Spacing.size18
                                    implicitWidth: deltaText.implicitWidth + Spacing.size8
                                    radius: Spacing.size2
                                    color: {
                                        var d = modelData.equity_delta;
                                        if (!d || d === "0.00" || d === "0") return Theme.surfaceSelected;
                                        return String(d).startsWith("-") ? Theme.negativeSurface : Theme.positiveSurface;
                                    }

                                    Text {
                                        id: deltaText
                                        anchors.centerIn: parent
                                        text: {
                                            var d = modelData.equity_delta || "--";
                                            if (d !== "--" && !String(d).startsWith("-") && !String(d).startsWith("+") && d !== "0.00" && d !== "0") {
                                                return "+" + d;
                                            }
                                            return d;
                                        }
                                        color: {
                                            var d = modelData.equity_delta;
                                            if (!d || d === "0.00" || d === "0") return Theme.textMuted;
                                            return String(d).startsWith("-") ? Theme.negativeStrong : Theme.positiveStrong;
                                        }
                                        font.pixelSize: Theme.typeLabelSmall
                                        font.bold: true
                                    }

                                    ToolTip.visible: deltaMouse.containsMouse
                                    ToolTip.text: {
                                        var time = modelData.bar_close_time ? Format.formatIsoTime(modelData.bar_close_time) : "";
                                        var mp = modelData.mark_price ? (" · Mark: " + modelData.mark_price) : "";
                                        return time + mp;
                                    }

                                    MouseArea {
                                        id: deltaMouse
                                        anchors.fill: parent
                                        hoverEnabled: true
                                    }
                                }
                            }

                            Text {
                                visible: root.getPerformanceMarks().length === 0
                                text: "No marks yet"
                                color: Theme.textMuted
                                font.pixelSize: Theme.typeLabelSmall
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }
            }
        }

        // Tab Navigation Bar
        Rectangle {
            Layout.fillWidth: true
            height: root.hasSelection ? Spacing.size32 : Spacing.none
            visible: root.hasSelection
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
            height: (root.hasSelection && root.currentTabIndex === 4) ? Spacing.accountSummaryHeight : Spacing.none
            visible: root.hasSelection && root.currentTabIndex === 4
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
            Layout.fillHeight: root.hasSelection
            visible: root.hasSelection
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
            height: root.hasSelection ? Spacing.size32 : Spacing.none
            visible: root.hasSelection
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
