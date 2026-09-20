import QtQuick
import QtQuick.Controls
import "../theme"

Control {
    id: root
    property string text: "COLUMN"
    property string previewState: "rest"
    implicitHeight: Theme.tableHeaderHeight; implicitWidth: Spacing.dialogWidth
    contentItem: Text { text: root.text; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold; verticalAlignment: Text.AlignVCenter; leftPadding: Theme.spaceMd }
    background: Rectangle { color: Theme.surfaceOverlay; border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth }
}
