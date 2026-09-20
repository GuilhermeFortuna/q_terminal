import QtQuick
import QtQuick.Controls
import qml

Control {
    id: root
    property string previewState: "rest"
    property bool alternate: false
    default property alias contentData: row.data
    implicitHeight: Theme.tableRowHeight; implicitWidth: Spacing.dialogWidth
    focusPolicy: Qt.StrongFocus; hoverEnabled: true
    contentItem: Row { id: row; spacing: Theme.spaceMd }
    background: Rectangle {
        color: root.previewState === "rest" && root.alternate ? Theme.surfaceRaised : ControlState.background(root.previewState)
        border.color: root.activeFocus ? Theme.accent : ControlState.border(root.previewState)
        border.width: root.activeFocus ? Theme.focusBorderWidth : Theme.spaceNone
    }
}
