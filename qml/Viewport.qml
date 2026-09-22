import QtQuick
import qml

QtObject {
    id: root

    property int barCount: 0
    property var lastBarTime: 0
    // Fallback price range, used when there is no range source (probes and plain series).
    property real low: 0.0
    property real high: 0.0
    property int revision: 0
    property int barsVisible: 100
    property real priceMargin: 0.05
    // Feed that answers visible_range_json(firstBar, lastBar) with the high/low of exactly
    // the visible bars, forming bar included. Queried synchronously on every update so the
    // bounds never lag a live tick.
    property var rangeSource: null
    readonly property int minimumVisibleBars: Theme.chartMinimumVisibleBars
    readonly property int maximumVisibleBars: Theme.chartMaximumVisibleBars
    // Visible-bar multiplier for one zoom-in step; a zoom-out step divides by it.
    readonly property real zoomStepFactor: 0.8

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

    // Bar range the price bounds were last fitted to. While it is unchanged, updates only
    // widen the bounds, so a forming bar cannot make the axis oscillate tick by tick.
    property int fittedFirstBar: -1
    property int fittedLastBar: -1

    function clampSpan(requested) {
        return Math.min(barCount, maximumVisibleBars,
                        Math.max(1, Math.max(minimumVisibleBars, requested)));
    }

    function visibleCount() {
        if (barCount <= 0) {
            return 0;
        }
        return clampSpan(visibleBars > 0 ? visibleBars : barsVisible);
    }

    function maxFirstBar() {
        return Math.max(0, barCount - visibleCount());
    }

    function visiblePriceRange() {
        if (rangeSource && rangeSource.visible_range_json && lastBar > firstBar) {
            try {
                var range = JSON.parse(rangeSource.visible_range_json(firstBar, lastBar));
                if (range.valid) {
                    return { low: range.low, high: range.high };
                }
            } catch (error) {
            }
        }
        return { low: low, high: high };
    }

    function clearPriceBounds() {
        lowPrice = 0.0;
        highPrice = 0.0;
        fittedFirstBar = -1;
        fittedLastBar = -1;
    }

    // Fits the bounds to the visible bars when the visible range changed (or `force`), and
    // otherwise only widens them when a visible bar left them.
    function updatePriceBounds(force) {
        if (barCount <= 0) {
            clearPriceBounds();
            return;
        }
        var range = visiblePriceRange();
        if (range.high < range.low) {
            clearPriceBounds();
            return;
        }
        var rangeChanged = firstBar !== fittedFirstBar || lastBar !== fittedLastBar;
        var hasBounds = lowPrice !== 0.0 || highPrice !== 0.0;
        if (!force && !rangeChanged && hasBounds
                && range.low >= lowPrice && range.high <= highPrice) {
            return;
        }
        var span = range.high - range.low;
        if (span <= 0.0) {
            span = (range.high !== 0.0 ? Math.abs(range.high) * 0.01 : 1.0);
        }
        var margin = span * priceMargin;
        lowPrice = range.low - margin;
        highPrice = range.high + margin;
        fittedFirstBar = firstBar;
        fittedLastBar = lastBar;
    }

    function fitPriceBounds() {
        updatePriceBounds(true);
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
            updatePriceBounds(false);
            return;
        }
        var span = visibleCount();
        if (followingLive) {
            lastBar = barCount;
            firstBar = Math.max(0, lastBar - span);
        } else {
            clampViewport();
        }
        updatePriceBounds(false);
    }

    // Moves the view to [nextFirst, nextFirst + span). Leaving the right edge enters
    // Inspecting history; a move that keeps the newest bar visible while following does
    // not. Only returnToLive() resumes following once inspecting.
    function moveTo(nextFirst, span) {
        nextFirst = Math.max(0, Math.min(barCount - span, nextFirst));
        if (nextFirst + span < barCount) {
            modeState = "inspecting";
        }
        firstBar = nextFirst;
        lastBar = nextFirst + span;
        fitPriceBounds();
    }

    function panBars(delta) {
        if (barCount <= 0 || delta === 0) {
            return;
        }
        var span = visibleCount();
        var nextFirst = Math.max(0, Math.min(maxFirstBar(), firstBar + delta));
        if (nextFirst === firstBar) {
            return;
        }
        moveTo(nextFirst, span);
    }

    // steps > 0 zooms in and steps < 0 zooms out, each step scaling the visible bar count
    // by zoomStepFactor. anchorFraction is the normalized plot x of the bar kept in place.
    function zoomAt(anchorFraction, steps) {
        if (barCount <= 0 || steps === 0) {
            return;
        }
        var oldSpan = visibleCount();
        var nextSpan = Math.round(oldSpan * Math.pow(zoomStepFactor, steps));
        if (nextSpan === oldSpan) {
            nextSpan = oldSpan + (steps > 0 ? -1 : 1);
        }
        nextSpan = clampSpan(nextSpan);
        if (nextSpan === oldSpan) {
            return;
        }
        var anchor = Math.max(0.0, Math.min(1.0, anchorFraction));
        var anchorBar = firstBar + anchor * Math.max(0, oldSpan - 1);
        visibleBars = nextSpan;
        moveTo(Math.round(anchorBar - anchor * Math.max(0, nextSpan - 1)), nextSpan);
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
        clearPriceBounds();
        updateViewport();
    }

    onBarCountChanged: updateViewport()
    onLowChanged: updatePriceBounds(false)
    onHighChanged: updatePriceBounds(false)
    onRevisionChanged: updateViewport()
    onRangeSourceChanged: updateViewport()
    onBarsVisibleChanged: {
        visibleBars = 0;
        updateViewport();
    }
    onPriceMarginChanged: fitPriceBounds()
}
