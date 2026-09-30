pragma Singleton
import QtQuick

// Distinct study line colours (Q-075); consumed by StudyPicker and pushed to BarFeed.
QtObject {
    // High-contrast study colors that remain distinct on dark theme and don't clash with green/red candles
    readonly property var colors: [
        "#00e5ff",
        "#ffb300",
        "#b388ff",
        "#ff7043",
        "#2979ff",
        "#aeea00",
        "#f50057",
        "#90a4ae"
    ]

    function colorAt(index) {
        return colors[index % colors.length];
    }

    function rgbaHex(index) {
        return colors[index % colors.length];
    }

    function paletteJson() {
        return JSON.stringify(colors);
    }
}
