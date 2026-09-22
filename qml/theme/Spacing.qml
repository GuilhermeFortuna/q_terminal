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

    readonly property int deploymentPaneWidth: 320
    readonly property int chartMinimumHeight: 220
    readonly property int chartMinimumVisibleBars: 5
    readonly property int chartMaximumVisibleBars: 500
    readonly property int bannerHeight: 32
    readonly property int mainHeaderHeight: 48
    readonly property int actionHeight: 44
    readonly property int tableTimeWidth: 140
    readonly property int tableIdWidth: 90
    readonly property int tableIntentWidth: 110
    readonly property int tableSideWidth: 60
    readonly property int tablePriceWidth: 90
    readonly property int tableQuantityWidth: 80
    readonly property int tableFeeWidth: 70
    readonly property int tableTypeWidth: 100
    readonly property int tableAmountWidth: 110
    readonly property int tableBalanceWidth: 120
    readonly property int tableMessageWidth: 200
    readonly property int tableContextWidth: 200
    readonly property int dialogCompactWidth: 420
    readonly property int dialogListHeight: 160
    readonly property int windowWidth: 1280
    readonly property int windowHeight: 800

    // Named compatibility points for legacy layout measurements. Keeping these
    // in the scale makes migration mechanical without allowing screen literals.
    readonly property int size0: 0
    readonly property int size1: 1
    readonly property int size2: 2
    readonly property int size3: 3
    readonly property int size4: 4
    readonly property int size6: 6
    readonly property int size8: 8
    readonly property int size10: 10
    readonly property int size12: 12
    readonly property int size16: 16
    readonly property int size18: 18
    readonly property int size20: 20
    readonly property int size22: 22
    readonly property int size24: 24
    readonly property int size28: 28
    readonly property int size32: 32
    readonly property int size36: 36
    readonly property int size40: 40
    readonly property int size44: 44
    readonly property int size48: 48
    readonly property int size60: 60
    readonly property int size64: 64
    readonly property int size70: 70
    readonly property int size72: 72
    readonly property int size80: 80
    readonly property int size84: 84
    readonly property int size90: 90
    readonly property int size100: 100
    readonly property int size110: 110
    readonly property int size120: 120
    readonly property int size140: 140
    readonly property int size160: 160
    readonly property int size200: 200
    readonly property int size220: 220
    readonly property int size320: 320
    readonly property int size420: 420
    readonly property int size480: 480
    readonly property int size800: 800
    readonly property int size1280: 1280

    readonly property int durationFast: 80
    readonly property int durationNormal: 140
}
