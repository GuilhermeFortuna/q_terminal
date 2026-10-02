//! The tape panel's rows: newest first, filtered for display only.
//!
//! A `TapeView` belongs to one panel. It pulls from the shared feed's history, keeps at most
//! [`MAX_TAPE_ROWS`] matching rows and never feeds anything back: filters decide what is
//! shown, not what the analysis counted.

use std::collections::VecDeque;

use q_indicators::volume::AggressorSide;
use serde::{Deserialize, Serialize};

use super::feed::{Phase, TradeFeed};
use super::history::Trade;

pub const MAX_TAPE_ROWS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SideFilter {
    #[default]
    All,
    Buy,
    Sell,
    Unknown,
}

impl SideFilter {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "all" => Some(Self::All),
            "buy" => Some(Self::Buy),
            "sell" => Some(Self::Sell),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Unknown => "unknown",
        }
    }

    fn admits(self, side: AggressorSide) -> bool {
        matches!(
            (self, side),
            (Self::All, _)
                | (Self::Buy, AggressorSide::Buy)
                | (Self::Sell, AggressorSide::Sell)
                | (Self::Unknown, AggressorSide::Unknown)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TapeFilter {
    pub min_volume: f64,
    pub side: SideFilter,
}

impl Default for TapeFilter {
    fn default() -> Self {
        Self {
            min_volume: 0.0,
            side: SideFilter::All,
        }
    }
}

impl TapeFilter {
    pub fn admits(&self, trade: &Trade) -> bool {
        trade.volume >= self.min_volume
            && self.side.admits(AggressorSide::from_flags(trade.raw_flags))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TapeRow {
    pub time_msc: i64,
    pub price: f64,
    pub volume: f64,
    pub side: AggressorSide,
    pub large: bool,
}

pub fn side_name(side: AggressorSide) -> &'static str {
    match side {
        AggressorSide::Buy => "buy",
        AggressorSide::Sell => "sell",
        AggressorSide::Unknown => "unknown",
    }
}

#[derive(Debug)]
pub struct TapeView {
    filter: TapeFilter,
    rows: VecDeque<TapeRow>,
    next_row: u64,
    history_epoch: u64,
    large_threshold: f64,
}

impl Default for TapeView {
    fn default() -> Self {
        Self::new(TapeFilter::default())
    }
}

impl TapeView {
    pub fn new(filter: TapeFilter) -> Self {
        Self {
            filter,
            rows: VecDeque::new(),
            next_row: 0,
            history_epoch: u64::MAX,
            large_threshold: f64::INFINITY,
        }
    }

    pub fn filter(&self) -> TapeFilter {
        self.filter
    }

    /// Newest first.
    pub fn rows(&self) -> &VecDeque<TapeRow> {
        &self.rows
    }

    /// Changing the filter shows a different slice of the same tape; the next sync reloads it.
    pub fn set_filter(&mut self, filter: TapeFilter) {
        if filter != self.filter {
            self.filter = filter;
            self.history_epoch = u64::MAX;
        }
    }

    fn row_of(&self, trade: &Trade) -> TapeRow {
        TapeRow {
            time_msc: trade.time_msc,
            price: trade.price,
            volume: trade.volume,
            side: AggressorSide::from_flags(trade.raw_flags),
            large: trade.volume >= self.large_threshold,
        }
    }

    /// Brings the view up to date with the feed. Returns whether the rows changed.
    ///
    /// While a snapshot loads the view holds what it has: pages arrive oldest first and would
    /// only churn a list the trader is reading.
    pub fn sync(&mut self, feed: &TradeFeed, large_threshold: f64) -> bool {
        if feed.phase() != &Phase::Live {
            return false;
        }
        let history = feed.history();
        let threshold_changed = self.large_threshold.to_bits() != large_threshold.to_bits();
        self.large_threshold = large_threshold;
        if self.history_epoch != feed.history_epoch() || history.rows() < self.next_row {
            self.rows.clear();
            let filter = self.filter;
            let found: Vec<TapeRow> = history
                .iter_rev()
                .filter(|t| filter.admits(t))
                .take(MAX_TAPE_ROWS)
                .map(|t| self.row_of(&t))
                .collect();
            self.rows.extend(found);
            self.history_epoch = feed.history_epoch();
            self.next_row = history.rows();
            return true;
        }
        let mut changed = false;
        if threshold_changed {
            for row in &mut self.rows {
                row.large = row.volume >= large_threshold;
            }
            changed = true;
        }
        if history.rows() > self.next_row {
            for trade in history.iter_from(self.next_row) {
                if self.filter.admits(&trade) {
                    self.rows.push_front(self.row_of(&trade));
                    changed = true;
                }
            }
            self.rows.truncate(MAX_TAPE_ROWS);
            self.next_row = history.rows();
        }
        changed
    }
}
