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
