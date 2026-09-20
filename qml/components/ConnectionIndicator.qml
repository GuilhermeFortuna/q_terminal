import QtQuick
import QtQuick.Controls
import qml

Control {
    id: root
    property string role: Semantic.positive
    property string previewState: "rest"
    readonly property string effectiveRole: previewState === "stale" ? Semantic.stale : previewState === "degraded" ? Semantic.critical : role
    property alias text: label.text
    implicitWidth: row.implicitWidth; implicitHeight: Theme.controlHeight
    contentItem: Row {
        id: row; spacing: Theme.spaceSm
        Rectangle { width: Theme.spaceMd; height: Theme.spaceMd; radius: Theme.radiusMedium; anchors.verticalCenter: parent.verticalCenter; color: Semantic.foreground(root.effectiveRole) }
        Text { id: label; text: "CONNECTED"; anchors.verticalCenter: parent.verticalCenter; color: ControlState.foreground(root.previewState); font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall }
    }
}
