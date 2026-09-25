.pragma library

// ChartAxis.js - Pure tick-generation and time-formatting helpers for the chart's time
// and price axes (Q-065). No QML/Qt Quick types are touched here so this file can be
// exercised headlessly by qml/tests/tst_ChartAxis.qml. All time formatting uses the local
// JS Date accessors (getHours/getMinutes/...), never the UTC ones, so labels reflect the
// workstation's timezone including its DST transitions.

var MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function pad2(n) {
    return (n < 10 ? "0" : "") + n;
}

// Offset (in minutes) of the local timezone at a given epoch-ms instant, positive east of
// UTC. Computed per-instant (not once) so a DST transition is reflected correctly.
function offsetMinutesAt(ms) {
    return -new Date(ms).getTimezoneOffset();
}

// "UTC+HH:MM" / "UTC-HH:MM" / "UTC" for a given epoch-ms instant.
function formatUtcOffset(ms) {
    var minutes = offsetMinutesAt(ms);
    if (minutes === 0) {
        return "UTC";
    }
    var sign = minutes > 0 ? "+" : "-";
    var abs = Math.abs(minutes);
    var hh = Math.floor(abs / 60);
    var mm = abs % 60;
    return "UTC" + sign + pad2(hh) + ":" + pad2(mm);
}

function formatLocalTime(ms) {
    var d = new Date(ms);
    return pad2(d.getHours()) + ":" + pad2(d.getMinutes());
}

function formatLocalDate(ms) {
    var d = new Date(ms);
    return MONTH_NAMES[d.getMonth()] + " " + d.getDate();
}

// Calendar-day key in local time, used to detect a day boundary between two ticks.
function localDateKey(ms) {
    var d = new Date(ms);
    return d.getFullYear() * 10000 + (d.getMonth() + 1) * 100 + d.getDate();
}

// Local midnight (00:00:00.000) of the day containing `ms`.
function localMidnight(ms) {
    var d = new Date(ms);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
}

// ---------------------------------------------------------------------------------------
// Time ticks
// ---------------------------------------------------------------------------------------

// Ladder of "nice" step sizes in milliseconds, ascending, covering seconds through days.
var TIME_STEP_LADDER = (function() {
    var steps = [];
    var secondUnits = [1, 2, 5, 10, 15, 30];
    var minuteUnits = [1, 2, 5, 10, 15, 30];
    var hourUnits = [1, 2, 3, 4, 6, 8, 12];
    var dayUnits = [1, 2, 5, 10, 30, 90, 180, 365];
    var i;
    for (i = 0; i < secondUnits.length; i++) steps.push(secondUnits[i] * 1000);
    for (i = 0; i < minuteUnits.length; i++) steps.push(minuteUnits[i] * 60000);
    for (i = 0; i < hourUnits.length; i++) steps.push(hourUnits[i] * 3600000);
    for (i = 0; i < dayUnits.length; i++) steps.push(dayUnits[i] * 86400000);
    return steps;
})();

// Picks the smallest ladder step whose tick count over `spanMs` does not exceed maxTicks.
function pickTimeStep(spanMs, maxTicks) {
    if (spanMs <= 0) {
        return TIME_STEP_LADDER[0];
    }
    for (var i = 0; i < TIME_STEP_LADDER.length; i++) {
        var step = TIME_STEP_LADDER[i];
        if (spanMs / step <= Math.max(1, maxTicks - 1)) {
            return step;
        }
    }
    return TIME_STEP_LADDER[TIME_STEP_LADDER.length - 1];
}

// The first boundary >= ms for the given step, expressed in local time so sub-day steps
// land on round local minutes/hours and day-or-larger steps land on local midnight.
function firstBoundaryAtOrAfter(ms, stepMs) {
    if (stepMs >= 86400000) {
        var dayCount = Math.max(1, Math.round(stepMs / 86400000));
        var boundary = localMidnight(ms);
        if (boundary < ms) {
            var d = new Date(boundary);
            d.setDate(d.getDate() + dayCount);
            boundary = d.getTime();
        }
        return { boundary: boundary, dayCount: dayCount };
    }
    var offsetMs = offsetMinutesAt(ms) * 60000;
    var localMs = ms + offsetMs;
    var boundaryLocal = Math.ceil(localMs / stepMs) * stepMs;
    return { boundary: boundaryLocal - offsetMs, dayCount: 0 };
}

function nextBoundary(boundary, stepMs, dayCount) {
    if (dayCount > 0) {
        var d = new Date(boundary);
        d.setDate(d.getDate() + dayCount);
        return d.getTime();
    }
    return boundary + stepMs;
}

// Finds the visible bar index whose time is nearest to `targetMs`, searching only within
// [firstBar, lastBar) via binary search (bar times are assumed non-decreasing). `barTimeAt`
// is a callback (index) -> epoch-ms, bounded to O(log n) calls per lookup.
function nearestBarIndex(firstBar, lastBar, barTimeAt, targetMs) {
    if (lastBar <= firstBar) {
        return -1;
    }
    var lo = firstBar;
    var hi = lastBar - 1;
    while (lo < hi) {
        var mid = Math.floor((lo + hi) / 2);
        var t = barTimeAt(mid);
        if (t < targetMs) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if (lo > firstBar) {
        var prevT = barTimeAt(lo - 1);
        var curT = barTimeAt(lo);
        if (Math.abs(prevT - targetMs) <= Math.abs(curT - targetMs)) {
            return lo - 1;
        }
    }
    return lo;
}

// Computes a bounded set of major time ticks from the visible bar range.
//
// opts:
//   firstBar, lastBar: visible bar index range (exclusive lastBar), as in Viewport.qml
//   barTimeAt(index): epoch-ms of a bar index, valid for [firstBar, lastBar)
//   maxTicks: upper bound on the number of ticks returned (default 8)
//
// Returns an array of { index, time, x, label, dateLabel, showDate, offsetLabel,
// offsetDiffers }, ordered by increasing index, x in [0, 1] (fraction of plot width,
// bar-centered exactly like the crosshair transform).
function computeTimeTicks(opts) {
    var firstBar = opts.firstBar;
    var lastBar = opts.lastBar;
    var barTimeAt = opts.barTimeAt;
    var maxTicks = opts.maxTicks || 8;
    var ticks = [];
    if (lastBar <= firstBar || typeof barTimeAt !== "function") {
        return ticks;
    }
    var span = Math.max(1, lastBar - firstBar);
    var firstTime = barTimeAt(firstBar);
    var lastTime = barTimeAt(lastBar - 1);
    if (lastBar - firstBar <= 1 || !(lastTime > firstTime)) {
        // One visible bar (or a degenerate/non-increasing range): a single tick at it.
        var onlyIndex = firstBar;
        var onlyTime = firstTime;
        return [{
            index: onlyIndex,
            time: onlyTime,
            x: (onlyIndex - firstBar + 0.5) / span,
            label: formatLocalTime(onlyTime),
            dateLabel: formatLocalDate(onlyTime),
            showDate: true,
            offsetLabel: formatUtcOffset(onlyTime),
            offsetDiffers: false
        }];
    }
    var referenceOffset = offsetMinutesAt(lastTime);
    var spanMs = lastTime - firstTime;
    var step = pickTimeStep(spanMs, maxTicks);
    var b = firstBoundaryAtOrAfter(firstTime, step);
    var boundary = b.boundary;
    var lastDateKey = null;
    var lastIndex = -1;
    var guard = 0;
    while (boundary <= lastTime && ticks.length < maxTicks && guard < maxTicks * 4) {
        guard++;
        var idx = nearestBarIndex(firstBar, lastBar, barTimeAt, boundary);
        if (idx >= firstBar && idx !== lastIndex) {
            var t = barTimeAt(idx);
            var dateKey = localDateKey(t);
            var showDate = lastDateKey === null || dateKey !== lastDateKey;
            var offset = offsetMinutesAt(t);
            ticks.push({
                index: idx,
                time: t,
                x: (idx - firstBar + 0.5) / span,
                label: formatLocalTime(t),
                dateLabel: formatLocalDate(t),
                showDate: showDate,
                offsetLabel: formatUtcOffset(t),
                offsetDiffers: offset !== referenceOffset
            });
            lastDateKey = dateKey;
            lastIndex = idx;
        }
        boundary = nextBoundary(boundary, step, b.dayCount);
    }
    return ticks;
}

// ---------------------------------------------------------------------------------------
// Price ticks
// ---------------------------------------------------------------------------------------

var PRICE_STEP_LADDER = [1, 2, 5, 10];

function stepMagnitudeIndex(step) {
    var mag = Math.pow(10, Math.floor(Math.log(step) / Math.LN10 + 1e-9));
    var residual = Math.round((step / mag) * 1000) / 1000;
    var closestIndex = 0;
    var closestDelta = Infinity;
    for (var i = 0; i < PRICE_STEP_LADDER.length; i++) {
        var delta = Math.abs(PRICE_STEP_LADDER[i] - residual);
        if (delta < closestDelta) {
            closestDelta = delta;
            closestIndex = i;
        }
    }
    return { mag: mag, index: closestIndex };
}

// Steps the 1/2/5x10^n ladder up (dir > 0, coarser) or down (dir < 0, finer) by one rung.
function bumpPriceStep(step, dir) {
    var found = stepMagnitudeIndex(step);
    var mag = found.mag;
    var index = found.index + dir;
    if (index < 0) {
        return PRICE_STEP_LADDER[PRICE_STEP_LADDER.length - 2] * (mag / 10);
    }
    if (index >= PRICE_STEP_LADDER.length) {
        return PRICE_STEP_LADDER[0] * (mag * 10);
    }
    return PRICE_STEP_LADDER[index] * mag;
}

function niceStep(rawStep) {
    if (!(rawStep > 0)) {
        return 1;
    }
    var mag = Math.pow(10, Math.floor(Math.log(rawStep) / Math.LN10));
    var residual = rawStep / mag;
    var niceResidual;
    if (residual <= 1) niceResidual = 1;
    else if (residual <= 2) niceResidual = 2;
    else if (residual <= 5) niceResidual = 5;
    else niceResidual = 10;
    return niceResidual * mag;
}

function priceTickCount(low, high, step) {
    var start = Math.ceil(low / step) * step;
    var count = 0;
    for (var v = start; v <= high + step * 1e-9; v += step) {
        count++;
        if (count > 1000) {
            break;
        }
    }
    return count;
}

// Computes five to seven major price ticks from the visible high/low bounds using a stable
// 1/2/5 x 10^n step. Returns [{ value, fraction }] with fraction in [0, 1] measuring down
// from the top of the plot (0 = highPrice, 1 = lowPrice), matching the existing crosshairY
// transform. A flat or invalid range collapses to a single tick.
function computePriceTicks(low, high, opts) {
    opts = opts || {};
    var minTicks = opts.minTicks || 5;
    var maxTicks = opts.maxTicks || 7;
    var target = opts.targetTicks || 6;
    if (!(high > low) || !isFinite(low) || !isFinite(high)) {
        var flatValue = isFinite(low) ? low : (isFinite(high) ? high : 0);
        return [{ value: flatValue, fraction: 0.5 }];
    }
    var span = high - low;
    var step = niceStep(span / target);
    var count = priceTickCount(low, high, step);
    var guard = 0;
    while (count > maxTicks && guard < 20) {
        step = bumpPriceStep(step, 1);
        count = priceTickCount(low, high, step);
        guard++;
    }
    guard = 0;
    while (count < minTicks && guard < 20) {
        var finer = bumpPriceStep(step, -1);
        var finerCount = priceTickCount(low, high, finer);
        if (finerCount > maxTicks) {
            break;
        }
        step = finer;
        count = finerCount;
        guard++;
    }
    var ticks = [];
    var start = Math.ceil(low / step) * step;
    for (var v = start; v <= high + step * 1e-9 && ticks.length < maxTicks; v += step) {
        var value = Math.round(v / step) * step;
        ticks.push({ value: value, fraction: (high - value) / span });
    }
    if (ticks.length === 0) {
        ticks.push({ value: (low + high) / 2, fraction: 0.5 });
    }
    return ticks;
}
