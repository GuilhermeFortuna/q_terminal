import QtQuick
import qml

Item {
    id: root
    property string text: "SECTION"
    property string previewState: "rest"
    implicitHeight: Spacing.controlHeight
    implicitWidth: Spacing.dialogWidth
    Text {
        anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter
        text: root.text; color: ControlState.foreground(root.previewState)
        font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold
    }
    Rectangle {
        anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
        height: Theme.borderWidth; color: ControlState.border(root.previewState)
    }
}
