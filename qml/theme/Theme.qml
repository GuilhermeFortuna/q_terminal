pragma Singleton
import QtQuick

QtObject {
    readonly property FontLoader interFont: FontLoader {
        source: "qrc:/assets/fonts/Inter-Variable.ttf"
    }
    readonly property FontLoader numericFont: FontLoader {
        source: "qrc:/assets/fonts/JetBrainsMono-Variable.ttf"
    }

    readonly property string uiFont: interFont.status === FontLoader.Ready ? interFont.name : "Inter"
    readonly property string numericFontFamily: numericFont.status === FontLoader.Ready ? numericFont.name : "JetBrains Mono"

    readonly property color surfaceSunken: "#0b0f17"
    readonly property color surfaceBase: "#131722"
    readonly property color surfaceRaised: "#161c28"
    readonly property color surfaceOverlay: "#182030"
    readonly property color surfaceElevated: "#1e222d"
    readonly property color surfaceSelected: "#1e293b"
    readonly property color surfaceHover: "#243044"
    readonly property color borderSubtle: "#2a2e39"
    readonly property color borderDefault: "#334155"
    readonly property color borderStrong: "#475569"
    readonly property color textMuted: "#64748b"
    readonly property color textSecondary: "#94a3b8"
    readonly property color textPrimary: "#d1d4dc"
    readonly property color textStrong: "#f8fafc"
    readonly property color textOnAccent: "#ffffff"
    readonly property color accent: "#38bdf8"
    readonly property color accentStrong: "#2563eb"
    readonly property color accentPressed: "#1d4ed8"
    readonly property color positive: "#34d399"
    readonly property color negative: "#f87171"
    readonly property color warning: "#fbbf24"
    readonly property color critical: "#f23645"
    readonly property color transparent: "transparent"

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

    readonly property int typeLabelSmall: 9
    readonly property int typeLabel: 10
    readonly property int typeBodySmall: 11
    readonly property int typeBody: 12
    readonly property int typeBodyLarge: 13
    readonly property int typeTitle: 16
    readonly property int typeHeading: 20
}
