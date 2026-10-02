import QtQuick
import qml

// Gallery-only: the real ChartPane over a real BarFeed with deterministic preview bars, so
// chart navigation and inspection states (Q-060, Q-061) are captured as they render live.
Rectangle {
    id: root
    property string previewState: "following"

    color: Theme.surfaceSunken
    border.color: Theme.borderDefault
    border.width: Theme.borderWidth
    clip: true

    BarFeed {
        id: previewFeed
    }

    ChartPane {
        id: pane
        anchors.fill: parent
        anchors.margins: Theme.borderWidth
        feed: previewFeed
        barsVisible: 60
    }

    function applyPreviewState() {
        switch (root.previewState) {
        case "inspecting":
            pane.viewport.panBars(-90);
            break;
        case "crosshair":
            pane.viewport.panBars(-40);
            applyHover();
            break;
        default:
            break;
        }
    }

    // The hover point follows the laid-out size, which is not known at completion.
    function applyHover() {
        if (root.previewState === "crosshair" && pane.width > 0 && pane.height > 0) {
            pane.testSetHover(pane.width * 0.45, pane.height * 0.4);
        }
    }

    onWidthChanged: applyHover()
    onHeightChanged: applyHover()

    Component.onCompleted: {
        previewFeed.load_preview_bars(240);
        previewFeed.set_study_palette_json(StudyPalette.paletteJson());
        if (root.previewState.indexOf("volume") === 0) {
            // Tape studies over a fixture session: delta, cumulative delta, rate and prints.
            previewFeed.bind_fixture_trades(root.previewState === "volume-stale" ? "stale" : "live", 241);
            previewFeed.add_study("ema", 21, "close", 2.0);
            previewFeed.add_volume_study("delta", 10000, 100.0);
            previewFeed.add_volume_study("cumulative_delta", 10000, 100.0);
            previewFeed.add_volume_study("trade_rate", 10000, 100.0);
            previewFeed.add_volume_study("large_prints", 10000, 100.0);
        } else {
            previewFeed.add_study("ema", 21, "close", 2.0);
            previewFeed.add_study("rsi", 14, "close", 2.0);
            previewFeed.add_study("sma", 500, "close", 2.0);
            previewFeed.add_study("vwap", 1, "close", 2.0);
        }
        applyPreviewState();
    }
}
