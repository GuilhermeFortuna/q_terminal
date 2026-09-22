pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml
import "Format.js" as Format

Rectangle {
    id: root
    height: bannerColumn.height
    color: Theme.surfaceElevated

    required property OpsStatus opsStatus
    required property ExecutionControls executionControls
    required property ExecutionModels executionModels

    property string killSwitchActionId: ""
    readonly property bool workerUnavailable: !opsStatus.worker_heartbeat_known
                                          || opsStatus.worker_status !== "active"
                                          || opsStatus.worker_heartbeat_age_s > 30.0
    readonly property string heartbeatAge: opsStatus.worker_heartbeat_known
                                        ? Format.formatAge(opsStatus.worker_heartbeat_age_s) : "unknown"
    readonly property string summary: {
        if (!opsStatus.postgres_available) return "DATABASE UNAVAILABLE — execution state may be unknown"
        if (opsStatus.kill_switch_enabled) return "KILL SWITCH ENGAGED — new risk is halted"
        if (opsStatus.unknown_orders > 0) return "RECONCILIATION REQUIRED — " + opsStatus.unknown_orders + " unknown orders"
        if (workerUnavailable) return "EXECUTION WORKER UNAVAILABLE — heartbeat " + heartbeatAge
        if (opsStatus.live_locked) return "LIVE TRADING LOCKED"
        return "OPERATIONS HEALTHY"
    }

    // Shell command entry point (Q-052): the palette and shortcuts open the same
    // confirmations the buttons do.
    function trigger(command) {
        if (command === "kill-switch.set") {
            killEngageDialog.open();
        } else if (command === "kill-switch.clear") {
            killReleaseDialog.open();
        }
    }

    ConfirmDialog {
        id: killEngageDialog
        actionTitle: "Engage kill switch"
        consequenceText: "Halts new risk across all deployments until released."
        onConfirmed: {
            var id = root.executionControls.request("kill_switch_set", JSON.stringify({ reason: "Operator engaged kill switch" }));
            root.killSwitchActionId = id;
        }
    }

    ConfirmDialog {
        id: killReleaseDialog
        actionTitle: "Release kill switch"
        consequenceText: "Allows trading to resume when other gates permit."
        onConfirmed: {
            var id = root.executionControls.request("kill_switch_clear", "{}");
            root.killSwitchActionId = id;
        }
    }

    ColumnLayout {
        id: bannerColumn
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        spacing: Spacing.size0

        Rectangle {
            Layout.fillWidth: true
            height: Spacing.size32
            color: !root.opsStatus.postgres_available || root.opsStatus.kill_switch_enabled || root.opsStatus.unknown_orders > 0
                   ? Theme.criticalStrong : (root.workerUnavailable || root.opsStatus.live_locked ? Theme.warningStrong : Theme.surfaceRaised)
            Text {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceLg
                anchors.rightMargin: Theme.spaceLg
                verticalAlignment: Text.AlignVCenter
                elide: Text.ElideRight
                text: root.summary
                color: Theme.textOnAccent
                font.pixelSize: Theme.typeLabel
                font.bold: true
            }
        }

        // Warning banner: Postgres down
        Rectangle {
            id: postgresBanner
            Layout.fillWidth: true
            height: Spacing.none
            visible: false
            color: Theme.negativeStrong
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16

                Text {
                    text: "⚠ Database Unavailable (503): Postgres is down — all deployments marked unknown"
                    color: Theme.textOnAccent
                    font.pixelSize: Theme.typeBody
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Warning banner: Worker lease offline or stale
        Rectangle {
            id: workerBanner
            Layout.fillWidth: true
            height: Spacing.none
            visible: false
            color: Theme.warningStrong
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16

                Text {
                    text: "⚠ Execution Worker Offline / Stale (Status: " + root.opsStatus.worker_status + ", Heartbeat age: " + Format.formatAge(root.opsStatus.worker_heartbeat_age_s) + ")"
                    color: Theme.warningForeground
                    font.pixelSize: Theme.typeBody
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Warning banner: Unknown orders requiring reconciliation
        Rectangle {
            id: unkBanner
            Layout.fillWidth: true
            height: Spacing.none
            visible: false
            color: Theme.criticalStrong
            clip: true

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16

                Text {
                    text: "⚠ Reconciliation Required: " + root.opsStatus.unknown_orders + " unknown order(s) detected"
                    color: Theme.textOnAccent
                    font.pixelSize: Theme.typeBody
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }

        // Main status bar
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: healthFlow.implicitHeight + Theme.spaceMd * 2
            height: implicitHeight
            color: Theme.surfaceElevated

            Flow {
                id: healthFlow
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.leftMargin: Spacing.size16
                anchors.rightMargin: Spacing.size16
                anchors.topMargin: Theme.spaceMd
                spacing: Spacing.size12

                // Workspace Title
                Text {
                    text: "OPERATIONS"
                    color: Theme.textStrong
                    font.pixelSize: Theme.typeTitle
                    font.bold: true
                    Layout.alignment: Qt.AlignVCenter
                }

                Rectangle {
                    width: Spacing.size1
                    height: Spacing.size20
                    color: Theme.borderDefault
                    Layout.alignment: Qt.AlignVCenter
                }

                // Stream Connection & Age
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: streamRow.implicitWidth + 16
                    radius: Spacing.size4
                    color: Semantic.background(Semantic.health(root.opsStatus.stream_state))
                    border.color: Semantic.border(Semantic.health(root.opsStatus.stream_state))
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: streamRow
                        anchors.centerIn: parent
                        spacing: Spacing.size6

                        Rectangle {
                            width: Spacing.size6
                            height: Spacing.size6
                            radius: Spacing.size3
                            color: Semantic.foreground(Semantic.health(root.opsStatus.stream_state))
                        }

                        Text {
                            text: "Stream: " + root.opsStatus.stream_state + (root.opsStatus.stream_age_s > 0 ? " (" + Format.formatAge(root.opsStatus.stream_age_s) + ")" : "")
                            color: Theme.textOnAccent
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: true
                        }
                    }
                }

                // API Status
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: apiText.implicitWidth + 16
                    radius: Spacing.size4
                    color: Semantic.background(Semantic.health(root.opsStatus.api_status))
                    border.color: Semantic.border(Semantic.health(root.opsStatus.api_status))
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: apiText
                        anchors.centerIn: parent
                        text: "API: " + root.opsStatus.api_status
                        color: Semantic.foreground(Semantic.health(root.opsStatus.api_status))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                    }
                }

                // Worker Status & Heartbeat
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: workerRow.implicitWidth + 16
                    radius: Spacing.size4
                    color: Semantic.background(Semantic.workerHealth(root.opsStatus.worker_status, root.opsStatus.worker_heartbeat_age_s))
                    border.color: Semantic.border(Semantic.workerHealth(root.opsStatus.worker_status, root.opsStatus.worker_heartbeat_age_s))
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: workerRow
                        anchors.centerIn: parent
                        spacing: Spacing.size6

                        Text {
                            text: "Worker: " + root.opsStatus.worker_status + " (" + root.heartbeatAge + ")"
                            color: Semantic.foreground(Semantic.workerHealth(root.opsStatus.worker_status, root.opsStatus.worker_heartbeat_age_s))
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: true
                        }
                    }
                }

                // MT5 Edge & Terminal Build
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: edgeRow.implicitWidth + 16
                    radius: Spacing.size4
                    color: Semantic.background(Semantic.edgeHealth(root.opsStatus.edge_reachable, root.opsStatus.edge_mt5_connected))
                    border.color: Semantic.border(Semantic.edgeHealth(root.opsStatus.edge_reachable, root.opsStatus.edge_mt5_connected))
                    Layout.alignment: Qt.AlignVCenter

                    RowLayout {
                        id: edgeRow
                        anchors.centerIn: parent
                        spacing: Spacing.size6

                        Text {
                            text: "MT5: " + (root.opsStatus.edge_mt5_connected ? "connected (b" + root.opsStatus.terminal_build + ")" : (root.opsStatus.edge_reachable ? "terminal down" : "edge down"))
                            color: Semantic.foreground(Semantic.edgeHealth(root.opsStatus.edge_reachable, root.opsStatus.edge_mt5_connected))
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: true
                        }
                    }
                }

                // Unknown Orders Chip
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: unkText.implicitWidth + 16
                    radius: Spacing.size4
                    visible: root.opsStatus.unknown_orders > 0
                    color: Theme.criticalSurface
                    border.color: Theme.negativeStrong
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: unkText
                        anchors.centerIn: parent
                        text: "Unknown orders: " + root.opsStatus.unknown_orders
                        color: Theme.negativeSoft
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                    }
                }

                // Kill Switch Badge + control
                RowLayout {
                    spacing: Spacing.size6
                    Layout.alignment: Qt.AlignVCenter

                    Rectangle {
                        height: Spacing.size24
                        implicitWidth: killText.implicitWidth + 16
                        radius: Spacing.size4
                        color: Semantic.background(Semantic.killSwitch(root.opsStatus.kill_switch_enabled))
                        border.color: Semantic.border(Semantic.killSwitch(root.opsStatus.kill_switch_enabled))

                        Text {
                            id: killText
                            anchors.centerIn: parent
                            text: "Kill Switch: " + (root.opsStatus.kill_switch_enabled ? "ENGAGED" : "OFF")
                            color: Semantic.foreground(Semantic.killSwitch(root.opsStatus.kill_switch_enabled))
                            font.pixelSize: Theme.typeBodySmall
                            font.bold: true
                        }
                    }

                    Rectangle {
                        height: Spacing.size24
                        implicitWidth: killBtnText.implicitWidth + 16
                        radius: Spacing.size4
                        visible: !root.opsStatus.kill_switch_enabled
                        color: Semantic.background(Semantic.critical)
                        border.color: Semantic.border(Semantic.critical)
                        opacity: root.executionControls.is_command_enabled("kill_switch_set") ? 1.0 : 0.4

                        Text {
                            id: killBtnText
                            anchors.centerIn: parent
                            text: "Engage"
                            color: Theme.negativeSoft
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }

                        MouseArea {
                            id: killEngageMouse
                            anchors.fill: parent
                            enabled: root.executionControls.is_command_enabled("kill_switch_set")
                            onClicked: killEngageDialog.open()
                        }
                    }

                    Rectangle {
                        height: Spacing.size24
                        implicitWidth: releaseBtnText.implicitWidth + 16
                        radius: Spacing.size4
                        visible: root.opsStatus.kill_switch_enabled
                        color: Semantic.background(Semantic.neutral)
                        border.color: Semantic.border(Semantic.neutral)
                        opacity: root.executionControls.is_command_enabled("kill_switch_clear") ? 1.0 : 0.4

                        Text {
                            id: releaseBtnText
                            anchors.centerIn: parent
                            text: "Release"
                            color: Theme.textPrimary
                            font.pixelSize: Theme.typeLabel
                            font.bold: true
                        }

                        MouseArea {
                            anchors.fill: parent
                            enabled: root.executionControls.is_command_enabled("kill_switch_clear")
                            onClicked: killReleaseDialog.open()
                        }
                    }
                }

                // Live Lock Badge
                Rectangle {
                    height: Spacing.size24
                    implicitWidth: liveLockText.implicitWidth + 16
                    radius: Spacing.size4
                    color: Semantic.background(Semantic.liveLock(root.opsStatus.live_locked))
                    border.color: Semantic.border(Semantic.liveLock(root.opsStatus.live_locked))
                    Layout.alignment: Qt.AlignVCenter

                    Text {
                        id: liveLockText
                        anchors.centerIn: parent
                        text: root.opsStatus.live_locked ? "Live locked" : "LIVE UNLOCKED"
                        color: Semantic.foreground(Semantic.liveLock(root.opsStatus.live_locked))
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                    }
                }
            }
        }
    }
}
