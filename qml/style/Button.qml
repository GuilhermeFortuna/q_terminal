import QtQuick
import QtQuick.Controls.Basic as Basic
import qml

Basic.Button {
    id: control
    implicitHeight: Theme.controlHeight
    font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall
    contentItem: Text { text: control.text; color: control.enabled ? Theme.textStrong : Theme.textMuted; font: control.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    background: Rectangle { color: control.down ? Theme.accentPressed : control.hovered ? Theme.surfaceHover : Theme.surfaceBase; border.color: control.activeFocus ? Theme.accent : Theme.borderDefault; border.width: control.activeFocus ? Theme.focusBorderWidth : Theme.borderWidth; radius: Theme.radiusMedium }
}
