import QtQuick
import QtQuick.Controls as Controls
import qml

Controls.TabBar {
    id: root
    property string previewState: "rest"
    implicitHeight: Spacing.controlHeightLarge
    background: Rectangle { color: Theme.surfaceOverlay; border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth }
    Controls.TabButton { text: "Orders"; font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall }
    Controls.TabButton { text: "Fills"; font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall }
}
