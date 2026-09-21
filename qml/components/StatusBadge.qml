import QtQuick
import QtQuick.Controls
import qml

Control {
    id: root
    property string role: Semantic.neutral
    property string previewState: "rest"
    readonly property string effectiveRole: previewState === "stale" ? Semantic.stale : previewState === "degraded" ? Semantic.critical : previewState === "disabled" ? Semantic.disabled : role
    property alias text: label.text
    implicitWidth: label.implicitWidth + Theme.spaceMd * 2; implicitHeight: Spacing.badgeHeight
    contentItem: Text { id: label; text: "STATUS"; color: Semantic.foreground(root.effectiveRole); font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall; font.weight: Typography.weightBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    background: Rectangle { radius: Theme.radiusSmall; color: Semantic.background(root.effectiveRole); border.color: Semantic.border(root.effectiveRole); border.width: Theme.borderWidth; opacity: root.previewState === "disabled" ? 0.5 : 1.0 }
}
