import QtQuick
import qml

PanelFrame {
    id: root
    kind: "chart"
    commandTarget: identity

    function trigger(command) {
        if (command === "chart.focus-symbol") {
            identity.focusSymbol();
        }
    }

    function focusCanvas() {
        chart.focusCanvas();
    }

    function testFocusCanvas() {
        return chart.testFocusCanvas();
    }

    Column {
        anchors.fill: parent
        spacing: Spacing.none

        ChartIdentity {
            id: identity
            objectName: "chartIdentity"
            width: parent.width
            context: root.shell.activeChartContext
            feed: root.shell.activeFeed
            executionModels: root.shell.activeExecutionModels
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
