pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Workspace create, rename, duplicate, delete and switch (Q-053).
RowLayout {
    id: root

    required property var shell
    required property var workspace
    required property var window

    spacing: Theme.spaceSm

    AppComboBox {
        id: picker
        Layout.preferredWidth: Spacing.size160
        model: JSON.parse(workspace.workspace_list)
        textRole: ""
        displayText: workspace.active_workspace || "Workspace"
        onActivated: function (index) {
            root.shell.switchWorkspace(model[index]);
        }
    }

    AppButton {
        text: "Save"
        implicitWidth: Spacing.size72
        onClicked: root.shell.saveWorkspace(workspace.active_workspace)
    }

    AppButton {
        text: "Duplicate"
        implicitWidth: Spacing.size100
        onClicked: duplicateDialog.open()
    }

    Rectangle {
        Layout.preferredWidth: Theme.dividerWidth
        Layout.preferredHeight: Spacing.size20
        color: Theme.borderDefault
    }

    AppButton {
        text: "Commands"
        implicitWidth: Spacing.size100
        Accessible.name: "Open command palette"
        onClicked: root.window.openPalette()
    }

    AppButton {
        text: root.window.merged ? "Split out" : "Merge windows"
        implicitWidth: Spacing.size120
        visible: root.window.merged || root.shell.controller.window_count > 1
        onClicked: root.window.merged ? root.shell.controller.split_all(root.window.windowId) : root.shell.controller.merge_into(root.window.windowId)
    }

    AppButton {
        text: root.window.detached ? "Reattach selection" : "Detach selection"
        implicitWidth: Spacing.size140
        onClicked: root.window.toggleDetach()
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
