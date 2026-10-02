//! Q-082 criteria 1–2: the snapshot/live join, identity deduplication, resync causes and
//! capacity, exercised on the feed state machine against the Q-081 kernel as oracle.
mod trade_common;

use q_indicators::volume::VolumeConfig;
use q_terminal::trades::analysis::VolumeParams;
use q_terminal::trades::feed::Delivery;
use q_terminal::trades::feed::{
    FeedError, LiveOutcome, Phase, ResyncReason, TradeFeed, TradeHandle,
};
use q_terminal::trades::history::{TradeHistory, ROW_BYTES};
use trade_common::*;

const MIN: i64 = 60_000;

fn session_rows() -> Vec<Row> {
    vec![
        (0, 128_450.0, 2.0, BUY, 0),
        (0, 128_450.0, 2.0, BUY, 1),
        (10_000, 128_455.0, 5.0, SELL, 0),
        (59_999, 128_450.0, 1.0, UNKNOWN, 0),
        (MIN, 128_460.0, 120.0, BUY, 0),
        (MIN + 500, 128_455.0, 3.0, SELL, 0),
        (2 * MIN + 1, 128_445.0, 4.0, UNKNOWN, 0),
    ]
}

/// Loads a snapshot of `rows` (frozen at `frozen_seq`) into a fresh feed.
fn loaded_feed(rows: &[Row], frozen_seq: i64) -> (TradeFeed, u64) {
    let mut feed = TradeFeed::new();
    feed.set_interval_ms(MIN);
    assert!(feed.retarget(SYMBOL));
    let gen = feed.load_generation();
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", rows.len() as i64, frozen_seq);
    feed.begin_snapshot(gen, &snap).unwrap();
    // Two pages, as the loader would deliver them.
    let (a, b) = rows.split_at(rows.len() / 2);
    feed.apply_page(gen, &page_headers(&snap), &columns(a), 0)
        .unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(b), 0)
        .unwrap();
    assert_eq!(feed.finish_snapshot(gen).unwrap(), None);
    (feed, gen)
}

#[test]
fn snapshot_pages_yield_exactly_the_kernel_outputs() {
    let rows = session_rows();
    let (feed, _) = loaded_feed(&rows, 100);
    assert_eq!(feed.phase(), &Phase::Live);
    let bars = feed.base_analysis().unwrap().bars();
    assert_matches_kernel(bars, &oracle(&rows, MIN, VolumeConfig::default()));
}

#[test]
fn equal_prints_in_one_millisecond_both_count() {
    let rows = session_rows();
    let (feed, _) = loaded_feed(&rows, 100);
    let first = &feed.base_analysis().unwrap().bars()[0];
    // Two identical BUY 2.0 prints at T0, occurrence 0 and 1.
    assert_eq!(first.buy_volume.to_bits(), 4.0f64.to_bits());
    assert_eq!(feed.history().rows(), rows.len() as u64);
}

#[test]
fn live_entries_above_the_watermark_apply_and_those_at_or_below_never_do() {
    let rows = session_rows();
    let mut feed = TradeFeed::new();
    feed.set_interval_ms(MIN);
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    // Deliveries arrive (and are buffered) before the snapshot is known.
    let overlap: Vec<Row> = rows[5..].to_vec(); // already inside the snapshot
    let fresh: Vec<Row> = vec![(2 * MIN + 2, 128_446.0, 7.0, BUY, 0)];
    assert_eq!(
        feed.on_delivery(EPOCH, 99, batch(&overlap)),
        LiveOutcome::Buffered
    );
    assert_eq!(
        feed.on_delivery(EPOCH, 100, batch(&overlap)),
        LiveOutcome::Buffered
    );
    assert_eq!(
        feed.on_delivery(EPOCH, 101, batch(&fresh)),
        LiveOutcome::Buffered
    );
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", rows.len() as i64, 100);
    feed.begin_snapshot(gen, &snap).unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(&rows), 0)
        .unwrap();
    assert_eq!(feed.finish_snapshot(gen).unwrap(), None);
    let mut all = rows.clone();
    all.extend(fresh);
    assert_matches_kernel(
        feed.base_analysis().unwrap().bars(),
        &oracle(&all, MIN, VolumeConfig::default()),
    );
    assert_eq!(feed.history().rows(), all.len() as u64);
}

#[test]
fn a_delivery_repeated_by_the_transport_is_dropped_by_sequence() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    let rows = [(3 * MIN, 128_470.0, 9.0, BUY, 0)];
    assert_eq!(
        feed.on_delivery(EPOCH, 101, batch(&rows)),
        LiveOutcome::Applied { rows: 1 }
    );
    assert_eq!(
        feed.on_delivery(EPOCH, 101, batch(&rows)),
        LiveOutcome::Duplicate
    );
    assert_eq!(feed.history().rows(), session_rows().len() as u64 + 1);
    assert_eq!(feed.counters.duplicate_batches, 1);
}

#[test]
fn replayed_records_are_dropped_by_identity_but_equal_new_prints_are_kept() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    let n = feed.history().rows();
    // New sequence repeating the last stored identity, then an equal-looking print with a
    // higher occurrence: the first is a replay, the second a real print.
    let last = session_rows().pop().unwrap();
    let rows = [last, (last.0, last.1, last.2, last.3, last.4 + 1)];
    assert_eq!(
        feed.on_delivery(EPOCH, 101, batch(&rows)),
        LiveOutcome::Applied { rows: 1 }
    );
    assert_eq!(feed.history().rows(), n + 1);
    assert_eq!(feed.report().duplicate_rows, 1);
}

#[test]
fn a_sequence_hole_resyncs_and_keeps_the_prior_view_stale() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    // Other symbols' deliveries keep the sequence continuous without being decoded.
    assert_eq!(
        feed.on_delivery(EPOCH, 101, Delivery::OtherSymbol),
        LiveOutcome::Ignored
    );
    assert_eq!(
        feed.on_delivery(EPOCH, 103, batch(&[(3 * MIN, 1.0, 1.0, BUY, 0)])),
        LiveOutcome::Resync(ResyncReason::Gap)
    );
    let gen = feed.begin_resync(ResyncReason::Gap);
    let report = feed.report();
    assert_eq!(report.phase, "loading");
    assert!(report.stale, "the old view is shown, marked stale");
    assert!(!report.complete);
    assert!(feed.base_analysis().is_some(), "prior output stays visible");
    // A fresh snapshot replaces it.
    let rows = session_rows();
    let snap = snapshot(SYMBOL, GENERATION, "snap-2", rows.len() as i64, 110);
    feed.begin_snapshot(gen, &snap).unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(&rows), 0)
        .unwrap();
    feed.finish_snapshot(gen).unwrap();
    assert!(feed.report().complete);
}

#[test]
fn source_correction_clears_derived_output_before_the_new_snapshot() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    let status = status_for(SYMBOL, "gen-2", "partial", 101);
    assert_eq!(
        feed.on_status(EPOCH, 5, &status),
        Some(ResyncReason::SourceCorrection)
    );
    feed.begin_resync(ResyncReason::SourceCorrection);
    assert!(
        feed.base_analysis().is_none(),
        "no old-generation value survives"
    );
    assert_eq!(feed.history().rows(), 0);
    assert_eq!(feed.report().phase, "loading");
}

#[test]
fn a_changed_generation_in_a_live_batch_is_a_correction_too() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    let mut ctx = context();
    ctx.source_generation = "gen-2".into();
    let delivery = Delivery::Batch(Box::new(q_terminal::trades::feed::DecodedBatch {
        context: ctx,
        columns: columns(&[(3 * MIN, 1.0, 1.0, BUY, 0)]),
        invalid_rows: 0,
    }));
    assert_eq!(
        feed.on_delivery(EPOCH, 101, delivery),
        LiveOutcome::Resync(ResyncReason::SourceCorrection)
    );
    assert!(feed
        .report()
        .notes
        .iter()
        .any(|n| n.code == "mixed_generations"));
}

#[test]
fn status_changes_update_coverage_without_a_reload() {
    let (mut feed, gen) = loaded_feed(&session_rows(), 100);
    let status = status_for(SYMBOL, GENERATION, "partial", 101);
    assert_eq!(feed.on_status(EPOCH, 6, &status), None);
    assert_eq!(feed.load_generation(), gen);
    let report = feed.report();
    assert_eq!(report.coverage.state, "partial");
    assert!(
        !report.complete,
        "partial coverage is never reported complete"
    );
    // An older sequence of the same epoch does not undo it.
    assert_eq!(
        feed.on_status(EPOCH, 5, &status_for(SYMBOL, GENERATION, "complete", 1)),
        None
    );
    assert_eq!(feed.report().coverage.state, "partial");
}

#[test]
fn a_new_epoch_needs_a_new_snapshot() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    assert_eq!(
        feed.on_delivery("trades-epoch-9", 1, batch(&[(3 * MIN, 1.0, 1.0, BUY, 0)])),
        LiveOutcome::Resync(ResyncReason::EpochChanged)
    );
}

#[test]
fn retargeting_discards_every_response_of_the_old_symbol() {
    let mut feed = TradeFeed::new();
    assert!(feed.retarget(SYMBOL));
    let old = feed.load_generation();
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", 1, 100);
    assert!(feed.retarget("WDOZ26"));
    assert_eq!(feed.begin_snapshot(old, &snap), Err(FeedError::Stale));
    assert_eq!(
        feed.apply_page(
            old,
            &page_headers(&snap),
            &columns(&[(0, 1.0, 1.0, BUY, 0)]),
            0
        ),
        Err(FeedError::Stale)
    );
    assert_eq!(feed.snapshot_pending(old), Err(FeedError::Stale));
    assert_eq!(feed.history().rows(), 0);
    // The same symbol again changes nothing.
    assert!(!feed.retarget("WDOZ26"));
}

#[test]
fn a_page_from_another_snapshot_is_refused() {
    let mut feed = TradeFeed::new();
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", 1, 100);
    feed.begin_snapshot(gen, &snap).unwrap();
    let other = snapshot(SYMBOL, GENERATION, "snap-other", 1, 100);
    assert!(matches!(
        feed.apply_page(
            gen,
            &page_headers(&other),
            &columns(&[(0, 1.0, 1.0, BUY, 0)]),
            0
        ),
        Err(FeedError::Mismatch(_))
    ));
}

#[test]
fn a_descriptor_whose_context_disagrees_is_refused() {
    let mut feed = TradeFeed::new();
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    let mut snap = snapshot(SYMBOL, GENERATION, "snap-1", 1, 100);
    snap.volume_unit = "provider-lots".into();
    assert!(matches!(
        feed.begin_snapshot(gen, &snap),
        Err(FeedError::Mismatch(_))
    ));
}

#[test]
fn a_pending_backfill_is_partial_coverage_never_an_empty_complete_session() {
    let mut feed = TradeFeed::new();
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    feed.snapshot_pending(gen).unwrap();
    let report = feed.report();
    assert!(report.backfill_pending && report.loading && !report.complete);
    assert!(report.notes.iter().any(|n| n.code == "backfill"));
}

#[test]
fn disconnect_freezes_totals_with_their_timestamp() {
    let (mut feed, _) = loaded_feed(&session_rows(), 100);
    feed.on_disconnect(T0 + 5 * MIN);
    let report = feed.report();
    assert!(report.stale);
    assert_eq!(report.stale_since_ms, Some(T0 + 5 * MIN));
    assert!(report
        .notes
        .iter()
        .any(|n| n.code == "stale" && n.text.contains("12:05:00 UTC")));
    assert!(!feed.output_is_current());
}

#[test]
fn history_capacity_exhaustion_is_visible_and_retry_reloads() {
    let mut feed = TradeFeed::with_history(TradeHistory::with_limits(4, 2 * 4 * ROW_BYTES));
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    let rows = session_rows(); // 7 rows > 2 chunks of 4 would hold only after a refused batch
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", rows.len() as i64, 100);
    feed.begin_snapshot(gen, &snap).unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(&rows[..5]), 0)
        .unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(&rows[5..]), 0)
        .unwrap();
    // 5 + 2 rows need 2 chunks; push enough more to overflow.
    let more: Vec<Row> = (0..8).map(|i| (3 * MIN + i, 1.0, 1.0, BUY, 0)).collect();
    feed.finish_snapshot(gen).unwrap();
    feed.on_delivery(EPOCH, 101, batch(&more));
    assert!(feed.capacity_exhausted());
    let report = feed.report();
    assert!(report.capacity_exhausted && !report.complete);
    assert!(report.notes.iter().any(|n| n.code == "capacity"));
    assert!(feed.retry_capacity().is_some());
    assert!(!feed.capacity_exhausted());
}

#[test]
fn a_timeframe_change_regroups_from_cache_without_a_new_load() {
    let rows = session_rows();
    let (mut feed, gen) = loaded_feed(&rows, 100);
    assert!(feed.set_interval_ms(5 * MIN));
    assert_eq!(
        feed.load_generation(),
        gen,
        "no new snapshot or subscription"
    );
    assert!(feed.needs_rebuild());
    let job = feed.plan_rebuild().unwrap();
    assert!(feed.install_rebuild(job.run()));
    assert_matches_kernel(
        feed.base_analysis().unwrap().bars(),
        &oracle(&rows, 5 * MIN, VolumeConfig::default()),
    );
    assert_eq!(feed.history().rows(), rows.len() as u64);
}

#[test]
fn a_threshold_edit_replays_the_whole_cached_session() {
    let rows = session_rows();
    let (mut feed, _) = loaded_feed(&rows, 100);
    let params = VolumeParams {
        window_ms: 10_000,
        large_print_threshold: 4.0,
    };
    assert!(feed.set_params(&[params]));
    let job = feed.plan_rebuild().unwrap();
    assert!(feed.install_rebuild(job.run()));
    let cfg = VolumeConfig {
        window_ms: 10_000,
        large_print_threshold: 4.0,
    };
    let analysis = feed.analysis_for(params).unwrap();
    assert_matches_kernel(analysis.bars(), &oracle(&rows, MIN, cfg));
    // Prints of 5.0 and 120.0 and 4.0 qualify: volumes >= 4.
    assert_eq!(analysis.prints().len(), 3);
}

#[test]
fn a_rebuild_overtaken_by_new_rows_catches_up_and_a_stale_one_is_discarded() {
    let rows = session_rows();
    let (mut feed, _) = loaded_feed(&rows, 100);
    feed.set_interval_ms(5 * MIN);
    let job = feed.plan_rebuild().unwrap();
    let live = [(3 * MIN, 128_470.0, 9.0, BUY, 0)];
    feed.on_delivery(EPOCH, 101, batch(&live));
    let result = job.run();
    assert!(feed.install_rebuild(result));
    let mut all = rows.clone();
    all.extend(live);
    assert_matches_kernel(
        feed.base_analysis().unwrap().bars(),
        &oracle(&all, 5 * MIN, VolumeConfig::default()),
    );
    // A job planned before another interval change must not land.
    let stale = feed.plan_rebuild().unwrap();
    feed.set_interval_ms(MIN);
    assert!(!feed.install_rebuild(stale.run()));
}

#[test]
fn the_shared_handle_notifies_listeners_once_per_change() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    let handle = TradeHandle::new();
    let hits = Arc::new(AtomicUsize::new(0));
    let h = hits.clone();
    handle.subscribe(move || {
        h.fetch_add(1, Ordering::SeqCst);
        true
    });
    handle.mutate(|f| f.retarget(SYMBOL));
    handle.mutate(|f| f.retarget(SYMBOL)); // no change
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}
