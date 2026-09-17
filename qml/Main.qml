import qml
import QtQuick

Window {
    id: window
    visible: true
    width: 640
    height: 480
    title: "q_terminal"

    AppInfo {
        id: appInfo
        objectName: "appInfo"
    }

    BarSeries {
        id: barSeries
        Component.onCompleted: barSeries.load_history_sample(100)
    }

    Item {
        id: chartHost
        anchors.fill: parent
        anchors.margins: 8

        BarChartItem {
            id: chart
            objectName: "benchChart"
            width: chartHost.width
            height: chartHost.height
            series: barSeries
            firstBar: 0
            lastBar: 100
            lowPrice: 95.0
            highPrice: 105.0
        }
    }

    Column {
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.margins: 8
        spacing: 8
        z: 1

        Text {
            text: "Application: " + appInfo.app_version
            font.pixelSize: 14
            color: "white"
        }
        Text {
            text: "Core: " + appInfo.core_version
            font.pixelSize: 14
            color: "white"
        }
        Text {
            text: "Contracts: " + appInfo.contracts_rev
            font.pixelSize: 14
            color: "white"
        }
        Text {
            text: "Render Backend: " + appInfo.render_backend
            font.pixelSize: 14
            color: "white"
        }
    }
}
