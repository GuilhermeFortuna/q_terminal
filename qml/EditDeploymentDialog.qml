pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

Dialog {
    id: root
    title: inReview ? "Review configuration changes" : ("Edit configuration · Rev " + root.currentRevision)
    modal: true
    standardButtons: Dialog.NoButton
    width: Spacing.size480
    anchors.centerIn: parent

    required property ExecutionControls executionControls
    property string deploymentId: ""
    property int currentRevision: 1
    property bool inReview: false
    property bool hasConflict: false
    property string conflictMessage: ""
    property string actionId: ""
    property string submittedMessage: ""

    // Read-only server values
    property string strategyName: ""
    property string symbol: ""
    property string timeframe: ""
    property string accountId: ""
    property string brokerMode: "paper"

    // Editable parameter fields
    property string paramFastPeriod: "10"
    property string paramSlowPeriod: "30"
    property string paramStopLossPct: "1.5"
    property string paramQuantity: "1"
    property string paramMaxPositionSize: "2"
    property string paramMaxDailyDrawdown: "500.00"
    property string paramPointValue: "0.2"
    property string paramSlippage: "0.5"
    property string paramCommission: "1.0"

    // Baseline original values to detect changes
    property string origFastPeriod: "10"
    property string origSlowPeriod: "30"
    property string origStopLossPct: "1.5"
    property string origQuantity: "1"
    property string origMaxPositionSize: "2"
    property string origMaxDailyDrawdown: "500.00"
    property string origPointValue: "0.2"
    property string origSlippage: "0.5"
    property string origCommission: "1.0"

    function loadDetail() {
        root.hasConflict = false;
        root.conflictMessage = "";
        root.submittedMessage = "";
        if (root.deploymentId !== "") {
            root.executionControls.fetch_deployment_detail(root.deploymentId);
        }
    }

    onOpened: loadDetail()

    Connections {
        target: root.executionControls
        function onDeployment_detail_jsonChanged() {
            try {
                var d = JSON.parse(root.executionControls.deployment_detail_json);
                if (!d || !d.id || d.id !== root.deploymentId) return;

                root.currentRevision = d.config_revision || 1;
                root.strategyName = d.strategy_name || "";
                root.symbol = d.symbol || "";
                root.timeframe = d.timeframe || "";
                root.accountId = d.paper_account_id || d.account_id || "";
                root.brokerMode = d.broker_mode || "paper";

                // Extract strategy params
                var cfg = d.compiled_config || {};
                var sparams = cfg.strategy_params || {};
                if (sparams.fast_period !== undefined) {
                    root.paramFastPeriod = "" + sparams.fast_period;
                    root.origFastPeriod = root.paramFastPeriod;
                }
                if (sparams.slow_period !== undefined) {
                    root.paramSlowPeriod = "" + sparams.slow_period;
                    root.origSlowPeriod = root.paramSlowPeriod;
                }
                var eparams = cfg.exit_params || {};
                if (eparams.stop_loss_pct !== undefined) {
                    root.paramStopLossPct = "" + eparams.stop_loss_pct;
                    root.origStopLossPct = root.paramStopLossPct;
                }

                // Sizing & Risk
                var scfg = d.sizing_config || {};
                if (scfg.quantity !== undefined) {
                    root.paramQuantity = "" + scfg.quantity;
                    root.origQuantity = root.paramQuantity;
                }
                var rcfg = d.risk_config || {};
                if (rcfg.max_position_size !== undefined) {
                    root.paramMaxPositionSize = "" + rcfg.max_position_size;
                    root.origMaxPositionSize = root.paramMaxPositionSize;
                }
                if (rcfg.max_daily_drawdown !== undefined) {
                    root.paramMaxDailyDrawdown = "" + rcfg.max_daily_drawdown;
                    root.origMaxDailyDrawdown = root.paramMaxDailyDrawdown;
                }

                // Paper costs
                var pcfg = d.paper_cost_config || {};
                if (pcfg.point_value !== undefined) {
                    root.paramPointValue = "" + pcfg.point_value;
                    root.origPointValue = root.paramPointValue;
                }
                if (pcfg.slippage !== undefined) {
                    root.paramSlippage = "" + pcfg.slippage;
                    root.origSlippage = root.paramSlippage;
                }
                if (pcfg.commission !== undefined) {
                    root.paramCommission = "" + pcfg.commission;
                    root.origCommission = root.paramCommission;
                }
            } catch (e) {}
        }
    }

    Timer {
        id: actionPollTimer
        interval: 100
        repeat: true
        running: root.actionId !== ""
        onTriggered: {
            if (root.actionId === "") return;
            var phase = root.executionControls.action_phase(root.actionId);
            var status = root.executionControls.action_status_code(root.actionId);
            var errCode = root.executionControls.action_error_code(root.actionId);

            if (status === 409 || errCode === "conflict" || (phase === "refused" && status === 409)) {
                root.hasConflict = true;
                root.conflictMessage = "Revision conflict (409): The deployment configuration was modified by another revision. Please refresh.";
                root.actionId = "";
            } else if (phase === "refused") {
                root.hasConflict = true;
                root.conflictMessage = root.executionControls.action_message(root.actionId);
                root.actionId = "";
            } else if (phase === "awaiting_stream" || phase === "pending_stream") {
                root.submittedMessage = "Configuration accepted. Resume evaluates only newly completed bars.";
            } else if (phase === "settled") {
                root.actionId = "";
                root.accept();
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size8

        // Read-only Deployment Identity
        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size36
            radius: Spacing.size4
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size10
                anchors.rightMargin: Spacing.size10
                spacing: Spacing.size8

                Text {
                    text: root.strategyName + " · " + root.symbol + " " + root.timeframe
                    color: Theme.textStrong
                    font.pixelSize: Theme.typeLabel
                    font.bold: true
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: "REV " + root.currentRevision
                    color: Theme.accent
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }
            }
        }

        // Conflict Warning Banner (if 409 returned)
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: conflictCol.implicitHeight + 16
            visible: root.hasConflict
            radius: Spacing.size4
            color: Theme.criticalSurface
            border.color: Theme.criticalStrong
            border.width: Spacing.size1

            ColumnLayout {
                id: conflictCol
                anchors.fill: parent
                anchors.margins: Spacing.size8
                spacing: Spacing.size6

                RowLayout {
                    spacing: Spacing.size6
                    Text { text: "⚠"; color: Theme.critical; font.pixelSize: Theme.typeLabel }
                    Text {
                        text: "Revision conflict (409)"
                        color: Theme.critical
                        font.pixelSize: Theme.typeLabel
                        font.bold: true
                    }
                }
                Text {
                    text: root.conflictMessage
                    color: Theme.critical
                    font.pixelSize: Theme.typeLabelSmall
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // Submitted Status Banner
        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size28
            visible: root.submittedMessage !== "" && !root.hasConflict
            radius: Spacing.size4
            color: Theme.surfaceElevated
            border.color: Theme.accent

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size10
                Text {
                    text: root.submittedMessage
                    color: Theme.accent
                    font.pixelSize: Theme.typeLabelSmall
                }
            }
        }

        // ==================== EDIT FORM ====================
        ScrollView {
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size320
            clip: true
            visible: !root.inReview
            contentWidth: availableWidth

            ColumnLayout {
                width: parent.width
                spacing: Spacing.size10

                Text {
                    text: "Strategy identity, symbol, timeframe, and account are read-only during edit."
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                }

                // 1. Sizing & Risk Limits
                Text {
                    text: "SIZING & RISK LIMITS"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Quantity"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramQuantity
                            onTextChanged: root.paramQuantity = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Max Position Size"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramMaxPositionSize
                            onTextChanged: root.paramMaxPositionSize = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Max Daily Loss (BRL)"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramMaxDailyDrawdown
                            onTextChanged: root.paramMaxDailyDrawdown = text.trim()
                        }
                    }
                }

                // 2. Paper Costs
                Text {
                    text: "PAPER COSTS & POINT VALUE"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Point Value"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramPointValue
                            onTextChanged: root.paramPointValue = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Slippage"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramSlippage
                            onTextChanged: root.paramSlippage = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Commission"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramCommission
                            onTextChanged: root.paramCommission = text.trim()
                        }
                    }
                }

                // 3. Strategy Parameters
                Text {
                    text: "STRATEGY PARAMETERS"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size8

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Fast Period"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramFastPeriod
                            onTextChanged: root.paramFastPeriod = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Slow Period"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramSlowPeriod
                            onTextChanged: root.paramSlowPeriod = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Stop Loss (%)"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramStopLossPct
                            onTextChanged: root.paramStopLossPct = text.trim()
                        }
                    }
                }
            }
        }

        // ==================== REVIEW CHANGED VALUES STEP ====================
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size320
            visible: root.inReview
            radius: Spacing.size4
            color: Theme.surfaceOverlay
            border.color: Theme.accent
            border.width: Spacing.size1

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: Spacing.size16
                spacing: Spacing.size8

                Text {
                    text: "REVIEW CONFIGURATION CHANGES"
                    color: Theme.accent
                    font.pixelSize: Theme.typeBodySmall
                    font.bold: true
                }

                Rectangle { Layout.fillWidth: true; height: Spacing.size1; color: Theme.borderDefault }

                Text {
                    text: "Expected Revision: " + root.currentRevision + " → " + (root.currentRevision + 1)
                    color: Theme.textStrong
                    font.pixelSize: Theme.typeLabel
                    font.bold: true
                }

                GridLayout {
                    columns: 3
                    columnSpacing: Spacing.size16
                    rowSpacing: Spacing.size4
                    Layout.fillWidth: true

                    Text { text: "FIELD"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                    Text { text: "PREVIOUS"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }
                    Text { text: "NEW"; color: Theme.textMuted; font.pixelSize: Theme.typeLabelSmall; font.bold: true }

                    Text { text: "Fast Period"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origFastPeriod; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramFastPeriod; color: root.origFastPeriod !== root.paramFastPeriod ? Theme.accent : Theme.textStrong; font.bold: root.origFastPeriod !== root.paramFastPeriod; font.pixelSize: Theme.typeLabel }

                    Text { text: "Slow Period"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origSlowPeriod; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramSlowPeriod; color: root.origSlowPeriod !== root.paramSlowPeriod ? Theme.accent : Theme.textStrong; font.bold: root.origSlowPeriod !== root.paramSlowPeriod; font.pixelSize: Theme.typeLabel }

                    Text { text: "Stop Loss %"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origStopLossPct; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramStopLossPct; color: root.origStopLossPct !== root.paramStopLossPct ? Theme.accent : Theme.textStrong; font.bold: root.origStopLossPct !== root.paramStopLossPct; font.pixelSize: Theme.typeLabel }

                    Text { text: "Quantity"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origQuantity; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramQuantity; color: root.origQuantity !== root.paramQuantity ? Theme.accent : Theme.textStrong; font.bold: root.origQuantity !== root.paramQuantity; font.pixelSize: Theme.typeLabel }

                    Text { text: "Max Pos Size"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origMaxPositionSize; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramMaxPositionSize; color: root.origMaxPositionSize !== root.paramMaxPositionSize ? Theme.accent : Theme.textStrong; font.bold: root.origMaxPositionSize !== root.paramMaxPositionSize; font.pixelSize: Theme.typeLabel }

                    Text { text: "Point Value"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origPointValue; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramPointValue; color: root.origPointValue !== root.paramPointValue ? Theme.accent : Theme.textStrong; font.bold: root.origPointValue !== root.paramPointValue; font.pixelSize: Theme.typeLabel }

                    Text { text: "Slippage"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origSlippage; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramSlippage; color: root.origSlippage !== root.paramSlippage ? Theme.accent : Theme.textStrong; font.bold: root.origSlippage !== root.paramSlippage; font.pixelSize: Theme.typeLabel }

                    Text { text: "Commission"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origCommission; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramCommission; color: root.origCommission !== root.paramCommission ? Theme.accent : Theme.textStrong; font.bold: root.origCommission !== root.paramCommission; font.pixelSize: Theme.typeLabel }
                }

                Item { Layout.fillHeight: true }

                Text {
                    text: "Resume evaluates only newly completed bars. 409 conflict refuses stale revisions."
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                }
            }
        }

        // ==================== DIALOG ACTIONS ====================
        RowLayout {
            Layout.fillWidth: true
            spacing: Spacing.size8

            Item { Layout.fillWidth: true }

            Button {
                visible: root.hasConflict
                text: "Refresh from server"
                onClicked: root.loadDetail()
            }

            Button {
                text: root.inReview ? "Back" : "Cancel"
                onClicked: {
                    if (root.inReview) {
                        root.inReview = false;
                    } else {
                        root.reject();
                    }
                }
            }

            Button {
                visible: !root.inReview && !root.hasConflict
                text: "Review Changes"
                onClicked: root.inReview = true
            }

            Button {
                visible: root.inReview && !root.hasConflict
                text: "Submit Replacement"
                onClicked: {
                    var fast = parseInt(root.paramFastPeriod, 10) || 10;
                    var slow = parseInt(root.paramSlowPeriod, 10) || 30;
                    var stopLoss = parseFloat(root.paramStopLossPct) || 1.5;

                    var payload = {
                        deployment_id: root.deploymentId,
                        expected_revision: root.currentRevision,
                        configuration: {
                            strategy_params: {
                                fast_period: fast,
                                slow_period: slow
                            },
                            exit_params: {
                                stop_loss_pct: stopLoss
                            },
                            sizing_config: {
                                quantity: root.paramQuantity
                            },
                            risk_config: {
                                max_position_size: root.paramMaxPositionSize,
                                max_daily_drawdown: root.paramMaxDailyDrawdown
                            },
                            paper_cost_config: {
                                point_value: root.paramPointValue,
                                slippage: root.paramSlippage,
                                commission: root.paramCommission
                            }
                        }
                    };

                    var actId = root.executionControls.request("edit_deployment", JSON.stringify(payload));
                    if (actId !== "") {
                        root.actionId = actId;
                        root.inReview = false;
                    }
                }
            }
        }
    }
}
