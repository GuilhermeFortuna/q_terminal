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
    property var strategyParams: ({})
    property var originalStrategyParams: ({})
    property var exitParams: ({})
    property var originalSizingConfig: ({})
    property var originalRiskConfig: ({})
    property var originalPaperCostConfig: ({})

    function setStrategyParam(name, value) {
        var next = Object.assign({}, root.strategyParams);
        next[name] = value;
        root.strategyParams = next;
    }

    function submittedStrategyParams() {
        var values = {};
        var names = Object.keys(root.strategyParams);
        for (var i = 0; i < names.length; ++i) {
            var name = names[i];
            var raw = root.strategyParams[name];
            var old = root.originalStrategyParams[name];
            if (typeof old === "number") {
                var number = Number(raw);
                if (raw === "" || !Number.isFinite(number)
                        || Number.isInteger(old) && !Number.isInteger(number)) {
                    root.submittedMessage = name + " needs a valid number";
                    return null;
                }
                values[name] = number;
            } else {
                values[name] = raw;
            }
        }
        return values;
    }

    // Editable parameter fields
    property string paramQuantity: "1"
    property string paramMaxNotional: "500000"
    property string paramMaxDailyDrawdown: "500.00"
    property string paramPointValue: "0.2"
    property string paramSlippage: "0.5"
    property string paramCommission: "1.0"

    // Baseline original values to detect changes
    property string origQuantity: "1"
    property string origMaxNotional: "500000"
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
                root.originalStrategyParams = Object.assign({}, sparams);
                root.strategyParams = Object.assign({}, sparams);
                root.exitParams = Object.assign({}, cfg.exit_params || {});

                // Sizing & Risk
                var scfg = d.sizing_config || {};
                root.originalSizingConfig = Object.assign({}, scfg);
                if (scfg.quantity !== undefined) {
                    root.paramQuantity = "" + scfg.quantity;
                    root.origQuantity = root.paramQuantity;
                }
                var rcfg = d.risk_config || {};
                root.originalRiskConfig = Object.assign({}, rcfg);
                if (rcfg.max_notional !== undefined) {
                    root.paramMaxNotional = "" + rcfg.max_notional;
                    root.origMaxNotional = root.paramMaxNotional;
                }
                if (rcfg.max_daily_loss !== undefined) {
                    root.paramMaxDailyDrawdown = "" + rcfg.max_daily_loss;
                    root.origMaxDailyDrawdown = root.paramMaxDailyDrawdown;
                }

                // Paper costs
                var pcfg = d.paper_cost_config || {};
                root.originalPaperCostConfig = Object.assign({}, pcfg);
                if (pcfg.point_value !== undefined) {
                    root.paramPointValue = "" + pcfg.point_value;
                    root.origPointValue = root.paramPointValue;
                }
                if (pcfg.slippage_points !== undefined) {
                    root.paramSlippage = "" + pcfg.slippage_points;
                    root.origSlippage = root.paramSlippage;
                }
                if (pcfg.cost_per_contract !== undefined) {
                    root.paramCommission = "" + pcfg.cost_per_contract;
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
                        Text { text: "Max Notional (BRL)"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramMaxNotional
                            onTextChanged: root.paramMaxNotional = text.trim()
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

                Repeater {
                    model: Object.keys(root.strategyParams)
                    delegate: ColumnLayout {
                        required property string modelData
                        Layout.fillWidth: true
                        Text { text: parent.modelData; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: "" + root.strategyParams[parent.modelData]
                            onTextEdited: root.setStrategyParam(parent.modelData, text.trim())
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

                    Text { text: "Parameters"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: JSON.stringify(root.originalStrategyParams); color: Theme.textMuted; font.pixelSize: Theme.typeLabel; wrapMode: Text.WordWrap }
                    Text { text: JSON.stringify(root.strategyParams); color: Theme.textStrong; font.pixelSize: Theme.typeLabel; wrapMode: Text.WordWrap }

                    Text { text: "Quantity"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origQuantity; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramQuantity; color: root.origQuantity !== root.paramQuantity ? Theme.accent : Theme.textStrong; font.bold: root.origQuantity !== root.paramQuantity; font.pixelSize: Theme.typeLabel }

                    Text { text: "Max Notional"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }
                    Text { text: root.origMaxNotional; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.paramMaxNotional; color: root.origMaxNotional !== root.paramMaxNotional ? Theme.accent : Theme.textStrong; font.bold: root.origMaxNotional !== root.paramMaxNotional; font.pixelSize: Theme.typeLabel }

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
                    var strategyParams = root.submittedStrategyParams();
                    if (strategyParams === null) return;

                    var payload = {
                        deployment_id: root.deploymentId,
                        expected_revision: root.currentRevision,
                        configuration: {
                            strategy_params: strategyParams,
                            exit_params: root.exitParams,
                            sizing_config: Object.assign({}, root.originalSizingConfig, { quantity: root.paramQuantity }),
                            risk_config: Object.assign({}, root.originalRiskConfig, {
                                max_notional: root.paramMaxNotional,
                                max_daily_loss: root.paramMaxDailyDrawdown
                            }),
                            paper_cost_config: Object.assign({}, root.originalPaperCostConfig, {
                                point_value: root.paramPointValue,
                                slippage_points: root.paramSlippage,
                                cost_per_contract: root.paramCommission
                            })
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
