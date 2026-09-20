pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import qml

SplitView {
    id: root
    property string previewState: "rest"
    orientation: Qt.Horizontal
    implicitWidth: Spacing.dialogWidth; implicitHeight: Spacing.accountSummaryHeight
    handle: Rectangle { implicitWidth: Theme.spaceXs; color: ControlState.border(root.previewState) }
    Rectangle { SplitView.fillWidth: true; color: Theme.surfaceBase }
    Rectangle { SplitView.preferredWidth: Spacing.dialogWidth / 3; color: Theme.surfaceRaised }
}
