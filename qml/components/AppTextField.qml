import QtQuick
import QtQuick.Controls
import qml

TextField {
    id: root
    property string previewState: "rest"
    property bool numeric: false
    implicitHeight: Theme.controlHeight; implicitWidth: Spacing.dialogWidth / 2
    enabled: previewState !== "disabled"
    color: ControlState.foreground(previewState); selectionColor: Theme.accentStrong; selectedTextColor: Theme.textOnAccent
    font.family: numeric ? Theme.numericFontFamily : Theme.uiFont; font.pixelSize: Theme.typeBodySmall; font.features: numeric ? { "tnum": 1 } : {}
    leftPadding: Theme.spaceMd; rightPadding: Theme.spaceMd
    background: Rectangle { color: ControlState.background(root.previewState); border.color: root.activeFocus || root.previewState === "focused" ? Theme.accent : ControlState.border(root.previewState); border.width: root.activeFocus || root.previewState === "focused" ? Theme.focusBorderWidth : Theme.borderWidth; radius: Theme.radiusMedium }
}
