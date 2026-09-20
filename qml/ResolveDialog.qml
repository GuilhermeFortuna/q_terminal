pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root
    title: "Resolve unknown order"
    modal: true
    standardButtons: Dialog.NoButton
    width: 420
    anchors.centerIn: parent

    property string orderId: ""
    property string actionId: ""

    signal resolveRequested(var payload)

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        Text {
            text: "Order: " + root.orderId
            color: "#94a3b8"
            font.pixelSize: 10
            Layout.fillWidth: true
        }

        Label { text: "Outcome (required)"; color: "#94a3b8"; font.pixelSize: 10 }

        RowLayout {
            Layout.fillWidth: true
            RadioButton {
                id: filledRadio
                text: "Filled"
                checked: false
            }
            RadioButton {
                id: notFilledRadio
                text: "Not filled"
                checked: false
            }
        }

        TextField {
            id: reasonField
            Layout.fillWidth: true
            placeholderText: "Reason (required)"
        }

        TextField {
            id: priceField
            Layout.fillWidth: true
            placeholderText: "Price"
            visible: filledRadio.checked
        }

        TextField {
            id: qtyField
            Layout.fillWidth: true
            placeholderText: "Quantity"
            visible: filledRadio.checked
        }

        TextField {
            id: timeField
            Layout.fillWidth: true
            placeholderText: "Filled at (ISO-8601)"
            visible: filledRadio.checked
        }

        function canSubmit() {
            if (!filledRadio.checked && !notFilledRadio.checked) return false;
            if (reasonField.text.trim() === "") return false;
            if (filledRadio.checked) {
                return priceField.text.trim() !== "" && qtyField.text.trim() !== "" && timeField.text.trim() !== "";
            }
            return true;
        }

        RowLayout {
            Layout.fillWidth: true
            Item { Layout.fillWidth: true }
            Button { text: "Cancel"; onClicked: root.reject() }
            Button {
                text: "Submit"
                enabled: canSubmit()
                onClicked: {
                    var payload = {
                        order_id: root.orderId,
                        outcome: filledRadio.checked ? "filled" : "not_filled",
                        reason: reasonField.text.trim()
                    };
                    if (filledRadio.checked) {
                        payload.price = priceField.text.trim();
                        payload.quantity = qtyField.text.trim();
                        payload.filled_at = timeField.text.trim();
                    }
                    resolveRequested(payload);
                    root.accept();
                }
            }
        }
    }
}
