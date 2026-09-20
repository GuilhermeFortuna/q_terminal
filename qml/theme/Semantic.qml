pragma Singleton
import QtQuick

QtObject {
    readonly property string neutral: "neutral"
    readonly property string positive: "positive"
    readonly property string negative: "negative"
    readonly property string warning: "warning"
    readonly property string critical: "critical"
    readonly property string stale: "stale"
    readonly property string disabled: "disabled"
    readonly property string accent: "accent"
    readonly property string live: "live"

    function foreground(role) {
        switch (role) {
        case positive: return Theme.positive;
        case negative: return Theme.negative;
        case warning: return Theme.warning;
        case critical: return Theme.critical;
        case stale: return Theme.warning;
        case disabled: return Theme.textMuted;
        case accent: return Theme.accent;
        case live: return Theme.negative;
        default: return Theme.textSecondary;
        }
    }

    function background(role) {
        switch (role) {
        case positive: return "#064e3b";
        case negative: return "#450a0a";
        case warning: return "#451a03";
        case critical: return "#450a0a";
        case stale: return "#451a03";
        case accent: return Theme.surfaceSelected;
        case live: return "#450a0a";
        default: return Theme.surfaceOverlay;
        }
    }

    function border(role) {
        switch (role) {
        case positive: return "#10b981";
        case negative: return "#ef4444";
        case warning: return "#f59e0b";
        case critical: return Theme.critical;
        case stale: return "#f59e0b";
        case accent: return Theme.accent;
        case live: return Theme.negative;
        default: return Theme.borderDefault;
        }
    }

    function direction(value) {
        if (value > 0) return positive;
        if (value < 0) return negative;
        return neutral;
    }

    function lifecycle(value) {
        switch (String(value).toLowerCase()) {
        case "running": return positive;
        case "paused": return warning;
        case "stopped": return negative;
        case "failed": return critical;
        default: return neutral;
        }
    }

    function side(value) {
        return String(value).toLowerCase() === "buy" ? positive : negative;
    }

    function reconciliation(value) {
        var normalized = String(value).toLowerCase();
        return normalized === "reconciled" || normalized === "none" ? neutral : warning;
    }

    function brokerMode(value) {
        var normalized = String(value).toLowerCase();
        return normalized === "live" || normalized === "mt5_live" ? live : neutral;
    }

    function health(value) {
        switch (String(value).toLowerCase()) {
        case "connected":
        case "healthy":
        case "online": return positive;
        case "connecting":
        case "reconnecting": return warning;
        case "stale": return stale;
        case "error":
        case "offline":
        case "disconnected": return critical;
        default: return neutral;
        }
    }

    function freshness(isStale) { return isStale ? stale : positive; }
}
