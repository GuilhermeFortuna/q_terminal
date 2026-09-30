pragma Singleton
import QtQuick

// Distinct study line colours (Q-075); consumed by StudyPicker and pushed to BarFeed.
QtObject {
    readonly property var colors: [
        Palette.accentLegacy,
        Palette.warningStrong,
        Palette.positiveStrong,
        Palette.negativeStrong,
        Palette.accent,
        Palette.warning,
        Palette.positive,
        Palette.textSecondary
    ]

    function rgbaHex(index) {
        var c = colors[index % colors.length];
        var r = Math.round(c.r * 255);
        var g = Math.round(c.g * 255);
        var b = Math.round(c.b * 255);
        function p2(n) { return (n < 16 ? "0" : "") + n.toString(16); }
        return "#" + p2(r) + p2(g) + p2(b);
    }

    function paletteJson() {
        var out = [];
        for (var i = 0; i < colors.length; i++) {
            out.push(rgbaHex(i));
        }
        return JSON.stringify(out);
    }
}
