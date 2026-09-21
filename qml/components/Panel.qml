import QtQuick
import qml

Rectangle {
    id: root
    property string previewState: "rest"
    default property alias contentData: content.data
    color: ControlState.background(previewState)
    border.color: ControlState.border(previewState)
    border.width: previewState === "focused" ? Theme.focusBorderWidth : Theme.borderWidth
    radius: Theme.radiusMedium
    implicitWidth: Spacing.dialogWidth
    implicitHeight: Spacing.accountSummaryHeight

    Item { id: content; anchors.fill: parent; anchors.margins: Theme.spaceMd }
}
