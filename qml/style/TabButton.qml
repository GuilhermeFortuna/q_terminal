import QtQuick
import QtQuick.Controls.Basic as Basic
import "../theme"

Basic.TabButton {
    id: control
    implicitHeight: Spacing.controlHeightLarge
    font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall
    contentItem: Text { text: control.text; color: control.checked ? Theme.accent : Theme.textSecondary; font: control.font; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    background: Rectangle { color: control.hovered ? Theme.surfaceHover : Theme.transparent; border.color: control.activeFocus ? Theme.accent : Theme.transparent; border.width: control.activeFocus ? Theme.focusBorderWidth : Theme.spaceNone }
}
