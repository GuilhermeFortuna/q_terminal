import QtQuick
import QtQuick.Controls
import "../theme"

Frame {
    id: root
    property string previewState: "rest"
    property alias rows: body.data
    padding: Theme.spaceNone
    implicitWidth: Spacing.dialogWidth; implicitHeight: Spacing.accountSummaryHeight
    background: Rectangle { color: ControlState.background(root.previewState); border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth }
    contentItem: Column { id: body; spacing: Theme.spaceNone }
}
