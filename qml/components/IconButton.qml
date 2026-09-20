import QtQuick
import QtQuick.Controls
import "../theme"

Button {
    id: root
    property url iconSource: Icons.settings
    property string previewState: "rest"
    property string toolTip: text
    implicitWidth: Theme.controlHeight; implicitHeight: Theme.controlHeight
    enabled: previewState !== "disabled"; hoverEnabled: true
    contentItem: Image { source: root.iconSource; sourceSize.width: Spacing.iconMedium; sourceSize.height: Spacing.iconMedium; fillMode: Image.PreserveAspectFit; opacity: root.enabled ? 1.0 : 0.45 }
    background: Rectangle { radius: Theme.radiusMedium; color: ControlState.background(root.previewState); border.color: root.activeFocus ? Theme.accent : ControlState.border(root.previewState); border.width: root.activeFocus ? Theme.focusBorderWidth : Theme.borderWidth }
    ToolTip.visible: hovered && toolTip.length > 0; ToolTip.text: toolTip
}
