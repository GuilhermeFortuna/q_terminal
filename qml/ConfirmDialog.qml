pragma ComponentBehavior: Bound
import QtQuick
import qml
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root
    modal: true
    focus: true
    standardButtons: Dialog.NoButton
    anchors.centerIn: parent

    property string actionTitle: ""
    property string consequenceText: ""
    property bool liveWarning: false
    property string actionId: ""

    signal confirmed()
    signal cancelled()

    onRejected: root.cancelled()

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.size12

        Text {
            text: root.actionTitle
            font.pixelSize: Theme.typeBodyLarge
            font.bold: true
            color: Theme.textStrong
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Rectangle {
            visible: root.liveWarning
            Layout.fillWidth: true
            height: liveBanner.implicitHeight + 16
            radius: Spacing.size4
            color: Theme.criticalSurface
            border.color: Theme.negativeStrong

            Text {
                id: liveBanner
                anchors.centerIn: parent
                width: parent.width - 24
                text: "LIVE DEPLOYMENT — real broker orders"
                color: Theme.negativeSoft
                font.pixelSize: Theme.typeBody
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }

        Text {
            text: root.consequenceText
            font.pixelSize: Theme.typeBody
            color: Theme.textPrimary
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Spacing.size8
            Item { Layout.fillWidth: true }

            Button {
                text: "Cancel"
                focusPolicy: Qt.StrongFocus
                onClicked: {
                    root.reject();
                    root.cancelled();
                }
            }

            Button {
                text: "Confirm"
                focusPolicy: Qt.NoFocus
                highlighted: false
                onClicked: {
                    root.accept();
                    root.confirmed();
                }
            }
        }
    }
}
