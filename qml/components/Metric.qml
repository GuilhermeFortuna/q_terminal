import QtQuick
import QtQuick.Controls
import "../theme"

Control {
    id: root
    property string label: "METRIC"
    property string value: "12,345.67"
    property string role: Semantic.neutral
    property string previewState: "rest"
    implicitWidth: Spacing.dialogWidth / 3; implicitHeight: Spacing.metricHeight
    contentItem: Column {
        spacing: Theme.spaceXxs
        Text { text: root.label; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall; font.weight: Typography.weightBold }
        Text { text: root.value; color: Semantic.foreground(root.role); font.family: Theme.numericFontFamily; font.pixelSize: Theme.typeBodySmall; font.features: { "tnum": 1 } }
    }
    background: Rectangle { color: ControlState.background(root.previewState); border.color: ControlState.border(root.previewState); border.width: Theme.borderWidth; radius: Theme.radiusMedium }
}
