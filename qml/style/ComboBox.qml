import QtQuick
import QtQuick.Controls.Basic as Basic
import qml

Basic.ComboBox {
    id: control
    implicitHeight: Theme.controlHeight
    font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall
    background: Rectangle { color: control.hovered ? Theme.surfaceHover : Theme.surfaceBase; border.color: control.activeFocus ? Theme.accent : Theme.borderDefault; border.width: control.activeFocus ? Theme.focusBorderWidth : Theme.borderWidth; radius: Theme.radiusMedium }
}
