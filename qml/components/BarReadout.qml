pragma ComponentBehavior: Bound
import QtQuick
import qml

Rectangle {
    id: root

    property var snapshot: null
    property string pointerPrice: ""
    property string previewState: "rest"

    QtObject {
        id: preview
        property bool valid: true
        property int index: 0
        property double time: 1789725600000
        property string time_text: "2026-09-22 13:00 UTC"
        property real open: 10.0
        property real high: 11.5
        property real low: 9.5
        property real close: 11.0
        property string open_text: "10.00"
        property string high_text: "11.50"
        property string low_text: "9.50"
        property string close_text: "11.00"
        property bool forming: false
        property string pointer_price: ""
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "forming":
            preview.time_text = "2026-09-22 13:01 UTC";
            preview.open_text = "11.00";
            preview.high_text = "12.25";
            preview.low_text = "10.80";
            preview.close_text = "11.90";
            preview.forming = true;
            preview.pointer_price = "";
            break;
        case "hover":
            preview.time_text = "2026-09-22 13:00 UTC";
            preview.open_text = "10.00";
            preview.high_text = "11.50";
            preview.low_text = "9.50";
            preview.close_text = "11.00";
            preview.forming = false;
            preview.pointer_price = "10.75";
            break;
        default:
            preview.time_text = "2026-09-22 13:00 UTC";
            preview.open_text = "10.00";
            preview.high_text = "11.50";
            preview.low_text = "9.50";
            preview.close_text = "11.00";
            preview.forming = false;
            preview.pointer_price = "";
            break;
        }
    }

    Component.onCompleted: applyPreviewState()
    onPreviewStateChanged: applyPreviewState()

    // Gallery states render the preview fixture; the live chart passes previewState "".
    // The fixture never writes to pointerPrice, which would break the caller's binding.
    readonly property bool previewing: root.previewState !== "" && root.previewState !== "none"
    readonly property string effectivePointerPrice: root.previewing ? preview.pointer_price : root.pointerPrice
    readonly property var effectiveSnapshot: (root.snapshot && root.snapshot.valid) ? root.snapshot : (root.previewState !== "" && root.previewState !== "none" ? preview : null)
    readonly property bool hasValidData: (root.snapshot && root.snapshot.valid) || (root.previewState !== "rest" && root.previewState !== "" && root.previewState !== "none")

    readonly property string accessibleText: {
        if (!effectiveSnapshot) return "";
        var res = "Bar " + (effectiveSnapshot.time_text || "") +
                  (effectiveSnapshot.forming ? " forming" : "") +
                  " open " + (effectiveSnapshot.open_text || "--") +
                  " high " + (effectiveSnapshot.high_text || "--") +
                  " low " + (effectiveSnapshot.low_text || "--") +
                  " close " + (effectiveSnapshot.close_text || "--");
        if (root.effectivePointerPrice.length > 0) {
            res += " pointer " + root.effectivePointerPrice;
        }
        return res;
    }

    // Width available to the readout; 0 means unconstrained. When the values do not fit on
    // one line they wrap instead of being clipped.
    property real maximumWidth: 0
    readonly property real naturalRowWidth: {
        var groups = [timeTextItem, openGroup, highGroup, lowGroup, closeGroup, pointerPriceRow, formingBadge];
        var total = 0;
        var visibleCount = 0;
        for (var i = 0; i < groups.length; ++i) {
            if (groups[i].visible) {
                total += groups[i].implicitWidth;
                visibleCount += 1;
            }
        }
        return total + Math.max(0, visibleCount - 1) * readoutRow.spacing;
    }
    readonly property real rowWidth: root.maximumWidth > 0
        ? Math.min(root.naturalRowWidth, root.maximumWidth - Spacing.size16)
        : root.naturalRowWidth

    implicitHeight: Math.max(Spacing.size24, readoutRow.implicitHeight + Spacing.size8)
    implicitWidth: root.rowWidth + Spacing.size16
    color: Theme.surfaceElevated
    border.color: Theme.borderSubtle
    radius: Spacing.radiusSmall
    clip: true

    Accessible.role: Accessible.StaticText
    Accessible.name: accessibleText

    Flow {
        id: readoutRow
        anchors.left: parent.left
        anchors.leftMargin: Spacing.size8
        anchors.verticalCenter: parent.verticalCenter
        width: root.rowWidth
        spacing: Spacing.size8

        Text {
            id: timeTextItem
            objectName: "readoutTime"
            text: root.effectiveSnapshot ? (root.effectiveSnapshot.time_text || "") : ""
            color: Theme.textSecondary
            font.pixelSize: Theme.typeLabelSmall
            font.family: Theme.numericFontFamily
            font.features: { "tnum": 1 }
        }

        Row {
            id: openGroup
            spacing: Spacing.size4
            Text {
                text: "O"
                color: Theme.textMuted
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightMedium
            }
            Text {
                id: openTextItem
                objectName: "readoutOpen"
                text: root.effectiveSnapshot ? (root.effectiveSnapshot.open_text || "--") : "--"
                color: Theme.textPrimary
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
        }

        Row {
            id: highGroup
            spacing: Spacing.size4
            Text {
                text: "H"
                color: Theme.textMuted
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightMedium
            }
            Text {
                id: highTextItem
                objectName: "readoutHigh"
                text: root.effectiveSnapshot ? (root.effectiveSnapshot.high_text || "--") : "--"
                color: Theme.textPrimary
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
        }

        Row {
            id: lowGroup
            spacing: Spacing.size4
            Text {
                text: "L"
                color: Theme.textMuted
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightMedium
            }
            Text {
                id: lowTextItem
                objectName: "readoutLow"
                text: root.effectiveSnapshot ? (root.effectiveSnapshot.low_text || "--") : "--"
                color: Theme.textPrimary
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
        }

        Row {
            id: closeGroup
            spacing: Spacing.size4
            Text {
                text: "C"
                color: Theme.textMuted
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightMedium
            }
            Text {
                id: closeTextItem
                objectName: "readoutClose"
                text: root.effectiveSnapshot ? (root.effectiveSnapshot.close_text || "--") : "--"
                color: Theme.textPrimary
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
        }

        Row {
            id: pointerPriceRow
            visible: root.effectivePointerPrice !== ""
            spacing: Spacing.size4
            Text {
                text: "CUR"
                color: Theme.textMuted
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightMedium
            }
            Text {
                id: pointerPriceItem
                objectName: "readoutPointerPrice"
                text: root.effectivePointerPrice
                color: Theme.accent
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
        }

        Rectangle {
            id: formingBadge
            objectName: "readoutFormingBadge"
            visible: root.effectiveSnapshot ? !!root.effectiveSnapshot.forming : false
            height: Spacing.size16
            width: formingLabel.implicitWidth + Spacing.size8
            radius: Spacing.radiusSmall
            color: Theme.accentDark
            border.color: Theme.accent

            Text {
                id: formingLabel
                objectName: "readoutForming"
                anchors.centerIn: parent
                text: "FORMING"
                color: Theme.accent
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightBold
            }
        }
    }
}
