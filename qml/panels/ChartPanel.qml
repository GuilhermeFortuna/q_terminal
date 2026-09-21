import QtQuick
import qml

PanelFrame {
    id: root
    kind: "chart"
    ChartPane {
        id: chart
        objectName: "chartPane"
        anchors.fill: parent
        feed: root.shell.activeFeed
    }
}
