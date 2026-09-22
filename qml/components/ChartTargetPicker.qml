pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Keyboard-accessible chart target controls (Q-056): symbol direct entry,
// suggestions (configured, deployments, recents), timeframe choice, explicit apply,
// and follow-deployment restoration.
Rectangle {
    id: root

    property var context: null
    property var executionModels: null
    property string previewState: "rest"

    signal targetApplied(string symbol, string timeframe)
    signal followRequested()

    implicitHeight: Theme.controlHeight
    implicitWidth: controlsRow.implicitWidth + Spacing.size16
    color: Theme.transparent

    // Preview state model for gallery and headless testing
    QtObject {
        id: preview
        property string active_symbol: "PETR4"
        property string active_timeframe: "1m"
        property string chart_mode: "following"
        property bool is_manual: false
        property bool is_diverged: false
        property string target_error: ""
        property string configured_symbol: "PETR4"
        property string configured_timeframe: "1m"
        property string recent_symbols_json: "[\"PETR4\", \"VALE3\"]"
        property bool openSuggestions: false
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "open":
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            preview.target_error = "";
            preview.openSuggestions = true;
            break;
        case "manual":
            preview.active_symbol = "VALE3";
            preview.active_timeframe = "5m";
            preview.chart_mode = "manual";
            preview.is_manual = true;
            preview.target_error = "";
            preview.openSuggestions = false;
            break;
        case "following":
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            preview.target_error = "";
            preview.openSuggestions = false;
            break;
        case "invalid":
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            preview.target_error = "Invalid chart target 'BAD SYM · 1m': invalid symbol characters";
            preview.openSuggestions = false;
            break;
        case "no-data":
            preview.active_symbol = "UNKNOWN";
            preview.active_timeframe = "1m";
            preview.chart_mode = "manual";
            preview.is_manual = true;
            preview.target_error = "";
            preview.openSuggestions = false;
            break;
        default:
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            preview.target_error = "";
            preview.openSuggestions = false;
            break;
        }
        if (root.previewState !== "rest") {
            symbolInput.text = preview.active_symbol;
            var idx = timeframeBox.find(preview.active_timeframe);
            if (idx >= 0) timeframeBox.currentIndex = idx;
            if (preview.openSuggestions) {
                suggestionsPopup.open();
            } else {
                suggestionsPopup.close();
            }
        }
    }

    Component.onCompleted: applyPreviewState()
    onPreviewStateChanged: applyPreviewState()

    readonly property var effectiveContext: root.previewState !== "rest" ? preview : (root.context ? root.context : preview)
    readonly property string activeSymbol: effectiveContext ? effectiveContext.active_symbol : ""
    readonly property string activeTimeframe: effectiveContext ? effectiveContext.active_timeframe : "1m"
    readonly property bool isManual: effectiveContext ? effectiveContext.is_manual : false
    readonly property string targetError: effectiveContext ? effectiveContext.target_error : ""
    readonly property string configuredSymbol: effectiveContext ? effectiveContext.configured_symbol : ""
    readonly property var recentSymbols: {
        try {
            return JSON.parse(effectiveContext ? effectiveContext.recent_symbols_json : "[]");
        } catch (e) {
            return [];
        }
    }
    readonly property var deploymentSymbols: {
        try {
            return root.executionModels ? JSON.parse(root.executionModels.deployment_symbols_json()) : [];
        } catch (e) {
            return [];
        }
    }

    onActiveSymbolChanged: {
        if (!symbolInput.activeFocus && root.previewState === "rest") {
            symbolInput.text = root.activeSymbol;
        }
    }

    onActiveTimeframeChanged: {
        if (root.previewState === "rest") {
            var idx = timeframeBox.find(root.activeTimeframe);
            if (idx >= 0) {
                timeframeBox.currentIndex = idx;
            }
        }
    }

    function getSuggestions(query) {
        var q = (query || "").trim().toUpperCase();
        var list = [];
        var seen = {};

        function add(sym, category) {
            if (!sym) return;
            var s = sym.trim().toUpperCase();
            if (s.length > 0 && !seen[s]) {
                seen[s] = true;
                if (q.length === 0 || s.indexOf(q) >= 0) {
                    list.push({ symbol: s, category: category });
                }
            }
        }

        if (root.configuredSymbol) {
            add(root.configuredSymbol, "Configured");
        }
        for (var i = 0; i < root.deploymentSymbols.length; ++i) {
            add(root.deploymentSymbols[i], "Deployment");
        }
        for (var j = 0; j < root.recentSymbols.length; ++j) {
            add(root.recentSymbols[j], "Recent");
        }
        return list;
    }

    function applyTarget() {
        var sym = symbolInput.text.trim().toUpperCase();
        var tf = timeframeBox.currentText;
        suggestionsPopup.close();
        if (root.context) {
            var ok = root.context.request_target(sym, tf);
            if (ok) {
                root.targetApplied(sym, tf);
            }
        } else {
            root.targetApplied(sym, tf);
        }
    }

    function focusSymbol() {
        symbolInput.forceActiveFocus();
        symbolInput.selectAll();
    }

    RowLayout {
        id: controlsRow
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        spacing: Spacing.size4

        AppTextField {
            id: symbolInput
            objectName: "chartTargetSymbolInput"
            Accessible.name: "Symbol input"
            Accessible.description: "Direct symbol input for manual chart target"
            placeholderText: root.activeSymbol !== "" ? root.activeSymbol : "Symbol"
            text: root.activeSymbol
            Layout.preferredWidth: Spacing.size80
            Layout.preferredHeight: Theme.controlHeight

            onTextEdited: {
                if (root.context && root.context.target_error !== "") {
                    root.context.clear_error();
                }
                suggestionsModel.clear();
                var items = root.getSuggestions(symbolInput.text);
                for (var i = 0; i < items.length; ++i) {
                    suggestionsModel.append(items[i]);
                }
                if (items.length > 0 && !suggestionsPopup.opened) {
                    suggestionsPopup.open();
                }
            }

            onActiveFocusChanged: {
                if (activeFocus) {
                    suggestionsModel.clear();
                    var items = root.getSuggestions(symbolInput.text);
                    for (var i = 0; i < items.length; ++i) {
                        suggestionsModel.append(items[i]);
                    }
                    if (items.length > 0) {
                        suggestionsPopup.open();
                    }
                }
            }

            Keys.onReturnPressed: root.applyTarget()
            Keys.onEnterPressed: root.applyTarget()
            Keys.onDownPressed: {
                if (suggestionsPopup.opened && suggestionsList.count > 0) {
                    suggestionsList.forceActiveFocus();
                    suggestionsList.currentIndex = 0;
                }
            }
            Keys.onEscapePressed: {
                suggestionsPopup.close();
            }

            Popup {
                id: suggestionsPopup
                objectName: "chartTargetSuggestionsPopup"
                y: symbolInput.height + Spacing.size2
                width: Spacing.size200
                implicitHeight: Math.min(Spacing.size160, suggestionsContent.implicitHeight + Spacing.size12)
                padding: Spacing.size4
                closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

                background: Rectangle {
                    color: Theme.surfaceElevated
                    border.color: Theme.borderDefault
                    border.width: Theme.borderWidth
                    radius: Theme.radiusMedium
                }

                ColumnLayout {
                    id: suggestionsContent
                    anchors.fill: parent
                    spacing: Spacing.size2

                    Text {
                        text: "SUGGESTIONS"
                        color: Theme.textMuted
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeLabelSmall
                        Layout.leftMargin: Spacing.size4
                        Layout.topMargin: Spacing.size2
                    }

                    ListView {
                        id: suggestionsList
                        objectName: "chartTargetSuggestionsList"
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: ListModel { id: suggestionsModel }
                        delegate: Rectangle {
                            id: suggestionItem
                            required property string symbol
                            required property string category
                            required property int index

                            width: suggestionsList.width
                            implicitHeight: Spacing.size24
                            radius: Theme.radiusSmall
                            color: suggestionMouse.containsMouse || suggestionsList.currentIndex === suggestionItem.index ? Theme.surfaceSelected : Theme.transparent

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Spacing.size6
                                anchors.rightMargin: Spacing.size6
                                spacing: Spacing.size4

                                Text {
                                    text: suggestionItem.symbol
                                    color: Theme.textPrimary
                                    font.family: Theme.uiFont
                                    font.pixelSize: Theme.typeBodySmall
                                    font.weight: Typography.weightMedium
                                    Layout.fillWidth: true
                                }

                                Text {
                                    text: suggestionItem.category
                                    color: Theme.textMuted
                                    font.family: Theme.uiFont
                                    font.pixelSize: Theme.typeLabelSmall
                                }
                            }

                            MouseArea {
                                id: suggestionMouse
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: {
                                    symbolInput.text = suggestionItem.symbol;
                                    suggestionsPopup.close();
                                    root.applyTarget();
                                }
                            }
                        }

                        Keys.onReturnPressed: {
                            if (currentIndex >= 0 && currentIndex < suggestionsModel.count) {
                                symbolInput.text = suggestionsModel.get(currentIndex).symbol;
                                suggestionsPopup.close();
                                root.applyTarget();
                            }
                        }
                        Keys.onEscapePressed: {
                            suggestionsPopup.close();
                            symbolInput.forceActiveFocus();
                        }
                    }
                }
            }
        }

        AppComboBox {
            id: timeframeBox
            objectName: "chartTargetTimeframeBox"
            Accessible.name: "Timeframe selector"
            Accessible.description: "Timeframe resolution for chart data"
            Layout.preferredWidth: Spacing.size70
            Layout.preferredHeight: Theme.controlHeight
            model: ["1s", "5s", "1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"]
            currentIndex: 2 // default to 1m
        }

        AppButton {
            id: applyButton
            objectName: "chartTargetApplyButton"
            Accessible.name: "Apply chart target"
            text: "Apply"
            variant: "primary"
            Layout.preferredWidth: Spacing.size60
            Layout.preferredHeight: Theme.controlHeight
            onClicked: root.applyTarget()
        }

        AppButton {
            id: followButton
            objectName: "chartTargetFollowButton"
            Accessible.name: "Follow selected deployment"
            text: "Follow deployment"
            variant: "secondary"
            visible: root.isManual
            Layout.preferredWidth: Spacing.size120
            Layout.preferredHeight: Theme.controlHeight
            onClicked: {
                if (root.context) {
                    root.context.follow_deployment();
                }
                root.followRequested();
            }
        }

        // Inline recoverable error notice
        Rectangle {
            id: errorChip
            objectName: "chartTargetErrorChip"
            visible: root.targetError !== ""
            Layout.preferredHeight: Theme.controlHeight
            implicitWidth: errorRow.implicitWidth + Spacing.size12
            color: Theme.criticalSurface
            border.color: Theme.critical
            border.width: Theme.borderWidth
            radius: Theme.radiusSmall

            RowLayout {
                id: errorRow
                anchors.fill: parent
                anchors.leftMargin: Spacing.size6
                anchors.rightMargin: Spacing.size6
                spacing: Spacing.size4

                Text {
                    id: errorText
                    objectName: "chartTargetErrorText"
                    text: root.targetError
                    color: Theme.critical
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    Layout.maximumWidth: Spacing.size220
                    elide: Text.ElideRight
                    maximumLineCount: 1
                }

                Text {
                    text: "✕"
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            if (root.context) {
                                root.context.clear_error();
                            }
                        }
                    }
                }
            }
        }
    }
}
