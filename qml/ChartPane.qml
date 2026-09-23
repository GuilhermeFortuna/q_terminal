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
    readonly property bool inspectingHistory: viewport.inspectingHistory
    readonly property bool atLoadedHistoryBoundary: !viewport.empty && viewport.firstBar === 0
    readonly property string topPriceLabel: viewport.empty ? "" : (root.feed && root.feed.format_price ? root.feed.format_price(viewport.highPrice) : viewport.highPrice.toFixed(2))
    readonly property string bottomPriceLabel: viewport.empty ? "" : (root.feed && root.feed.format_price ? root.feed.format_price(viewport.lowPrice) : viewport.lowPrice.toFixed(2))

    property int keyboardSelectedBar: -1
    property bool inspectionDismissed: false
    property bool testHoverActive: false
    property real testHoverX: 0.0
    property real testHoverY: 0.0

    readonly property bool effectiveHovered: (hover.hovered || testHoverActive)
    readonly property real effectiveHoverX: testHoverActive ? testHoverX : hover.point.position.x
    readonly property real effectiveHoverY: testHoverActive ? testHoverY : hover.point.position.y

    readonly property bool isTargetSwitching: (root.context && root.context.is_switching) || (root.feed && root.feed.history_loading && viewport.empty)

    readonly property int pointerBarIndex: {
        if (!effectiveHovered || inspectionDismissed || viewport.empty || chartArea.width <= 0) {
            return -1;
        }
        var span = Math.max(1, viewport.lastBar - viewport.firstBar);
        var barWidth = chartArea.width / span;
        if (barWidth <= 0) {
            return -1;
        }
        var relX = Math.max(0, Math.min(chartArea.width - 1, effectiveHoverX));
        var offset = Math.floor(relX / barWidth);
        offset = Math.max(0, Math.min(span - 1, offset));
        var idx = viewport.firstBar + offset;
        return Math.max(0, Math.min(viewport.barCount - 1, idx));
    }

    readonly property int activeBarIndex: {
        if (viewport.empty || isTargetSwitching) {
            return -1;
        }
        if (pointerBarIndex >= 0) {
            return pointerBarIndex;
        }
        // A selected bar that scrolled out of view is hidden rather than clamped, so the
        // readout never shows a bar other than the one selected.
        if (keyboardSelectedBar >= viewport.firstBar && keyboardSelectedBar < viewport.lastBar
                && !inspectionDismissed) {
            return keyboardSelectedBar;
        }
        return -1;
    }

    readonly property var barSnapshot: {
        var rev = root.feed ? root.feed.revision : 0;
        if (!root.feed || activeBarIndex < 0 || !root.feed.bar_snapshot_json || isTargetSwitching || viewport.empty) {
            return ({ valid: false });
        }
        try {
            return JSON.parse(root.feed.bar_snapshot_json(activeBarIndex));
        } catch (error) {
            return ({ valid: false });
        }
    }

    readonly property bool crosshairVisible: !viewport.empty && !isTargetSwitching && activeBarIndex >= 0 && activeBarIndex >= viewport.firstBar && activeBarIndex < viewport.lastBar && barSnapshot.valid

    readonly property real crosshairX: {
        var span = Math.max(1, viewport.lastBar - viewport.firstBar);
        var barWidth = chartArea.width / span;
        var offset = activeBarIndex - viewport.firstBar;
        return (offset + 0.5) * barWidth;
    }

    readonly property real crosshairY: {
        if (effectiveHovered && !inspectionDismissed) {
            return Math.max(0, Math.min(chartArea.height - 1, effectiveHoverY));
        }
        if (barSnapshot.valid && chartArea.height > 0) {
            var span = viewport.highPrice - viewport.lowPrice;
            if (span > 0) {
                var fraction = (viewport.highPrice - barSnapshot.close) / span;
                return Math.max(0, Math.min(chartArea.height - 1, fraction * chartArea.height));
            }
        }
        return chartArea.height * 0.5;
    }

    readonly property string pointerPriceText: {
        if (!root.feed || !effectiveHovered || inspectionDismissed || chartArea.height <= 0 || viewport.empty) {
            return "";
        }
        var fraction = Math.max(0.0, Math.min(1.0, effectiveHoverY / chartArea.height));
        var pVal = viewport.highPrice - fraction * (viewport.highPrice - viewport.lowPrice);
        return root.feed.format_price ? root.feed.format_price(pVal) : pVal.toFixed(2);
    }

    readonly property string accessibleReadoutText: {
        if (!barSnapshot.valid) {
            return "";
        }
        var res = "Bar " + (barSnapshot.time_text || "") + (barSnapshot.forming ? " forming" : "") +
                  " open " + (barSnapshot.open_text || "--") + " high " + (barSnapshot.high_text || "--") +
                  " low " + (barSnapshot.low_text || "--") + " close " + (barSnapshot.close_text || "--");
        if (pointerPriceText !== "") {
            res += " pointer " + pointerPriceText;
        }
        return res;
    }

    function selectAdjacentBar(delta) {
        if (viewport.empty) {
            return;
        }
        inspectionDismissed = false;
        var minIdx = viewport.firstBar;
        var maxIdx = Math.max(viewport.firstBar, viewport.lastBar - 1);
        if (keyboardSelectedBar < minIdx || keyboardSelectedBar > maxIdx) {
            if (delta < 0) {
                keyboardSelectedBar = maxIdx;
            } else {
                keyboardSelectedBar = minIdx;
            }
        } else {
            var next = keyboardSelectedBar + delta;
            keyboardSelectedBar = Math.max(minIdx, Math.min(maxIdx, next));
        }
    }

    function clearKeyboardSelection() {
        keyboardSelectedBar = -1;
        inspectionDismissed = true;
    }

    // A new symbol or timeframe starts from the live edge with nothing selected (Q-060).
    function resetForTarget() {
        keyboardSelectedBar = -1;
        viewport.resetForTarget();
    }

    Connections {
        target: root.feed
        ignoreUnknownSignals: true
        function onSymbolChanged() { root.resetForTarget(); }
        function onTimeframeChanged() { root.resetForTarget(); }
    }

    function updateTip() {
        if (!root.feed || !effectiveHovered) {
            tipText.text = "";
            return;
        }
        tipText.text = root.feed.marker_detail_at(effectiveHoverX, effectiveHoverY, 10);
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

    function testSelectAdjacentBar(delta) {
        selectAdjacentBar(delta);
    }

    function testClearSelection() {
        clearKeyboardSelection();
    }

    function testSetHover(x, y) {
        testHoverActive = true;
        testHoverX = x;
        testHoverY = y;
        inspectionDismissed = false;
        updateTip();
    }

    function testClearHover() {
        testHoverActive = false;
        updateTip();
    }

    function testActiveBarIndex() {
        return activeBarIndex;
    }

    function testCrosshairVisible() {
        return crosshairVisible;
    }

    function testReadoutTime() {
        return barSnapshot.time_text || "";
    }

    function testReadoutOpen() {
        return barSnapshot.open_text || "";
    }

    function testReadoutHigh() {
        return barSnapshot.high_text || "";
    }

    function testReadoutLow() {
        return barSnapshot.low_text || "";
    }

    function testReadoutClose() {
        return barSnapshot.close_text || "";
    }

    function testReadoutForming() {
        return !!barSnapshot.forming;
    }

    function testReadoutPointerPrice() {
        return pointerPriceText;
    }

    function testCrosshairX() {
        return crosshairX;
    }

    function testCrosshairY() {
        return crosshairY;
    }

    function testAccessibleText() {
        return accessibleReadoutText;
    }

    function testMarkerTooltipText() {
        return tipText.text;
    }

    function testMarkerTooltipVisible() {
        return tip.visible;
    }

    function testKeyboardSelectedBar() {
        return keyboardSelectedBar;
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
        low: root.feed ? root.feed.low : 0.0
        high: root.feed ? root.feed.high : 0.0
        rangeSource: root.feed && root.feed.visible_range_json ? root.feed : null
        revision: root.feed ? root.feed.revision : 0
        barsVisible: root.barsVisible
        priceMargin: root.priceMargin
        onEmptyChanged: {
            if (empty) {
                root.keyboardSelectedBar = -1;
            }
        }
    }

    FocusScope {
        id: chartArea
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: priceAxis.left
        anchors.bottom: timeAxis.top

        Accessible.role: Accessible.Grouping
        Accessible.name: root.accessibleReadoutText !== "" ? root.accessibleReadoutText : "Chart Canvas"

        function isPrintableTargetKey(event) {
            // Shift (capitals) and the keypad are ordinary typing; any other modifier is a
            // shortcut and never opens the prompt.
            if (!event || (event.modifiers & ~(Qt.ShiftModifier | Qt.KeypadModifier)) !== Qt.NoModifier) {
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
            if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal) {
                viewport.zoomAt(0.5, 1);
                event.accepted = true;
                return;
            }
            if (event.key === Qt.Key_Minus) {
                viewport.zoomAt(0.5, -1);
                event.accepted = true;
                return;
            }
            if (event.key === Qt.Key_Left) {
                root.selectAdjacentBar(-1);
                event.accepted = true;
                return;
            }
            if (event.key === Qt.Key_Right) {
                root.selectAdjacentBar(1);
                event.accepted = true;
                return;
            }
            if (event.key === Qt.Key_Escape) {
                root.clearKeyboardSelection();
                event.accepted = true;
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

            // Crosshair vertical line at nearest visible bar
            Rectangle {
                id: crosshairVertical
                objectName: "crosshairVertical"
                visible: root.crosshairVisible
                x: Math.round(root.crosshairX)
                y: 0
                width: Spacing.size1
                height: chartContainer.height
                color: Theme.borderStrong
                z: 1
            }

            // Crosshair horizontal line at pointer price
            Rectangle {
                id: crosshairHorizontal
                objectName: "crosshairHorizontal"
                visible: root.crosshairVisible
                x: 0
                y: Math.round(root.crosshairY)
                width: chartContainer.width
                height: Spacing.size1
                color: Theme.borderStrong
                z: 1
            }

            HoverHandler {
                id: hover
                onPointChanged: {
                    root.inspectionDismissed = false;
                    root.updateTip();
                }
                onHoveredChanged: {
                    if (!hovered) {
                        root.updateTip();
                    }
                }
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
                // One mouse notch is 120 angle units; touchpads send many smaller deltas,
                // which accumulate so a gesture zooms as far as the same wheel travel.
                readonly property int unitsPerStep: 120
                property int pendingUnits: 0
                onWheel: function(event) {
                    if (event.angleDelta.y === 0) {
                        return;
                    }
                    pendingUnits += event.angleDelta.y;
                    var steps = Math.trunc(pendingUnits / unitsPerStep);
                    if (steps !== 0) {
                        pendingUnits -= steps * unitsPerStep;
                        // WheelEvent carries x/y (not position), in this handler's parent
                        // item coordinates, which is the plot.
                        viewport.zoomAt(event.x / Math.max(1, chartContainer.width), steps);
                    }
                    event.accepted = true;
                }
            }

            Rectangle {
                id: tip
                objectName: "markerTooltip"
                visible: tipText.text !== ""
                x: Math.min(root.effectiveHoverX + Spacing.size12, chartContainer.width - width)
                y: Math.min(root.effectiveHoverY + Spacing.size12, chartContainer.height - height)
                width: tipText.implicitWidth + Spacing.size12
                height: tipText.implicitHeight + Spacing.size8
                color: Theme.surfaceElevated
                border.color: Theme.borderStrong
                z: 4

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

        focus: true
        activeFocusOnTab: true
    }

    // Beside the navigation status when both fit on one line; otherwise below it, wrapping
    // within the plot width so no value is clipped at narrow panel sizes.
    readonly property real readoutAvailableWidth: chartArea.width - 2 * Spacing.size8
    readonly property bool readoutFitsBeside: barReadout.naturalRowWidth + Spacing.size16
        + Spacing.size8 + navigationStatus.width <= root.readoutAvailableWidth

    BarReadout {
        id: barReadout
        objectName: "barReadout"
        x: Spacing.size8
        y: root.readoutFitsBeside ? Spacing.size4 : navigationStatus.y + navigationStatus.height + Spacing.size4
        maximumWidth: root.readoutFitsBeside
            ? root.readoutAvailableWidth - navigationStatus.width - Spacing.size8
            : root.readoutAvailableWidth
        width: implicitWidth
        height: implicitHeight
        z: 2
        previewState: ""
        snapshot: root.barSnapshot
        pointerPrice: root.pointerPriceText
        visible: root.crosshairVisible && !!root.barSnapshot.valid
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

        Rectangle {
            id: crosshairPriceBadge
            objectName: "crosshairPriceBadge"
            visible: root.crosshairVisible && root.pointerPriceText !== ""
            anchors.left: parent.left
            y: Math.max(0, Math.min(priceAxis.height - height, root.crosshairY - height / 2))
            width: crosshairPriceText.implicitWidth + Spacing.size8
            height: Spacing.badgeHeight
            color: Theme.surfaceElevated
            border.color: Theme.borderStrong
            radius: Spacing.radiusSmall
            z: 3

            Text {
                id: crosshairPriceText
                objectName: "crosshairPriceText"
                anchors.centerIn: parent
                text: root.pointerPriceText
                color: Theme.textPrimary
                font.pixelSize: Theme.typeLabelSmall
                font.family: Theme.numericFontFamily
                font.features: { "tnum": 1 }
            }
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
