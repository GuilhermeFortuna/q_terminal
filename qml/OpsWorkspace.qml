pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml

Item {
    id: root

    required property ExecutionModels executionModels
    required property OpsStatus opsStatus
    required property ExecutionControls executionControls
    property var feed: null

    readonly property alias opsHeader: header
    readonly property alias deploymentList: depList
    readonly property alias deploymentDetail: depDetail
    readonly property alias chartPane: chart

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size0

        // Status Header
        OpsHeader {
            id: header
            Layout.fillWidth: true
            opsStatus: root.opsStatus
            executionControls: root.executionControls
            executionModels: root.executionModels
        }

        // Main Area: Left (Deployments) + Right (Chart + Tables)
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Spacing.size0

            // Left Pane: Deployment List
            DeploymentList {
                id: depList
                Layout.preferredWidth: Spacing.size320
                Layout.fillHeight: true
                executionModels: root.executionModels
                executionControls: root.executionControls
                opsStatus: root.opsStatus
            }

            // Vertical Divider
            Rectangle {
                Layout.preferredWidth: Spacing.size1
                Layout.fillHeight: true
                color: Theme.surfaceSelected
            }

            // Right Pane: Top Chart, Bottom Detail Tables
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Spacing.size0

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
                    Layout.preferredHeight: Spacing.size1
                    color: Theme.surfaceSelected
                }

                // Bottom Half: Deployment Detail & Tabbed Tables
                DeploymentDetail {
                    id: depDetail
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    executionModels: root.executionModels
                    opsStatus: root.opsStatus
                    executionControls: root.executionControls
                }
            }
        }

        // Bottom Status Strip
        StatusStrip {
            id: statusStrip
            Layout.fillWidth: true
            height: Spacing.size24
            feed: root.feed
        }
    }
}
