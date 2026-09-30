pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Compact study add/remove control (Q-075), adapted from TradingView indicator legend density.
Rectangle {
    id: root

    property var feed: null
    property string previewState: "rest"

    implicitHeight: Theme.controlHeight
    color: Theme.transparent

    function syncPalette() {
        if (root.feed) {
            root.feed.set_study_palette_json(StudyPalette.paletteJson());
        }
    }

    function studyCount() {
        try {
            var data = JSON.parse(root.feed ? root.feed.study_list_json() : "{}");
            return data.studies ? data.studies.length : 0;
        } catch (e) {
            return 0;
        }
    }

    Component.onCompleted: syncPalette()
    onFeedChanged: syncPalette()

    IconButton {
        id: addBtn
        objectName: "studyAddButton"
        iconSource: Icons.plus
        ToolTip.text: qsTr("Add study")
        enabled: root.feed && studyCount() < 8
        onClicked: menu.open()
    }

    Menu {
        id: menu
        title: qsTr("Add study")

        MenuItem {
            text: qsTr("EMA (20)")
            onTriggered: root.feed.add_study("ema", 20, "close", 2.0)
        }
        MenuItem {
            text: qsTr("SMA (20)")
            onTriggered: root.feed.add_study("sma", 20, "close", 2.0)
        }
        MenuItem {
            text: qsTr("Bollinger (20, 2σ)")
            onTriggered: root.feed.add_study("bollinger", 20, "close", 2.0)
        }
        MenuItem {
            text: qsTr("Session VWAP (2σ)")
            onTriggered: root.feed.add_study("vwap", 1, "close", 2.0)
        }
        MenuItem {
            text: qsTr("RSI (14)")
            onTriggered: root.feed.add_study("rsi", 14, "close", 2.0)
        }
        MenuItem {
            text: qsTr("ATR (14)")
            onTriggered: root.feed.add_study("atr", 14, "close", 2.0)
        }
    }

    Popup {
        id: listPopup
        x: addBtn.x
        y: addBtn.height + Spacing.size4
        width: Math.max(addBtn.width, Spacing.size200)
        padding: Spacing.size8

        contentItem: ColumnLayout {
            spacing: Spacing.size4

            Repeater {
                model: {
                    try {
                        return JSON.parse(root.feed ? root.feed.study_list_json() : "{}").studies || [];
                    } catch (e) {
                        return [];
                    }
                }
                delegate: RowLayout {
                    Layout.fillWidth: true
                    spacing: Spacing.size8

                    Rectangle {
                        width: Spacing.size12
                        height: Spacing.size12
                        radius: Spacing.radiusSmall
                        color: StudyPalette.colors[modelData.palette_index % 8]
                    }

                    Text {
                        Layout.fillWidth: true
                        text: modelData.kind.toUpperCase() + " " + modelData.period
                        color: Theme.textPrimary
                        font.pixelSize: Theme.typeLabel
                        elide: Text.ElideRight
                    }

                    IconButton {
                        iconSource: Icons.close
                        onClicked: root.feed.remove_study(modelData.id)
                    }
                }
            }

            Text {
                visible: root.feed && JSON.parse(root.feed.study_list_json()).vwap_unavailable
                text: JSON.parse(root.feed.study_list_json()).vwap_unavailable || ""
                color: Theme.warningStrong
                font.pixelSize: Theme.typeCaption
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    Connections {
        target: root.feed
        function onOverlay_revisionChanged() {
            if (studyCount() > 0) {
                listPopup.open();
            }
        }
    }
}
