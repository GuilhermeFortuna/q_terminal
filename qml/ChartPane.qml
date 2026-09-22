pragma ComponentBehavior: Bound
import QtQuick
import qml

Item {
    id: root

    property var feed: null
    property var context: null
    property int barsVisible: 100
    property real priceMargin: 0.05

    readonly property alias viewport: viewport
    readonly property bool empty: viewport.empty
    readonly property bool inspectingHistory: viewport.inspectingHistory
    readonly property bool atLoadedHistoryBoundary: !viewport.empty && viewport.firstBar === 0
    readonly property var visibleRange: {
        if (!root.feed || viewport.empty || !root.feed.visible_range_json) {
            return ({ valid: false });
        }
        try {
            return JSON.parse(root.feed.visible_range_json(viewport.firstBar, viewport.lastBar));
        } catch (error) {
            return ({ valid: false });
        }
    }

    readonly property string topPriceLabel: viewport.empty ? "" : viewport.highPrice.toFixed(2)
    readonly property string bottomPriceLabel: viewport.empty ? "" : viewport.lowPrice.toFixed(2)

    function updateTip() {
        if (!root.feed || !hover.hovered) {
            tipText.text = "";
            return;
        }
        tipText.text = root.feed.marker_detail_at(hover.point.position.x, hover.point.position.y, 10);
    }

    function pad2(n) {
        return (n < 10 ? "0" : "") + n;
    }

    function normalizeMs(t) {
        if (!t || t <= 0) {
            return 0;
        }
        if (t > 1e16) {
            return Math.floor(t / 1e6);
        }
        if (t > 1e13) {
            return Math.floor(t / 1e3);
        }
        if (t < 1e11) {
            return Math.floor(t * 1e3);
        }
        return Math.floor(t);
    }

    function formatTime(t) {
        var ms = normalizeMs(t);
        if (ms <= 0) {
            return "";
        }
        var d = new Date(ms);
        return pad2(d.getUTCHours()) + ":" + pad2(d.getUTCMinutes());
    }

    readonly property string leftTimeLabel: {
        if (viewport.empty || !root.feed) {
            return "";
        }
        if (!root.feed.bar_time_at) {
            var tf = (root.feed.timeframe_ms && root.feed.timeframe_ms > 0) ? root.feed.timeframe_ms : 60000;
            return formatTime(root.feed.first_time + Math.max(0, viewport.firstBar) * tf);
        }
        return formatTime(root.feed.bar_time_at(viewport.firstBar));
    }

    readonly property string rightTimeLabel: {
        if (viewport.empty || !root.feed) {
            return "";
        }
        if (!root.feed.bar_time_at) {
            return formatTime(root.feed.last_time);
        }
        return formatTime(root.feed.bar_time_at(Math.max(viewport.firstBar, viewport.lastBar - 1)));
    }

    Viewport {
        id: viewport
        barCount: root.feed ? root.feed.bar_count + (root.feed.has_forming ? 1 : 0) : 0
        lastBarTime: root.feed ? root.feed.last_time : 0
        low: root.visibleRange.valid ? root.visibleRange.low : (root.feed ? root.feed.low : 0.0)
        high: root.visibleRange.valid ? root.visibleRange.high : (root.feed ? root.feed.high : 0.0)
        revision: root.feed ? root.feed.revision : 0
        barsVisible: root.barsVisible
        priceMargin: root.priceMargin
    }

    Item {
        id: chartArea
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: priceAxis.left
        anchors.bottom: timeAxis.top

        Repeater {
            model: 4
            Rectangle {
                required property int index
                width: chartArea.width
                height: Spacing.size1
                y: (index + 1) * chartArea.height / 5
                color: Theme.borderSubtle
            }
        }

        Repeater {
            model: 4
            Rectangle {
                required property int index
                width: Spacing.size1
                height: chartArea.height
                x: (index + 1) * chartArea.width / 5
                color: Theme.borderSubtle
            }
        }

        Item {
            id: chartContainer
            anchors.fill: chartArea
            visible: !viewport.empty

            BarChartItem {
                id: chart
                objectName: "benchChart"
                width: chartContainer.width
                height: chartContainer.height
                series: root.feed
                firstBar: viewport.firstBar
                lastBar: viewport.lastBar
                lowPrice: viewport.lowPrice
                highPrice: viewport.highPrice
            }

            // Decision and fill markers and indicator overlays share the bar item's
            // viewport, so they pan and zoom with the bars.
            OverlayChartItem {
                id: overlay
                objectName: "overlayChart"
                width: chartContainer.width
                height: chartContainer.height
                series: root.feed
                firstBar: viewport.firstBar
                lastBar: viewport.lastBar
                lowPrice: viewport.lowPrice
                highPrice: viewport.highPrice
            }

            HoverHandler {
                id: hover
                onPointChanged: root.updateTip()
            }

            DragHandler {
                id: panHandler
                target: null
                property real appliedBars: 0
                onActiveChanged: {
                    if (active) {
                        chartArea.forceActiveFocus();
                        appliedBars = 0;
                    }
                }
                onTranslationChanged: {
                    var count = Math.max(1, viewport.lastBar - viewport.firstBar);
                    var barWidth = chartArea.width / count;
                    if (barWidth <= 0.0) {
                        return;
                    }
                    var requested = Math.round(-translation.x / barWidth);
                    var delta = requested - appliedBars;
                    if (delta !== 0) {
                        viewport.panBars(delta);
                        appliedBars = requested;
                    }
                }
            }

            WheelHandler {
                id: zoomHandler
                acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                onWheel: function(event) {
                    if (event.angleDelta.y === 0) {
                        return;
                    }
                    viewport.zoomAt(event.position.x / Math.max(1, chartArea.width),
                                    event.angleDelta.y > 0 ? 1 : -1);
                    event.accepted = true;
                }
            }

            Rectangle {
                id: tip
                objectName: "markerTooltip"
                visible: tipText.text !== ""
                x: Math.min(hover.point.position.x + 12, chartContainer.width - width)
                y: Math.min(hover.point.position.y + 12, chartContainer.height - height)
                width: tipText.implicitWidth + 12
                height: tipText.implicitHeight + 8
                color: Theme.surfaceElevated
                border.color: Theme.borderStrong

                Text {
                    id: tipText
                    anchors.centerIn: parent
                    color: Theme.textPrimary
                    font.pixelSize: Theme.typeBodySmall
                }
            }
        }

        ChartEmptyState {
            id: emptyState
            objectName: "emptyState"
            anchors.fill: chartArea
            visible: viewport.empty
            feed: root.feed
            context: root.context
        }

        Keys.onPressed: function(event) {
            if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal) {
                viewport.zoomAt(0.5, 1);
                event.accepted = true;
            } else if (event.key === Qt.Key_Minus) {
                viewport.zoomAt(0.5, -1);
                event.accepted = true;
            }
        }

        focus: true
        activeFocusOnTab: true
    }

    Row {
        id: navigationStatus
        objectName: "navigationStatus"
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: Spacing.size4
        anchors.rightMargin: Spacing.priceAxisWidth + Spacing.size8
        spacing: Spacing.size8
        z: 2

        Text {
            objectName: "historyBoundaryText"
            text: root.atLoadedHistoryBoundary ? "Loaded history limit" : ""
            color: Theme.textMuted
            font.pixelSize: Theme.typeLabelSmall
            visible: text !== ""
            anchors.verticalCenter: parent.verticalCenter
        }

        Text {
            objectName: "navigationModeText"
            text: root.inspectingHistory ? "Inspecting history" : "Following live"
            color: root.inspectingHistory ? Theme.warning : Theme.textSecondary
            font.pixelSize: Theme.typeLabelSmall
            Accessible.name: text
            anchors.verticalCenter: parent.verticalCenter
        }

        AppButton {
            objectName: "returnToLiveButton"
            text: "Return to live"
            variant: "primary"
            visible: root.inspectingHistory
            Accessible.name: "Return to live"
            onClicked: viewport.returnToLive()
        }
    }

    Item {
        id: priceAxis
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: timeAxis.top
        width: Spacing.size64

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Spacing.size1
            color: Theme.borderSubtle
        }

        Text {
            id: topPriceText
            objectName: "topPriceText"
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.margins: Spacing.size4
            text: root.topPriceLabel
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
            visible: !viewport.empty
        }

        Text {
            id: bottomPriceText
            objectName: "bottomPriceText"
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.margins: Spacing.size4
            text: root.bottomPriceLabel
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
            visible: !viewport.empty
        }
    }

    Item {
        id: timeAxis
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: Spacing.size24

        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: Spacing.size1
            color: Theme.borderSubtle
        }

        Text {
            id: leftTimeText
            objectName: "leftTimeText"
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: Spacing.size4
            text: root.leftTimeLabel
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
            visible: !viewport.empty
        }

        Text {
            id: rightTimeText
            objectName: "rightTimeText"
            anchors.right: parent.right
            anchors.rightMargin: Spacing.size70
            anchors.verticalCenter: parent.verticalCenter
            text: root.rightTimeLabel
            color: Theme.textTertiary
            font.pixelSize: Theme.typeBodySmall
            visible: !viewport.empty
        }
    }
}
