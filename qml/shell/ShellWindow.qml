pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import QtQml.Models
import qml

// One top-level window of the shell (Q-052): a composition of panels over the shared
// stores. It owns nothing but its arrangement; closing it never touches shared state.
Window {
    id: win

    required property var shell
    required property string windowId

    property var layout: null
    readonly property string composition: win.layout ? win.layout.composition : ""
    readonly property bool merged: win.layout ? win.layout.merged : false
    readonly property bool detached: {
        void win.shell.controller.selection_revision;
        return win.shell.controller.is_detached(win.windowId);
    }
    readonly property string selection: {
        void win.shell.controller.selection_revision;
        return win.shell.controller.effective_selection(win.windowId);
    }

    function reload() {
        win.layout = JSON.parse(win.shell.controller.window_layout(win.windowId));
    }

    function openPalette() {
        palette.open();
    }

    function toggleDetach() {
        if (win.detached) {
            win.shell.controller.attach(win.windowId);
        } else {
            win.shell.controller.detach(win.windowId);
        }
    }

    visible: true
    width: Spacing.size1280
    height: Spacing.size800
    title: "q_terminal · " + win.composition
    color: Theme.surfaceBase

    Component.onCompleted: win.reload()
    onClosing: win.shell.windowClosing(win.windowId)

    Connections {
        target: win.shell.controller
        function onRevisionChanged() {
            win.reload();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.none

        Toolbar {
            Layout.fillWidth: true
            tools: [
                AppButton {
                    text: "Commands"
                    implicitWidth: Spacing.size100
                    onClicked: win.openPalette()
                },
                AppButton {
                    text: win.merged ? "Split out" : "Merge windows"
                    implicitWidth: Spacing.size120
                    visible: win.merged || win.shell.controller.window_count > 1
                    onClicked: win.merged ? win.shell.controller.split_all(win.windowId) : win.shell.controller.merge_into(win.windowId)
                },
                AppButton {
                    text: win.detached ? "Reattach selection" : "Detach selection"
                    implicitWidth: Spacing.size140
                    onClicked: win.toggleDetach()
                }
            ]
        }

        Rectangle {
            id: detachedBanner
            objectName: "detachedBanner"
            Layout.fillWidth: true
            Layout.preferredHeight: win.detached ? Spacing.size24 : Spacing.none
            visible: win.detached
            color: Theme.warningSurface
            Text {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                verticalAlignment: Text.AlignVCenter
                text: "DETACHED — this window keeps its own selection (" + (win.selection === "" ? "none" : win.selection) + "). Detail and chart still follow the global selection."
                color: Theme.warningText
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
                elide: Text.ElideRight
            }
        }

        PanelHost {
            Layout.fillWidth: true
            Layout.fillHeight: true
            shell: win.shell
            windowId: win.windowId
            node: win.layout ? win.layout.root : null
            visible: win.layout !== null
        }
    }

    CommandPalette {
        id: palette
        shell: win.shell
        win: win
    }

    // App-wide shortcuts work from whichever window has focus.
    Instantiator {
        model: win.shell.commands.filter(function (c) {
            return c.scope === "app";
        })
        delegate: Shortcut {
            required property var modelData
            sequence: modelData.shortcut
            onActivated: {
                if (win.shell.commandEnabled(modelData.id)) {
                    win.shell.runCommand(modelData.id, win);
                }
            }
        }
    }
}
