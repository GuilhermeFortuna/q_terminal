import QtQuick
import QtQuick.Controls
import qml

Dialog {
    id: root
    property string previewState: "rest"
    modal: true; focus: true
    width: Spacing.dialogWidth
    padding: Theme.spaceLg
    palette.window: Theme.surfaceElevated
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surfaceBase
    palette.alternateBase: Theme.surfaceRaised
    palette.text: Theme.textPrimary
    palette.placeholderText: Theme.textMuted
    palette.button: Theme.surfaceRaised
    palette.buttonText: Theme.textPrimary
    palette.light: Theme.surfaceHover
    palette.midlight: Theme.surfaceRaised
    palette.mid: Theme.borderDefault
    palette.dark: Theme.surfaceSunken
    palette.shadow: Theme.surfaceSunken
    palette.highlight: Theme.accentStrong
    palette.highlightedText: Theme.textOnAccent
    header: SectionHeader { text: root.title; previewState: root.previewState }
    background: Rectangle {
        color: root.previewState === "rest" ? Theme.surfaceElevated : ControlState.background(root.previewState)
        border.color: ControlState.border(root.previewState)
        border.width: Theme.focusBorderWidth
        radius: Spacing.radiusLarge
    }
}
