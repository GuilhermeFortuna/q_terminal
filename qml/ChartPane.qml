pragma ComponentBehavior: Bound
import QtQuick
import qml

Item {
    id: root

    property var feed: null
    property int barsVisible: 100
    property real priceMargin: 0.05

    readonly property alias viewport: viewport
    readonly property bool empty: viewport.empty

    readonly property string topPriceLabel: viewport.empty ? "" : viewport.highPrice.toFixed(2)
    readonly property string bottomPriceLabel: viewport.empty ? "" : viewport.lowPrice.toFixed(2)

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
        if (root.feed.first_time && root.feed.first_time > 0) {
            var tf = (root.feed.timeframe_ms && root.feed.timeframe_ms > 0) ? root.feed.timeframe_ms : 60000;
            var t = root.feed.first_time + Math.max(0, viewport.firstBar) * tf;
            return formatTime(t);
        }
        return "";
    }

    readonly property string rightTimeLabel: {
        if (viewport.empty || !root.feed) {
            return "";
        }
        if (root.feed.last_time && root.feed.last_time > 0) {
            return formatTime(root.feed.last_time);
        }
        return "";
    }

    Viewport {
        id: viewport
        barCount: root.feed ? root.feed.bar_count : 0
        lastBarTime: root.feed ? root.feed.last_time : 0
        low: root.feed ? root.feed.low : 0.0
        high: root.feed ? root.feed.high : 0.0
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
                height: 1
                y: (index + 1) * chartArea.height / 5
                color: "#2a2e39"
            }
        }

        Repeater {
            model: 4
            Rectangle {
                required property int index
                width: 1
                height: chartArea.height
                x: (index + 1) * chartArea.width / 5
                color: "#2a2e39"
            }
        }

        Item {
            id: chartContainer
            anchors.fill: chartArea
            visible: !viewport.empty

            BarChartItem {
                id: chart
                objectName: "chartItem"
                width: chartContainer.width
                height: chartContainer.height
                series: root.feed
                firstBar: viewport.firstBar
                lastBar: viewport.lastBar
                lowPrice: viewport.lowPrice
                highPrice: viewport.highPrice
            }
        }

        EmptyState {
            id: emptyState
            objectName: "emptyState"
            anchors.fill: chartArea
            visible: viewport.empty
            feed: root.feed
        }
    }

    Item {
        id: priceAxis
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: timeAxis.top
        width: 64

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 1
            color: "#2a2e39"
        }

        Text {
            id: topPriceText
            objectName: "topPriceText"
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.margins: 4
            text: root.topPriceLabel
            color: "#787b86"
            font.pixelSize: 11
            visible: !viewport.empty
        }

        Text {
            id: bottomPriceText
            objectName: "bottomPriceText"
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.margins: 4
            text: root.bottomPriceLabel
            color: "#787b86"
            font.pixelSize: 11
            visible: !viewport.empty
        }
    }

    Item {
        id: timeAxis
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 24

        Rectangle {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            height: 1
            color: "#2a2e39"
        }

        Text {
            id: leftTimeText
            objectName: "leftTimeText"
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 4
            text: root.leftTimeLabel
            color: "#787b86"
            font.pixelSize: 11
            visible: !viewport.empty
        }

        Text {
            id: rightTimeText
            objectName: "rightTimeText"
            anchors.right: parent.right
            anchors.rightMargin: 70
            anchors.verticalCenter: parent.verticalCenter
            text: root.rightTimeLabel
            color: "#787b86"
            font.pixelSize: 11
            visible: !viewport.empty
        }
    }
}
