pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml
import "Format.js" as Format

AppDialog {
    id: root
    title: inReview ? "Review deployment" : "New deployment"
    modal: true
    standardButtons: Dialog.NoButton
    width: Spacing.size480
    anchors.centerIn: parent

    required property ExecutionControls executionControls
    required property ExecutionModels executionModels
    property string accountId: ""
    property string actionId: ""

    signal deployRequested(var payload)
    signal liveDeployConfirmed(var payload)

    property var savedRuns: []
    property var catalogStrategies: []
    property var symbolResults: []
    property int selectedStrategyIndex: 0
    property bool isCatalogMode: true
    property bool inReview: false
    property var parameterValues: ({})
    property string formError: ""

    // Form fields for catalog mode
    property string chosenStrategyName: ""
    property string chosenStrategyType: "candle"
    property string chosenProvenance: "builtin"
    property string chosenSymbol: "WIN$N"
    property string chosenTimeframe: "M15"
    property string paramQuantity: "1"
    property string paramMaxNotional: "500000"
    property string paramMaxDailyDrawdown: "500.00"
    property string paramPointValue: "0.2"
    property string paramSlippage: "0.5"
    property string paramCommission: "1.0"

    function refreshAll() {
        root.executionControls.fetch_strategy_catalog();
        root.executionControls.fetch_saved_runs();
    }

    onOpened: {
        root.accountId = root.executionModels.selected_account_id;
        root.inReview = false;
        root.formError = "";
        root.refreshAll();
    }

    function parameterValue(spec) {
        var value = root.parameterValues[spec.name];
        return value === undefined ? "" + spec.default : "" + value;
    }

    function setParameter(name, value) {
        var next = Object.assign({}, root.parameterValues);
        next[name] = value;
        root.parameterValues = next;
    }

    function typedParameters() {
        var strategy = {};
        var exits = {};
        var specs = root.catalogStrategies[root.selectedStrategyIndex].params || [];
        for (var i = 0; i < specs.length; ++i) {
            var spec = specs[i];
            var raw = root.parameterValue(spec);
            var value = raw;
            if (spec.type === "int") value = Number(raw);
            if (spec.type === "float") value = Number(raw);
            if (raw.trim() === "" || (spec.type !== "categorical" && !Number.isFinite(value))) {
                root.formError = (spec.label || spec.name) + " needs a valid value";
                return null;
            }
            if (spec.min !== null && spec.min !== undefined && value < spec.min
                    || spec.max !== null && spec.max !== undefined && value > spec.max
                    || spec.type === "int" && !Number.isInteger(value)
                    || spec.choices && spec.choices.indexOf(raw) < 0) {
                root.formError = (spec.label || spec.name) + " is outside its allowed range";
                return null;
            }
            if (spec.exit_group) exits[spec.name] = value;
            else strategy[spec.name] = value;
        }
        root.formError = "";
        return { strategy_params: strategy, exit_params: exits };
    }

    Component.onCompleted: refreshAll()

    Connections {
        target: root.executionControls
        function onSaved_runs_jsonChanged() {
            try {
                root.savedRuns = JSON.parse(root.executionControls.saved_runs_json);
            } catch (e) {
                root.savedRuns = [];
            }
        }
        function onStrategy_catalog_jsonChanged() {
            try {
                var raw = JSON.parse(root.executionControls.strategy_catalog_json);
                var list = raw.strategies || [];
                // Only allow candle strategies; tick/genome entries cannot be selected
                var filtered = [];
                for (var i = 0; i < list.length; ++i) {
                    var s = list[i];
                    if (s.strategy_type !== "tick" && s.strategy_type !== "genome") {
                        filtered.push(s);
                    }
                }
                root.catalogStrategies = filtered;
                if (filtered.length > 0) {
                    root.selectStrategy(0);
                }
            } catch (e) {
                root.catalogStrategies = [];
            }
        }
        function onSymbol_search_results_jsonChanged() {
            try {
                root.symbolResults = JSON.parse(root.executionControls.symbol_search_results_json);
            } catch (e) {
                root.symbolResults = [];
            }
        }
    }

    function selectStrategy(index) {
        if (index < 0 || index >= root.catalogStrategies.length) return;
        root.selectedStrategyIndex = index;
        var s = root.catalogStrategies[index];
        root.chosenStrategyName = s.name || "";
        root.chosenStrategyType = s.strategy_type || "candle";
        root.chosenProvenance = s.source_kind || s.provenance || "builtin";
        if (s.default_symbol) root.chosenSymbol = s.default_symbol;
        if (s.default_timeframe) root.chosenTimeframe = s.default_timeframe;
        if (nameField.text === "" || nameField.text.indexOf(" ") > 0) {
            nameField.text = (s.label || s.name || "Strategy") + " " + root.chosenSymbol;
        }

        // Apply typed defaults from the selected catalog entry.
        var params = s.params || s.parameters || [];
        var defaults = {};
        for (var p = 0; p < params.length; ++p) {
            var item = params[p];
            defaults[item.name] = "" + item.default;
        }
        root.parameterValues = defaults;
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size8

        // Source Switcher (Strategy Catalog vs Saved Run)
        RowLayout {
            Layout.fillWidth: true
            visible: !root.inReview
            spacing: Spacing.size4

            Rectangle {
                id: tabCatalog
                height: Spacing.size28
                Layout.fillWidth: true
                radius: Spacing.size4
                color: root.isCatalogMode ? Theme.surfaceSelected : Theme.surfaceOverlay
                border.color: root.isCatalogMode ? Theme.accent : Theme.borderDefault

                Text {
                    anchors.centerIn: parent
                    text: "Strategy Catalog"
                    color: root.isCatalogMode ? Theme.accent : Theme.textSecondary
                    font.pixelSize: Theme.typeLabel
                    font.bold: true
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.isCatalogMode = true
                }
            }

            Rectangle {
                id: tabSaved
                height: Spacing.size28
                Layout.fillWidth: true
                radius: Spacing.size4
                color: !root.isCatalogMode ? Theme.surfaceSelected : Theme.surfaceOverlay
                border.color: !root.isCatalogMode ? Theme.accent : Theme.borderDefault

                Text {
                    anchors.centerIn: parent
                    text: "Saved Run"
                    color: !root.isCatalogMode ? Theme.accent : Theme.textSecondary
                    font.pixelSize: Theme.typeLabel
                    font.bold: true
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.isCatalogMode = false
                }
            }
        }

        // Deployment Name
        TextField {
            id: nameField
            Layout.fillWidth: true
            visible: !root.inReview
            placeholderText: "Deployment name"
        }

        ComboBox {
            id: accountCombo
            Layout.fillWidth: true
            visible: !root.inReview
            model: root.executionModels.accounts
            textRole: "name"
            valueRole: "id"
            currentIndex: indexOfValue(root.accountId)
            onActivated: root.accountId = currentValue
        }

        Text {
            Layout.fillWidth: true
            visible: root.formError !== ""
            text: root.formError
            color: Theme.negativeSoft
            font.pixelSize: Theme.typeLabel
            wrapMode: Text.WordWrap
        }

        // ==================== CATALOG FIRST WORKFLOW ====================
        ScrollView {
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size320
            clip: true
            visible: root.isCatalogMode && !root.inReview
            contentWidth: availableWidth

            ColumnLayout {
                width: parent.width
                spacing: Spacing.size10

                // 1. Strategy Identity & Provenance
                Text {
                    text: "STRATEGY IDENTITY"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                ComboBox {
                    id: strategyCombo
                    Layout.fillWidth: true
                    model: {
                        var names = [];
                        for (var i = 0; i < root.catalogStrategies.length; ++i) {
                            var s = root.catalogStrategies[i];
                            names.push((s.label || s.name) + " (" + (s.source_kind || s.provenance || "builtin") + ")");
                        }
                        return names;
                    }
                    currentIndex: root.selectedStrategyIndex
                    onActivated: function(index) {
                        root.selectStrategy(index);
                    }
                }

                Text {
                    property var curr: (root.catalogStrategies.length > root.selectedStrategyIndex) ? root.catalogStrategies[root.selectedStrategyIndex] : null
                    text: curr ? (curr.description || "Candle strategy") : ""
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabel
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                // 2. Exact Symbol & Timeframe
                Text {
                    text: "MARKET DISCOVERY"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size8

                    TextField {
                        id: symbolField
                        Layout.fillWidth: true
                        placeholderText: "Exact Symbol (e.g. WIN$N)"
                        text: root.chosenSymbol
                        onTextChanged: root.chosenSymbol = text.trim()
                    }

                    Rectangle {
                        height: Spacing.size28
                        implicitWidth: searchSymbolText.implicitWidth + 16
                        radius: Spacing.size4
                        color: Theme.surfaceElevated
                        border.color: Theme.borderDefault

                        Text {
                            id: searchSymbolText
                            anchors.centerIn: parent
                            text: "Search"
                            color: Theme.accent
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: {
                                var query = symbolField.text.trim();
                                if (query !== "") root.executionControls.search_symbols(query);
                            }
                        }
                    }

                    ComboBox {
                        id: timeframeCombo
                        Layout.preferredWidth: Spacing.size90
                        model: ["M15", "H1", "D1"]
                        currentIndex: 0
                        onActivated: function(index) {
                            root.chosenTimeframe = currentText;
                        }
                    }
                }

                // Symbol Search Results popup list if any
                ListView {
                    id: symbolResultsList
                    Layout.fillWidth: true
                    Layout.preferredHeight: root.symbolResults.length > 0 ? Spacing.size72 : Spacing.none
                    visible: root.symbolResults.length > 0
                    clip: true
                    model: root.symbolResults

                    delegate: Rectangle {
                        required property var modelData
                        width: symbolResultsList.width
                        height: Spacing.size24
                        color: symbolMouse.containsMouse ? Theme.surfaceSelected : Theme.surfaceOverlay
                        border.color: Theme.borderDefault

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Spacing.size8
                            anchors.rightMargin: Spacing.size8
                            Text {
                                text: modelData.symbol
                                color: Theme.accent
                                font.bold: true
                                font.pixelSize: Theme.typeBodySmall
                            }
                            Text {
                                text: modelData.name || ""
                                color: Theme.textMuted
                                font.pixelSize: Theme.typeLabel
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }

                        MouseArea {
                            id: symbolMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                root.chosenSymbol = modelData.symbol;
                                symbolField.text = modelData.symbol;
                                root.symbolResults = [];
                            }
                        }
                    }
                }

                // 3. Sizing & Risk (MuseScore inspector grouping)
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
                        Text { text: "Quantity (Contracts)"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
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

                // 4. Paper Costs
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
                        Text { text: "Slippage (Points)"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramSlippage
                            onTextChanged: root.paramSlippage = text.trim()
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Spacing.size2
                        Text { text: "Commission / Order"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabelSmall }
                        TextField {
                            Layout.fillWidth: true
                            text: root.paramCommission
                            onTextChanged: root.paramCommission = text.trim()
                        }
                    }
                }

                // 5. Strategy Parameters
                Text {
                    text: "STRATEGY PARAMETERS"
                    color: Theme.textMuted
                    font.pixelSize: Theme.typeLabelSmall
                    font.bold: true
                }

                Repeater {
                    model: root.catalogStrategies.length > root.selectedStrategyIndex
                           ? (root.catalogStrategies[root.selectedStrategyIndex].params || []) : []
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        Text {
                            text: parent.modelData.label || parent.modelData.name
                            color: Theme.textSecondary
                            font.pixelSize: Theme.typeLabelSmall
                        }
                        TextField {
                            Layout.fillWidth: true
                            visible: parent.modelData.type !== "categorical"
                            text: root.parameterValue(parent.modelData)
                            onTextEdited: root.setParameter(parent.modelData.name, text.trim())
                        }
                        ComboBox {
                            Layout.fillWidth: true
                            visible: parent.modelData.type === "categorical"
                            model: parent.modelData.choices || []
                            currentIndex: (parent.modelData.choices || []).indexOf(root.parameterValue(parent.modelData))
                            onActivated: root.setParameter(parent.modelData.name, currentText)
                        }
                    }
                }

                // 6. Broker Mode: Paper Only
                Rectangle {
                    Layout.fillWidth: true
                    height: Spacing.size32
                    radius: Spacing.size4
                    color: Theme.surfaceOverlay
                    border.color: Theme.borderDefault

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Spacing.size10
                        anchors.rightMargin: Spacing.size10
                        Text {
                            text: "MODE:"
                            color: Theme.textMuted
                            font.pixelSize: Theme.typeLabelSmall
                            font.bold: true
                        }
                        Text {
                            text: "PAPER (dev profile — mt5_live locked)"
                            color: Theme.accent
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }
                    }
                }
            }
        }

        // ==================== SAVED RUN SECONDARY SOURCE ====================
        ColumnLayout {
            Layout.fillWidth: true
            visible: !root.isCatalogMode && !root.inReview
            spacing: Spacing.size8

            Label { text: "Saved backtest run"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }

            ListView {
                id: runList
                Layout.fillWidth: true
                Layout.preferredHeight: Spacing.size160
                clip: true
                model: root.savedRuns
                currentIndex: -1

                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: runList.width
                    height: Spacing.size44
                    color: runList.currentIndex === index ? Theme.surfaceSelected : (savedMouseArea.containsMouse ? Theme.surfaceOverlay : Theme.surfaceBase)
                    border.color: runList.currentIndex === index ? Theme.accent : Theme.borderDefault

                    MouseArea {
                        id: savedMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: runList.currentIndex = index
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Spacing.size8
                        Text {
                            text: (modelData.strategy_name || "?") + " · " + (modelData.symbol || "?") + " " + (modelData.timeframe || "?")
                            color: Theme.textStrong
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: true
                        }
                        Text {
                            text: modelData.saved_at ? Format.formatIsoTime(modelData.saved_at) : ""
                            color: Theme.textMuted
                            font.pixelSize: Theme.typeLabel
                        }
                    }
                }
            }
        }

        // ==================== REVIEW SUMMARY STEP ====================
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
                    text: "REVIEW DEPLOYMENT CONFIGURATION"
                    color: Theme.accent
                    font.pixelSize: Theme.typeBodySmall
                    font.bold: true
                }

                Rectangle { Layout.fillWidth: true; height: Spacing.size1; color: Theme.borderDefault }

                GridLayout {
                    columns: 2
                    columnSpacing: Spacing.size16
                    rowSpacing: Spacing.size6
                    Layout.fillWidth: true

                    Text { text: "Strategy:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.chosenStrategyName + " (" + root.chosenProvenance + ")"; color: Theme.textStrong; font.bold: true; font.pixelSize: Theme.typeLabel }

                    Text { text: "Symbol & TF:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.chosenSymbol + " · " + root.chosenTimeframe; color: Theme.textStrong; font.bold: true; font.pixelSize: Theme.typeLabel }

                    Text { text: "Parameters:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: JSON.stringify(root.parameterValues); color: Theme.textStrong; font.pixelSize: Theme.typeLabel; wrapMode: Text.WordWrap; Layout.fillWidth: true }

                    Text { text: "Sizing & Risk:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: "Qty=" + root.paramQuantity + ", MaxNotional=" + root.paramMaxNotional + ", MaxLoss=" + root.paramMaxDailyDrawdown; color: Theme.textStrong; font.pixelSize: Theme.typeLabel }

                    Text { text: "Costs:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: "PtVal=" + root.paramPointValue + ", Slip=" + root.paramSlippage + ", Comm=" + root.paramCommission; color: Theme.textStrong; font.pixelSize: Theme.typeLabel }

                    Text { text: "Account ID:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: root.accountId !== "" ? root.accountId : "Selected paper account"; color: Theme.textStrong; font.pixelSize: Theme.typeLabel }

                    Text { text: "Broker Mode:"; color: Theme.textMuted; font.pixelSize: Theme.typeLabel }
                    Text { text: "PAPER (Paper only in dev profile)"; color: Theme.accent; font.bold: true; font.pixelSize: Theme.typeLabel }
                }

                Item { Layout.fillHeight: true }

                Text {
                    text: "Authoritative validation performed by backend. Stream event will settle row."
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
                text: root.inReview ? "Back" : "Cancel"
                onClicked: {
                    if (root.inReview) {
                        root.inReview = false;
                    } else {
                        root.reject();
                    }
                }
            }

            // Catalog Mode: Review button then Deploy
            Button {
                visible: root.isCatalogMode && !root.inReview
                text: "Review"
                enabled: nameField.text.trim() !== "" && root.chosenSymbol !== "" && root.chosenTimeframe !== "" && root.accountId !== "" && root.catalogStrategies.length > 0
                onClicked: {
                    if (root.typedParameters() !== null) root.inReview = true;
                }
            }

            Button {
                visible: root.isCatalogMode && root.inReview
                text: "Deploy Paper Strategy"
                onClicked: {
                    var params = root.typedParameters();
                    if (params === null) return;

                    var payload = {
                        paper_account_id: root.accountId,
                        name: nameField.text.trim(),
                        broker_mode: "paper",
                        catalog: {
                            strategy_name: root.chosenStrategyName,
                            strategy_params: params.strategy_params,
                            exit_params: params.exit_params,
                            symbol: root.chosenSymbol,
                            timeframe: root.chosenTimeframe,
                            sizing_config: {
                                quantity: root.paramQuantity
                            },
                            risk_config: {
                                max_notional: root.paramMaxNotional,
                                max_daily_loss: root.paramMaxDailyDrawdown
                            },
                            paper_cost_config: {
                                point_value: root.paramPointValue,
                                slippage_points: root.paramSlippage,
                                cost_per_contract: root.paramCommission,
                                cost_bps: "0"
                            }
                        }
                    };

                    // For dev profile, brokerMode is paper; live Deploy is checked for safety
                    var brokerMode = "paper";
                    if (brokerMode === "mt5_live") {
                        liveDeployConfirmed(payload);
                    } else {
                        deployRequested(payload);
                        root.accept();
                    }
                }
            }

            // Saved Run Mode: Deploy button
            Button {
                visible: !root.isCatalogMode && !root.inReview
                text: "Deploy"
                enabled: nameField.text.trim() !== "" && runList.currentIndex >= 0 && root.accountId !== ""
                onClicked: {
                    var run = root.savedRuns[runList.currentIndex];
                    var payload = {
                        paper_account_id: root.accountId,
                        name: nameField.text.trim(),
                        broker_mode: "paper",
                        source_backtest_run_id: run.id
                    };
                    var brokerMode = "paper";
                    if (brokerMode === "mt5_live") {
                        liveDeployConfirmed(payload);
                    } else {
                        deployRequested(payload);
                        root.accept();
                    }
                }
            }
        }
    }
}
