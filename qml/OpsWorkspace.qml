pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml

Item {
    id: root

    required property ExecutionModels executionModels
    required property OpsStatus opsStatus
    property var feed: null

    readonly property alias opsHeader: header
    readonly property alias deploymentList: depList
    readonly property alias deploymentDetail: depDetail
    readonly property alias chartPane: chart

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Status Header
        OpsHeader {
            id: header
            Layout.fillWidth: true
            opsStatus: root.opsStatus
        }

        // Main Area: Left (Deployments) + Right (Chart + Tables)
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Left Pane: Deployment List
            DeploymentList {
                id: depList
                Layout.preferredWidth: 320
                Layout.fillHeight: true
                executionModels: root.executionModels
            }

            // Vertical Divider
            Rectangle {
                Layout.preferredWidth: 1
                Layout.fillHeight: true
                color: "#1e293b"
            }

            // Right Pane: Top Chart, Bottom Detail Tables
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                // Top Half: Candlestick Chart Pane
                ChartPane {
                    id: chart
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.max(220, Math.floor(parent.height * 0.42))
                    feed: root.feed
                }

                // Horizontal Divider
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: "#1e293b"
                }

                // Bottom Half: Deployment Detail & Tabbed Tables
                DeploymentDetail {
                    id: depDetail
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    executionModels: root.executionModels
                    opsStatus: root.opsStatus
                }
            }
        }

        // Bottom Status Strip
        StatusStrip {
            id: statusStrip
            Layout.fillWidth: true
            height: 24
            feed: root.feed
        }
    }
}
