pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Lists every registered command with its shortcut and whether it is enabled right now
// (Q-052). Modal to the window that opened it, never to the application.
Popup {
    id: root

    required property var shell
    required property var win

    property var rows: []

    function refresh() {
        var needle = field.text.toLowerCase();
        var out = [];
        for (var c of root.shell.commands) {
            if ((c.label + " " + c.id + " " + c.shortcut).toLowerCase().indexOf(needle) < 0) {
                continue;
            }
            out.push({
                id: c.id,
                label: c.label,
                shortcut: c.shortcut,
                enabled: root.shell.commandEnabled(c.id),
                reason: root.shell.commandReason(c.id)
            });
        }
        root.rows = out;
        list.currentIndex = out.length > 0 ? 0 : -1;
    }

    function run() {
        if (list.currentIndex < 0) {
            return;
        }
        var row = root.rows[list.currentIndex];
        if (!row.enabled) {
            return;
        }
        root.close();
        root.shell.runCommand(row.id, root.win);
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    modal: true
    focus: true
    width: Spacing.size480
    height: Spacing.size420
    padding: Theme.spaceMd
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    onOpened: {
        field.text = "";
        root.refresh();
        field.forceActiveFocus();
    }

    background: Rectangle {
        color: Theme.surfaceRaised
        border.color: Theme.accent
        border.width: Theme.focusBorderWidth
        radius: Spacing.radiusLarge
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceSm

        AppTextField {
            id: field
            Layout.fillWidth: true
            placeholderText: "Type a command"
            onTextChanged: root.refresh()
            onAccepted: root.run()
            Keys.onDownPressed: list.incrementCurrentIndex()
            Keys.onUpPressed: list.decrementCurrentIndex()
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.rows
            currentIndex: 0
            delegate: Rectangle {
                id: row
                required property var modelData
                required property int index
                width: list.width
                height: Spacing.size32
                color: row.index === list.currentIndex ? Theme.surfaceSelected : Theme.transparent

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.spaceMd
                    anchors.rightMargin: Theme.spaceMd
                    Text {
                        Layout.fillWidth: true
                        text: row.modelData.label + (row.modelData.enabled || row.modelData.reason === "" ? "" : "  (" + row.modelData.reason + ")")
                        color: row.modelData.enabled ? Theme.textStrong : Theme.textMuted
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeBodySmall
                        elide: Text.ElideRight
                    }
                    Text {
                        text: row.modelData.shortcut
                        color: Theme.textSecondary
                        font.family: Theme.numericFontFamily
                        font.pixelSize: Theme.typeLabel
                    }
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: {
                        list.currentIndex = row.index;
                        root.run();
                    }
                }
            }
        }
    }
}
