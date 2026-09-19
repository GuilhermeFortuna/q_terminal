pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    height: bannerColumn.height + 48
    color: "#1e222d"

    required property OpsStatus opsStatus

    ColumnLayout {
        id: bannerColumn
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 0

        // Warning banner: Postgres down
        Rectangle {
            id: postgresBanner
            Layout.fillWidth: true
            height: (!root.opsStatus.postgres_available) ? 32 : 0
            visible: !root.opsStatus.postgres_available
            color: "#ef4444"
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16

                Text {
                    text: "⚠ Database Unavailable (503): Postgres is down — all deployments marked unknown"
                    color: "#ffffff"
                    font.pixelSize: 12
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Warning banner: Worker lease offline or stale
        Rectangle {
            id: workerBanner
            Layout.fillWidth: true
            height: (root.opsStatus.postgres_available && (root.opsStatus.worker_status !== "active" || root.opsStatus.worker_heartbeat_age_s > 30.0)) ? 32 : 0
            visible: root.opsStatus.postgres_available && (root.opsStatus.worker_status !== "active" || root.opsStatus.worker_heartbeat_age_s > 30.0)
            color: "#f59e0b"
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16

                Text {
                    text: "⚠ Execution Worker Offline / Stale (Status: " + root.opsStatus.worker_status + ", Heartbeat age: " + Format.formatAge(root.opsStatus.worker_heartbeat_age_s) + ")"
                    color: "#000000"
                    font.pixelSize: 12
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Warning banner: Unknown orders requiring reconciliation
        Rectangle {
            id: unkBanner
            Layout.fillWidth: true
            height: (root.opsStatus.unknown_orders > 0) ? 32 : 0
            visible: root.opsStatus.unknown_orders > 0
            color: "#dc2626"
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16

                Text {
                    text: "⚠ Reconciliation Required: " + root.opsStatus.unknown_orders + " unknown order(s) detected"
                    color: "#ffffff"
                    font.pixelSize: 12
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Main status bar
        Rectangle {
            Layout.fillWidth: true
            height: 48
            color: "#1e222d"

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 16
                anchors.rightMargin: 16
                spacing: 12

                // Workspace Title
                Text {
                    text: "OPERATIONS"
                    color: "#f8fafc"
                    font.pixelSize: 15
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }

                Rectangle {
                    width: 1
                    height: 20
                    color: "#334155"
                    Layout.alignment: Qt.AlignVCenter
                }

                // Stream Connection & Age
                Rectangle {
                    height: 24
                    implicitWidth: streamRow.implicitWidth + 16
                    radius: 4
                    color: (root.opsStatus.stream_state === "connected") ? "#064e3b" : "#7f1d1d"
                    border.color: (root.opsStatus.stream_state === "connected") ? "#10b981" : "#ef4444"
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: streamRow
                        anchors.centerIn: parent
                        spacing: 6

                        Rectangle {
                            width: 6
                            height: 6
                            radius: 3
                            color: (root.opsStatus.stream_state === "connected") ? "#34d399" : "#f87171"
                        }

                        Text {
                            text: "Stream: " + root.opsStatus.stream_state + (root.opsStatus.stream_age_s > 0 ? " (" + Format.formatAge(root.opsStatus.stream_age_s) + ")" : "")
                            color: "#ffffff"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }
                }

                // API Status
                Rectangle {
                    height: 24
                    implicitWidth: apiText.implicitWidth + 16
                    radius: 4
                    color: (root.opsStatus.api_status === "ok") ? "#0f2f24" : "#450a0a"
                    border.color: (root.opsStatus.api_status === "ok") ? "#059669" : "#dc2626"
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: apiText
                        anchors.centerIn: parent
                        text: "API: " + root.opsStatus.api_status
                        color: (root.opsStatus.api_status === "ok") ? "#34d399" : "#f87171"
                        font.pixelSize: 11
                        font.bold: true
                    }
                }

                // Worker Status & Heartbeat
                Rectangle {
                    height: 24
                    implicitWidth: workerRow.implicitWidth + 16
                    radius: 4
                    color: (root.opsStatus.worker_status === "active" && root.opsStatus.worker_heartbeat_age_s <= 30.0) ? "#0f2f24" : "#451a03"
                    border.color: (root.opsStatus.worker_status === "active" && root.opsStatus.worker_heartbeat_age_s <= 30.0) ? "#059669" : "#d97706"
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: workerRow
                        anchors.centerIn: parent
                        spacing: 6

                        Text {
                            text: "Worker: " + root.opsStatus.worker_status + " (" + Format.formatAge(root.opsStatus.worker_heartbeat_age_s) + ")"
                            color: (root.opsStatus.worker_status === "active" && root.opsStatus.worker_heartbeat_age_s <= 30.0) ? "#34d399" : "#fbbf24"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }
                }

                // MT5 Edge & Terminal Build
                Rectangle {
                    height: 24
                    implicitWidth: edgeRow.implicitWidth + 16
                    radius: 4
                    color: (root.opsStatus.edge_reachable && root.opsStatus.edge_mt5_connected) ? "#0f2f24" : "#3b1111"
                    border.color: (root.opsStatus.edge_reachable && root.opsStatus.edge_mt5_connected) ? "#059669" : "#b91c1c"
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: edgeRow
                        anchors.centerIn: parent
                        spacing: 6

                        Text {
                            text: "MT5: " + (root.opsStatus.edge_mt5_connected ? "connected (b" + root.opsStatus.terminal_build + ")" : (root.opsStatus.edge_reachable ? "terminal down" : "edge down"))
                            color: (root.opsStatus.edge_reachable && root.opsStatus.edge_mt5_connected) ? "#34d399" : "#f87171"
                            font.pixelSize: 11
                            font.bold: true
                        }
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                // Unknown Orders Chip
                Rectangle {
                    height: 24
                    implicitWidth: unkText.implicitWidth + 16
                    radius: 4
                    visible: root.opsStatus.unknown_orders > 0
                    color: "#7f1d1d"
                    border.color: "#ef4444"
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: unkText
                        anchors.centerIn: parent
                        text: "Unknown orders: " + root.opsStatus.unknown_orders
                        color: "#fca5a5"
                        font.pixelSize: 11
                        font.bold: true
                    }
                }

                // Kill Switch Badge
                Rectangle {
                    height: 24
                    implicitWidth: killText.implicitWidth + 16
                    radius: 4
                    color: root.opsStatus.kill_switch_enabled ? "#7f1d1d" : "#1e293b"
                    border.color: root.opsStatus.kill_switch_enabled ? "#ef4444" : "#475569"
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: killText
                        anchors.centerIn: parent
                        text: "Kill Switch: " + (root.opsStatus.kill_switch_enabled ? "ENGAGED" : "OFF")
                        color: root.opsStatus.kill_switch_enabled ? "#fca5a5" : "#94a3b8"
                        font.pixelSize: 11
                        font.bold: true
                    }
                }

                // Live Lock Badge
                Rectangle {
                    height: 24
                    implicitWidth: liveLockText.implicitWidth + 16
                    radius: 4
                    color: root.opsStatus.live_locked ? "#334155" : "#b45309"
                    border.color: root.opsStatus.live_locked ? "#64748b" : "#f59e0b"
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: liveLockText
                        anchors.centerIn: parent
                        text: root.opsStatus.live_locked ? "Live locked" : "LIVE UNLOCKED"
                        color: root.opsStatus.live_locked ? "#94a3b8" : "#fef3c7"
                        font.pixelSize: 11
                        font.bold: true
                    }
                }
            }
        }
    }
}
