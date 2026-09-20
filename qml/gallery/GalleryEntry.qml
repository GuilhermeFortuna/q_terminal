import QtQuick
import QtQuick.Layouts
import "../theme"
import "../components"

Rectangle {
    id: root
    required property var entry
    implicitHeight: Spacing.galleryCardHeight
    color: Theme.surfaceRaised
    border.color: Theme.borderDefault
    border.width: Theme.borderWidth
    radius: Theme.radiusMedium

    ColumnLayout {
        anchors.fill: parent; anchors.margins: Theme.spaceMd; spacing: Theme.spaceSm
        RowLayout {
            Layout.fillWidth: true
            Text { text: root.entry.name; color: Theme.textStrong; font.family: Theme.uiFont; font.pixelSize: Theme.typeBody; font.weight: Typography.weightBold }
            Item { Layout.fillWidth: true }
            Text { text: "REFERENCE ADAPTED"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
        }
        ComponentPreview { Layout.fillWidth: true; Layout.preferredHeight: Spacing.galleryPreviewHeight; componentName: root.entry.name }
        RowLayout {
            spacing: Theme.spaceXs
            Repeater {
                model: root.entry.states
                Rectangle {
                    id: stateChip
                    required property string modelData
                    implicitWidth: stateText.implicitWidth + Theme.spaceMd
                    implicitHeight: Spacing.badgeHeight
                    radius: Theme.radiusSmall
                    color: ControlState.background(modelData)
                    border.color: ControlState.border(modelData)
                    border.width: Theme.borderWidth
                    Text { id: stateText; anchors.centerIn: parent; text: stateChip.modelData.toUpperCase(); color: ControlState.foreground(stateChip.modelData); font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                }
            }
        }
    }
}
