import QtQuick
import qml

PanelFrame {
    id: root
    kind: "chart"

    Column {
        anchors.fill: parent
        spacing: Spacing.none

        ChartIdentity {
            id: identity
            objectName: "chartIdentity"
            width: parent.width
            context: root.shell.activeChartContext
            feed: root.shell.activeFeed
        }

        ChartPane {
            id: chart
            objectName: "chartPane"
            width: parent.width
            height: parent.height - identity.height
            context: root.shell.activeChartContext
            feed: root.shell.activeFeed
        }
    }
}
