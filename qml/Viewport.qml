import QtQuick
import qml

QtObject {
    id: root

    property int barCount: 0
    property var lastBarTime: 0
    property real low: 0.0
    property real high: 0.0
    property int revision: 0
    property int barsVisible: 100
    property real priceMargin: 0.05
    readonly property int minimumVisibleBars: Theme.chartMinimumVisibleBars
    readonly property int maximumVisibleBars: Theme.chartMaximumVisibleBars

    readonly property bool empty: barCount <= 0
    readonly property bool isEmpty: empty
    property int firstBar: 0
    property int lastBar: 0
    property int visibleBars: 0
    property real lowPrice: 0.0
    property real highPrice: 0.0
    property string modeState: "following"
    readonly property string mode: modeState
    readonly property bool followingLive: modeState === "following"
    readonly property bool inspectingHistory: modeState === "inspecting"

    function visibleCount() {
        if (barCount <= 0) {
            return 0;
        }
        var requested = visibleBars > 0 ? visibleBars : barsVisible;
        return Math.min(barCount, maximumVisibleBars,
                        Math.max(1, Math.max(minimumVisibleBars, requested)));
    }

    function maxFirstBar() {
        return Math.max(0, barCount - visibleCount());
    }

    function updatePriceBounds() {
        if (barCount <= 0 || high < low) {
            lowPrice = 0.0;
            highPrice = 0.0;
            return;
        }
        if (lowPrice !== 0.0 && highPrice !== 0.0 && low >= lowPrice && high <= highPrice) {
            return;
        }
        var span = high - low;
        if (span <= 0.0) {
            span = (high !== 0.0 ? Math.abs(high) * 0.01 : 1.0);
        }
        var margin = span * priceMargin;
        lowPrice = low - margin;
        highPrice = high + margin;
    }

    function fitPriceBounds() {
        lowPrice = 0.0;
        highPrice = 0.0;
        updatePriceBounds();
    }

    function clampViewport() {
        var span = visibleCount();
        if (span <= 0) {
            firstBar = 0;
            lastBar = 0;
            return;
        }
        firstBar = Math.max(0, Math.min(firstBar, maxFirstBar()));
        lastBar = Math.min(barCount, firstBar + span);
        firstBar = Math.max(0, lastBar - span);
    }

    function updateViewport() {
        if (barCount <= 0) {
            firstBar = 0;
            lastBar = 0;
            modeState = "following";
            updatePriceBounds();
            return;
        }
        var span = visibleCount();
        if (followingLive) {
            lastBar = barCount;
            firstBar = Math.max(0, lastBar - span);
        } else {
            clampViewport();
        }
        updatePriceBounds();
    }

    function panBars(delta) {
        if (barCount <= 0 || delta === 0) {
            return;
        }
        modeState = "inspecting";
        var span = visibleCount();
        firstBar = Math.max(0, Math.min(maxFirstBar(), firstBar + delta));
        lastBar = firstBar + span;
        fitPriceBounds();
    }

    // direction 1 zooms in; -1 zooms out. Anchor is normalized plot x.
    function zoomAt(anchorFraction, direction) {
        if (barCount <= 0 || direction === 0) {
            return;
        }
        var oldSpan = visibleCount();
        if (oldSpan <= 1) {
            return;
        }
        var nextSpan = oldSpan + (direction > 0 ? -2 : 2);
        nextSpan = Math.min(barCount, maximumVisibleBars,
                            Math.max(1, Math.max(minimumVisibleBars, nextSpan)));
        if (nextSpan === oldSpan) {
            return;
        }
        var anchor = Math.max(0.0, Math.min(1.0, anchorFraction));
        var anchorBar = firstBar + anchor * Math.max(0, oldSpan - 1);
        var nextFirst = Math.round(anchorBar - anchor * Math.max(0, nextSpan - 1));
        modeState = "inspecting";
        visibleBars = nextSpan;
        firstBar = Math.max(0, Math.min(barCount - nextSpan, nextFirst));
        lastBar = firstBar + nextSpan;
        fitPriceBounds();
    }

    function returnToLive() {
        modeState = "following";
        visibleBars = 0;
        updateViewport();
        fitPriceBounds();
    }

    function resetForTarget() {
        modeState = "following";
        visibleBars = 0;
        firstBar = 0;
        lastBar = 0;
        fitPriceBounds();
    }

    onBarCountChanged: updateViewport()
    onLowChanged: updatePriceBounds()
    onHighChanged: updatePriceBounds()
    onRevisionChanged: updateViewport()
    onBarsVisibleChanged: {
        visibleBars = 0;
        updateViewport();
    }
    onPriceMarginChanged: updatePriceBounds()
}
