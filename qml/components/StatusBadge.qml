import QtQuick
import QtQuick.Controls
import "../theme"

Control {
    id: root
    property string role: Semantic.neutral
    property string previewState: "rest"
    property alias text: label.text
    implicitWidth: label.implicitWidth + Theme.spaceMd * 2; implicitHeight: Spacing.badgeHeight
    contentItem: Text { id: label; text: "STATUS"; color: Semantic.foreground(root.previewState === "stale" ? Semantic.stale : root.previewState === "degraded" ? Semantic.critical : root.role); font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall; font.weight: Typography.weightBold; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter }
    background: Rectangle { radius: Theme.radiusSmall; color: Semantic.background(root.previewState === "stale" ? Semantic.stale : root.previewState === "degraded" ? Semantic.critical : root.role); border.color: Semantic.border(root.previewState === "stale" ? Semantic.stale : root.previewState === "degraded" ? Semantic.critical : root.role); border.width: Theme.borderWidth; opacity: root.previewState === "disabled" ? 0.5 : 1.0 }
}
