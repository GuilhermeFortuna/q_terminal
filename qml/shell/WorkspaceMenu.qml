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
