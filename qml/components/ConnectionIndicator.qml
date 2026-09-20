import QtQuick
import QtQuick.Controls
import "../theme"

Control {
    id: root
    property string role: Semantic.positive
    property string previewState: "rest"
    property alias text: label.text
    implicitWidth: row.implicitWidth; implicitHeight: Theme.controlHeight
    contentItem: Row {
        id: row; spacing: Theme.spaceSm
        Rectangle { width: Theme.spaceMd; height: Theme.spaceMd; radius: Theme.radiusMedium; anchors.verticalCenter: parent.verticalCenter; color: Semantic.foreground(root.previewState === "stale" ? Semantic.stale : root.previewState === "degraded" ? Semantic.critical : root.role) }
        Text { id: label; text: "CONNECTED"; anchors.verticalCenter: parent.verticalCenter; color: ControlState.foreground(root.previewState); font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall }
    }
}
