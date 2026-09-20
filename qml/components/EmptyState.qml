import QtQuick
import QtQuick.Controls
import qml

Control {
    id: root
    property string title: "No data yet"
    property string detail: "Waiting for the first confirmed update."
    property string previewState: "rest"
    implicitWidth: Spacing.dialogWidth; implicitHeight: Spacing.accountSummaryHeight
    contentItem: Column {
        spacing: Theme.spaceSm
        Text { text: root.title; color: ControlState.foreground(root.previewState); font.family: Theme.uiFont; font.pixelSize: Theme.typeTitle; font.weight: Typography.weightMedium }
        Text { text: root.detail; color: Theme.textSecondary; font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall }
    }
    background: Rectangle { color: ControlState.background(root.previewState); border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth; radius: Theme.radiusMedium }
}
