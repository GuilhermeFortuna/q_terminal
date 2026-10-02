import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import qml
import "../components/ComponentCatalog.js" as ComponentCatalog

ApplicationWindow {
    id: root
    objectName: "componentGallery"
    visible: true
    width: Spacing.workstationWidth
    height: Spacing.workstationHeight
    color: Theme.surfaceSunken
    title: "Q Terminal — component gallery"

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        ColumnLayout {
            width: parent.width
            spacing: Theme.spaceLg
            anchors.margins: Theme.spaceLg

            RowLayout {
                Layout.fillWidth: true
                ColumnLayout {
                    Text { text: "Q TERMINAL / DESIGN SYSTEM"; color: Theme.accent; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold }
                    Text { text: "Operational states, controls, type and assets"; color: Theme.textStrong; font.family: Theme.uiFont; font.pixelSize: Theme.typeHeading; font.weight: Typography.weightMedium }
                }
                Item { Layout.fillWidth: true }
                ConnectionIndicator { text: "GALLERY BUILD"; role: Semantic.positive }
            }

            Text { text: "VOLUME STUDIES (Q-082)"; color: Theme.accent; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold }
            RowLayout {
                Layout.fillWidth: true; spacing: Theme.spaceMd
                Repeater {
                    model: ["volume", "volume-stale"]
                    ColumnLayout {
                        id: volumeState
                        required property string modelData
                        Layout.fillWidth: true; spacing: Theme.spaceXs
                        Text { text: volumeState.modelData.toUpperCase(); color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        ChartPreview { previewState: volumeState.modelData; Layout.fillWidth: true; Layout.preferredHeight: Spacing.galleryChartHeight }
                    }
                }
            }

            Text { text: "TRADE TAPE (Q-082)"; color: Theme.accent; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold }
            GridLayout {
                Layout.fillWidth: true; columns: 4; columnSpacing: Theme.spaceMd; rowSpacing: Theme.spaceMd; uniformCellWidths: true
                Repeater {
                    model: ["live", "partial", "loading", "backfill", "stale", "capacity", "unavailable", "unknown"]
                    ColumnLayout {
                        id: tapeState
                        required property string modelData
                        Layout.fillWidth: true; spacing: Theme.spaceXs
                        Text { text: tapeState.modelData.toUpperCase(); color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        TapePreview { previewState: tapeState.modelData; Layout.fillWidth: true; Layout.preferredHeight: Spacing.galleryChartHeight }
                    }
                }
            }

            Text { text: "CHART STATES"; color: Theme.accent; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold }
            RowLayout {
                Layout.fillWidth: true; spacing: Theme.spaceMd
                Repeater {
                    model: ["following", "inspecting", "crosshair"]
                    ColumnLayout {
                        id: chartState
                        required property string modelData
                        Layout.fillWidth: true; spacing: Theme.spaceXs
                        Text { text: chartState.modelData.toUpperCase(); color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        ChartPreview { previewState: chartState.modelData; Layout.fillWidth: true; Layout.preferredHeight: Spacing.galleryChartHeight }
                    }
                }
                ColumnLayout {
                    spacing: Theme.spaceXs
                    Text { text: "NARROW · CROSSHAIR"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    ChartPreview { previewState: "crosshair"; Layout.preferredWidth: Spacing.galleryNarrowChartWidth; Layout.preferredHeight: Spacing.galleryChartHeight }
                }
            }

            Text { text: "STATUS HIERARCHY & OPERATIONAL ALERTS (Q-064)"; color: Theme.accent; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightBold }
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spaceMd

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "HEALTHY (QUIET)"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        StatusBadge { text: "LIVE"; role: Semantic.positive }
                        Text { text: "PETR4 · 1m · Candles"; color: Theme.textPrimary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.bold: true }
                        Text { text: "Operations hidden · No alerts"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "STALE FEED"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        StatusBadge { text: "STALE 2m 5s"; role: Semantic.stale }
                        Text { text: "Following: momentum-alpha"; color: Theme.textSecondary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel }
                        Text { text: "Distinct from disconnected"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "DISCONNECTED"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        StatusBadge { text: "DISCONNECTED"; role: Semantic.critical }
                        Text { text: "API retrying connection"; color: Theme.textSecondary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel }
                        Text { text: "Never labeled live"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "WORKER OFFLINE"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        Rectangle {
                            height: Spacing.size24
                            implicitWidth: workerAlertText.implicitWidth + Spacing.size16
                            radius: Theme.radiusMedium
                            color: Theme.criticalSurface
                            border.color: Theme.criticalStrong
                            border.width: Theme.borderWidth
                            Row {
                                anchors.centerIn: parent
                                spacing: Theme.spaceXs
                                Text { text: "⚠"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel }
                                Text { id: workerAlertText; text: "Worker unavailable"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightMedium }
                            }
                        }
                        Text { text: "Reveals Ops on click"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "UNKNOWN ORDERS"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        Rectangle {
                            height: Spacing.size24
                            implicitWidth: unkAlertText.implicitWidth + Spacing.size16
                            radius: Theme.radiusMedium
                            color: Theme.criticalSurface
                            border.color: Theme.criticalStrong
                            border.width: Theme.borderWidth
                            Row {
                                anchors.centerIn: parent
                                spacing: Theme.spaceXs
                                Text { text: "⚠"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel }
                                Text { id: unkAlertText; text: "Unknown orders: 2"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightMedium }
                            }
                        }
                        Text { text: "Reconciliation required"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "KILL SWITCH"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        Rectangle {
                            height: Spacing.size24
                            implicitWidth: killAlertText.implicitWidth + Spacing.size16
                            radius: Theme.radiusMedium
                            color: Theme.criticalSurface
                            border.color: Theme.criticalStrong
                            border.width: Theme.borderWidth
                            Row {
                                anchors.centerIn: parent
                                spacing: Theme.spaceXs
                                Text { text: "⚠"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel }
                                Text { id: killAlertText; text: "Kill switch engaged"; color: Theme.critical; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabel; font.weight: Typography.weightMedium }
                            }
                        }
                        Text { text: "Trading risk halted"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    implicitHeight: Spacing.size100
                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Theme.spaceSm
                        spacing: Theme.spaceXs
                        Text { text: "DOM PLACEHOLDER"; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        Text { text: "DOM"; color: Theme.textPrimary; font.family: Theme.uiFont; font.pixelSize: Theme.typeBodySmall; font.bold: true }
                        Text { text: "Reserved for a later release."; color: Theme.textMuted; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                        Text { text: "Honest placeholder copy"; color: Theme.textTertiary; font.family: Theme.uiFont; font.pixelSize: Theme.typeLabelSmall }
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: root.width >= Spacing.workstationWideWidth ? Spacing.galleryWideColumns : Spacing.galleryColumns
                columnSpacing: Theme.spaceMd; rowSpacing: Theme.spaceMd
                Repeater {
                    model: ComponentCatalog.entries
                    GalleryEntry { required property var modelData; entry: modelData; Layout.fillWidth: true }
                }
            }

            RowLayout {
                Layout.fillWidth: true; spacing: Theme.spaceLg
                Panel {
                    Layout.fillWidth: true; implicitHeight: Spacing.accountSummaryHeight
                    Row { anchors.fill: parent; spacing: Theme.spaceLg
                        Repeater { model: [Semantic.positive, Semantic.negative, Semantic.warning, Semantic.critical, Semantic.neutral, Semantic.stale, Semantic.live]
                            StatusBadge { required property string modelData; role: modelData; text: modelData.toUpperCase() }
                        }
                    }
                }
                Panel {
                    Layout.fillWidth: true; implicitHeight: Spacing.accountSummaryHeight
                    Row { anchors.fill: parent; spacing: Theme.spaceLg
                        Text { text: "Inter UI  Aa"; color: Theme.textStrong; font.family: Theme.uiFont; font.pixelSize: Theme.typeTitle }
                        Text { text: "0123456789"; color: Theme.accent; font.family: Theme.numericFontFamily; font.pixelSize: Theme.typeTitle; font.features: { "tnum": 1 } }
                        Repeater { model: [Icons.plus, Icons.refresh, Icons.warning, Icons.success, Icons.settings]
                            Image { required property url modelData; source: modelData; width: Spacing.iconLarge; height: Spacing.iconLarge }
                        }
                    }
                }
            }
        }
    }
}
