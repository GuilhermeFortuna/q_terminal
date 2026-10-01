import QtQuick
import qml

PanelFrame {
    id: root
    kind: "chart"
    commandTarget: identity
    readonly property alias chartPane: chart

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
            z: 10
            context: root.shell.activeChartContext
            feed: root.shell.activeFeed
            executionModels: root.shell.activeExecutionModels
        }

        ChartPane {
            id: chart
            objectName: "chartPane"
            width: parent.width
            height: parent.height - identity.height
            z: 1
            context: root.shell.activeChartContext
            feed: root.shell.activeFeed
        }
    }

    Connections {
        target: chart.viewport
        function onVisibleBarsChanged() {
            if (root.shell && typeof root.shell.scheduleAutosave === "function") {
                root.shell.scheduleAutosave();
            }
        }
    }
}
