//! Per-bar volume statistics for the current session, computed by the `q-indicators` kernel.
//!
//! This module only feeds trades to `VolumeState` in source order and keeps the output of
//! each bar once the kernel moves past it; every number comes from the kernel.

use std::collections::VecDeque;

use q_indicators::volume::{
    AggressorSide, TradeInput, VolumeConfig, VolumeError, VolumeOutput, VolumeState,
};

use super::history::{Trade, TradeHistory};
use super::time::BarGrid;

/// Large-print markers kept for the chart: the latest 1000.
pub const MAX_PRINT_MARKERS: usize = 1000;

/// The kernel parameters a volume study carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeParams {
    pub window_ms: i64,
    pub large_print_threshold: f64,
}

impl Default for VolumeParams {
    fn default() -> Self {
        let config = VolumeConfig::default();
        Self {
            window_ms: config.window_ms,
            large_print_threshold: config.large_print_threshold,
        }
    }
}

impl VolumeParams {
    pub fn config(&self) -> VolumeConfig {
        VolumeConfig {
            window_ms: self.window_ms,
            large_print_threshold: self.large_print_threshold,
        }
    }

    /// Whether the kernel accepts these parameters.
    pub fn validate(&self) -> Result<(), VolumeError> {
        VolumeState::new(self.config()).map(|_| ())
    }

    /// Equal parameters share one analysis.
    pub fn key(&self) -> (i64, u64) {
        (self.window_ms, self.large_print_threshold.to_bits())
    }
}

/// The kernel's output for one chart bar, taken after the bar's last trade.
#[derive(Debug, Clone, PartialEq)]
pub struct BarAggregate {
    pub open_ms: i64,
    pub close_ms: i64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub unknown_volume: f64,
    pub total_volume: f64,
    pub delta: f64,
    pub cumulative_delta: f64,
    pub classified_share: Option<f64>,
    pub trade_rate: Option<f64>,
    pub rate_warmed_up: bool,
    pub large_prints: usize,
}

impl BarAggregate {
    fn from_output(out: &VolumeOutput, open_ms: i64, close_ms: i64) -> Self {
        Self {
            open_ms,
            close_ms,
            buy_volume: out.buy_volume,
            sell_volume: out.sell_volume,
            unknown_volume: out.unknown_volume,
            total_volume: out.total_volume,
            delta: out.delta,
            cumulative_delta: out.cumulative_delta,
            classified_share: out.classified_share,
            trade_rate: out.trade_rate,
            rate_warmed_up: out.rate_warmed_up,
            large_prints: out.large_prints.len(),
        }
    }
}

/// A print that met the large-print threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintMark {
    pub bar_open_ms: i64,
    pub side: AggressorSide,
    pub time_msc: i64,
    pub price: f64,
    pub volume: f64,
}

#[derive(Debug)]
pub struct Analysis {
    params: VolumeParams,
    grid: BarGrid,
    session_key: u64,
    state: VolumeState,
    bars: Vec<BarAggregate>,
    prints: VecDeque<PrintMark>,
    bar_prints_seen: usize,
    trades: u64,
    failure: Option<VolumeError>,
}

impl Analysis {
    pub fn new(params: VolumeParams, grid: BarGrid, session_key: u64) -> Result<Self, VolumeError> {
        Ok(Self {
            params,
            grid,
            session_key,
            state: VolumeState::new(params.config())?,
            bars: Vec::new(),
            prints: VecDeque::new(),
            bar_prints_seen: 0,
            trades: 0,
            failure: None,
        })
    }

    pub fn params(&self) -> VolumeParams {
        self.params
    }

    pub fn grid(&self) -> BarGrid {
        self.grid
    }

    pub fn trades(&self) -> u64 {
        self.trades
    }

    /// Set once the kernel rejected a trade; the analysis then stops rather than skip it.
    pub fn failure(&self) -> Option<&VolumeError> {
        self.failure.as_ref()
    }

    pub fn bars(&self) -> &[BarAggregate] {
        &self.bars
    }

    pub fn prints(&self) -> &VecDeque<PrintMark> {
        &self.prints
    }

    /// The aggregate of the bar opening at `open_ms` (UTC), if the tape reached it.
    pub fn bar(&self, open_ms: i64) -> Option<&BarAggregate> {
        self.bars
            .binary_search_by_key(&open_ms, |b| b.open_ms)
            .ok()
            .map(|i| &self.bars[i])
    }

    pub fn latest(&self) -> Option<&BarAggregate> {
        self.bars.last()
    }

    pub fn push(&mut self, trade: Trade) {
        if self.failure.is_some() {
            return;
        }
        let (open, close) = self.grid.bounds(trade.time_msc);
        let input = TradeInput {
            time_msc: trade.time_msc,
            price: trade.price,
            volume: trade.volume,
            raw_flags: trade.raw_flags,
            session_key: self.session_key,
            bar_open_msc: open,
            bar_close_msc: close,
        };
        let out = match self.state.push(input) {
            Ok(out) => out,
            Err(e) => {
                self.failure = Some(e);
                return;
            }
        };
        self.trades += 1;
        let aggregate = BarAggregate::from_output(&out, open, close);
        match self.bars.last_mut() {
            Some(last) if last.open_ms == open => *last = aggregate,
            _ => {
                self.bar_prints_seen = 0;
                self.bars.push(aggregate);
            }
        }
        for print in out.large_prints.iter().skip(self.bar_prints_seen) {
            if self.prints.len() == MAX_PRINT_MARKERS {
                self.prints.pop_front();
            }
            self.prints.push_back(PrintMark {
                bar_open_ms: open,
                side: print.side,
                time_msc: print.time_msc,
                price: print.price,
                volume: print.volume,
            });
        }
        self.bar_prints_seen = out.large_prints.len();
    }

    /// Feeds every trade from `from_row` on.
    pub fn extend_from(&mut self, history: &TradeHistory, from_row: u64) {
        for trade in history.iter_from(from_row) {
            if self.failure.is_some() {
                return;
            }
            self.push(trade);
        }
    }
}
