//! Builders for trade feed tests: snapshots, pages, deliveries and a kernel oracle.
#![allow(dead_code)]

use q_indicators::volume::{batch_volume, TradeInput, VolumeConfig, BUY_FLAG, SELL_FLAG};
use q_terminal::contracts_stream::{
    TradeHistoryPageHeaders, TradeSnapshotResponse, TradeSourceStatus, TradeWatermark,
};
use q_terminal::trades::analysis::BarAggregate;
use q_terminal::trades::feed::{DecodedBatch, Delivery, SourceContext};
use q_terminal::trades::history::TradeColumns;
use q_terminal::trades::time::{BarGrid, ExchangeClock};
use serde_json::json;

pub const SYMBOL: &str = "WINZ26";
pub const GENERATION: &str = "gen-1";
pub const SESSION: &str = "2026-10-01";
pub const EPOCH: &str = "trades-epoch-4";
/// 2026-10-01 12:00:00 UTC, i.e. 09:00 in São Paulo.
pub const T0: i64 = 1_790_856_000_000;

pub fn context() -> SourceContext {
    context_for(SYMBOL, GENERATION)
}

pub fn context_for(symbol: &str, generation: &str) -> SourceContext {
    SourceContext {
        provider_id: "mt5-broker-a".into(),
        symbol: symbol.into(),
        source_generation: generation.into(),
        exchange_timezone: "America/Sao_Paulo".into(),
        session_key: SESSION.into(),
        volume_field: "volume_real".into(),
        volume_unit: "contracts".into(),
    }
}

pub fn status_for(symbol: &str, generation: &str, state: &str, seq: i64) -> TradeSourceStatus {
    TradeSourceStatus {
        classification_coverage: "partial".into(),
        coverage_reason: serde_json::Value::Null,
        coverage_state: state.into(),
        covered_from: json!("2026-10-01T12:00:00Z"),
        covered_to: json!("2026-10-01T20:55:00Z"),
        last_trade_watermark: TradeWatermark {
            epoch: EPOCH.into(),
            seq,
        },
        provider_id: "mt5-broker-a".into(),
        source_generation: generation.into(),
        symbol: symbol.into(),
        volume_field: "volume_real".into(),
        volume_unit: "contracts".into(),
    }
}

pub fn snapshot(
    symbol: &str,
    generation: &str,
    id: &str,
    rows: i64,
    frozen_seq: i64,
) -> TradeSnapshotResponse {
    TradeSnapshotResponse {
        coverage: status_for(symbol, generation, "complete", frozen_seq),
        exchange_timezone: "America/Sao_Paulo".into(),
        expires_at: "2026-10-01T21:05:00Z".into(),
        first_page_url: format!("/api/v1/market/trades/history?snapshot_id={id}"),
        frozen_watermark: TradeWatermark {
            epoch: EPOCH.into(),
            seq: frozen_seq,
        },
        invalid_trade_count: 0,
        provider_id: "mt5-broker-a".into(),
        session_from: json!("2026-10-01T09:00:00-03:00"),
        session_key: SESSION.into(),
        session_to: json!("2026-10-01T17:55:00-03:00"),
        snapshot_id: id.into(),
        source_generation: generation.into(),
        symbol: symbol.into(),
        trade_count: rows,
        volume_field: "volume_real".into(),
        volume_unit: "contracts".into(),
    }
}

pub fn page_headers(snap: &TradeSnapshotResponse) -> TradeHistoryPageHeaders {
    TradeHistoryPageHeaders {
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

/// `(offset_ms from T0, price, volume, flags, occurrence)`.
pub type Row = (i64, f64, f64, u32, u32);

pub const BUY: u32 = BUY_FLAG | 8;
pub const SELL: u32 = SELL_FLAG | 8;
pub const UNKNOWN: u32 = 8;

pub fn columns(rows: &[Row]) -> TradeColumns {
    let mut c = TradeColumns::default();
    for &(dt, price, volume, flags, occ) in rows {
        c.push(T0 + dt, price, volume, flags, occ);
    }
    c
}

pub fn batch(rows: &[Row]) -> Delivery {
    Delivery::Batch(Box::new(DecodedBatch {
        context: context(),
        columns: columns(rows),
        invalid_rows: 0,
    }))
}

pub fn grid(interval_ms: i64) -> BarGrid {
    BarGrid::new(
        interval_ms,
        ExchangeClock::for_zone("America/Sao_Paulo").unwrap(),
    )
    .unwrap()
}

/// The Q-081 kernel run directly over `rows`: the last output of each chart bar.
pub fn oracle(
    rows: &[Row],
    interval_ms: i64,
    config: VolumeConfig,
) -> Vec<(i64, q_indicators::volume::VolumeOutput)> {
    let g = grid(interval_ms);
    let trades: Vec<TradeInput> = rows
        .iter()
        .map(|&(dt, price, volume, flags, _)| {
            let (open, close) = g.bounds(T0 + dt);
            TradeInput {
                time_msc: T0 + dt,
                price,
                volume,
                raw_flags: flags,
                session_key: 20_261_001,
                bar_open_msc: open,
                bar_close_msc: close,
            }
        })
        .collect();
    let outputs = batch_volume(config, &trades, &[]).unwrap();
    let mut per_bar: Vec<(i64, q_indicators::volume::VolumeOutput)> = Vec::new();
    for (t, out) in trades.iter().zip(outputs) {
        match per_bar.last_mut() {
            Some((open, last)) if *open == t.bar_open_msc => *last = out,
            _ => per_bar.push((t.bar_open_msc, out)),
        }
    }
    per_bar
}

/// Asserts the feed's bars are bit-identical to the kernel oracle.
pub fn assert_matches_kernel(
    bars: &[BarAggregate],
    expected: &[(i64, q_indicators::volume::VolumeOutput)],
) {
    assert_eq!(bars.len(), expected.len(), "bar count");
    for (bar, (open, out)) in bars.iter().zip(expected) {
        assert_eq!(bar.open_ms, *open);
        assert_eq!(bar.buy_volume.to_bits(), out.buy_volume.to_bits());
        assert_eq!(bar.sell_volume.to_bits(), out.sell_volume.to_bits());
        assert_eq!(bar.unknown_volume.to_bits(), out.unknown_volume.to_bits());
        assert_eq!(bar.total_volume.to_bits(), out.total_volume.to_bits());
        assert_eq!(bar.delta.to_bits(), out.delta.to_bits());
        assert_eq!(
            bar.cumulative_delta.to_bits(),
            out.cumulative_delta.to_bits()
        );
        assert_eq!(
            bar.classified_share.map(f64::to_bits),
            out.classified_share.map(f64::to_bits)
        );
        assert_eq!(
            bar.trade_rate.map(f64::to_bits),
            out.trade_rate.map(f64::to_bits)
        );
        assert_eq!(bar.rate_warmed_up, out.rate_warmed_up);
        assert_eq!(bar.large_prints, out.large_prints.len());
    }
}

use q_terminal::stream::fake_trades::FakeSnapshot;

/// A fake snapshot of `rows` in pages of `page_rows`, frozen at `frozen_seq`.
pub fn fake_snapshot(
    symbol: &str,
    generation: &str,
    id: &str,
    rows: &[Row],
    page_rows: usize,
    frozen_seq: i64,
) -> FakeSnapshot {
    FakeSnapshot {
        descriptor: snapshot(symbol, generation, id, rows.len() as i64, frozen_seq),
        context: context_for(symbol, generation),
        pages: rows.chunks(page_rows.max(1)).map(columns).collect(),
    }
}

use q_terminal::trades::feed::{Delivery as FeedDelivery, TradeFeed};

pub const MIN: i64 = 60_000;

/// A live feed that loaded `rows` as one snapshot, grouped into `interval_ms` bars.
pub fn live_feed(rows: &[Row], interval_ms: i64) -> TradeFeed {
    let mut feed = TradeFeed::new();
    feed.set_interval_ms(interval_ms);
    feed.retarget(SYMBOL);
    let gen = feed.load_generation();
    let snap = snapshot(SYMBOL, GENERATION, "snap-1", rows.len() as i64, 100);
    feed.begin_snapshot(gen, &snap).unwrap();
    feed.apply_page(gen, &page_headers(&snap), &columns(rows), 0)
        .unwrap();
    feed.finish_snapshot(gen).unwrap();
    let _: Option<FeedDelivery> = None;
    feed
}
