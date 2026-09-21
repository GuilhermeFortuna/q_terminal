import QtQuick
import qml

PanelFrame {
    id: root
    kind: "instrument"
    StatusStrip {
        anchors.fill: parent
        feed: root.shell.activeFeed
    }
}
