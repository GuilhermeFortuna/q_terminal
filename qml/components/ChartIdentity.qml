pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Compact chart identity strip (Q-055): symbol, source, freshness and last bar time.
Rectangle {
    id: root

    property var context: null
    property var feed: null
    property var executionModels: null
    property string previewState: "rest"

    function focusSymbol() {
        picker.focusSymbol();
    }

    readonly property var effectiveContext: root.previewState !== "rest" ? preview : (root.context ? root.context : preview)
    readonly property string conditionRole: root.effectiveContext ? root.effectiveContext.condition_role : Semantic.neutral

    implicitHeight: Spacing.size60
    color: Theme.surfaceElevated

    QtObject {
        id: preview
        property string symbol_line: "PETR4 · 1m · Candles"
        property string source_label: "Configured"
        property string source_tooltip: ""
        property string condition_label: "Live"
        property string condition_role: Semantic.positive
        property string last_bar_label: "2026-09-18 15:30:00 UTC"
        property bool is_switching: false
        property string active_symbol: "PETR4"
        property string active_timeframe: "1m"
        property string chart_mode: "following"
        property bool is_manual: false
        property bool is_diverged: false
        property string target_error: ""
        property string configured_symbol: "PETR4"
        property string configured_timeframe: "1m"
        property string recent_symbols_json: "[\"PETR4\", \"VALE3\"]"
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "switching":
            preview.symbol_line = "Switching to VALE3 · 5m";
            preview.source_label = "Following: momentum-alpha";
            preview.source_tooltip = "momentum-alpha";
            preview.condition_label = "Loading";
            preview.condition_role = Semantic.warning;
            preview.last_bar_label = "Last bar unavailable";
            preview.is_switching = true;
            preview.active_symbol = "VALE3";
            preview.active_timeframe = "5m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            break;
        case "loading":
            preview.symbol_line = "PETR4 · 1m · Candles";
            preview.source_label = "Configured";
            preview.source_tooltip = "";
            preview.condition_label = "Loading";
            preview.condition_role = Semantic.warning;
            preview.last_bar_label = "Last bar unavailable";
            preview.is_switching = false;
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            break;
        case "stale":
            preview.symbol_line = "PETR4 · 1m · Candles";
            preview.source_label = "Following: momentum-alpha";
            preview.source_tooltip = "momentum-alpha";
            preview.condition_label = "Stale 2m 5s";
            preview.condition_role = Semantic.stale;
            preview.last_bar_label = "2026-09-18 15:28:00 UTC";
            preview.is_switching = false;
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            break;
        case "disconnected":
            preview.symbol_line = "PETR4 · 1m · Candles";
            preview.source_label = "Configured";
            preview.source_tooltip = "";
            preview.condition_label = "Disconnected";
            preview.condition_role = Semantic.critical;
            preview.last_bar_label = "2026-09-18 15:28:00 UTC";
            preview.is_switching = false;
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            break;
        default:
            preview.symbol_line = "PETR4 · 1m · Candles";
            preview.source_label = "Configured";
            preview.source_tooltip = "";
            preview.condition_label = "Live";
            preview.condition_role = Semantic.positive;
            preview.last_bar_label = "2026-09-18 15:30:00 UTC";
            preview.is_switching = false;
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.chart_mode = "following";
            preview.is_manual = false;
            break;
        }
    }

    Component.onCompleted: applyPreviewState()
    onPreviewStateChanged: applyPreviewState()

    Timer {
        id: freshnessTimer
        interval: 1000
        running: root.previewState === "rest" && root.feed && root.context
        repeat: true
        onTriggered: {
            if (root.context) {
                root.context.sync();
            }
        }
    }

    Connections {
        target: root.feed
        enabled: root.previewState === "rest" && root.feed && root.context
        function onRevisionChanged() {
            root.context.sync();
        }
        function onConnection_stateChanged() {
            root.context.sync();
        }
        function onHistory_loadingChanged() {
            root.context.sync();
        }
        function onHistory_errorChanged() {
            root.context.sync();
        }
        function onStaleChanged() {
            root.context.sync();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Spacing.size8
        anchors.rightMargin: Spacing.size8
        anchors.topMargin: Spacing.size4
        anchors.bottomMargin: Spacing.size4
        spacing: Spacing.size2

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.controlHeight
            spacing: Spacing.size8

            ChartTargetPicker {
                id: picker
                objectName: "chartTargetPicker"
                context: root.context
                executionModels: root.executionModels
                previewState: root.previewState
                Layout.alignment: Qt.AlignVCenter
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                id: sourceLabel
                objectName: "sourceLabelText"
                Layout.alignment: Qt.AlignVCenter
                text: root.effectiveContext ? root.effectiveContext.source_label : ""
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
                elide: Text.ElideRight
                maximumLineCount: 1
                ToolTip.visible: sourceMouse.containsMouse && root.effectiveContext && root.effectiveContext.source_tooltip !== ""
                ToolTip.text: root.effectiveContext ? root.effectiveContext.source_tooltip : ""

                MouseArea {
                    id: sourceMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.NoButton
                }
            }

            StatusBadge {
                id: conditionBadge
                objectName: "conditionBadge"
                Layout.alignment: Qt.AlignVCenter
                text: root.effectiveContext ? root.effectiveContext.condition_label : ""
                role: root.conditionRole
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size16
            spacing: Spacing.size8

            Text {
                id: symbolLine
                objectName: "symbolLineText"
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                text: root.effectiveContext ? root.effectiveContext.symbol_line : ""
                color: Theme.textPrimary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeBodySmall
                font.bold: true
                elide: Text.ElideRight
                maximumLineCount: 1
            }

            Text {
                id: lastBarLabel
                objectName: "lastBarLabelText"
                Layout.alignment: Qt.AlignVCenter
                text: root.effectiveContext ? ("Last bar: " + root.effectiveContext.last_bar_label) : ""
                color: Theme.textTertiary
                font.pixelSize: Theme.typeLabel
                font.family: Theme.numericFontFamily
                elide: Text.ElideRight
                maximumLineCount: 1
            }
        }
    }
}
