pragma Singleton
import QtQuick

QtObject {
    // Radix-inspired dark surface ladder, preserving the incumbent terminal palette.
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
    readonly property color accentLegacy: "#2962ff"

    readonly property color positive: "#34d399"
    readonly property color positiveStrong: "#10b981"
    readonly property color positiveSurface: "#064e3b"
    readonly property color negative: "#f87171"
    readonly property color negativeStrong: "#ef4444"
    readonly property color negativeSurface: "#450a0a"
    readonly property color warning: "#fbbf24"
    readonly property color warningStrong: "#f59e0b"
    readonly property color warningSurface: "#451a03"
    readonly property color critical: "#f23645"
    readonly property color neutral: textSecondary
    readonly property color stale: warningStrong
    readonly property color disabled: textMuted
    readonly property color transparent: "transparent"
}
