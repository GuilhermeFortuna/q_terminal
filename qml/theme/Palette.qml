pragma Singleton
import QtQuick

QtObject {
    // Radix-style stepwise neutral ladder on a true-black canvas; equal RGB, no blue cast.
    readonly property color surfaceSunken: "#000000"
    readonly property color surfaceBase: "#000000"
    readonly property color surfaceRaised: "#0f0f0f"
    readonly property color surfaceOverlay: "#171717"
    readonly property color surfaceElevated: "#1f1f1f"
    readonly property color surfaceSelected: "#2a2a2a"
    readonly property color surfaceHover: "#333333"

    readonly property color borderSubtle: "#2a2a2a"
    readonly property color borderDefault: "#3d3d3d"
    readonly property color borderStrong: "#666666"

    readonly property color textMuted: "#8c8c8c"
    readonly property color textTertiary: "#9e9e9e"
    readonly property color textSecondary: "#b8b8b8"
    readonly property color textPrimary: "#e2e2e2"
    readonly property color textStrong: "#fafafa"
    readonly property color textOnAccent: "#ffffff"

    readonly property color accent: "#38bdf8"
    readonly property color accentStrong: "#2563eb"
    readonly property color accentPressed: "#1d4ed8"
    readonly property color accentDark: "#1e3a8a"
    readonly property color accentLegacy: "#2962ff"

    readonly property color positive: "#34d399"
    readonly property color positiveStrong: "#10b981"
    readonly property color positiveSurface: "#064e3b"
    readonly property color negative: "#f87171"
    readonly property color negativeStrong: "#ef4444"
    readonly property color negativeSurface: "#450a0a"
    readonly property color negativeSoft: "#fca5a5"
    readonly property color negativeText: "#fef2f2"
    readonly property color warning: "#fbbf24"
    readonly property color warningStrong: "#f59e0b"
    readonly property color warningSurface: "#451a03"
    readonly property color warningText: "#fef3c7"
    readonly property color critical: "#f23645"
    readonly property color criticalStrong: "#dc2626"
    readonly property color criticalSurface: "#7f1d1d"
    readonly property color criticalDeep: "#3b1111"
    readonly property color positiveDeep: "#0f2f24"
    readonly property color neutral: textSecondary
    readonly property color stale: warningStrong
    readonly property color disabled: textMuted
    readonly property color transparent: "transparent"
    readonly property color warningForeground: "#000000"
}
