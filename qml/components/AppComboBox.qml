import QtQuick
import QtQuick.Controls
import qml

ComboBox {
    id: root
    property string previewState: "rest"
    implicitHeight: Theme.controlHeight; implicitWidth: Spacing.dialogWidth / 2
    enabled: previewState !== "disabled"
    font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall
    contentItem: Text { leftPadding: Theme.spaceMd; text: root.displayText; color: ControlState.foreground(root.previewState); font: root.font; verticalAlignment: Text.AlignVCenter }
    indicator: Image { x: root.width - width - Theme.spaceMd; anchors.verticalCenter: parent.verticalCenter; width: Spacing.iconSmall; height: Spacing.iconSmall; source: Icons.chevronDown }
    background: Rectangle { color: ControlState.background(root.previewState); border.color: root.activeFocus || root.previewState === "focused" ? Theme.accent : ControlState.border(root.previewState); border.width: root.activeFocus || root.previewState === "focused" ? Theme.focusBorderWidth : Theme.borderWidth; radius: Theme.radiusMedium }
}
