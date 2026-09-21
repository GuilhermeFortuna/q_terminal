pragma Singleton
import QtQuick
import qml

QtObject {
    function treatment(state) {
        switch (state) {
        case "rest": return "surface-base";
        case "hover": return "surface-hover";
        case "pressed": return "accent-pressed";
        case "focused": return "focus-ring";
        case "selected": return "surface-selected";
        case "disabled": return "content-disabled";
        case "degraded": return "status-critical";
        case "stale": return "status-stale";
        default: return "surface-base";
        }
    }

    function background(state) {
        if (state === "hover") return Theme.surfaceHover;
        if (state === "pressed") return Theme.accentPressed;
        if (state === "selected") return Theme.surfaceSelected;
        if (state === "degraded") return Semantic.background(Semantic.critical);
        if (state === "stale") return Semantic.background(Semantic.stale);
        return Theme.surfaceBase;
    }

    function border(state) {
        if (state === "focused" || state === "selected") return Theme.accent;
        if (state === "degraded") return Semantic.border(Semantic.critical);
        if (state === "stale") return Semantic.border(Semantic.stale);
        return Theme.borderDefault;
    }

    function foreground(state) {
        return state === "disabled" ? Theme.textMuted : Theme.textStrong;
    }
}
