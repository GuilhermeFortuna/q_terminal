pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml
import "Format.js" as Format

Dialog {
    id: root
    title: "New deployment"
    modal: true
    standardButtons: Dialog.NoButton
    width: Spacing.size480
    anchors.centerIn: parent

    required property ExecutionControls executionControls
    property string accountId: ""
    property string actionId: ""

    signal deployRequested(var payload)
    signal liveDeployConfirmed(var payload)

    property var savedRuns: []

    function refreshRuns() {
        root.executionControls.fetch_saved_runs();
    }

    Component.onCompleted: refreshRuns()

    Connections {
        target: root.executionControls
        function onSaved_runs_jsonChanged() {
            try {
                root.savedRuns = JSON.parse(root.executionControls.saved_runs_json);
            } catch (e) {
                root.savedRuns = [];
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size8

        TextField {
            id: nameField
            Layout.fillWidth: true
            placeholderText: "Deployment name"
        }

        ComboBox {
            id: brokerMode
            Layout.fillWidth: true
            model: ["paper", "mt5_live"]
        }

        Label { text: "Saved backtest run"; color: Theme.textSecondary; font.pixelSize: Theme.typeLabel }

        ListView {
            id: runList
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size160
            clip: true
            model: root.savedRuns
            currentIndex: -1

            delegate: Rectangle {
                required property var modelData
                required property int index
                width: runList.width
                height: Spacing.size44
                color: runList.currentIndex === index ? Theme.surfaceSelected : (mouseArea.containsMouse ? Theme.surfaceOverlay : Theme.surfaceBase)
                border.color: runList.currentIndex === index ? Theme.accent : Theme.borderDefault

                MouseArea {
                    id: mouseArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: runList.currentIndex = index
                }

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Spacing.size8
                    Text {
                        text: (modelData.strategy_name || "?") + " · " + (modelData.symbol || "?") + " " + (modelData.timeframe || "?")
                        color: Theme.textStrong
                        font.pixelSize: Theme.typeBodySmall
                        font.bold: true
                    }
                    Text {
                        text: modelData.saved_at ? Format.formatIsoTime(modelData.saved_at) : ""
                        color: Theme.textMuted
                        font.pixelSize: Theme.typeLabel
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            Button { text: "Cancel"; onClicked: root.reject() }
            Button {
                text: "Deploy"
                enabled: nameField.text.trim() !== "" && runList.currentIndex >= 0 && root.accountId !== ""
                onClicked: {
                    var run = root.savedRuns[runList.currentIndex];
                    var payload = {
                        paper_account_id: root.accountId,
                        name: nameField.text.trim(),
                        broker_mode: brokerMode.currentText,
                        source_backtest_run_id: run.id
                    };
                    if (brokerMode.currentText === "mt5_live") {
                        liveDeployConfirmed(payload);
                    } else {
                        deployRequested(payload);
                        root.accept();
                    }
                }
            }
        }
    }
}
