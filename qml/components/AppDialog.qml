import QtQuick
import QtQuick.Controls
import qml

Dialog {
    id: root
    property string previewState: "rest"
    modal: true; focus: true
    width: Spacing.dialogWidth
    padding: Theme.spaceXl
    standardButtons: Dialog.Ok | Dialog.Cancel
    background: Rectangle { color: ControlState.background(root.previewState); border.color: ControlState.border(root.previewState); border.width: Theme.focusBorderWidth; radius: Spacing.radiusLarge }
    header: SectionHeader { text: root.title; previewState: root.previewState }
}
