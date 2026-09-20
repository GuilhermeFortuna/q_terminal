pragma ComponentBehavior: Bound
import QtQuick
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
        spacing: 12

        Text {
            text: root.actionTitle
            font.pixelSize: 14
            font.bold: true
            color: "#f8fafc"
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Rectangle {
            visible: root.liveWarning
            Layout.fillWidth: true
            height: liveBanner.implicitHeight + 16
            radius: 4
            color: "#7f1d1d"
            border.color: "#ef4444"

            Text {
                id: liveBanner
                anchors.centerIn: parent
                width: parent.width - 24
                text: "LIVE DEPLOYMENT — real broker orders"
                color: "#fca5a5"
                font.pixelSize: 12
                font.bold: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
            }
        }

        Text {
            text: root.consequenceText
            font.pixelSize: 12
            color: "#cbd5e1"
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
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
