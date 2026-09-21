pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls as Controls
import qml

// Renders one node of a window's arrangement: a split of panes, or a tab group of panels
// (Q-052). Panels are not created here. Each slot borrows the shell's persistent panel
// item by reparenting it, which is what lets a panel move between windows with its state.
Item {
    id: root

    required property var shell
    required property string windowId
    required property var node
    property var path: []

    Loader {
        anchors.fill: parent
        sourceComponent: !root.node ? null : (root.node.type === "split" ? splitComponent : tabsComponent)
    }

    Component {
        id: splitComponent

        Controls.SplitView {
            id: split
            readonly property bool horizontal: root.node.orientation === "horizontal"
            readonly property real total: root.node.weights.reduce(function (a, b) {
                return a + b;
            }, 0)
            orientation: split.horizontal ? Qt.Horizontal : Qt.Vertical

            handle: Rectangle {
                implicitWidth: Spacing.size4
                implicitHeight: Spacing.size4
                color: Controls.SplitHandle.pressed ? Theme.accent : (Controls.SplitHandle.hovered ? Theme.borderStrong : Theme.surfaceSelected)
            }

            // Pane sizes are reported once a drag ends; the window keeps them while it is open.
            onResizingChanged: {
                if (split.resizing) {
                    return;
                }
                var sizes = [];
                for (var i = 0; i < panes.count; ++i) {
                    var item = panes.itemAt(i);
                    sizes.push(split.horizontal ? item.width : item.height);
                }
                root.shell.controller.set_weights(root.windowId, JSON.stringify(root.path), JSON.stringify(sizes));
            }

            Repeater {
                id: panes
                model: root.node.children

                // QML cannot instantiate a type from inside its own file by name, so a child
                // node is loaded by URL.
                delegate: Loader {
                    id: pane
                    required property var modelData
                    required property int index
                    readonly property bool last: pane.index === root.node.children.length - 1
                    readonly property real share: root.node.weights[pane.index] / split.total
                    Controls.SplitView.fillWidth: split.horizontal && pane.last
                    Controls.SplitView.fillHeight: !split.horizontal && pane.last
                    Controls.SplitView.preferredWidth: split.horizontal ? split.width * pane.share : -1
                    Controls.SplitView.preferredHeight: split.horizontal ? -1 : split.height * pane.share
                    Controls.SplitView.minimumWidth: split.horizontal ? Spacing.size160 : 0
                    Controls.SplitView.minimumHeight: split.horizontal ? 0 : Spacing.size60

                    Component.onCompleted: pane.setSource("PanelHost.qml", {
                        shell: root.shell,
                        windowId: root.windowId,
                        node: pane.modelData,
                        path: root.path.concat([pane.index])
                    })
                    onLoaded: {
                        pane.item.width = Qt.binding(function () {
                            return pane.width;
                        });
                        pane.item.height = Qt.binding(function () {
                            return pane.height;
                        });
                    }
                }
            }
        }
    }

    Component {
        id: tabsComponent

        Item {
            id: tabs
            property int active: root.node.active

            Controls.TabBar {
                id: strip
                visible: root.node.panels.length > 1
                height: strip.visible ? Spacing.controlHeight : Spacing.none
                background: Rectangle {
                    color: Theme.surfaceOverlay
                    border.color: Theme.borderDefault
                    border.width: Theme.borderWidth
                }
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.right: parent.right
                currentIndex: tabs.active
                onCurrentIndexChanged: {
                    if (strip.currentIndex !== tabs.active && strip.currentIndex >= 0) {
                        tabs.active = strip.currentIndex;
                        root.shell.controller.set_active(root.windowId, root.node.panels[strip.currentIndex]);
                    }
                }

                Repeater {
                    model: root.node.panels
                    delegate: Controls.TabButton {
                        id: tab
                        required property string modelData
                        text: root.shell.panelSpec(tab.modelData).title
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeBodySmall
                        contentItem: Text {
                            text: tab.text
                            color: tab.checked ? Theme.accent : Theme.textSecondary
                            font: tab.font
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                        }
                        background: Rectangle {
                            color: tab.hovered ? Theme.surfaceHover : Theme.transparent
                            border.color: tab.activeFocus ? Theme.accent : Theme.transparent
                            border.width: tab.activeFocus ? Theme.focusBorderWidth : Spacing.none
                        }
                    }
                }
            }

            Item {
                id: area
                anchors.top: strip.bottom
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right

                Repeater {
                    model: root.node.panels
                    delegate: Item {
                        id: slot
                        required property string modelData
                        required property int index
                        anchors.fill: parent
                        visible: slot.index === tabs.active
                        Component.onCompleted: {
                            var panel = root.shell.panelItem(slot.modelData);
                            panel.parent = slot;
                            panel.anchors.fill = slot;
                        }
                    }
                }
            }
        }
    }
}
