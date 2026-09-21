pragma Singleton
import QtQuick
import qml

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
        case positive: return Theme.positiveSurface;
        case negative: return Theme.negativeSurface;
        case warning: return Theme.warningSurface;
        case critical: return Theme.criticalSurface;
        case stale: return Theme.warningSurface;
        case accent: return Theme.surfaceSelected;
        case live: return Theme.negativeSurface;
        default: return Theme.surfaceOverlay;
        }
    }

    function border(role) {
        switch (role) {
        case positive: return Theme.positiveStrong;
        case negative: return Theme.negativeStrong;
        case warning: return Theme.warningStrong;
        case critical: return Theme.critical;
        case stale: return Theme.warningStrong;
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
        case "ok":
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

    // Model values are translated here so QML only renders a role.
    function orderStatus(value) {
        switch (String(value).toLowerCase()) {
        case "filled":
        case "submitted": return positive;
        case "pending": return warning;
        case "rejected":
        case "cancelled": return negative;
        default: return neutral;
        }
    }

    function decisionAction(value) {
        var action = String(value).toLowerCase();
        if (action === "buy" || action === "open_long") return positive;
        if (action === "sell" || action === "open_short") return negative;
        return neutral;
    }

    function decisionOutcome(value) {
        var outcome = String(value).toLowerCase();
        if (outcome === "executed" || outcome === "submitted") return positive;
        if (outcome === "rejected" || outcome === "blocked") return negative;
        return warning;
    }

    function ledgerEntry(value) {
        var entry = String(value).toLowerCase();
        if (entry === "deposit" || entry === "trade_pnl_positive") return positive;
        if (entry === "withdrawal" || entry === "fee" || entry === "trade_pnl_negative") return negative;
        return neutral;
    }

    function signedString(value) {
        var text = String(value);
        if (text.startsWith("-")) return negative;
        if (text !== "" && text !== "0" && text !== "0.00") return positive;
        return neutral;
    }

    function position(value) {
        var sideValue = String(value).toLowerCase();
        if (sideValue === "long" || sideValue === "buy") return positive;
        if (sideValue === "short" || sideValue === "sell") return negative;
        return neutral;
    }

    function workerHealth(status, age) {
        return String(status).toLowerCase() === "active" && age <= 30.0 ? positive : warning;
    }

    function edgeHealth(reachable, connected) {
        return reachable && connected ? positive : critical;
    }

    function killSwitch(enabled) { return enabled ? critical : accent; }

    function liveLock(locked) { return locked ? neutral : warning; }
}
