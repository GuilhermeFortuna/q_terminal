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
        applyPreviewState();
    }
}
