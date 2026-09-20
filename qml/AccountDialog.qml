pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root
    title: "Create paper account"
    modal: true
    standardButtons: Dialog.NoButton
    anchors.centerIn: parent

    property string actionId: ""

    signal createRequested(string name, string balance, string currency)

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        TextField {
            id: nameField
            Layout.fillWidth: true
            placeholderText: "Account name"
            focus: true
        }

        TextField {
            id: balanceField
            Layout.fillWidth: true
            placeholderText: "Initial balance"
            text: "100000"
        }

        TextField {
            id: currencyField
            Layout.fillWidth: true
            placeholderText: "Currency"
            text: "BRL"
        }

        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            Button {
                text: "Cancel"
                onClicked: root.reject()
            }
            Button {
                text: "Create"
                enabled: nameField.text.trim() !== ""
                onClicked: {
                    root.createRequested(nameField.text.trim(), balanceField.text.trim(), currencyField.text.trim());
                    root.accept();
                }
            }
        }
    }
}
