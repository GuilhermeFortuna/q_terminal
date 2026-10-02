//! Q-082 criteria 3–4: tape studies, markers and the tape rows agree with the Q-081 kernel,
//! exchange-local chart labels line up with UTC trades, and display filters change nothing
//! the analysis counted.
mod trade_common;

use q_buffers::frame::{BarColumns, TimeLabel};
use q_indicators::volume::VolumeConfig;
use q_terminal::studies::{StudyKind, StudySet, TradeInputs, TradeLegend};
use q_terminal::trades::analysis::VolumeParams;
use q_terminal::trades::feed::TradeFeed;
use q_terminal::trades::model::{SideFilter, TapeFilter, TapeView, MAX_TAPE_ROWS};
use q_terminal::trades::time::ExchangeClock;
use trade_common::*;

fn rows() -> Vec<Row> {
    vec![
        (0, 128_450.0, 2.0, BUY, 0),
        (0, 128_450.0, 2.0, BUY, 1),
        (10_000, 128_455.0, 5.0, SELL, 0),
        (59_999, 128_450.0, 1.0, UNKNOWN, 0),
        (MIN, 128_460.0, 120.0, BUY, 0),
        (MIN + 500, 128_455.0, 3.0, SELL, 0),
        (2 * MIN + 1, 128_445.0, 4.0, UNKNOWN, 0),
        (2 * MIN + 2, 128_446.0, 150.0, UNKNOWN, 0),
    ]
}

/// Chart bars labelled in naive exchange-local time, as the stream and lake deliver them.
fn local_bars(count: usize, first_utc_offset_min: i64) -> BarColumns {
    let clock = ExchangeClock::for_zone("America/Sao_Paulo").unwrap();
    // Microseconds, like the bar series: local wall clock of each bar open.
    let time: Vec<i64> = (0..count as i64)
        .map(|i| clock.utc_to_local_ms(T0 + (first_utc_offset_min + i) * MIN) * 1000)
        .collect();
    let n = count;
    BarColumns {
        time,
        open: vec![128_450.0; n],
        high: vec![128_470.0; n],
        low: vec![128_440.0; n],
        close: vec![128_455.0; n],
        tick_volume: None,
        spread: None,
        real_volume: Some(vec![10; n]),
        label: TimeLabel::BrasiliaWallclock,
    }
}

fn legend(feed: &TradeFeed) -> TradeLegend {
    let report = feed.report();
    TradeLegend {
        unit: report.volume_unit,
        field: report.volume_field,
        coverage: if report.complete {
            "complete"
        } else {
            "partial"
        }
        .to_string(),
        source_coverage: report.coverage.state,
        classified_share: report.classified_share,
    }
}

fn rebuild(set: &mut StudySet, feed: &TradeFeed, bars: &BarColumns) {
    let inputs = TradeInputs {
        analyses: feed.analyses(),
        clock: feed.clock().unwrap(),
        legend: legend(feed),
    };
    set.rebuild_with(
        TimeLabel::BrasiliaWallclock,
        std::slice::from_ref(bars),
        None,
        Some(&inputs),
    );
}

fn series<'a>(set: &'a StudySet, suffix: &str) -> &'a [(i64, Option<f64>)] {
    &set.overlays()
        .iter()
        .find(|s| s.key.ends_with(suffix))
        .unwrap_or_else(|| panic!("no {suffix} series"))
        .points
}

fn ms_label(bars: &BarColumns, i: usize) -> i64 {
    bars.time[i] / 1000
}

#[test]
fn delta_cvd_and_rate_are_the_kernel_outputs_aligned_to_local_chart_labels() {
    let feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    set.add_volume_study(StudyKind::Delta, VolumeParams::default())
        .unwrap();
    set.add_volume_study(StudyKind::CumulativeDelta, VolumeParams::default())
        .unwrap();
    set.add_volume_study(StudyKind::TradeRate, VolumeParams::default())
        .unwrap();
    let bars = local_bars(3, 0);
    rebuild(&mut set, &feed, &bars);
    let expected = oracle(&rows(), MIN, VolumeConfig::default());
    for (i, (_, out)) in expected.iter().enumerate() {
        let label = ms_label(&bars, i);
        let delta = series(&set, "-delta")[i];
        assert_eq!(delta.0, label, "points are keyed by the chart's own label");
        // The third bar holds only unknown-side prints: no direction is reported.
        let directional = out.classified_share.is_some_and(|c| c > 0.0);
        assert_eq!(
            delta.1.map(f64::to_bits),
            directional.then_some(out.delta.to_bits())
        );
        assert_eq!(
            series(&set, "-cvd")[i].1.map(f64::to_bits),
            Some(out.cumulative_delta.to_bits())
        );
        assert_eq!(
            series(&set, "-rate")[i].1.map(f64::to_bits),
            out.trade_rate.map(f64::to_bits)
        );
    }
    assert_eq!(
        series(&set, "-delta")[2].1,
        None,
        "unknown-only volume has no delta"
    );
}

#[test]
fn each_tape_study_gets_its_own_oscillator_pane() {
    let feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    for kind in [
        StudyKind::Delta,
        StudyKind::CumulativeDelta,
        StudyKind::TradeRate,
    ] {
        set.add_volume_study(kind, VolumeParams::default()).unwrap();
    }
    rebuild(&mut set, &feed, &local_bars(3, 0));
    let slots: Vec<_> = set.overlays().iter().map(|s| s.osc_slot).collect();
    assert_eq!(slots.len(), 3);
    assert!(slots.iter().all(Option::is_some));
    let mut unique = slots.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3, "independent panes");
}

#[test]
fn bars_before_the_loaded_tape_are_unavailable_gaps_not_zeros() {
    let feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    set.add_volume_study(StudyKind::CumulativeDelta, VolumeParams::default())
        .unwrap();
    // Two earlier bars the tape never covered, then the three it did.
    let bars = local_bars(5, -2);
    rebuild(&mut set, &feed, &bars);
    let cvd = series(&set, "-cvd");
    let at = |i: usize| cvd.iter().find(|p| p.0 == ms_label(&bars, i));
    assert!(
        at(0).is_none() && at(1).is_none(),
        "no point where the tape never reached"
    );
    assert!(at(2).and_then(|p| p.1).is_some());
    let values: serde_json::Value =
        serde_json::from_str(&set.values_json(ms_label(&bars, 0))).unwrap();
    assert_eq!(values[0]["text"], "no tape for this bar");
}

#[test]
fn utc_labelled_bars_need_no_conversion() {
    let feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    set.add_volume_study(StudyKind::Delta, VolumeParams::default())
        .unwrap();
    let mut bars = local_bars(3, 0);
    bars.time = (0..3).map(|i| (T0 + i * MIN) * 1000).collect();
    bars.label = TimeLabel::Utc;
    let inputs = TradeInputs {
        analyses: feed.analyses(),
        clock: feed.clock().unwrap(),
        legend: legend(&feed),
    };
    set.rebuild_with(
        TimeLabel::Utc,
        std::slice::from_ref(&bars),
        None,
        Some(&inputs),
    );
    let expected = oracle(&rows(), MIN, VolumeConfig::default());
    assert_eq!(
        series(&set, "-delta")[0].1.map(f64::to_bits),
        Some(expected[0].1.delta.to_bits())
    );
}

#[test]
fn large_print_markers_follow_the_threshold_and_cover_the_latest_prints() {
    let mut feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    let id = set
        .add_volume_study(
            StudyKind::LargePrints,
            VolumeParams {
                window_ms: 10_000,
                large_print_threshold: 100.0,
            },
        )
        .unwrap();
    feed.set_params(&set.volume_params());
    let job = feed.plan_rebuild().unwrap();
    feed.install_rebuild(job.run());
    let bars = local_bars(3, 0);
    let opens: Vec<i64> = (0..3).map(|i| ms_label(&bars, i)).collect();
    fn inputs(feed: &TradeFeed) -> TradeInputs<'_> {
        TradeInputs {
            analyses: feed.analyses(),
            clock: feed.clock().unwrap(),
            legend: legend(feed),
        }
    }
    rebuild(&mut set, &feed, &bars);
    let markers = set.large_print_markers(&opens, &inputs(&feed));
    // 120 BUY in bar 1 and 150 with unknown side in bar 2.
    let kinds: Vec<_> = markers.iter().map(|m| m.kind.as_str()).collect();
    assert_eq!(kinds, vec!["large_buy", "large_unknown"]);
    assert_eq!(markers[0].bar_index, 1);
    assert!(markers[0].detail.contains("120 contracts"));
    assert!(markers[1].label.contains("UNKNOWN SIDE"));

    // Editing the threshold replays the whole cached session, not the tape rows.
    set.update_volume_study(
        id,
        VolumeParams {
            window_ms: 10_000,
            large_print_threshold: 4.0,
        },
    )
    .unwrap();
    feed.set_params(&set.volume_params());
    let job = feed.plan_rebuild().unwrap();
    feed.install_rebuild(job.run());
    rebuild(&mut set, &feed, &bars);
    let markers = set.large_print_markers(&opens, &inputs(&feed));
    assert_eq!(markers.len(), 4, "prints of 5, 120, 4 and 150");
}

#[test]
fn the_readout_names_units_parameters_and_coverage() {
    let feed = live_feed(&rows(), MIN);
    let mut set = StudySet::new();
    set.add_volume_study(StudyKind::Delta, VolumeParams::default())
        .unwrap();
    set.add_volume_study(
        StudyKind::TradeRate,
        VolumeParams {
            window_ms: 5_000,
            large_print_threshold: 100.0,
        },
    )
    .unwrap();
    let bars = local_bars(3, 0);
    rebuild(&mut set, &feed, &bars);
    let list: serde_json::Value = serde_json::from_str(&set.study_list_json()).unwrap();
    assert_eq!(list["studies"][0]["name"], "Delta (contracts)");
    assert_eq!(list["studies"][1]["name"], "Trade rate (trades/s, 5s)");
    assert_eq!(list["tape"]["unit"], "contracts");
    assert_eq!(list["studies"][0]["unit"], "contracts");
    let values: serde_json::Value =
        serde_json::from_str(&set.values_json(ms_label(&bars, 0))).unwrap();
    let delta = values[0]["text"].as_str().unwrap();
    assert!(
        delta.contains("contracts") && delta.contains("classified"),
        "{delta}"
    );
    // Unknown-only bar: no direction, never a recommendation.
    let unknown: serde_json::Value =
        serde_json::from_str(&set.values_json(ms_label(&bars, 2))).unwrap();
    assert_eq!(unknown[0]["text"], "no aggressor side reported");
}

#[test]
fn restoring_saved_volume_studies_keeps_typed_parameters() {
    let mut set = StudySet::new();
    set.restore_json(
        r#"[{"kind":"trade_rate","window_ms":30000,"large_print_threshold":100.0,"palette_index":2,"visible":false},
            {"kind":"large_prints","window_ms":10000,"large_print_threshold":250.0},
            {"kind":"large_prints","window_ms":10000,"large_print_threshold":-1.0},
            {"kind":"ema","period":9,"source":"close","num_std":2.0}]"#,
    )
    .unwrap();
    assert_eq!(
        set.specs().len(),
        3,
        "the invalid threshold is dropped, the rest kept"
    );
    assert_eq!(set.specs()[0].volume.window_ms, 30_000);
    assert!(!set.specs()[0].visible);
    assert_eq!(
        set.specs()[1].volume.large_print_threshold.to_bits(),
        250.0f64.to_bits()
    );
    assert_eq!(set.specs()[2].kind, StudyKind::Ema);
    let list: serde_json::Value = serde_json::from_str(&set.study_list_json()).unwrap();
    assert_eq!(list["studies"][0]["window_ms"], 30_000);
}

#[test]
fn eight_studies_remain_the_total_maximum() {
    let mut set = StudySet::new();
    for _ in 0..6 {
        set.add_study(
            StudyKind::Ema,
            5,
            q_terminal::studies::PriceSource::Close,
            2.0,
        )
        .unwrap();
    }
    set.add_volume_study(StudyKind::Delta, VolumeParams::default())
        .unwrap();
    set.add_volume_study(StudyKind::TradeRate, VolumeParams::default())
        .unwrap();
    assert!(set
        .add_volume_study(StudyKind::LargePrints, VolumeParams::default())
        .is_err());
    assert!(set
        .add_study(
            StudyKind::Sma,
            5,
            q_terminal::studies::PriceSource::Close,
            2.0
        )
        .is_err());
}

#[test]
fn invalid_kernel_parameters_are_refused() {
    let mut set = StudySet::new();
    assert!(set
        .add_volume_study(
            StudyKind::TradeRate,
            VolumeParams {
                window_ms: 10,
                large_print_threshold: 100.0
            }
        )
        .is_err());
    assert!(set
        .add_volume_study(
            StudyKind::LargePrints,
            VolumeParams {
                window_ms: 10_000,
                large_print_threshold: 0.0
            }
        )
        .is_err());
    assert!(set
        .add_volume_study(StudyKind::Ema, VolumeParams::default())
        .is_err());
}

// -- the tape rows -------------------------------------------------------------------------

#[test]
fn tape_rows_are_newest_first_and_flag_large_prints() {
    let feed = live_feed(&rows(), MIN);
    let mut view = TapeView::default();
    assert!(view.sync(&feed, 100.0));
    let shown = view.rows();
    assert_eq!(shown.len(), rows().len());
    assert_eq!(shown[0].volume.to_bits(), 150.0f64.to_bits());
    assert!(shown[0].large && shown[0].side == q_indicators::volume::AggressorSide::Unknown);
    assert!(shown
        .iter()
        .zip(shown.iter().skip(1))
        .all(|(a, b)| a.time_msc >= b.time_msc));
    // A second sync with nothing new changes nothing.
    assert!(!view.sync(&feed, 100.0));
}

#[test]
fn the_tape_keeps_at_most_a_thousand_rows_but_the_analysis_counts_them_all() {
    let many: Vec<Row> = (0..3000).map(|i| (i, 128_450.0, 1.0, BUY, 0)).collect();
    let feed = live_feed(&many, 60 * MIN);
    let mut view = TapeView::default();
    view.sync(&feed, 100.0);
    assert_eq!(view.rows().len(), MAX_TAPE_ROWS);
    assert_eq!(feed.history().rows(), 3000);
    let total: f64 = feed
        .base_analysis()
        .unwrap()
        .bars()
        .iter()
        .map(|b| b.total_volume)
        .sum();
    assert_eq!(total.to_bits(), 3000.0f64.to_bits());
}

#[test]
fn display_filters_never_change_what_the_studies_counted() {
    let feed = live_feed(&rows(), MIN);
    let before = feed.base_analysis().unwrap().bars().to_vec();
    let mut view = TapeView::new(TapeFilter {
        min_volume: 100.0,
        side: SideFilter::All,
    });
    view.sync(&feed, 100.0);
    assert_eq!(view.rows().len(), 2, "only the two large prints are shown");
    view.set_filter(TapeFilter {
        min_volume: 0.0,
        side: SideFilter::Sell,
    });
    view.sync(&feed, 100.0);
    assert!(view
        .rows()
        .iter()
        .all(|r| r.side == q_indicators::volume::AggressorSide::Sell));
    assert_eq!(view.rows().len(), 2);
    assert_eq!(feed.base_analysis().unwrap().bars(), before.as_slice());
    assert_eq!(
        feed.history().rows(),
        rows().len() as u64,
        "the cached session is whole"
    );
}

#[test]
fn a_filter_change_reloads_from_the_cached_session_not_the_displayed_rows() {
    let mut data: Vec<Row> = (0..2000).map(|i| (i, 100.0, 1.0, BUY, 0)).collect();
    data.insert(0, (-5, 100.0, 50.0, SELL, 0)); // the oldest print, long off a 1000-row tape
    data.sort_by_key(|r| r.0);
    let feed = live_feed(&data, 60 * MIN);
    let mut view = TapeView::default();
    view.sync(&feed, 100.0);
    assert!(
        view.rows().iter().all(|r| r.volume < 50.0),
        "pushed off the unfiltered tape"
    );
    view.set_filter(TapeFilter {
        min_volume: 50.0,
        side: SideFilter::All,
    });
    view.sync(&feed, 100.0);
    assert_eq!(view.rows().len(), 1, "found again in the full cache");
}

#[test]
fn a_view_holds_its_rows_while_a_replacement_snapshot_loads() {
    let mut feed = live_feed(&rows(), MIN);
    let mut view = TapeView::default();
    view.sync(&feed, 100.0);
    let shown = view.rows().len();
    feed.begin_resync(q_terminal::trades::feed::ResyncReason::Gap);
    assert!(!view.sync(&feed, 100.0));
    assert_eq!(view.rows().len(), shown);
}
