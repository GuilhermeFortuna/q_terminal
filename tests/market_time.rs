use q_buffers::frame::TimeLabel;
use q_terminal::bridge::bar_feed::{make_bar_columns, BarDelivery, BarFeedRust};
use q_terminal::history::{Loaded, Source};

// 2026-10-02 16:15 in the MT5 wall-clock representation.
const LOCAL_OPEN: i64 = 1_790_957_700_000_000;
const UTC_OPEN: i64 = LOCAL_OPEN + 3 * 3_600 * 1_000_000;

#[test]
fn lake_history_and_live_bars_share_utc_opens() {
    let mut feed = BarFeedRust::new("WDO$", "5m");
    let mut history = make_bar_columns(LOCAL_OPEN, 5243.0, 5245.0, 5241.5, 5243.5);
    history.label = TimeLabel::BrasiliaWallclock;
    feed.complete_history_load(Loaded {
        bars: history,
        source: Source::Lake,
        dataset: None,
        shortfall: 0,
        reason: None,
    });
    assert_eq!(feed.bar_times, vec![UTC_OPEN]);
    assert_eq!(feed.series_label, TimeLabel::Utc);
}

#[test]
fn forming_stream_bar_updates_the_same_utc_interval() {
    let mut feed = BarFeedRust::new("WDO$", "5m");
    feed.history_controller().open_gate();
    let mut bar = make_bar_columns(LOCAL_OPEN, 5243.0, 5245.0, 5241.5, 5243.5);
    bar.label = TimeLabel::BrasiliaWallclock;
    feed.apply_delivery(BarDelivery::Forming(bar));
    assert_eq!(feed.forming.unwrap().0, UTC_OPEN);
}

#[test]
fn history_seam_keeps_the_bars_before_the_live_utc_open() {
    let mut bars = make_bar_columns(LOCAL_OPEN, 5243.0, 5245.0, 5241.5, 5243.5);
    bars.label = TimeLabel::BrasiliaWallclock;
    let kept = q_terminal::history::trim_to_seam(bars, Some(UTC_OPEN));
    assert!(kept.time.is_empty());
}

#[test]
fn historical_wall_clock_uses_sao_paulo_daylight_saving() {
    let mut bars = make_bar_columns(1_516_017_600_000_123, 1.0, 2.0, 0.5, 1.5);
    bars.label = TimeLabel::BrasiliaWallclock;
    let utc = q_terminal::history::time::bars_to_utc(bars);
    // 2018-01-15 12:00 local was 14:00 UTC, with the microseconds preserved.
    assert_eq!(utc.time, vec![1_516_024_800_000_123]);
}

#[test]
fn stream_drainer_preserves_the_live_candle_at_the_history_boundary() {
    use cxx_qt::CxxQtType;
    use q_terminal::{chart_bridge, chart_target, stream};

    let _guard = chart_bridge::QT_TEST_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    chart_bridge::ensure_application();
    unsafe {
        let feed = chart_bridge::make_test_feed();
        q_terminal::bridge::bar_feed::apply_test_bar(
            feed.cast(),
            UTC_OPEN - 300_000_000,
            5245.0,
            5246.0,
            5242.0,
            5243.0,
            false,
        );
        let sink = stream::sink::BarSink::new();
        chart_target::install_bar_drainer(&sink, feed);
        sink.deliver_forming(stream::topic_state::BarColumns::single(
            LOCAL_OPEN, 5243.0, 5245.0, 5241.5, 5243.5, 100.0,
        ));
        chart_bridge::process_events();
        let feed = &*feed.cast::<q_terminal::bridge::bar_feed::ffi::BarFeed>();
        let latest = feed.rust().bar_snapshot(1);
        assert!(latest.valid);
        assert!(latest.forming);
        assert_eq!(latest.time, UTC_OPEN);
        assert_eq!(latest.open.to_bits(), 5243.0f64.to_bits());
        assert_eq!(latest.high.to_bits(), 5245.0f64.to_bits());
        assert_eq!(latest.low.to_bits(), 5241.5f64.to_bits());
        assert_eq!(latest.close.to_bits(), 5243.5f64.to_bits());
    }
}
