//! The shared trade feed: a frozen session snapshot joined with live deliveries.
//!
//! One `TradeFeed` exists per process and follows the chart's symbol. The stream client
//! subscribes to `trades` and `trades.status`, buffers deliveries while a loader task reads
//! the immutable snapshot pages, then applies the buffered sequences above the snapshot's
//! frozen `(epoch, seq)` watermark exactly once. Transport batches are deduplicated by
//! `(epoch, seq)`; individual records by their contracted identity, never by equal values.
//!
//! Nothing here blocks or touches the network: the driver calls these methods under the
//! handle's lock and reads the same state back to render it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;

use super::analysis::{Analysis, VolumeParams};
use super::history::{HistoryError, HistorySnapshot, TradeColumns, TradeHistory};
use super::time::{numeric_session_key, BarGrid, ExchangeClock};
use crate::contracts_stream::{TradeHistoryPageHeaders, TradeSnapshotResponse, TradeSourceStatus};

/// Live deliveries held back while a snapshot loads. Past this the load starts over.
pub const MAX_BUFFERED_DELIVERIES: usize = 100_000;

/// Provider, symbol, generation and unit bound to every row of a delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceContext {
    pub provider_id: String,
    pub symbol: String,
    pub source_generation: String,
    pub exchange_timezone: String,
    pub session_key: String,
    pub volume_field: String,
    pub volume_unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodedBatch {
    pub context: SourceContext,
    pub columns: TradeColumns,
    /// Rows the decoder refused (non-finite, non-positive or missing selected volume).
    pub invalid_rows: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Delivery {
    /// A delivery for another symbol: counted for sequence continuity, never decoded.
    OtherSymbol,
    Batch(Box<DecodedBatch>),
    /// A delivery for this symbol that could not be decoded: a hole in the tape.
    Undecodable(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncReason {
    Gap,
    Lagging,
    Reconnect,
    EpochChanged,
    SourceCorrection,
    SessionRoll,
    MixedGenerations,
    Expired,
    Retry,
}

impl ResyncReason {
    /// A fresh snapshot replaces everything, but some causes leave the old output a valid
    /// prefix (a hole, a reconnect) while others make it wrong (a corrected source).
    fn discards_output(self) -> bool {
        !matches!(
            self,
            Self::Gap | Self::Lagging | Self::Reconnect | Self::Retry
        )
    }

    pub fn text(self) -> &'static str {
        match self {
            Self::Gap => "sequence gap",
            Self::Lagging => "stream lagged",
            Self::Reconnect => "reconnected",
            Self::EpochChanged => "publisher epoch changed",
            Self::SourceCorrection => "source corrected",
            Self::SessionRoll => "new session",
            Self::MixedGenerations => "mixed source generations",
            Self::Expired => "snapshot expired",
            Self::Retry => "retry",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveOutcome {
    Applied { rows: usize },
    Duplicate,
    Buffered,
    Ignored,
    Resync(ResyncReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedError {
    /// The response belongs to a load that has been replaced.
    Stale,
    /// The response disagrees with the snapshot it claims to belong to.
    Mismatch(String),
    UnsupportedZone(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    Idle,
    /// Waiting for, or reading, the session snapshot.
    Loading,
    Live,
    Unavailable(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Coverage {
    pub state: String,
    pub classification: String,
    pub reason: Option<String>,
    pub covered_from: Option<String>,
    pub covered_to: Option<String>,
}

impl Coverage {
    fn from_status(status: &TradeSourceStatus) -> Self {
        let text = |v: &serde_json::Value| v.as_str().map(str::to_string);
        Self {
            state: status.coverage_state.clone(),
            classification: status.classification_coverage.clone(),
            reason: text(&status.coverage_reason),
            covered_from: text(&status.covered_from),
            covered_to: text(&status.covered_to),
        }
    }
}

#[derive(Debug)]
struct Buffered {
    epoch: String,
    delivery: Delivery,
}

/// Why the tape cannot be trusted as complete, for the panel and legends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    pub code: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StateReport {
    pub symbol: String,
    pub phase: &'static str,
    pub state_text: String,
    pub complete: bool,
    pub stale: bool,
    pub stale_since_ms: Option<i64>,
    pub loading: bool,
    pub backfill_pending: bool,
    pub progress: f64,
    pub loaded_rows: u64,
    pub expected_rows: Option<u64>,
    pub coverage: Coverage,
    pub volume_field: String,
    pub volume_unit: String,
    pub classified_share: Option<f64>,
    pub history_rows: u64,
    pub invalid_rows: u64,
    pub duplicate_rows: u64,
    pub duplicate_batches: u64,
    pub capacity_exhausted: bool,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    pub batches_applied: u64,
    pub duplicate_batches: u64,
    pub resyncs: u64,
    pub snapshots: u64,
    pub pages: u64,
}

struct Snapshot {
    id: String,
    expected_rows: u64,
    frozen: (String, i64),
}

pub struct TradeFeed {
    symbol: Option<String>,
    phase: Phase,
    /// Identifies the current load; responses carrying another one are discarded.
    load_generation: u64,
    context: Option<SourceContext>,
    clock: Option<ExchangeClock>,
    session_key: u64,
    coverage: Coverage,
    latest_status_pos: Option<(String, i64)>,
    snapshot: Option<Snapshot>,
    pending_backfill: bool,
    loaded_rows: u64,
    invalid_rows: u64,
    duplicate_rows: u64,
    history: TradeHistory,
    /// Bumps whenever the history is replaced, so tape views know to start over.
    history_epoch: u64,
    live_pos: Option<(String, i64)>,
    buffer: BTreeMap<i64, Buffered>,
    interval_ms: i64,
    params: Vec<VolumeParams>,
    analyses: Vec<Analysis>,
    /// The previous output, shown stale while a replacement snapshot loads.
    retained: Option<Vec<Analysis>>,
    analysis_token: u64,
    capacity_exhausted: bool,
    stale_since_ms: Option<i64>,
    mixed_generations: bool,
    /// Prints at or above this are flagged on the tape: the first large-print study's
    /// threshold, or the kernel default.
    display_threshold: f64,
    revision: u64,
    pub counters: Counters,
}

impl Default for TradeFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeFeed {
    pub fn new() -> Self {
        Self::with_history(TradeHistory::new())
    }

    pub fn with_history(history: TradeHistory) -> Self {
        Self {
            symbol: None,
            phase: Phase::Idle,
            load_generation: 0,
            context: None,
            clock: None,
            session_key: 0,
            coverage: Coverage::default(),
            latest_status_pos: None,
            snapshot: None,
            pending_backfill: false,
            loaded_rows: 0,
            invalid_rows: 0,
            duplicate_rows: 0,
            history,
            history_epoch: 0,
            live_pos: None,
            buffer: BTreeMap::new(),
            interval_ms: 60_000,
            params: vec![VolumeParams::default()],
            analyses: Vec::new(),
            retained: None,
            analysis_token: 0,
            capacity_exhausted: false,
            stale_since_ms: None,
            mixed_generations: false,
            display_threshold: VolumeParams::default().large_print_threshold,
            revision: 0,
            counters: Counters::default(),
        }
    }

    // -- read side ------------------------------------------------------------------------

    pub fn symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn load_generation(&self) -> u64 {
        self.load_generation
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn history(&self) -> &TradeHistory {
        &self.history
    }

    pub fn history_epoch(&self) -> u64 {
        self.history_epoch
    }

    pub fn context(&self) -> Option<&SourceContext> {
        self.context.as_ref()
    }

    pub fn interval_ms(&self) -> i64 {
        self.interval_ms
    }

    pub fn display_threshold(&self) -> f64 {
        self.display_threshold
    }

    /// Which prints the tape flags as large. `None` returns to the kernel default.
    pub fn set_display_threshold(&mut self, threshold: Option<f64>) -> bool {
        let next = threshold
            .filter(|t| t.is_finite() && *t > 0.0)
            .unwrap_or(VolumeParams::default().large_print_threshold);
        if next.to_bits() == self.display_threshold.to_bits() {
            return false;
        }
        self.display_threshold = next;
        self.touch();
        true
    }

    pub fn clock(&self) -> Option<ExchangeClock> {
        self.clock
    }

    pub fn is_live(&self) -> bool {
        self.phase == Phase::Live
    }

    pub fn capacity_exhausted(&self) -> bool {
        self.capacity_exhausted
    }

    /// The analyses the chart should draw: the replacement once it has data, otherwise the
    /// retained previous output while a resync is loading.
    pub fn analyses(&self) -> &[Analysis] {
        match &self.retained {
            Some(old) if self.phase == Phase::Loading => old,
            _ => &self.analyses,
        }
    }

    pub fn analysis_for(&self, params: VolumeParams) -> Option<&Analysis> {
        self.analyses()
            .iter()
            .find(|a| a.params().key() == params.key())
    }

    /// The analysis behind the header totals: the default-parameter one.
    pub fn base_analysis(&self) -> Option<&Analysis> {
        self.analysis_for(VolumeParams::default())
    }

    /// Whether what the analyses hold is the live session rather than a stale or partial view.
    pub fn output_is_current(&self) -> bool {
        self.phase == Phase::Live && self.stale_since_ms.is_none() && !self.capacity_exhausted
    }

    pub fn report(&self) -> StateReport {
        let phase = match &self.phase {
            Phase::Idle => "idle",
            Phase::Loading => "loading",
            Phase::Live => "live",
            Phase::Unavailable(_) => "unavailable",
        };
        let stale = self.stale_since_ms.is_some() || self.retained.is_some();
        let complete = self.phase == Phase::Live
            && !stale
            && !self.capacity_exhausted
            && self.coverage.state == "complete";
        let mut notes = Vec::new();
        let mut note = |code: &'static str, text: String| notes.push(Note { code, text });
        match &self.phase {
            Phase::Unavailable(reason) => {
                note("unavailable", format!("Trade source unavailable: {reason}"))
            }
            Phase::Loading if self.pending_backfill => note(
                "backfill",
                "Source is backfilling the session; coverage is partial".into(),
            ),
            Phase::Loading => note(
                "loading",
                "Loading session history; totals and cumulative delta are provisional".into(),
            ),
            _ => {}
        }
        if let Some(since) = self.stale_since_ms {
            note(
                "stale",
                format!("Disconnected; totals frozen as of {}", format_clock(since)),
            );
        } else if self.retained.is_some() {
            note(
                "resync",
                "Resynchronising; showing the previous session view".into(),
            );
        }
        if self.capacity_exhausted {
            note(
                "capacity",
                "Session history is full; the session is incomplete — retry to reload".into(),
            );
        }
        if self.mixed_generations {
            note(
                "mixed_generations",
                "Deliveries from mixed source generations were refused".into(),
            );
        }
        if matches!(self.coverage.state.as_str(), "partial" | "unavailable")
            && self.phase != Phase::Idle
        {
            note(
                "coverage",
                format!(
                    "Source coverage {}{}",
                    self.coverage.state,
                    self.coverage
                        .reason
                        .as_deref()
                        .map(|r| format!(" ({r})"))
                        .unwrap_or_default()
                ),
            );
        }
        if self.invalid_rows > 0 {
            note(
                "invalid_rows",
                format!("{} trade records had no usable volume", self.invalid_rows),
            );
        }
        let classified = self.classified_share();
        if classified == Some(0.0) {
            note(
                "unclassified",
                "No aggressor side was reported; delta carries no direction".into(),
            );
        }
        if let Some(base) = self.base_analysis() {
            if let Some(f) = base.failure() {
                note("analysis", format!("Volume analysis stopped: {f}"));
            } else if self.phase == Phase::Live && base.latest().is_some_and(|b| !b.rate_warmed_up)
            {
                note("warming_up", "Trade rate is warming up".into());
            }
        }
        let (loaded, expected) = (
            self.loaded_rows,
            self.snapshot.as_ref().map(|s| s.expected_rows),
        );
        let progress = match expected {
            Some(total) if total > 0 => (loaded as f64 / total as f64).min(1.0),
            Some(_) => 1.0,
            None => 0.0,
        };
        let state_text = match (&self.phase, complete) {
            (Phase::Idle, _) => "No symbol".to_string(),
            (Phase::Unavailable(_), _) => "Unavailable".to_string(),
            (Phase::Loading, _) => format!("Loading {:.0}%", progress * 100.0),
            (Phase::Live, true) => "Live · complete".to_string(),
            (Phase::Live, false) if self.stale_since_ms.is_some() => "Stale".to_string(),
            (Phase::Live, false) => "Live · partial".to_string(),
        };
        StateReport {
            symbol: self.symbol.clone().unwrap_or_default(),
            phase,
            state_text,
            complete,
            stale,
            stale_since_ms: self.stale_since_ms,
            loading: self.phase == Phase::Loading,
            backfill_pending: self.pending_backfill,
            progress,
            loaded_rows: loaded,
            expected_rows: expected,
            coverage: self.coverage.clone(),
            volume_field: self
                .context
                .as_ref()
                .map(|c| c.volume_field.clone())
                .unwrap_or_default(),
            volume_unit: self
                .context
                .as_ref()
                .map(|c| c.volume_unit.clone())
                .unwrap_or_default(),
            classified_share: classified,
            history_rows: self.history.rows(),
            invalid_rows: self.invalid_rows,
            duplicate_rows: self.duplicate_rows,
            duplicate_batches: self.counters.duplicate_batches,
            capacity_exhausted: self.capacity_exhausted,
            notes,
        }
    }

    /// Share of session volume with a known aggressor side, summed from the kernel's bars.
    pub fn classified_share(&self) -> Option<f64> {
        let base = self.base_analysis()?;
        let (known, total) = base.bars().iter().fold((0.0, 0.0), |(k, t), b| {
            (k + b.buy_volume + b.sell_volume, t + b.total_volume)
        });
        (total > 0.0).then_some(known / total)
    }

    // -- target ---------------------------------------------------------------------------

    /// Follows the chart to `symbol`. Returns whether the load must start over: a repeat of
    /// the current symbol changes nothing and costs no request.
    pub fn retarget(&mut self, symbol: &str) -> bool {
        if self.symbol.as_deref() == Some(symbol) {
            return false;
        }
        self.symbol = Some(symbol.to_string());
        self.reset_session();
        self.buffer.clear();
        self.live_pos = None;
        self.latest_status_pos = None;
        self.phase = Phase::Loading;
        self.load_generation += 1;
        self.touch();
        true
    }

    /// Clears the target (no chart symbol).
    pub fn clear_target(&mut self) {
        self.symbol = None;
        self.reset_session();
        self.buffer.clear();
        self.phase = Phase::Idle;
        self.load_generation += 1;
        self.touch();
    }

    fn reset_session(&mut self) {
        self.context = None;
        self.clock = None;
        self.coverage = Coverage::default();
        self.snapshot = None;
        self.pending_backfill = false;
        self.loaded_rows = 0;
        self.invalid_rows = 0;
        self.duplicate_rows = 0;
        self.history.clear();
        self.history_epoch += 1;
        self.analyses.clear();
        self.retained = None;
        self.analysis_token += 1;
        self.capacity_exhausted = false;
        self.stale_since_ms = None;
        self.mixed_generations = false;
    }

    /// Starts the load over for `reason`; returns the new load generation.
    pub fn begin_resync(&mut self, reason: ResyncReason) -> u64 {
        if self.symbol.is_none() {
            return self.load_generation;
        }
        self.counters.resyncs += 1;
        let keep = !reason.discards_output() && self.phase == Phase::Live;
        let old = std::mem::take(&mut self.analyses);
        let previous_retained = self.retained.take();
        self.history.clear();
        self.history_epoch += 1;
        self.analysis_token += 1;
        self.snapshot = None;
        self.pending_backfill = false;
        self.loaded_rows = 0;
        self.capacity_exhausted = false;
        self.context = None;
        self.clock = None;
        self.live_pos = None;
        self.retained = if keep {
            Some(old)
        } else if !reason.discards_output() {
            previous_retained
        } else {
            None
        };
        if reason.discards_output() {
            self.invalid_rows = 0;
            self.duplicate_rows = 0;
            self.coverage = Coverage::default();
        }
        if reason != ResyncReason::MixedGenerations {
            self.mixed_generations = false;
        }
        self.phase = Phase::Loading;
        self.load_generation += 1;
        self.touch();
        self.load_generation
    }

    // -- chart parameters -------------------------------------------------------------------

    /// A timeframe-only change: regroup bars from the cached history, no new request.
    pub fn set_interval_ms(&mut self, interval_ms: i64) -> bool {
        if interval_ms <= 0 || interval_ms == self.interval_ms {
            return false;
        }
        self.interval_ms = interval_ms;
        self.analysis_token += 1;
        self.touch();
        true
    }

    /// Sets the volume-study parameters (the default set is always kept for the header).
    pub fn set_params(&mut self, params: &[VolumeParams]) -> bool {
        let mut next = vec![VolumeParams::default()];
        for p in params {
            if p.validate().is_ok() && !next.iter().any(|q| q.key() == p.key()) {
                next.push(*p);
            }
        }
        if next.len() == self.params.len()
            && next
                .iter()
                .zip(&self.params)
                .all(|(a, b)| a.key() == b.key())
        {
            return false;
        }
        self.params = next;
        self.analysis_token += 1;
        self.touch();
        true
    }

    pub fn needs_rebuild(&self) -> bool {
        self.clock.is_some()
            && (self.analyses.is_empty()
                || self.analyses[0].grid().interval_ms() != self.interval_ms
                || self.analyses.len() != self.params.len()
                || self
                    .analyses
                    .iter()
                    .zip(&self.params)
                    .any(|(a, p)| a.params().key() != p.key()))
    }

    /// A replay of the cached session history under the current interval and parameters.
    pub fn plan_rebuild(&self) -> Option<RebuildJob> {
        let clock = self.clock?;
        Some(RebuildJob {
            token: self.analysis_token,
            grid: BarGrid::new(self.interval_ms, clock)?,
            session_key: self.session_key,
            params: self.params.clone(),
            history: self.history.snapshot(),
        })
    }

    /// Installs a finished replay if nothing it depended on changed meanwhile, catching up
    /// on the rows that arrived while it ran.
    pub fn install_rebuild(&mut self, result: RebuildResult) -> bool {
        if result.token != self.analysis_token {
            return false;
        }
        let mut analyses = result.analyses;
        for a in &mut analyses {
            a.extend_from(&self.history, result.rows);
        }
        self.analyses = analyses;
        self.touch();
        true
    }

    fn fresh_analyses(&self) -> Vec<Analysis> {
        let Some(grid) = self.clock.and_then(|c| BarGrid::new(self.interval_ms, c)) else {
            return Vec::new();
        };
        self.params
            .iter()
            .filter_map(|p| Analysis::new(*p, grid, self.session_key).ok())
            .collect()
    }

    // -- loader side ------------------------------------------------------------------------

    fn check(&self, generation: u64) -> Result<(), FeedError> {
        if generation == self.load_generation && self.symbol.is_some() {
            Ok(())
        } else {
            Err(FeedError::Stale)
        }
    }

    /// The source answered 202: it is backfilling, which is not an empty complete session.
    pub fn snapshot_pending(&mut self, generation: u64) -> Result<(), FeedError> {
        self.check(generation)?;
        self.pending_backfill = true;
        self.phase = Phase::Loading;
        self.touch();
        Ok(())
    }

    pub fn set_unavailable(&mut self, generation: u64, reason: &str) -> Result<(), FeedError> {
        self.check(generation)?;
        self.phase = Phase::Unavailable(reason.to_string());
        self.pending_backfill = false;
        self.touch();
        Ok(())
    }

    /// Accepts a snapshot descriptor and starts the page load.
    pub fn begin_snapshot(
        &mut self,
        generation: u64,
        snap: &TradeSnapshotResponse,
    ) -> Result<(), FeedError> {
        self.check(generation)?;
        let cov = &snap.coverage;
        let agree = snap.provider_id == cov.provider_id
            && snap.symbol == cov.symbol
            && snap.source_generation == cov.source_generation
            && snap.volume_field == cov.volume_field
            && snap.volume_unit == cov.volume_unit;
        if !agree {
            return Err(FeedError::Mismatch(
                "snapshot context disagrees with its coverage".into(),
            ));
        }
        if self.symbol.as_deref() != Some(snap.symbol.as_str()) {
            return Err(FeedError::Stale);
        }
        let clock = ExchangeClock::for_zone(&snap.exchange_timezone)
            .map_err(|e| FeedError::UnsupportedZone(e.to_string()))?;
        // Rows already held belong to an earlier snapshot of this session; the new one is
        // complete on its own, so it replaces them instead of being merged.
        self.history.clear();
        self.history_epoch += 1;
        self.loaded_rows = 0;
        self.invalid_rows = 0;
        self.duplicate_rows = 0;
        self.capacity_exhausted = false;
        self.pending_backfill = false;
        self.clock = Some(clock);
        self.session_key = numeric_session_key(&snap.session_key);
        self.context = Some(SourceContext {
            provider_id: snap.provider_id.clone(),
            symbol: snap.symbol.clone(),
            source_generation: snap.source_generation.clone(),
            exchange_timezone: snap.exchange_timezone.clone(),
            session_key: snap.session_key.clone(),
            volume_field: snap.volume_field.clone(),
            volume_unit: snap.volume_unit.clone(),
        });
        self.coverage = Coverage::from_status(cov);
        self.snapshot = Some(Snapshot {
            id: snap.snapshot_id.clone(),
            expected_rows: snap.trade_count.max(0) as u64,
            frozen: (
                snap.frozen_watermark.epoch.clone(),
                snap.frozen_watermark.seq,
            ),
        });
        self.analysis_token += 1;
        self.analyses = self.fresh_analyses();
        self.touch();
        Ok(())
    }

    /// Applies one immutable history page.
    pub fn apply_page(
        &mut self,
        generation: u64,
        headers: &TradeHistoryPageHeaders,
        columns: &TradeColumns,
        invalid_rows: usize,
    ) -> Result<(), FeedError> {
        self.check(generation)?;
        let (Some(snapshot), Some(ctx)) = (&self.snapshot, &self.context) else {
            return Err(FeedError::Mismatch("page without a snapshot".into()));
        };
        let same = headers.snapshot_id == snapshot.id
            && headers.source_generation == ctx.source_generation
            && headers.symbol == ctx.symbol
            && headers.volume_field == ctx.volume_field
            && headers.volume_unit == ctx.volume_unit
            && headers.frozen_epoch == snapshot.frozen.0
            && headers.frozen_seq == snapshot.frozen.1;
        if !same {
            return Err(FeedError::Mismatch(
                "page does not belong to the snapshot".into(),
            ));
        }
        self.counters.pages += 1;
        self.invalid_rows += invalid_rows as u64;
        self.append_rows(columns, true);
        Ok(())
    }

    fn append_rows(&mut self, columns: &TradeColumns, page: bool) -> usize {
        if self.capacity_exhausted {
            return 0;
        }
        let before = self.history.rows();
        match self.history.append(columns) {
            Ok(done) => {
                self.duplicate_rows += done.duplicates as u64;
                if page {
                    self.loaded_rows += columns.len() as u64;
                }
                for a in &mut self.analyses {
                    a.extend_from(&self.history, before);
                }
                self.touch();
                done.stored
            }
            Err(HistoryError::CapacityExceeded { .. }) => {
                self.capacity_exhausted = true;
                self.touch();
                0
            }
        }
    }

    /// The last page arrived: go live and apply what was buffered above the watermark.
    pub fn finish_snapshot(&mut self, generation: u64) -> Result<Option<ResyncReason>, FeedError> {
        self.check(generation)?;
        let Some(snapshot) = &self.snapshot else {
            return Err(FeedError::Mismatch("finish without a snapshot".into()));
        };
        let (epoch, seq) = snapshot.frozen.clone();
        self.counters.snapshots += 1;
        self.retained = None;
        self.phase = Phase::Live;
        self.stale_since_ms = None;
        self.live_pos = Some((epoch.clone(), seq));
        self.touch();
        // Buffered entries at or below the watermark are inside the snapshot already.
        let buffered = std::mem::take(&mut self.buffer);
        for (s, entry) in buffered {
            if entry.epoch != epoch {
                if entry.epoch < epoch {
                    continue;
                }
                return Ok(Some(ResyncReason::EpochChanged));
            }
            if s <= seq {
                self.counters.duplicate_batches += 1;
                continue;
            }
            if let LiveOutcome::Resync(reason) = self.apply_live(&entry.epoch, s, entry.delivery) {
                return Ok(Some(reason));
            }
        }
        Ok(None)
    }

    // -- stream side ------------------------------------------------------------------------

    /// One `trades` envelope. Every envelope of the topic must be reported, whatever its
    /// symbol: the sequence is topic-wide, and a hole is only visible by counting them all.
    pub fn on_delivery(&mut self, epoch: &str, seq: i64, delivery: Delivery) -> LiveOutcome {
        if self.symbol.is_none() {
            return LiveOutcome::Ignored;
        }
        match self.phase {
            Phase::Idle | Phase::Unavailable(_) => LiveOutcome::Ignored,
            Phase::Loading => {
                if self.buffer.len() >= MAX_BUFFERED_DELIVERIES {
                    return LiveOutcome::Resync(ResyncReason::Lagging);
                }
                if self
                    .buffer
                    .insert(
                        seq,
                        Buffered {
                            epoch: epoch.to_string(),
                            delivery,
                        },
                    )
                    .is_some()
                {
                    self.counters.duplicate_batches += 1;
                }
                LiveOutcome::Buffered
            }
            Phase::Live => self.apply_live(epoch, seq, delivery),
        }
    }

    fn apply_live(&mut self, epoch: &str, seq: i64, delivery: Delivery) -> LiveOutcome {
        let Some((live_epoch, live_seq)) = self.live_pos.clone() else {
            return LiveOutcome::Ignored;
        };
        if epoch != live_epoch {
            return if epoch < live_epoch.as_str() {
                self.counters.duplicate_batches += 1;
                LiveOutcome::Duplicate
            } else {
                LiveOutcome::Resync(ResyncReason::EpochChanged)
            };
        }
        if seq <= live_seq {
            self.counters.duplicate_batches += 1;
            return LiveOutcome::Duplicate;
        }
        if seq > live_seq + 1 {
            return LiveOutcome::Resync(ResyncReason::Gap);
        }
        self.live_pos = Some((live_epoch, seq));
        match delivery {
            Delivery::OtherSymbol => LiveOutcome::Ignored,
            Delivery::Undecodable(_) => LiveOutcome::Resync(ResyncReason::Gap),
            Delivery::Batch(batch) => self.apply_batch(*batch),
        }
    }

    fn apply_batch(&mut self, batch: DecodedBatch) -> LiveOutcome {
        let Some(ctx) = &self.context else {
            return LiveOutcome::Ignored;
        };
        if batch.context.symbol != ctx.symbol {
            return LiveOutcome::Ignored;
        }
        if batch.context.source_generation != ctx.source_generation {
            self.mixed_generations = true;
            return LiveOutcome::Resync(ResyncReason::SourceCorrection);
        }
        if batch.context.session_key != ctx.session_key {
            return LiveOutcome::Resync(ResyncReason::SessionRoll);
        }
        if batch.context.volume_field != ctx.volume_field
            || batch.context.volume_unit != ctx.volume_unit
        {
            self.mixed_generations = true;
            return LiveOutcome::Resync(ResyncReason::MixedGenerations);
        }
        self.invalid_rows += batch.invalid_rows as u64;
        self.counters.batches_applied += 1;
        let stored = self.append_rows(&batch.columns, false);
        LiveOutcome::Applied { rows: stored }
    }

    /// A `trades.status` entry. Returns a resync reason when the source generation moved.
    pub fn on_status(
        &mut self,
        epoch: &str,
        seq: i64,
        status: &TradeSourceStatus,
    ) -> Option<ResyncReason> {
        if self.symbol.as_deref() != Some(status.symbol.as_str()) {
            return None;
        }
        if let Some((e, s)) = &self.latest_status_pos {
            if e == epoch && seq <= *s {
                return None;
            }
        }
        self.latest_status_pos = Some((epoch.to_string(), seq));
        if let Some(ctx) = &self.context {
            if ctx.source_generation != status.source_generation {
                return Some(ResyncReason::SourceCorrection);
            }
        }
        self.coverage = Coverage::from_status(status);
        self.touch();
        None
    }

    /// The stream dropped: totals freeze as of `now_ms` until a resync replaces them.
    pub fn on_disconnect(&mut self, now_ms: i64) {
        if self.phase == Phase::Live && self.stale_since_ms.is_none() {
            self.stale_since_ms = Some(now_ms);
        }
        self.touch();
    }

    /// Offers the capacity retry: the session reloads from a fresh snapshot.
    pub fn retry_capacity(&mut self) -> Option<u64> {
        self.capacity_exhausted
            .then(|| self.begin_resync(ResyncReason::SourceCorrection))
    }

    fn touch(&mut self) {
        self.revision += 1;
    }
}

fn format_clock(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let day = secs.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02} UTC",
        day / 3600,
        day % 3600 / 60,
        day % 60
    )
}

/// A replay of the session history on another thread.
pub struct RebuildJob {
    token: u64,
    grid: BarGrid,
    session_key: u64,
    params: Vec<VolumeParams>,
    history: HistorySnapshot,
}

pub struct RebuildResult {
    token: u64,
    rows: u64,
    analyses: Vec<Analysis>,
}

impl RebuildJob {
    pub fn run(self) -> RebuildResult {
        let rows = self.history.rows();
        let analyses = self
            .params
            .iter()
            .filter_map(|p| Analysis::new(*p, self.grid, self.session_key).ok())
            .map(|mut a| {
                for trade in self.history.iter_from(0) {
                    a.push(trade);
                }
                a
            })
            .collect();
        RebuildResult {
            token: self.token,
            rows,
            analyses,
        }
    }
}

// -- shared handle ---------------------------------------------------------------------------

type Listener = Box<dyn Fn() -> bool + Send + Sync>;

struct Shared {
    feed: Mutex<TradeFeed>,
    listeners: Mutex<Vec<Listener>>,
    /// Where panels ask the stream client to start a load over (a capacity retry).
    restarts: Mutex<Option<tokio::sync::mpsc::UnboundedSender<ResyncReason>>>,
}

/// The process-level feed: the stream client writes it, panels and the chart read it.
#[derive(Clone)]
pub struct TradeHandle {
    shared: Arc<Shared>,
}

impl Default for TradeHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeHandle {
    /// The process-level feed every panel and the chart share.
    pub fn process() -> Self {
        static PROCESS: std::sync::OnceLock<TradeHandle> = std::sync::OnceLock::new();
        PROCESS.get_or_init(Self::new).clone()
    }

    pub fn new() -> Self {
        Self::from_feed(TradeFeed::new())
    }

    pub fn from_feed(feed: TradeFeed) -> Self {
        Self {
            shared: Arc::new(Shared {
                feed: Mutex::new(feed),
                listeners: Mutex::new(Vec::new()),
                restarts: Mutex::new(None),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, TradeFeed> {
        self.shared.feed.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn read<R>(&self, f: impl FnOnce(&TradeFeed) -> R) -> R {
        f(&self.lock())
    }

    /// Runs `f` on the feed and tells the listeners if it changed anything.
    pub fn mutate<R>(&self, f: impl FnOnce(&mut TradeFeed) -> R) -> R {
        let (result, changed) = {
            let mut feed = self.lock();
            let before = feed.revision;
            let result = f(&mut feed);
            (result, feed.revision != before)
        };
        if changed {
            self.notify();
        }
        result
    }

    /// The stream client's end of the restart channel; a new connection replaces the last.
    pub fn restart_receiver(&self) -> tokio::sync::mpsc::UnboundedReceiver<ResyncReason> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        *self
            .shared
            .restarts
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(tx);
        rx
    }

    /// Asks the stream client to reload the session. False if no client is listening.
    pub fn request_restart(&self, reason: ResyncReason) -> bool {
        self.shared
            .restarts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|tx| tx.send(reason).is_ok())
    }

    /// Registers a listener; it is dropped when it returns false.
    pub fn subscribe(&self, listener: impl Fn() -> bool + Send + Sync + 'static) {
        self.shared
            .listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Box::new(listener));
    }

    fn notify(&self) {
        let mut listeners = self
            .shared
            .listeners
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        listeners.retain(|l| l());
    }

    /// Replays the cached history on this thread. For fixtures and tests that need the
    /// result before they look; live code uses [`rebuild_in_background`](Self::rebuild_in_background).
    pub fn rebuild_blocking(&self) {
        let Some(job) = self.read(|f| f.needs_rebuild().then(|| f.plan_rebuild()).flatten()) else {
            return;
        };
        let result = job.run();
        self.mutate(|f| f.install_rebuild(result));
    }

    /// Replays the cached history for the current interval and parameters off-thread.
    pub fn rebuild_in_background(&self) {
        let Some(job) = self.read(|f| f.needs_rebuild().then(|| f.plan_rebuild()).flatten()) else {
            return;
        };
        let handle = self.clone();
        let _ = std::thread::Builder::new()
            .name("trade-rebuild".into())
            .spawn(move || {
                let result = job.run();
                handle.mutate(|f| f.install_rebuild(result));
            });
    }
}
