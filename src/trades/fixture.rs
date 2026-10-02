//! Deterministic trade sessions for gallery captures and benchmarks.
//!
//! The rows are synthetic but shaped like a real session: a few dozen prints a minute, a
//! mix of buy, sell and unknown sides, and the occasional block. Prices follow the preview
//! chart's closes so markers land where the candles are.

use super::feed::{ResyncReason, TradeFeed};
use super::history::{TradeColumns, TradeHistory, ROW_BYTES};
use crate::contracts_stream::{TradeSnapshotResponse, TradeWatermark};

/// Wall-clock origin of the preview bars (2026-09-18T10:00:00Z), in milliseconds.
pub const PREVIEW_T0_MS: i64 = 1_789_725_600_000;
const BUY: u32 = 32 | 8;
const SELL: u32 = 64 | 8;
const UNKNOWN: u32 = 8;
pub const PREVIEW_SYMBOL: &str = "PETR4";
const EPOCH: &str = "fixture-epoch";

/// The close of preview bar `i`, in cents, as `preview_bars` draws it.
pub fn preview_close_cents(i: usize) -> i64 {
    3800 + ((i as f64 * 0.15).sin() * 120.0).round() as i64 + i as i64
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

/// Prints for `minutes` one-minute bars from [`PREVIEW_T0_MS`]. With `sides_known` false
/// every print has an unknown aggressor.
pub fn preview_rows(minutes: usize, prints_per_minute: usize, sides_known: bool) -> TradeColumns {
    let mut rng = Lcg(0x9e37_79b9_7f4a_7c15);
    let mut cols = TradeColumns::default();
    for minute in 0..minutes {
        let base = preview_close_cents(minute);
        let mut offset = 0i64;
        for k in 0..prints_per_minute.max(1) {
            offset += 1 + (rng.next() % (60_000 / prints_per_minute.max(1) as u64 * 2)) as i64;
            let t = PREVIEW_T0_MS + minute as i64 * 60_000 + offset.min(59_999);
            let price = (base + (rng.next() % 5) as i64 - 2) as f64 / 100.0;
            let roll = rng.next() % 100;
            let volume = if roll < 3 {
                (100 + rng.next() % 200) as f64
            } else {
                (1 + rng.next() % 9) as f64
            };
            let flags = if !sides_known {
                UNKNOWN
            } else {
                match rng.next() % 100 {
                    0..=44 => BUY,
                    45..=84 => SELL,
                    _ => UNKNOWN,
                }
            };
            // Equal millisecond prints keep their source order as occurrence.
            let occurrence = cols
                .time_msc
                .iter()
                .rev()
                .take_while(|&&prev| prev == t)
                .count() as u32;
            let _ = k;
            cols.push(t, price, volume, flags, occurrence);
        }
    }
    cols
}

fn snapshot(
    rows: usize,
    frozen_seq: i64,
    coverage_state: &str,
    reason: Option<&str>,
) -> TradeSnapshotResponse {
    let status = serde_json::json!({
        "provider_id": "fixture", "symbol": PREVIEW_SYMBOL, "source_generation": "gen-1",
        "volume_field": "volume_real", "volume_unit": "shares",
        "covered_from": "2026-09-18T10:00:00Z", "covered_to": "2026-09-18T20:00:00Z",
        "coverage_state": coverage_state, "classification_coverage": "partial",
        "coverage_reason": reason,
        "last_trade_watermark": {"epoch": EPOCH, "seq": frozen_seq},
    });
    serde_json::from_value(serde_json::json!({
        "coverage": status,
        "exchange_timezone": "America/Sao_Paulo",
        "expires_at": "2026-09-18T20:10:00Z",
        "first_page_url": "/fixture",
        "frozen_watermark": TradeWatermark { epoch: EPOCH.into(), seq: frozen_seq },
        "invalid_trade_count": 0,
        "provider_id": "fixture",
        "session_from": "2026-09-18T07:00:00-03:00",
        "session_key": "2026-09-18",
        "session_to": "2026-09-18T17:55:00-03:00",
        "snapshot_id": "fixture-snapshot",
        "source_generation": "gen-1",
        "symbol": PREVIEW_SYMBOL,
        "trade_count": rows,
        "volume_field": "volume_real",
        "volume_unit": "shares",
    }))
    .expect("fixture snapshot is well formed")
}

fn headers(snap: &TradeSnapshotResponse) -> crate::contracts_stream::TradeHistoryPageHeaders {
    crate::contracts_stream::TradeHistoryPageHeaders {
        frozen_epoch: snap.frozen_watermark.epoch.clone(),
        frozen_seq: snap.frozen_watermark.seq,
        next_cursor: serde_json::Value::Null,
        page_count: 0,
        snapshot_id: snap.snapshot_id.clone(),
        source_generation: snap.source_generation.clone(),
        symbol: snap.symbol.clone(),
        volume_field: snap.volume_field.clone(),
        volume_unit: snap.volume_unit.clone(),
    }
}

/// A feed in one of the states a panel must show truthfully: `live`, `partial`, `loading`,
/// `backfill`, `stale`, `capacity`, `unavailable` or `unknown` (no aggressor sides).
pub fn preview_feed(state: &str, minutes: usize) -> TradeFeed {
    let rows = preview_rows(minutes, 24, state != "unknown");
    let history = if state == "capacity" {
        // Room for a few chunks only, so the live burst below overflows it.
        TradeHistory::with_limits(256, 6 * 256 * ROW_BYTES)
    } else {
        TradeHistory::new()
    };
    let mut feed = TradeFeed::with_history(history);
    feed.retarget(PREVIEW_SYMBOL);
    let generation = feed.load_generation();
    if state == "unavailable" {
        let _ = feed.set_unavailable(generation, "snapshot status 503");
        return feed;
    }
    if state == "backfill" {
        let _ = feed.snapshot_pending(generation);
        return feed;
    }
    let (coverage, reason) = match state {
        "partial" => ("partial", Some("invalid_trade_records")),
        _ => ("complete", None),
    };
    let snap = snapshot(rows.len(), 100, coverage, reason);
    feed.begin_snapshot(generation, &snap)
        .expect("fixture snapshot");
    if state == "loading" {
        let part = rows.len() * 2 / 5;
        let mut first = TradeColumns::default();
        for i in 0..part {
            first.push(
                rows.time_msc[i],
                rows.price[i],
                rows.volume[i],
                rows.raw_flags[i],
                rows.occurrence[i],
            );
        }
        let _ = feed.apply_page(generation, &headers(&snap), &first, 0);
        return feed;
    }
    let _ = feed.apply_page(generation, &headers(&snap), &rows, 0);
    let _ = feed.finish_snapshot(generation);
    match state {
        "stale" => feed.on_disconnect(PREVIEW_T0_MS + 90 * 60_000),
        "capacity" => {
            // A burst after the snapshot that the bounded history cannot take.
            let mut burst = TradeColumns::default();
            let last = *rows.time_msc.last().unwrap_or(&PREVIEW_T0_MS);
            for i in 0..4096i64 {
                burst.push(last + 1 + i, 38.0, 1.0, BUY, 0);
            }
            feed.on_delivery(
                EPOCH,
                101,
                super::feed::Delivery::Batch(Box::new(super::feed::DecodedBatch {
                    context: feed.context().cloned().expect("context"),
                    columns: burst,
                    invalid_rows: 0,
                })),
            );
        }
        "resync" => {
            feed.begin_resync(ResyncReason::Gap);
        }
        _ => {}
    }
    feed
}
