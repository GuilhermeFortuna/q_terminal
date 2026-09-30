pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml

// Comprehensive live study picker and active indicator manager (Q-075).
Rectangle {
    id: root

    property var feed: null
    property string previewState: "rest"

    implicitHeight: Theme.controlHeight
    implicitWidth: buttonFrame.implicitWidth
    color: Theme.transparent

    // Cached study data
    property var _cachedStudies: []
    property string _vwapUnavailable: ""

    function syncPalette() {
        if (root.feed) {
            root.feed.set_study_palette_json(StudyPalette.paletteJson());
        }
    }

    function refreshStudies() {
        if (root.previewState !== "rest") {
            applyPreviewState();
            return;
        }
        if (!root.feed) {
            root._cachedStudies = [];
            root._vwapUnavailable = "";
            return;
        }
        try {
            var raw = root.feed.study_list_json();
            var parsed = JSON.parse(raw);
            root._cachedStudies = parsed.studies || [];
            root._vwapUnavailable = parsed.vwap_unavailable || "";
        } catch (e) {
            root._cachedStudies = [];
            root._vwapUnavailable = "";
        }
    }

    readonly property int activeCount: root._cachedStudies.length
    readonly property var activeStudies: root._cachedStudies
    readonly property string vwapUnavailableReason: root._vwapUnavailable

    function addStudy(kind, period, source, numStd) {
        if (!root.feed || root.activeCount >= 8) {
            return;
        }
        root.feed.add_study(kind, period, source || "close", numStd !== undefined ? numStd : 2.0);
        refreshStudies();
    }

    function removeStudy(id) {
        if (!root.feed) {
            return;
        }
        root.feed.remove_study(id);
        refreshStudies();
    }

    function adjustPeriod(study, delta) {
        if (!root.feed) {
            return;
        }
        var newPeriod = Math.max(1, study.period + delta);
        if (newPeriod === study.period) {
            return;
        }
        root.feed.update_study(study.id, newPeriod, study.source, study.num_std);
        refreshStudies();
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "open":
            root._cachedStudies = [
                { id: 1, kind: "ema", name: "EMA (20)", period: 20, source: "close", num_std: 2.0, palette_index: 0 },
                { id: 2, kind: "bollinger", name: "Bollinger (20, 2σ)", period: 20, source: "close", num_std: 2.0, palette_index: 1 }
            ];
            root._vwapUnavailable = "";
            studyPopup.open();
            break;
        case "max-studies":
            var mock = [];
            for (var i = 0; i < 8; i++) {
                mock.push({
                    id: i + 1,
                    kind: i % 2 === 0 ? "ema" : "sma",
                    name: (i % 2 === 0 ? "EMA" : "SMA") + " (" + (10 + i * 5) + ")",
                    period: 10 + i * 5,
                    source: "close",
                    num_std: 2.0,
                    palette_index: i
                });
            }
            root._cachedStudies = mock;
            root._vwapUnavailable = "";
            studyPopup.open();
            break;
        default:
            break;
        }
    }

    Component.onCompleted: {
        syncPalette();
        refreshStudies();
        if (root.previewState !== "rest") {
            applyPreviewState();
        }
    }

    onFeedChanged: {
        syncPalette();
        refreshStudies();
    }

    onPreviewStateChanged: applyPreviewState()

    Connections {
        target: root.feed
        function onOverlay_revisionChanged() {
            root.refreshStudies();
        }
    }

    // Trigger button in the chart header toolbar
    Rectangle {
        id: buttonFrame
        objectName: "studyPickerButton"
        implicitHeight: Theme.controlHeight
        implicitWidth: buttonContent.implicitWidth + Spacing.size16
        radius: Theme.radiusMedium
        color: btnMouse.containsPress ? Theme.accentPressed : (btnMouse.containsMouse ? Theme.surfaceHover : Theme.surfaceBase)
        border.color: (studyPopup && studyPopup.opened) ? Theme.accent : (btnMouse.containsMouse ? Theme.borderDefault : Theme.borderSubtle)
        border.width: Theme.borderWidth

        RowLayout {
            id: buttonContent
            anchors.centerIn: parent
            spacing: Spacing.size6

            Image {
                source: Icons.pulse
                sourceSize.width: Spacing.iconMedium
                sourceSize.height: Spacing.iconMedium
                Layout.alignment: Qt.AlignVCenter
                opacity: root.feed ? 1.0 : 0.45
            }

            Text {
                text: qsTr("Studies")
                color: root.activeCount > 0 ? Theme.textPrimary : Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeBodySmall
                Layout.alignment: Qt.AlignVCenter
            }

            Rectangle {
                visible: root.activeCount > 0
                implicitWidth: countText.implicitWidth + Spacing.size8
                implicitHeight: Spacing.size16
                radius: Spacing.radiusSmall
                color: Theme.surfaceSelected
                Layout.alignment: Qt.AlignVCenter

                Text {
                    id: countText
                    anchors.centerIn: parent
                    text: root.activeCount.toString()
                    color: Theme.accent
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                }
            }
        }

        MouseArea {
            id: btnMouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            enabled: root.feed !== null || root.previewState !== "rest"
            onClicked: {
                if (studyPopup.opened) {
                    studyPopup.close();
                } else {
                    studyPopup.open();
                }
            }
        }
    }

    Popup {
        id: studyPopup
        objectName: "studyManagerPopup"
        y: root.height + Spacing.size4
        x: 0
        width: Spacing.size320
        padding: Spacing.size12
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.surfaceElevated
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            radius: Theme.radiusMedium
        }

        contentItem: ColumnLayout {
            spacing: Spacing.size8

            // Header row
            RowLayout {
                Layout.fillWidth: true
                spacing: Spacing.size8

                Text {
                    text: qsTr("CHART STUDIES")
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                    Layout.fillWidth: true
                }

                Text {
                    text: root.activeCount + " / 8"
                    color: root.activeCount >= 8 ? Theme.warningStrong : Theme.textTertiary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                }

                Rectangle {
                    implicitWidth: Spacing.size20
                    implicitHeight: Spacing.size20
                    radius: Theme.radiusSmall
                    color: closeHover.hovered ? Theme.surfaceHover : Theme.transparent
                    border.color: Theme.borderSubtle
                    border.width: Theme.borderWidth

                    Text {
                        anchors.centerIn: parent
                        text: "×"
                        color: Theme.textSecondary
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeLabel
                    }

                    HoverHandler {
                        id: closeHover
                        cursorShape: Qt.PointingHandCursor
                    }

                    TapHandler {
                        onTapped: studyPopup.close()
                    }
                }
            }

            // Active Studies List (if any)
            ColumnLayout {
                visible: root.activeCount > 0
                Layout.fillWidth: true
                spacing: Spacing.size4

                Text {
                    text: qsTr("ACTIVE")
                    color: Theme.textTertiary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                    topPadding: Spacing.size2
                }

                Repeater {
                    model: root.activeStudies
                    delegate: Rectangle {
                        id: activeRow
                        required property var modelData
                        Layout.fillWidth: true
                        implicitHeight: Spacing.size28
                        radius: Theme.radiusSmall
                        color: activeRowHover.hovered ? Theme.surfaceSelected : Theme.surfaceRaised
                        border.color: Theme.borderSubtle
                        border.width: Theme.borderWidth

                        HoverHandler {
                            id: activeRowHover
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: Spacing.size8
                            anchors.rightMargin: Spacing.size6
                            spacing: Spacing.size6

                            Rectangle {
                                width: Spacing.size10
                                height: Spacing.size10
                                radius: Spacing.radiusSmall
                                color: StudyPalette.colors[activeRow.modelData.palette_index % 8]
                                Layout.alignment: Qt.AlignVCenter
                            }

                            Text {
                                text: activeRow.modelData.name || (activeRow.modelData.kind.toUpperCase() + " " + activeRow.modelData.period)
                                color: Theme.textPrimary
                                font.family: Theme.uiFont
                                font.pixelSize: Theme.typeLabel
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                                Layout.alignment: Qt.AlignVCenter
                            }

                            // Stepper controls for studies with variable period
                            RowLayout {
                                visible: activeRow.modelData.kind !== "vwap"
                                spacing: Spacing.size2
                                Layout.alignment: Qt.AlignVCenter

                                Rectangle {
                                    implicitWidth: Spacing.size16
                                    implicitHeight: Spacing.size16
                                    radius: Spacing.radiusSmall
                                    color: decHover.hovered ? Theme.surfaceHover : Theme.transparent
                                    border.color: Theme.borderSubtle
                                    border.width: Theme.borderWidth

                                    Text {
                                        anchors.centerIn: parent
                                        text: "−"
                                        color: Theme.textSecondary
                                        font.family: Theme.uiFont
                                        font.pixelSize: Theme.typeLabelSmall
                                    }

                                    HoverHandler {
                                        id: decHover
                                        cursorShape: Qt.PointingHandCursor
                                    }

                                    TapHandler {
                                        onTapped: root.adjustPeriod(activeRow.modelData, -1)
                                    }
                                }

                                Rectangle {
                                    implicitWidth: Spacing.size16
                                    implicitHeight: Spacing.size16
                                    radius: Spacing.radiusSmall
                                    color: incHover.hovered ? Theme.surfaceHover : Theme.transparent
                                    border.color: Theme.borderSubtle
                                    border.width: Theme.borderWidth

                                    Text {
                                        anchors.centerIn: parent
                                        text: "+"
                                        color: Theme.textSecondary
                                        font.family: Theme.uiFont
                                        font.pixelSize: Theme.typeLabelSmall
                                    }

                                    HoverHandler {
                                        id: incHover
                                        cursorShape: Qt.PointingHandCursor
                                    }

                                    TapHandler {
                                        onTapped: root.adjustPeriod(activeRow.modelData, 1)
                                    }
                                }
                            }

                            Rectangle {
                                implicitWidth: Spacing.size18
                                implicitHeight: Spacing.size18
                                radius: Spacing.radiusSmall
                                color: removeHover.hovered ? Theme.surfaceHover : Theme.transparent
                                border.color: Theme.borderSubtle
                                border.width: Theme.borderWidth

                                Text {
                                    anchors.centerIn: parent
                                    text: "×"
                                    color: Theme.textSecondary
                                    font.family: Theme.uiFont
                                    font.pixelSize: Theme.typeLabelSmall
                                }

                                HoverHandler {
                                    id: removeHover
                                    cursorShape: Qt.PointingHandCursor
                                }

                                TapHandler {
                                    onTapped: root.removeStudy(activeRow.modelData.id)
                                }
                            }
                        }
                    }
                }
            }

            // VWAP unavailable warning banner
            Rectangle {
                visible: root.vwapUnavailableReason.length > 0
                Layout.fillWidth: true
                implicitHeight: vwapCol.implicitHeight + Spacing.size12
                radius: Theme.radiusSmall
                color: Theme.surfaceHover
                border.color: Theme.warningStrong
                border.width: Theme.borderWidth

                ColumnLayout {
                    id: vwapCol
                    anchors.fill: parent
                    anchors.margins: Spacing.size6
                    spacing: Spacing.size2

                    Text {
                        text: qsTr("VWAP UNAVAILABLE")
                        color: Theme.warningStrong
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeLabelSmall
                    }
                    Text {
                        text: root.vwapUnavailableReason
                        color: Theme.textSecondary
                        font.family: Theme.uiFont
                        font.pixelSize: Theme.typeLabelSmall
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }
            }

            // Subtle divider
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Spacing.hairline
                color: Theme.borderSubtle
            }

            // Add Available Study Section
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Spacing.size2

                Text {
                    text: qsTr("AVAILABLE STUDIES")
                    color: Theme.textTertiary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                    topPadding: Spacing.size2
                    bottomPadding: Spacing.size2
                }

                StudyAddRow {
                    label: qsTr("EMA (20)")
                    subtitle: qsTr("Exponential Moving Average")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("ema", 20, "close", 2.0)
                }
                StudyAddRow {
                    label: qsTr("SMA (20)")
                    subtitle: qsTr("Simple Moving Average")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("sma", 20, "close", 2.0)
                }
                StudyAddRow {
                    label: qsTr("Bollinger (20, 2σ)")
                    subtitle: qsTr("Volatility bands on moving average")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("bollinger", 20, "close", 2.0)
                }
                StudyAddRow {
                    label: qsTr("Session VWAP (2σ)")
                    subtitle: qsTr("Daily volume-weighted price & bands")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("vwap", 1, "close", 2.0)
                }
                StudyAddRow {
                    label: qsTr("RSI (14)")
                    subtitle: qsTr("Relative Strength Index (0–100 oscillator)")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("rsi", 14, "close", 2.0)
                }
                StudyAddRow {
                    label: qsTr("ATR (14)")
                    subtitle: qsTr("Average True Range (volatility oscillator)")
                    enabled: root.activeCount < 8
                    onActivated: root.addStudy("atr", 14, "close", 2.0)
                }
            }
        }
    }

    component StudyAddRow: Button {
        id: addRow
        required property string label
        required property string subtitle
        signal activated()

        Layout.fillWidth: true
        implicitHeight: Spacing.size36
        hoverEnabled: true

        HoverHandler {
            cursorShape: addRow.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        }

        background: Rectangle {
            radius: Theme.radiusSmall
            color: addRow.down ? Theme.accentPressed : (addRow.hovered ? Theme.surfaceSelected : Theme.transparent)
            border.color: addRow.visualFocus ? Theme.accent : Theme.transparent
            border.width: Theme.borderWidth
        }

        contentItem: RowLayout {
            spacing: Spacing.size6

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Spacing.none

                Text {
                    text: addRow.label
                    color: Theme.textPrimary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    elide: Text.ElideRight
                }

                Text {
                    text: addRow.subtitle
                    color: Theme.textTertiary
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabelSmall
                    elide: Text.ElideRight
                }
            }

            Text {
                text: "+"
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeBody
                font.weight: Typography.weightMedium
                Layout.alignment: Qt.AlignVCenter
            }
        }

        onClicked: addRow.activated()
    }
}
