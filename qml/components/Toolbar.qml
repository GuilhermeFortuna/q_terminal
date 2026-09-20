import QtQuick
import QtQuick.Controls
import qml

Pane {
    id: root
    property string previewState: "rest"
    property alias tools: row.data
    implicitHeight: Spacing.toolbarHeight
    padding: Theme.spaceXxs
    background: Rectangle { color: ControlState.background(root.previewState); border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth }
    contentItem: Row { id: row; spacing: Theme.spaceXs }
}
