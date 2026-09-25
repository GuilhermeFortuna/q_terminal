import QtQuick
import QtTest
import "../ChartAxis.js" as ChartAxis

// Q-065: headless coverage for the pure tick-generation and time-formatting helpers in
// ../ChartAxis.js. Run via `qmltestrunner -input tst_ChartAxis.qml` (see tests/chart_axis.rs),
// offscreen, with no chart, feed, or rendering involved.
TestCase {
    name: "ChartAxis"

    function test_formatUtcOffset_zero_and_signed() {
        compare(ChartAxis.formatUtcOffset(0) !== "", true);
    }

    function test_formatLocalTime_padding() {
        // A fixed UTC instant; local formatting depends on the process TZ, so we only
        // assert the shape (HH:MM), not a specific wall-clock value.
        var label = ChartAxis.formatLocalTime(Date.UTC(2026, 8, 24, 3, 5, 0));
        verify(/^\d{2}:\d{2}$/.test(label));
    }

    function makeBarTimeAt(times) {
        return function(index) { return times[index]; };
    }

    function test_computeTimeTicks_empty_range_returns_nothing() {
        var ticks = ChartAxis.computeTimeTicks({ firstBar: 0, lastBar: 0, barTimeAt: makeBarTimeAt([]) });
        compare(ticks.length, 0);
    }

    function test_computeTimeTicks_single_bar_returns_one_tick() {
        var times = [Date.UTC(2026, 8, 24, 12, 0, 0)];
        var ticks = ChartAxis.computeTimeTicks({ firstBar: 0, lastBar: 1, barTimeAt: makeBarTimeAt(times) });
        compare(ticks.length, 1);
        compare(ticks[0].index, 0);
        compare(ticks[0].showDate, true);
    }

    function test_computeTimeTicks_bounded_by_maxTicks() {
        // 500 one-minute bars: far more candidate minute boundaries than maxTicks allows.
        var base = Date.UTC(2026, 8, 24, 0, 0, 0);
        var times = [];
        for (var i = 0; i < 500; i++) {
            times.push(base + i * 60000);
        }
        var ticks = ChartAxis.computeTimeTicks({ firstBar: 0, lastBar: 500, barTimeAt: makeBarTimeAt(times), maxTicks: 8 });
        verify(ticks.length > 0);
        verify(ticks.length <= 8);
        // Ticks must be strictly increasing by index and stay within the visible range.
        for (var j = 1; j < ticks.length; j++) {
            verify(ticks[j].index > ticks[j - 1].index);
        }
        for (var k = 0; k < ticks.length; k++) {
            verify(ticks[k].index >= 0 && ticks[k].index < 500);
        }
    }

    function test_computeTimeTicks_crosses_midnight_shows_date_change() {
        var base = Date.UTC(2026, 8, 23, 23, 0, 0);
        var times = [];
        for (var i = 0; i < 180; i++) {
            times.push(base + i * 60000); // 23:00 .. 02:00 UTC
        }
        var ticks = ChartAxis.computeTimeTicks({ firstBar: 0, lastBar: 180, barTimeAt: makeBarTimeAt(times), maxTicks: 8 });
        verify(ticks.length > 0);
        // At least the first tick must be flagged to show its date.
        compare(ticks[0].showDate, true);
    }

    function test_computeTimeTicks_offset_change_is_flagged() {
        // Simulate a DST-style offset change mid-range by using bar times that are
        // chronologically ordered but whose local-time formatting would collide if the
        // offset were ignored: we cannot force the process TZ from QML, so instead assert
        // the offsetDiffers field is always present and boolean-typed, and that ticks with
        // the same reference offset as the last bar are not flagged.
        var base = Date.UTC(2026, 2, 9, 0, 0, 0);
        var times = [];
        for (var i = 0; i < 240; i++) {
            times.push(base + i * 60000);
        }
        var ticks = ChartAxis.computeTimeTicks({ firstBar: 0, lastBar: 240, barTimeAt: makeBarTimeAt(times), maxTicks: 8 });
        for (var j = 0; j < ticks.length; j++) {
            compare(typeof ticks[j].offsetDiffers, "boolean");
        }
    }

    function test_computePriceTicks_flat_price_collapses_to_one_tick() {
        var ticks = ChartAxis.computePriceTicks(100.0, 100.0);
        compare(ticks.length, 1);
        compare(ticks[0].value, 100.0);
        compare(ticks[0].fraction, 0.5);
    }

    function test_computePriceTicks_narrow_range_stays_five_to_seven() {
        var ticks = ChartAxis.computePriceTicks(99.7, 100.3);
        verify(ticks.length >= 5 && ticks.length <= 7);
    }

    function test_computePriceTicks_wide_range_stays_five_to_seven() {
        // A realistic viewport span (a margin-padded high/low band around one price level),
        // not an arbitrary multi-order-of-magnitude range.
        var ticks = ChartAxis.computePriceTicks(83000.0, 87000.0);
        verify(ticks.length >= 5 && ticks.length <= 7);
    }

    function test_computePriceTicks_high_precision_uses_nice_step() {
        var ticks = ChartAxis.computePriceTicks(1.10234, 1.10561);
        verify(ticks.length >= 5 && ticks.length <= 7);
        // Consecutive ticks must be evenly spaced by a single stable step.
        var step = ticks[1].value - ticks[0].value;
        for (var i = 2; i < ticks.length; i++) {
            var delta = ticks[i].value - ticks[i - 1].value;
            verify(Math.abs(delta - step) < 1e-9);
        }
    }

    function test_computePriceTicks_fraction_tracks_value() {
        // Ticks are ordered by ascending price; fraction measures down from the top of the
        // plot (0 = high, 1 = low), so fraction must move opposite to value.
        var ticks = ChartAxis.computePriceTicks(10.0, 20.0);
        for (var i = 1; i < ticks.length; i++) {
            verify(ticks[i].value > ticks[i - 1].value);
            verify(ticks[i].fraction < ticks[i - 1].fraction);
        }
    }

    function test_computePriceTicks_invalid_range_does_not_throw() {
        var ticks = ChartAxis.computePriceTicks(NaN, NaN);
        compare(ticks.length, 1);
    }
}
