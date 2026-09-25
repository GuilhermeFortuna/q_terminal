pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Compact shell chrome toolbar (Q-064): active workspace, Operations, Tape, Command palette, and secondary menu.
RowLayout {
    id: root

    required property var shell
    required property var workspace
    required property var window

    Layout.fillWidth: true
    Layout.leftMargin: Theme.spaceSm
    Layout.rightMargin: Theme.spaceSm
    Layout.topMargin: Theme.spaceXs
    Layout.bottomMargin: Theme.spaceXs
    spacing: Theme.spaceSm

    AppComboBox {
        id: picker
        Layout.preferredWidth: Spacing.size160
        model: JSON.parse(workspace.workspace_list)
        textRole: ""
        displayText: workspace.active_workspace || "Workspace"
        Accessible.name: "Active workspace selector"
        onActivated: function (index) {
            root.shell.switchWorkspace(model[index]);
        }
    }

    AppButton {
        id: opsButton
        text: "Operations"
        implicitWidth: Spacing.size100
        previewState: root.shell.controller.operations_visible ? "selected" : "rest"
        toolTip: "Ctrl+Shift+O"
        Accessible.name: "Toggle operations panels (Ctrl+Shift+O)"
        onClicked: root.shell.runCommand("operations.toggle", root.window)
    }

    AppButton {
        id: tapeButton
        text: "Tape"
        implicitWidth: Spacing.size72
        previewState: root.shell.controller.tape_visible ? "selected" : "rest"
        toolTip: "Ctrl+Shift+T"
        Accessible.name: "Toggle tape panel (Ctrl+Shift+T)"
        onClicked: root.shell.runCommand("tape.toggle", root.window)
    }

    Rectangle {
        Layout.preferredWidth: Theme.dividerWidth
        Layout.preferredHeight: Spacing.size20
        color: Theme.borderDefault
    }

    AppButton {
        id: commandsButton
        text: "Commands"
        implicitWidth: Spacing.size100
        toolTip: "Ctrl+K"
        Accessible.name: "Open command palette (Ctrl+K)"
        onClicked: root.window.openPalette()
    }

    Rectangle {
        id: opsAlertChip
        objectName: "opsAlertChip"
        visible: !root.shell.controller.operations_visible && criticalOpsCause !== ""
        implicitHeight: Spacing.size24
        implicitWidth: alertRow.implicitWidth + Spacing.size16
        radius: Theme.radiusMedium
        color: Theme.criticalSurface
        border.color: Theme.criticalStrong
        border.width: Theme.borderWidth
        Layout.alignment: Qt.AlignVCenter

        readonly property var opsStatus: root.shell.activeOpsStatus
        readonly property bool workerUnavailable: opsStatus ? (opsStatus.api_status !== "unknown" && (!opsStatus.worker_heartbeat_known || opsStatus.worker_status !== "active" || opsStatus.worker_heartbeat_age_s > 30.0)) : false
        readonly property string criticalOpsCause: {
            if (!opsStatus) return "";
            if (!opsStatus.postgres_available) return "Database unavailable";
            if (opsStatus.kill_switch_enabled) return "Kill switch engaged";
            if (opsStatus.unknown_orders > 0) return "Unknown orders: " + opsStatus.unknown_orders;
            if (workerUnavailable) return "Worker unavailable";
            return "";
        }

        function openOperations() {
            root.shell.controller.ensure_operations_visible(root.window.windowId);
            root.shell.syncWindows();
        }

        MouseArea {
            id: alertMouse
            objectName: "alertMouse"
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            Accessible.role: Accessible.Button
            Accessible.name: "Alert: " + opsAlertChip.criticalOpsCause + ". Click to open Operations."
            onClicked: opsAlertChip.openOperations()
        }

        RowLayout {
            id: alertRow
            anchors.centerIn: parent
            spacing: Theme.spaceXs

            Text {
                text: "⚠"
                color: Theme.critical
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }

            Text {
                id: alertText
                text: opsAlertChip.criticalOpsCause
                color: Theme.critical
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
                font.weight: Typography.weightMedium
            }
        }

        ToolTip.visible: alertMouse.containsMouse && opsAlertChip.criticalOpsCause !== ""
        ToolTip.text: "Click to open Operations and address " + opsAlertChip.criticalOpsCause
    }

    AppButton {
        id: moreButton
        text: "More"
        implicitWidth: Spacing.size72
        Accessible.name: "Workspace actions and placement details"
        onClicked: secondaryPopup.open()
    }

    Item {
        Layout.fillWidth: true
    }

    Popup {
        id: secondaryPopup
        x: Math.max(0, Math.min(moreButton.x, root.width - width))
        y: moreButton.y + moreButton.height + Theme.spaceXs
        width: Spacing.size220
        padding: Theme.spaceSm
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.surfaceRaised
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            radius: Theme.radiusMedium
        }

        contentItem: ColumnLayout {
            spacing: Theme.spaceXs

            AppButton {
                Layout.fillWidth: true
                text: "Save workspace"
                Accessible.name: "Save current workspace"
                onClicked: {
                    secondaryPopup.close();
                    root.shell.saveWorkspace(root.workspace.active_workspace);
                }
            }

            AppButton {
                Layout.fillWidth: true
                text: "Duplicate workspace..."
                Accessible.name: "Duplicate workspace"
                onClicked: {
                    secondaryPopup.close();
                    duplicateDialog.open();
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.dividerWidth
                color: Theme.borderDefault
            }

            AppButton {
                Layout.fillWidth: true
                text: root.window.merged ? "Split out windows" : "Merge windows"
                Accessible.name: root.window.merged ? "Split out windows" : "Merge windows"
                visible: root.window.merged || root.shell.controller.window_count > 1
                onClicked: {
                    secondaryPopup.close();
                    if (root.window.merged) {
                        root.shell.controller.split_all(root.window.windowId);
                    } else {
                        root.shell.controller.merge_into(root.window.windowId);
                    }
                }
            }

            AppButton {
                Layout.fillWidth: true
                text: root.window.detached ? "Reattach selection" : "Detach selection"
                Accessible.name: root.window.detached ? "Reattach selection" : "Detach selection"
                onClicked: {
                    secondaryPopup.close();
                    root.window.toggleDetach();
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.dividerWidth
                color: Theme.borderDefault
            }

            AppButton {
                Layout.fillWidth: true
                text: "Placement details..."
                Accessible.name: "Show placement details and compositor rules"
                onClicked: {
                    secondaryPopup.close();
                    placementDialog.open();
                }
            }
        }
    }

    AppDialog {
        id: placementDialog
        title: "Workspace placement"
        standardButtons: Dialog.Close
        width: Spacing.dialogWidth

        contentItem: ColumnLayout {
            spacing: Theme.spaceMd

            Text {
                text: "Placement mode: " + (root.workspace.placement_mode === "compositor" ? "compositor (Wayland)" : root.workspace.placement_mode)
                color: root.workspace.placement_mode === "compositor" ? Theme.warningText : Theme.textPrimary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
                font.weight: Typography.weightMedium
            }

            Text {
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: root.workspace.placement_mode === "compositor"
                      ? "Panels restore in Q Terminal. The compositor, not this application, places windows; export rules only for installation in your compositor configuration."
                      : "Window and panel arrangements restore within Q Terminal."
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }

            AppButton {
                visible: root.workspace.placement_mode === "compositor"
                text: "Export compositor rules"
                implicitWidth: Spacing.size160
                Accessible.name: "Export compositor rules"
                onClicked: {
                    var path = root.workspace.export_compositor_rules();
                    if (path !== "") {
                        exportLabel.text = "Exported " + path;
                    }
                }
            }

            Text {
                id: exportLabel
                Layout.fillWidth: true
                visible: exportLabel.text !== ""
                elide: Text.ElideRight
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }

            Repeater {
                model: {
                    try {
                        return JSON.parse(root.workspace.reports_json);
                    } catch (e) {
                        return [];
                    }
                }
                delegate: Text {
                    required property var modelData
                    Layout.fillWidth: true
                    wrapMode: Text.WordWrap
                    text: modelData.message
                    color: Theme.textSecondary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                }
            }
        }
    }

    AppDialog {
        id: duplicateDialog
        title: "Duplicate workspace"
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAccepted: {
            if (nameField.text.trim() !== "") {
                workspace.duplicate(workspace.active_workspace, nameField.text.trim());
            }
        }
        contentItem: ColumnLayout {
            spacing: Theme.spaceMd
            AppTextField {
                id: nameField
                Layout.fillWidth: true
                placeholderText: "New workspace name"
            }
        }
    }
}
