pragma Singleton
import QtQuick
import qml as Design

QtObject {
    readonly property FontLoader interFont: FontLoader {
        source: "qrc:/assets/fonts/Inter-Variable.ttf"
    }
    readonly property FontLoader numericFont: FontLoader {
        source: "qrc:/assets/fonts/JetBrainsMono-Variable.ttf"
    }

    readonly property string uiFont: interFont.status === FontLoader.Ready ? interFont.name : "Inter"
    readonly property string numericFontFamily: numericFont.status === FontLoader.Ready ? numericFont.name : "JetBrains Mono"

    readonly property color surfaceSunken: Design.Palette.surfaceSunken
    readonly property color surfaceBase: Design.Palette.surfaceBase
    readonly property color surfaceRaised: Design.Palette.surfaceRaised
    readonly property color surfaceOverlay: Design.Palette.surfaceOverlay
    readonly property color surfaceElevated: Design.Palette.surfaceElevated
    readonly property color surfaceSelected: Design.Palette.surfaceSelected
    readonly property color surfaceHover: Design.Palette.surfaceHover
    readonly property color borderSubtle: Design.Palette.borderSubtle
    readonly property color borderDefault: Design.Palette.borderDefault
    readonly property color borderStrong: Design.Palette.borderStrong
    readonly property color textMuted: Design.Palette.textMuted
    readonly property color textTertiary: Design.Palette.textTertiary
    readonly property color textSecondary: Design.Palette.textSecondary
    readonly property color textPrimary: Design.Palette.textPrimary
    readonly property color textStrong: Design.Palette.textStrong
    readonly property color textOnAccent: Design.Palette.textOnAccent
    readonly property color accent: Design.Palette.accent
    readonly property color accentStrong: Design.Palette.accentStrong
    readonly property color accentPressed: Design.Palette.accentPressed
    readonly property color accentDark: Design.Palette.accentDark
    readonly property color accentLegacy: Design.Palette.accentLegacy
    readonly property color positive: Design.Palette.positive
    readonly property color positiveStrong: Design.Palette.positiveStrong
    readonly property color positiveSurface: Design.Palette.positiveSurface
    readonly property color positiveDeep: Design.Palette.positiveDeep
    readonly property color negative: Design.Palette.negative
    readonly property color negativeStrong: Design.Palette.negativeStrong
    readonly property color negativeSurface: Design.Palette.negativeSurface
    readonly property color negativeSoft: Design.Palette.negativeSoft
    readonly property color negativeText: Design.Palette.negativeText
    readonly property color warning: Design.Palette.warning
    readonly property color warningStrong: Design.Palette.warningStrong
    readonly property color warningSurface: Design.Palette.warningSurface
    readonly property color warningText: Design.Palette.warningText
    readonly property color critical: Design.Palette.critical
    readonly property color criticalStrong: Design.Palette.criticalStrong
    readonly property color criticalSurface: Design.Palette.criticalSurface
    readonly property color criticalDeep: Design.Palette.criticalDeep
    readonly property color transparent: Design.Palette.transparent
    readonly property color warningForeground: Design.Palette.warningForeground

    readonly property int spaceNone: Spacing.none
    readonly property int spaceHairline: Spacing.hairline
    readonly property int spaceXxs: Spacing.xxs
    readonly property int spaceXs: Spacing.xs
    readonly property int spaceSm: Spacing.sm
    readonly property int spaceMd: Spacing.md
    readonly property int spaceLg: Spacing.lg
    readonly property int spaceXl: Spacing.xl
    readonly property int spaceXxl: Spacing.xxl
    readonly property int radiusSmall: Spacing.radiusSmall
    readonly property int radiusMedium: Spacing.radiusMedium
    readonly property int borderWidth: Spacing.border
    readonly property int focusBorderWidth: Spacing.focusBorder
    readonly property int controlHeight: Spacing.controlHeight
    readonly property int tableRowHeight: Spacing.tableRowHeight
    readonly property int tableHeaderHeight: Spacing.tableHeaderHeight
    readonly property int dividerWidth: Spacing.hairline
    readonly property int statusDotSize: Spacing.sm
    readonly property int badgeHeight: Spacing.badgeHeight
    readonly property int detailHeaderHeight: Spacing.detailHeaderHeight
    readonly property int accountSummaryHeight: Spacing.accountSummaryHeight
    readonly property int metricHeight: Spacing.metricHeight
    readonly property int dialogWidth: Spacing.dialogWidth
    readonly property int priceAxisWidth: Spacing.priceAxisWidth

    readonly property int typeLabelSmall: 9
    readonly property int typeLabel: 10
    readonly property int typeBodySmall: 11
    readonly property int typeBody: 12
    readonly property int typeBodyLarge: 13
    readonly property int typeTitle: 16
    readonly property int typeHeading: 20
}
