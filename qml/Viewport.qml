import QtQuick

QtObject {
    id: root

    // Inputs
    property int barCount: 0
    property var lastBarTime: 0
    property real low: 0.0
    property real high: 0.0
    property int revision: 0
    property int barsVisible: 100
    property real priceMargin: 0.05

    // Outputs
    readonly property bool empty: barCount <= 0
    readonly property bool isEmpty: empty
    property int firstBar: 0
    property int lastBar: 0
    property real lowPrice: 0.0
    property real highPrice: 0.0

    function updateViewport() {
        if (barCount <= 0) {
            firstBar = 0;
            lastBar = 0;
            lowPrice = 0.0;
            highPrice = 0.0;
            return;
        }

        lastBar = barCount;
        firstBar = Math.max(0, lastBar - barsVisible);

        var needsRecompute = (lowPrice === 0.0 && highPrice === 0.0) ||
                             (low < lowPrice) ||
                             (high > highPrice);

        if (needsRecompute) {
            var span = high - low;
            if (span <= 0.0) {
                span = (high !== 0.0 ? Math.abs(high) * 0.01 : 1.0);
            }
            var margin = span * priceMargin;
            lowPrice = low - margin;
            highPrice = high + margin;
        }
    }

    onBarCountChanged: updateViewport()
    onLowChanged: updateViewport()
    onHighChanged: updateViewport()
    onRevisionChanged: updateViewport()
    onBarsVisibleChanged: updateViewport()
    onPriceMarginChanged: updateViewport()
}
