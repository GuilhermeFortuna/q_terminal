pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Controls
import qml

Item {
    id: root

    property var feed: null
    property var context: null
    property int barsVisible: 100
    property real priceMargin: 0.05

    readonly property alias viewport: viewport
    readonly property bool empty: viewport.empty

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

    function testFocusCanvas() {
        chartArea.forceActiveFocus();
        return chartArea.activeFocus;
    }

    function ensureTargetPrompt(initialChar) {
        if (initialChar && initialChar.length > 0) {
            targetPromptLoader.pendingChar = initialChar.charAt(0);
        }
        if (!targetPromptLoader.active) {
            targetPromptLoader.active = true;
        }
        var prompt = targetPromptLoader.item;
        if (prompt && targetPromptLoader.pendingChar.length > 0 && !prompt.visible) {
            prompt.openWith(targetPromptLoader.pendingChar);
            targetPromptLoader.pendingChar = "";
        }
        return prompt;
    }

    function testCanvasTypeKey(text) {
        if (!text || text.length === 0 || !chartArea.activeFocus) {
            return false;
        }
        var prompt = ensureTargetPrompt(text.charAt(0));
        if (!prompt) {
            return targetPromptLoader.active;
        }
        if (!prompt.visible) {
            if (text.length > 1) {
                prompt.setDraftText(text);
            }
            return prompt.opened || prompt.visible;
        }
        prompt.setDraftText(prompt.draftText() + text);
        return prompt.opened || prompt.visible;
    }

    function testTargetPromptOpen() {
        var prompt = targetPromptLoader.item;
        return prompt ? (prompt.opened || prompt.visible) : false;
    }

    function testSetTargetPromptDraft(text) {
        var prompt = ensureTargetPrompt(text ? text.charAt(0) : "");
        if (!prompt) {
            return;
        }
        if (text && text.length > 0) {
            prompt.setDraftText(text);
        }
    }

    function testTargetPromptPreview() {
        var prompt = targetPromptLoader.item;
        return prompt ? prompt.previewLine : "";
    }

    function testSubmitTargetPrompt() {
        if (targetPromptLoader.item) {
            targetPromptLoader.item.submitDraft();
        }
    }

    function testCancelTargetPrompt() {
        if (targetPromptLoader.item) {
            targetPromptLoader.item.close();
        }
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

    FocusScope {
        id: chartArea
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: priceAxis.left
        anchors.bottom: timeAxis.top

        function isPrintableTargetKey(event) {
            if (!event || event.modifiers !== Qt.NoModifier) {
                return false;
            }
            if (event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) {
                return false;
            }
            return event.text && event.text.length === 1 && /[a-zA-Z0-9]/.test(event.text);
        }

        TapHandler {
            objectName: "chartCanvasTap"
            target: chartArea
            gesturePolicy: TapHandler.ReleaseWithinBounds
            grabPermissions: PointerHandler.ApprovesTakeOverByAnything
            onTapped: chartArea.forceActiveFocus()
        }

        Keys.onPressed: function(event) {
            var prompt = targetPromptLoader.item;
            if (prompt && prompt.opened) {
                return;
            }
            if (!chartArea.isPrintableTargetKey(event)) {
                return;
            }
            ensureTargetPrompt(event.text);
            event.accepted = true;
        }

        Loader {
            id: targetPromptLoader
            objectName: "chartTargetPrompt"
            active: false
            property string pendingChar: ""
            x: Spacing.size8
            y: Spacing.size8
            sourceComponent: ChartTargetPrompt {
                context: root.context
                onCancelled: chartArea.forceActiveFocus()
                onClosed: {
                    if (!opened) {
                        chartArea.forceActiveFocus();
                    }
                }
            }
            onLoaded: {
                if (pendingChar.length > 0 && item) {
                    item.openWith(pendingChar);
                    pendingChar = "";
                }
            }
        }

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
