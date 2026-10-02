//! The trades half of the stream client: live deliveries in, session snapshot loaded beside.
//!
//! The client subscribes to `trades` and `trades.status` with the bar topics. Deliveries go
//! straight to the shared [`TradeFeed`], which buffers them while a loader task reads the
//! frozen snapshot, so the websocket is never held up by a history download. Every response
//! the loader receives carries the load generation it was started for; the feed refuses any
//! other, so nothing of a replaced symbol or snapshot is ever applied.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::bridge::bar_feed::parse_timeframe_ms;
use crate::contracts_stream::{
    TradeHistoryPageHeaders, TradeHistoryPending, TradeSnapshotResponse, TradeSourceStatus,
};
use crate::stream::client::ClientShared;
use crate::stream::frame::EnvelopeHeader;
use crate::stream::trade_arrow::decode_arrow_trades;
use crate::trades::feed::{
    DecodedBatch, Delivery, FeedError, LiveOutcome, ResyncReason, TradeHandle,
};
use crate::trades::history::TradeColumns;

pub const TRADE_TOPICS: [&str; 2] = ["trades", "trades.status"];
const RETRY_MAX_MS: u64 = 30_000;

pub fn handles(topic: &str) -> bool {
    TRADE_TOPICS.contains(&topic)
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

pub struct TradeSession {
    handle: TradeHandle,
    http: reqwest::Client,
    api_base: String,
    shared: Arc<ClientShared>,
    is_shutdown: Arc<AtomicBool>,
    loader: Option<tokio::task::JoinHandle<()>>,
    subscribed: bool,
}

impl TradeSession {
    pub fn new(
        handle: TradeHandle,
        http: reqwest::Client,
        api_base: &str,
        shared: Arc<ClientShared>,
        is_shutdown: Arc<AtomicBool>,
    ) -> Self {
        Self {
            handle,
            http,
            api_base: api_base.trim_end_matches('/').to_string(),
            shared,
            is_shutdown,
            loader: None,
            subscribed: false,
        }
    }

    /// The websocket acknowledged the trade topics: deliveries are being buffered, so the
    /// snapshot may now be requested.
    pub fn on_subscribed(&mut self, symbol: &str, timeframe: &str) {
        self.subscribed = true;
        self.apply_interval(timeframe);
        let started = self.handle.mutate(|f| {
            if f.symbol() == Some(symbol) {
                Some(f.begin_resync(ResyncReason::Reconnect))
            } else {
                f.retarget(symbol).then(|| f.load_generation())
            }
        });
        if let Some(generation) = started {
            self.start_loader(symbol, generation);
        }
    }

    /// The chart moved to another symbol and/or timeframe. A timeframe-only change regroups
    /// the cached bars and sends nothing.
    pub fn on_target(&mut self, symbol: &str, timeframe: &str) {
        self.apply_interval(timeframe);
        let started = self
            .handle
            .mutate(|f| f.retarget(symbol).then(|| f.load_generation()));
        if let (Some(generation), true) = (started, self.subscribed) {
            self.start_loader(symbol, generation);
        }
    }

    fn apply_interval(&self, timeframe: &str) {
        let interval = parse_timeframe_ms(timeframe);
        if self.handle.mutate(|f| f.set_interval_ms(interval)) {
            self.handle.rebuild_in_background();
        }
    }

    /// The stream was lost: totals freeze with their timestamp and any load stops.
    pub fn lost(&mut self) {
        self.subscribed = false;
        if let Some(task) = self.loader.take() {
            task.abort();
        }
        self.handle.mutate(|f| f.on_disconnect(now_ms()));
    }

    /// The server dropped entries (`lagging`) or the cursor expired: the tape has a hole.
    pub fn on_gap(&mut self, reason: ResyncReason) {
        self.restart(reason);
    }

    /// The server does not offer the trade topics.
    pub fn on_rejected(&mut self, reason: &str) {
        self.handle.mutate(|f| {
            let generation = f.load_generation();
            let _ = f.set_unavailable(generation, reason);
        });
    }

    pub fn restart(&mut self, reason: ResyncReason) {
        let Some((symbol, generation)) = self.handle.mutate(|f| {
            f.symbol()
                .map(str::to_string)
                .map(|s| (s, f.begin_resync(reason)))
        }) else {
            return;
        };
        self.shared.inc_resnapshots();
        if self.subscribed {
            self.start_loader(&symbol, generation);
        }
    }

    /// A binary `trades` frame.
    pub fn on_binary(&mut self, header: &EnvelopeHeader, arrow: &[u8]) {
        let routed = header
            .key
            .as_ref()
            .and_then(|k| k.get("symbol"))
            .and_then(|s| s.as_str());
        let delivery = match routed {
            Some(symbol) if !self.handle.read(|f| f.symbol() == Some(symbol)) => {
                Delivery::OtherSymbol
            }
            _ => self.decode_delivery(arrow),
        };
        let outcome = self
            .handle
            .mutate(|f| f.on_delivery(&header.epoch, header.seq, delivery));
        match outcome {
            LiveOutcome::Applied { .. } => {
                self.shared.inc_applied();
                self.shared.record_applied();
            }
            LiveOutcome::Duplicate => self.shared.inc_dropped(),
            LiveOutcome::Resync(reason) => self.restart(reason),
            LiveOutcome::Buffered | LiveOutcome::Ignored => {}
        }
    }

    fn decode_delivery(&self, arrow: &[u8]) -> Delivery {
        match decode_arrow_trades(arrow, None) {
            Ok(decoded) => match decoded.context() {
                Some(context) => {
                    let ours = self
                        .handle
                        .read(|f| f.symbol() == Some(context.symbol.as_str()));
                    if ours {
                        Delivery::Batch(Box::new(DecodedBatch {
                            context,
                            columns: decoded.columns,
                            invalid_rows: decoded.invalid_rows,
                        }))
                    } else {
                        Delivery::OtherSymbol
                    }
                }
                None => Delivery::Undecodable("delivery without context".into()),
            },
            Err(e) => Delivery::Undecodable(e.to_string()),
        }
    }

    /// A `trades.status` entry.
    pub fn on_status(&mut self, epoch: &str, seq: i64, payload: &serde_json::Value) {
        let Ok(status) = serde_json::from_value::<TradeSourceStatus>(payload.clone()) else {
            self.shared.inc_dropped();
            return;
        };
        if let Some(reason) = self.handle.mutate(|f| f.on_status(epoch, seq, &status)) {
            self.restart(reason);
        }
    }

    fn start_loader(&mut self, symbol: &str, generation: u64) {
        if let Some(task) = self.loader.take() {
            task.abort();
        }
        let loader = Loader {
            handle: self.handle.clone(),
            http: self.http.clone(),
            api_base: self.api_base.clone(),
            shared: self.shared.clone(),
            is_shutdown: self.is_shutdown.clone(),
            symbol: symbol.to_string(),
        };
        self.loader = Some(tokio::spawn(loader.run(generation)));
    }
}

impl Drop for TradeSession {
    fn drop(&mut self) {
        if let Some(task) = self.loader.take() {
            task.abort();
        }
    }
}

struct Loader {
    handle: TradeHandle,
    http: reqwest::Client,
    api_base: String,
    shared: Arc<ClientShared>,
    is_shutdown: Arc<AtomicBool>,
    symbol: String,
}

enum Step {
    /// The load is finished, or no longer wanted.
    Done,
    /// Start again under this generation.
    Again(u64),
    /// The source is backfilling (202): ask again in a second.
    Pending,
    /// A request failed: ask again after a growing delay.
    Failed,
}

impl Loader {
    async fn run(self, mut generation: u64) {
        let mut failures = 0u32;
        loop {
            if self.is_shutdown.load(Ordering::SeqCst) {
                return;
            }
            match self.attempt(generation).await {
                Step::Done => return,
                Step::Again(next) => {
                    generation = next;
                    failures = 0;
                }
                Step::Pending => {
                    failures = 0;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Step::Failed => {
                    failures = failures.saturating_add(1);
                    let backoff = (500u64 << failures.min(6)).min(RETRY_MAX_MS);
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
            }
            if self.handle.read(|f| f.load_generation()) != generation {
                return;
            }
        }
    }

    fn stale(&self, generation: u64) -> bool {
        self.handle.read(|f| f.load_generation()) != generation
    }

    fn unavailable(&self, generation: u64, reason: &str) {
        self.handle
            .mutate(|f| f.set_unavailable(generation, reason).ok());
    }

    async fn attempt(&self, generation: u64) -> Step {
        self.shared.inc_rest_calls();
        let url = format!(
            "{}/api/v1/market/trades/snapshot?symbol={}",
            self.api_base,
            crate::stream::client::encode_query(&self.symbol)
        );
        let resp = match self.http.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                self.unavailable(generation, &e.to_string());
                return Step::Failed;
            }
        };
        match resp.status().as_u16() {
            200 => {}
            202 => {
                let _ = resp.json::<TradeHistoryPending>().await;
                return match self.handle.mutate(|f| f.snapshot_pending(generation)) {
                    Ok(()) => Step::Pending,
                    Err(_) => Step::Done,
                };
            }
            404 => {
                self.unavailable(generation, "symbol is not published");
                return Step::Done;
            }
            status => {
                self.unavailable(generation, &format!("snapshot status {status}"));
                return Step::Failed;
            }
        }
        let snapshot: TradeSnapshotResponse = match resp.json().await {
            Ok(s) => s,
            Err(e) => {
                self.unavailable(generation, &format!("snapshot: {e}"));
                return Step::Failed;
            }
        };
        match self
            .handle
            .mutate(|f| f.begin_snapshot(generation, &snapshot))
        {
            Ok(()) => {}
            Err(FeedError::Stale) => return Step::Done,
            Err(e) => {
                self.unavailable(generation, &format!("{e:?}"));
                return Step::Done;
            }
        }
        self.read_pages(generation, &snapshot).await
    }

    async fn read_pages(&self, generation: u64, snapshot: &TradeSnapshotResponse) -> Step {
        let mut url = self.page_url(&snapshot.first_page_url);
        loop {
            if self.stale(generation) {
                return Step::Done;
            }
            self.shared.inc_rest_calls();
            let resp = match self.http.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    self.unavailable(generation, &e.to_string());
                    return Step::Failed;
                }
            };
            match resp.status().as_u16() {
                200 => {}
                // The token expired or its source generation was replaced.
                410 => return self.resync(ResyncReason::Expired),
                status => {
                    self.unavailable(generation, &format!("history status {status}"));
                    return Step::Failed;
                }
            }
            let headers = match page_headers(resp.headers()) {
                Some(h) => h,
                None => {
                    self.unavailable(generation, "history page without watermark headers");
                    return Step::Done;
                }
            };
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    self.unavailable(generation, &e.to_string());
                    return Step::Failed;
                }
            };
            let decoded = match decode_arrow_trades(&bytes, Some(&headers.volume_field)) {
                Ok(d) => d,
                Err(e) => {
                    self.unavailable(generation, &format!("history page: {e}"));
                    return Step::Done;
                }
            };
            let columns: TradeColumns = decoded.columns;
            let applied = self
                .handle
                .mutate(|f| f.apply_page(generation, &headers, &columns, decoded.invalid_rows));
            match applied {
                Ok(()) => {}
                Err(FeedError::Stale) => return Step::Done,
                Err(e) => {
                    // A page that does not belong to this snapshot: start over.
                    self.shared.set_last_error(&format!("{e:?}"));
                    return self.resync(ResyncReason::SourceCorrection);
                }
            }
            match headers.next_cursor.as_str().filter(|c| !c.is_empty()) {
                Some(cursor) => {
                    url = format!(
                        "{}&cursor={}",
                        self.page_url(&snapshot.first_page_url),
                        crate::stream::client::encode_query(cursor)
                    );
                }
                None => break,
            }
        }
        match self.handle.mutate(|f| f.finish_snapshot(generation)) {
            Ok(None) | Err(FeedError::Stale) => Step::Done,
            Ok(Some(reason)) => self.resync(reason),
            Err(_) => self.resync(ResyncReason::SourceCorrection),
        }
    }

    fn resync(&self, reason: ResyncReason) -> Step {
        self.shared.inc_resnapshots();
        Step::Again(self.handle.mutate(|f| f.begin_resync(reason)))
    }

    fn page_url(&self, first_page_url: &str) -> String {
        if first_page_url.starts_with("http") {
            first_page_url.to_string()
        } else {
            format!("{}{}", self.api_base, first_page_url)
        }
    }
}

fn page_headers(map: &reqwest::header::HeaderMap) -> Option<TradeHistoryPageHeaders> {
    let text = |name: &str| map.get(name)?.to_str().ok().map(str::to_string);
    let next = text("X-Q-Trade-Next-Cursor").unwrap_or_default();
    Some(TradeHistoryPageHeaders {
        frozen_epoch: text("X-Q-Trade-Frozen-Epoch")?,
        frozen_seq: text("X-Q-Trade-Frozen-Seq")?.parse().ok()?,
        next_cursor: serde_json::Value::String(next),
        page_count: text("X-Q-Trade-Page-Count")?.parse().ok()?,
        snapshot_id: text("X-Q-Trade-Snapshot-Id")?,
        source_generation: text("X-Q-Trade-Source-Generation")?,
        symbol: text("X-Q-Trade-Symbol")?,
        volume_field: text("X-Q-Trade-Volume-Field")?,
        volume_unit: text("X-Q-Trade-Volume-Unit")?,
    })
}
