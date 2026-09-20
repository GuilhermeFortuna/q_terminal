pragma Singleton
import QtQuick

QtObject {
    readonly property int none: 0
    readonly property int hairline: 1
    readonly property int xxs: 2
    readonly property int xs: 4
    readonly property int sm: 6
    readonly property int md: 8
    readonly property int lg: 12
    readonly property int xl: 16
    readonly property int xxl: 24
    readonly property int xxxl: 32

    readonly property int radiusSmall: 3
    readonly property int radiusMedium: 4
    readonly property int radiusLarge: 6
    readonly property int border: 1
    readonly property int focusBorder: 2

    readonly property int iconSmall: 12
    readonly property int iconMedium: 16
    readonly property int iconLarge: 20
    readonly property int badgeHeight: 18
    readonly property int controlHeight: 28
    readonly property int controlHeightLarge: 32
    readonly property int tableRowHeight: 28
    readonly property int tableHeaderHeight: 28
    readonly property int toolbarHeight: 32
    readonly property int statusHeight: 24
    readonly property int detailHeaderHeight: 72
    readonly property int accountSummaryHeight: 92
    readonly property int metricHeight: 40
    readonly property int dialogWidth: 480
    readonly property int priceAxisWidth: 64
    readonly property int deploymentRowHeight: 76
    readonly property int galleryCardHeight: 116
    readonly property int galleryPreviewHeight: 44
    readonly property int galleryIconCell: 52
    readonly property int workstationWidth: 1920
    readonly property int workstationHeight: 1080
    readonly property int workstationWideWidth: 2560
    readonly property int workstationWideHeight: 1440
    readonly property int galleryColumns: 3
    readonly property int galleryWideColumns: 4

    readonly property int durationFast: 80
    readonly property int durationNormal: 140
}
