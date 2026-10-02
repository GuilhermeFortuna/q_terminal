pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml

// The content of a trade tape (Q-082), shared by the panel and the gallery: the source
// state, display filters and the newest-first list of prints. Layout adapted from Quantower
// Time & Sales. Every number comes from Rust; the filters only choose which prints the list
// shows and never reach the volume studies.
Item {
    id: root

    required property var tape
    signal filterEdited()

    // Filters persist per panel through the workspace.
    function filterJson() {
        return root.tape.filter_json();
    }
    function restoreFilter(json) {
        root.tape.restore_filter_json(json);
        sideBox.currentIndex = Math.max(0, root.sides.indexOf(root.tape.side));
        volumeField.text = root.tape.min_volume > 0 ? String(root.tape.min_volume) : "";
    }

    // A narrow panel keeps the four columns and drops the words around them.
    readonly property bool compact: root.width < Spacing.size320
    readonly property var sides: ["all", "buy", "sell", "unknown"]
    readonly property var report: {
        try {
            return JSON.parse(root.tape.state_json);
        } catch (e) {
            return ({});
        }
    }
    readonly property bool hasSource: report.phase !== undefined && report.phase !== "idle"
    readonly property string stateRole: {
        if (report.phase === "unavailable") return Semantic.critical;
        if (report.capacity_exhausted) return Semantic.warning;
        if (report.stale || report.loading) return Semantic.stale;
        return report.complete ? Semantic.positive : Semantic.warning;
    }

    function applyFilter() {
        var raw = volumeField.text.trim();
        var volume = raw.length === 0 ? 0 : Number(raw);
        if (isNaN(volume) || volume < 0) {
            return;
        }
        if (root.tape.set_filter(volume, root.sides[sideBox.currentIndex])) {
            root.filterEdited();
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Spacing.none

        // Source state: what the tape is, how complete, in which unit.
        Flow {
            id: stateRow
            objectName: "tapeState"
            Layout.fillWidth: true
            Layout.margins: Theme.spaceMd
            spacing: Theme.spaceSm

            StatusBadge {
                objectName: "tapeStateBadge"
                role: root.stateRole
                text: (root.report.state_text || "No source").toUpperCase()
            }
            StatusBadge {
                visible: root.hasSource && !!root.report.coverage && root.report.coverage.state !== undefined && root.report.coverage.state !== ""
                objectName: "tapeCoverageBadge"
                role: root.report.coverage && root.report.coverage.state === "complete" ? Semantic.neutral : Semantic.warning
                text: qsTr("COVERAGE ") + String(root.report.coverage ? root.report.coverage.state : "").toUpperCase()
            }
            StatusBadge {
                visible: root.hasSource && !!root.report.volume_unit
                objectName: "tapeUnitBadge"
                text: String(root.report.volume_field || "").toUpperCase() + " · " + String(root.report.volume_unit || "")
            }
            StatusBadge {
                visible: root.hasSource && root.report.classified_share !== null && root.report.classified_share !== undefined
                objectName: "tapeClassifiedBadge"
                role: root.report.classified_share === 0 ? Semantic.warning : Semantic.neutral
                text: qsTr("SIDE KNOWN ") + Math.round((root.report.classified_share || 0) * 100) + "%" + (root.report.loading ? qsTr(" · PROVISIONAL") : "")
            }
        }

        // Progress while the session snapshot loads.
        Rectangle {
            visible: root.report.loading === true
            Layout.fillWidth: true
            Layout.preferredHeight: Spacing.size3
            color: Theme.surfaceOverlay

            Rectangle {
                width: parent.width * (root.report.progress || 0)
                height: parent.height
                color: Theme.accent
            }
        }

        // Why the numbers cannot be trusted as complete, in words.
        Repeater {
            model: root.report.notes || []
            delegate: Text {
                required property var modelData
                objectName: "tapeNote"
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spaceMd
                Layout.rightMargin: Theme.spaceMd
                text: modelData.text
                wrapMode: Text.WordWrap
                color: Semantic.foreground(modelData.code === "warming_up" ? Semantic.neutral : Semantic.warning)
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }
        }

        AppButton {
            visible: root.report.capacity_exhausted === true
            objectName: "tapeRetry"
            Layout.leftMargin: Theme.spaceMd
            Layout.topMargin: Theme.spaceXs
            text: qsTr("Reload session")
            toolTip: qsTr("Load the session again from a fresh snapshot")
            onClicked: root.tape.retry()
        }

        // Display filters.
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.spaceMd
            spacing: Theme.spaceSm

            Text {
                visible: !root.compact
                text: qsTr("MIN VOLUME")
                color: Theme.textMuted
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabelSmall
                font.weight: Typography.weightBold
            }
            AppTextField {
                id: volumeField
                objectName: "tapeMinVolume"
                numeric: true
                Layout.preferredWidth: Spacing.size80
                placeholderText: root.compact ? qsTr("min vol") : "0"
                inputMethodHints: Qt.ImhFormattedNumbersOnly
                onEditingFinished: root.applyFilter()
            }
            AppComboBox {
                id: sideBox
                objectName: "tapeSide"
                Layout.preferredWidth: Spacing.size90
                model: [qsTr("All"), qsTr("Buy"), qsTr("Sell"), qsTr("Unknown")]
                onActivated: root.applyFilter()
            }
            Item {
                Layout.fillWidth: true
            }
            Text {
                visible: !root.compact
                objectName: "tapeRowCount"
                text: root.tape.row_count + qsTr(" shown")
                color: Theme.textMuted
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }
        }

        // Column header.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: Theme.tableHeaderHeight
            color: Theme.surfaceOverlay
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceMd
                anchors.rightMargin: Theme.spaceMd
                spacing: Theme.spaceMd

                Text {
                    Layout.preferredWidth: Spacing.size90
                    text: qsTr("TIME")
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    font.weight: Typography.weightBold
                }
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: Spacing.size48
                    horizontalAlignment: Text.AlignRight
                    text: qsTr("PRICE")
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    font.weight: Typography.weightBold
                }
                Text {
                    Layout.fillWidth: true
                    Layout.minimumWidth: Spacing.size40
                    horizontalAlignment: Text.AlignRight
                    text: qsTr("VOLUME")
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    font.weight: Typography.weightBold
                }
                Text {
                    Layout.preferredWidth: Spacing.size40
                    text: qsTr("SIDE")
                    color: Theme.textMuted
                    font.family: Theme.uiFont
                    font.pixelSize: Theme.typeLabel
                    font.weight: Typography.weightBold
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: list
                objectName: "tapeList"
                anchors.fill: parent
                clip: true
                model: root.tape.rows
                boundsBehavior: Flickable.StopAtBounds
                reuseItems: true

                delegate: Rectangle {
                    id: row
                    required property int index
                    required property string time
                    required property string price
                    required property string volume
                    required property string side
                    required property bool large
                    width: list.width
                    height: Theme.tableRowHeight
                    color: row.large ? Semantic.background(Semantic.aggressor(row.side)) : (row.index % 2 === 0 ? Theme.surfaceBase : Theme.surfaceRaised)

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: Theme.spaceMd
                        anchors.rightMargin: Theme.spaceMd
                        spacing: Theme.spaceMd

                        Text {
                            Layout.preferredWidth: Spacing.size90
                            text: row.time
                            color: Theme.textSecondary
                            font.family: Theme.numericFontFamily
                            font.pixelSize: Theme.typeBodySmall
                            font.features: { "tnum": 1 }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.minimumWidth: Spacing.size48
                            horizontalAlignment: Text.AlignRight
                            text: row.price
                            color: Semantic.foreground(Semantic.aggressor(row.side))
                            font.family: Theme.numericFontFamily
                            font.pixelSize: Theme.typeBodySmall
                            font.features: { "tnum": 1 }
                        }
                        Text {
                            Layout.fillWidth: true
                            Layout.minimumWidth: Spacing.size40
                            horizontalAlignment: Text.AlignRight
                            text: row.volume
                            color: row.large ? Theme.textStrong : Theme.textPrimary
                            font.family: Theme.numericFontFamily
                            font.pixelSize: Theme.typeBodySmall
                            font.weight: row.large ? Typography.weightBold : Typography.weightRegular
                            font.features: { "tnum": 1 }
                        }
                        Text {
                            Layout.preferredWidth: Spacing.size40
                            text: row.side === "unknown" ? "—" : row.side.toUpperCase()
                            color: Semantic.foreground(Semantic.aggressor(row.side))
                            font.family: Theme.uiFont
                            font.pixelSize: Theme.typeLabel
                            font.weight: Typography.weightBold
                        }
                        Text {
                            visible: row.large && !root.compact
                            text: qsTr("LARGE")
                            color: Semantic.foreground(Semantic.accent)
                            font.family: Theme.uiFont
                            font.pixelSize: Theme.typeLabelSmall
                            font.weight: Typography.weightBold
                        }
                    }
                }
            }

            EmptyState {
                anchors.centerIn: parent
                visible: list.count === 0
                width: Math.min(parent.width - Theme.spaceXl, Spacing.dialogCompactWidth)
                padding: Theme.spaceMd
                title: root.report.phase === "unavailable" ? qsTr("Trade source unavailable") : (root.report.loading ? qsTr("Loading the session tape") : qsTr("No prints to show"))
                detail: root.report.phase === "unavailable" ? qsTr("The backend has no trade source for this symbol.") : (root.report.loading ? qsTr("Prints appear when the session snapshot is in.") : qsTr("Nothing matches the filter, or the session has no trades yet."))
            }
        }
    }
}
