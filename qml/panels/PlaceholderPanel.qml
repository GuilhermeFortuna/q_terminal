import QtQuick
import qml

// Empty surface reserved for a phase 5 panel: nothing is drawn until that task fills it.
PanelFrame {
    id: root
    EmptyState {
        anchors.fill: parent
        title: root.spec.title
        detail: "Reserved for a later release."
    }
}
