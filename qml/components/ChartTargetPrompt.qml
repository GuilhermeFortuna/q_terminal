pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Compact chart-focused target entry (Q-059): opens from canvas typing, shows a parsed
// preview, and submits through the existing manual target path.
Popup {
    id: root

    property var context: null
    property string previewState: "rest"

    signal submitted(string symbol, string timeframe)
    signal cancelled()

    modal: false
    padding: Spacing.size8
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent

    QtObject {
        id: preview
        property string active_symbol: "PETR4"
        property string active_timeframe: "1m"
        property string target_error: ""
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "open":
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.target_error = "";
            draftInput.text = "P";
            root.open();
            break;
        case "error":
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.target_error = "Invalid chart target 'BAD SYM · 1m': invalid symbol characters";
            draftInput.text = "BAD SYM";
            root.open();
            break;
        default:
            preview.active_symbol = "PETR4";
            preview.active_timeframe = "1m";
            preview.target_error = "";
            draftInput.text = "";
            root.close();
            break;
        }
    }

    Component.onCompleted: applyPreviewState()
    onPreviewStateChanged: applyPreviewState()

    readonly property var effectiveContext: root.previewState !== "rest" ? preview : root.context
    readonly property string activeSymbol: effectiveContext ? effectiveContext.active_symbol : ""
    readonly property string activeTimeframe: effectiveContext ? effectiveContext.active_timeframe : "1m"
    readonly property string targetError: effectiveContext ? effectiveContext.target_error : ""

    readonly property var parsedDraft: {
        if (!root.effectiveContext || draftInput.text.trim() === "") {
            return { ok: false, error: "", symbol: "", timeframe: "", hint: "" };
        }
        if (root.previewState !== "rest") {
            if (root.previewState === "error") {
                return {
                    ok: true,
                    symbol: "BAD SYM",
                    timeframe: root.activeTimeframe,
                    hint: "Symbol only; keeping timeframe " + root.activeTimeframe
                };
            }
            return {
                ok: true,
                symbol: "PETR4",
                timeframe: root.activeTimeframe,
                hint: "Symbol only; keeping timeframe " + root.activeTimeframe
            };
        }
        try {
            return JSON.parse(root.effectiveContext.parse_target_draft(draftInput.text));
        } catch (e) {
            return { ok: false, error: "Could not parse draft", symbol: "", timeframe: "", hint: "" };
        }
    }

    readonly property string previewLine: {
        if (!parsedDraft.ok) {
            return parsedDraft.error || "Enter a symbol, timeframe, or both";
        }
        var pair = parsedDraft.symbol + " · " + parsedDraft.timeframe;
        if (parsedDraft.hint && parsedDraft.hint.length > 0) {
            return pair + " — " + parsedDraft.hint;
        }
        return pair;
    }

    function openWith(initialChar) {
        if (root.previewState !== "rest") {
            return;
        }
        draftInput.text = initialChar || "";
        root.open();
        draftInput.forceActiveFocus();
        draftInput.cursorPosition = draftInput.text.length;
    }

    function setDraftText(text) {
        draftInput.text = text;
    }

    function draftText() {
        return draftInput.text;
    }

    function submitDraft() {
        if (!parsedDraft.ok) {
            if (!root.opened) {
                root.open();
            }
            draftInput.forceActiveFocus();
            return;
        }
        var sym = parsedDraft.symbol;
        var tf = parsedDraft.timeframe;
        if (root.context) {
            var ok = root.context.request_target(sym, tf);
            if (ok) {
                root.close();
                root.submitted(sym, tf);
            } else {
                draftInput.forceActiveFocus();
            }
        } else {
            root.close();
            root.submitted(sym, tf);
        }
    }

    background: Rectangle {
        color: Theme.surfaceElevated
        border.color: Theme.borderStrong
        border.width: Theme.borderWidth
        radius: Theme.radiusMedium
    }

    contentItem: ColumnLayout {
        spacing: Spacing.size4
        width: Spacing.size220

        Accessible.name: "Chart target prompt"
        Accessible.description: "Type a symbol, timeframe, or both, then press Enter to apply"

        Text {
            text: "Chart target"
            color: Theme.textSecondary
            font.family: Theme.uiFont
            font.pixelSize: Theme.typeLabelSmall
            Layout.fillWidth: true
        }

        AppTextField {
            id: draftInput
            objectName: "chartTargetPromptInput"
            Accessible.name: "Chart target draft"
            Accessible.description: "Type a symbol, timeframe, or both"
            Layout.fillWidth: true
            placeholderText: activeSymbol + " · " + activeTimeframe
            previewState: root.activeFocus || draftInput.activeFocus ? "focused" : "rest"

            onTextEdited: {
                if (root.context && root.context.target_error !== "") {
                    root.context.clear_error();
                }
            }

            Keys.onReturnPressed: root.submitDraft()
            Keys.onEnterPressed: root.submitDraft()
            Keys.onEscapePressed: {
                root.close();
                root.cancelled();
            }
        }

        Text {
            id: previewText
            objectName: "chartTargetPromptPreview"
            text: root.previewLine
            color: parsedDraft.ok ? Theme.textSecondary : Theme.critical
            font.family: Theme.uiFont
            font.pixelSize: Theme.typeLabelSmall
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        Text {
            id: promptErrorText
            objectName: "chartTargetPromptError"
            visible: root.targetError !== ""
            text: root.targetError
            color: Theme.critical
            font.family: Theme.uiFont
            font.pixelSize: Theme.typeLabelSmall
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }

    onClosed: {
        if (root.previewState === "rest") {
            draftInput.text = "";
        }
    }

    onOpened: {
        if (root.previewState === "rest") {
            draftInput.forceActiveFocus();
            draftInput.cursorPosition = draftInput.text.length;
        }
    }
}
