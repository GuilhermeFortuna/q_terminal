import QtQuick
import QtQuick.Controls
import "../theme"

Button {
    id: root
    property string variant: "secondary"
    property string previewState: "rest"
    implicitHeight: Theme.controlHeight
    implicitWidth: Spacing.priceAxisWidth + Spacing.xl
    enabled: previewState !== "disabled"
    hoverEnabled: true
    font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightMedium
    contentItem: Text { text: root.text; color: ControlState.foreground(root.previewState); font: root.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    background: Rectangle {
        radius: Theme.radiusMedium
        color: root.variant === "primary" ? (root.down || root.previewState === "pressed" ? Theme.accentPressed : Theme.accentStrong) : ControlState.background(root.previewState)
        border.color: root.activeFocus || root.previewState === "focused" ? Theme.accent : ControlState.border(root.previewState)
        border.width: root.activeFocus || root.previewState === "focused" ? Theme.focusBorderWidth : Theme.borderWidth
        opacity: root.enabled ? 1.0 : 0.55
    }
}
