pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml

// Panel chrome (Q-052): title, icon, actions and a focus ring around one content surface.
// A panel is one persistent item owned by the shell; windows only lend it a place to sit,
// so moving it between windows keeps its state.
FocusScope {
    id: root

    required property var shell
    required property string panelId

    property string kind: ""
    // The item whose trigger(command) runs this panel's commands; null for none.
    property var commandTarget: null
    default property alias contentData: body.data

    readonly property var spec: root.shell.panelSpec(root.panelId)
    readonly property var hostWindow: Window.window
    readonly property string windowId: root.hostWindow && root.hostWindow.windowId ? root.hostWindow.windowId : ""
    readonly property bool floating: root.hostWindow && root.hostWindow.composition === "floating"

    function trigger(command) {
        if (root.commandTarget) {
            root.commandTarget.trigger(command);
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.surfaceBase
        border.color: root.activeFocus ? Theme.accent : Theme.surfaceSelected
        border.width: root.activeFocus ? Theme.focusBorderWidth : Theme.borderWidth
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.borderWidth
        spacing: Spacing.none

        Rectangle {
            id: titleBar
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size24
            color: Theme.surfaceElevated

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceXxs
                spacing: Theme.spaceXs

                Image {
                    source: root.shell.iconFor(root.spec.icon)
                    sourceSize.width: Spacing.iconSmall
                    sourceSize.height: Spacing.iconSmall
                    fillMode: Image.PreserveAspectFit
                    opacity: root.activeFocus ? 1.0 : 0.6
                }
                Text {
                    Layout.fillWidth: true
                    text: root.spec.title
                    color: root.activeFocus ? Theme.textStrong : Theme.textSecondary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    font.weight: Typography.weightMedium
                    elide: Text.ElideRight
                }
                IconButton {
                    implicitWidth: Spacing.size20
                    implicitHeight: Spacing.size20
                    iconSource: root.floating ? Icons.dock : Icons.popOut
                    toolTip: root.floating ? "Dock back" : "Float into its own window"
                    onClicked: root.floating ? root.shell.controller.dock_back(root.panelId) : root.shell.controller.float_panel(root.panelId)
                }
            }
        }

        Item {
            id: body
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
        }
    }

    // Focus follows a click anywhere in the panel without stealing it from the controls.
    TapHandler {
        gesturePolicy: TapHandler.ReleaseWithinBounds
        grabPermissions: PointerHandler.ApprovesTakeOverByAnything
        onTapped: root.forceActiveFocus()
    }

    // Panel-scoped shortcuts work only while this panel has focus.
    Instantiator {
        model: root.shell.commandsForPanel(root.kind)
        delegate: Shortcut {
            required property var modelData
            sequence: modelData.shortcut
            enabled: root.activeFocus
            onActivated: root.trigger(modelData.id)
        }
    }
}
