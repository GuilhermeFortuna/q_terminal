import QtQuick
import QtQuick.Controls.Basic as Basic
import "../theme"

Basic.TextField {
    id: control
    implicitHeight: Theme.controlHeight; color: Theme.textStrong
    font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall
    background: Rectangle { color: Theme.surfaceBase; border.color: control.activeFocus ? Theme.accent : Theme.borderDefault; border.width: control.activeFocus ? Theme.focusBorderWidth : Theme.borderWidth; radius: Theme.radiusMedium }
}
