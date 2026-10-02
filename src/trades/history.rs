//! The session's trades as bounded temporary columnar chunks.
//!
//! A busy WIN or WDO session holds tens of millions of prints, so history is five parallel
//! columns per chunk, never an object per trade. Timeframe and threshold edits replay it, so
//! the whole session is kept up to a hard cap and a batch that does not fit is refused whole:
//! the caller must then show the session as incomplete rather than truncate it silently.

use std::sync::Arc;

/// Rows per chunk.
pub const CHUNK_ROWS: usize = 65_536;
/// Bytes one row occupies across the five columns.
pub const ROW_BYTES: usize = 8 + 8 + 8 + 4 + 4;
/// Hard cap on one feed's history.
pub const DEFAULT_CAPACITY_BYTES: usize = 1 << 30;

/// Rows of one delivery or history page, in source order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TradeColumns {
    pub time_msc: Vec<i64>,
    pub price: Vec<f64>,
    /// The selected analysis volume (`volume` or `volume_real`), in the source's unit.
    pub volume: Vec<f64>,
    pub raw_flags: Vec<u32>,
    pub occurrence: Vec<u32>,
}

impl TradeColumns {
    pub fn len(&self) -> usize {
        self.time_msc.len()
    }

    pub fn is_empty(&self) -> bool {
        self.time_msc.is_empty()
    }

    pub fn push(
        &mut self,
        time_msc: i64,
        price: f64,
        volume: f64,
        raw_flags: u32,
        occurrence: u32,
    ) {
        self.time_msc.push(time_msc);
        self.price.push(price);
        self.volume.push(volume);
        self.raw_flags.push(raw_flags);
        self.occurrence.push(occurrence);
    }
}

/// One stored trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trade {
    pub time_msc: i64,
    pub price: f64,
    pub volume: f64,
    pub raw_flags: u32,
    pub occurrence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryError {
    /// The batch would take the history past its cap; nothing of it was stored.
    CapacityExceeded { capacity_bytes: usize },
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CapacityExceeded { capacity_bytes } => write!(
                f,
                "session trade history is full ({} MiB)",
                capacity_bytes / (1 << 20)
            ),
        }
    }
}

impl std::error::Error for HistoryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Appended {
    pub stored: usize,
    /// Rows whose identity was already stored: a replayed delivery, not a repeated print.
    pub duplicates: usize,
}

#[derive(Debug, Default, Clone)]
pub struct Chunk {
    time_msc: Vec<i64>,
    price: Vec<f64>,
    volume: Vec<f64>,
    raw_flags: Vec<u32>,
    occurrence: Vec<u32>,
}

impl Chunk {
    fn with_capacity(rows: usize) -> Self {
        Self {
            time_msc: Vec::with_capacity(rows),
            price: Vec::with_capacity(rows),
            volume: Vec::with_capacity(rows),
            raw_flags: Vec::with_capacity(rows),
            occurrence: Vec::with_capacity(rows),
        }
    }

    fn len(&self) -> usize {
        self.time_msc.len()
    }

    fn trade(&self, i: usize) -> Trade {
        Trade {
            time_msc: self.time_msc[i],
            price: self.price[i],
            volume: self.volume[i],
            raw_flags: self.raw_flags[i],
            occurrence: self.occurrence[i],
        }
    }
}

/// Reads shared by the live history and the immutable view a background rebuild replays.
fn iter_chunks<'a>(
    sealed: &'a [Arc<Chunk>],
    tail: &'a Chunk,
    chunk_rows: usize,
    start: u64,
) -> impl Iterator<Item = Trade> + 'a {
    let first = (start / chunk_rows as u64) as usize;
    let skip = (start % chunk_rows as u64) as usize;
    sealed
        .iter()
        .map(|c| c.as_ref())
        .chain(std::iter::once(tail))
        .skip(first)
        .enumerate()
        .flat_map(move |(n, chunk)| {
            let from = if n == 0 { skip } else { 0 };
            (from..chunk.len()).map(move |i| chunk.trade(i))
        })
}

#[derive(Debug)]
pub struct TradeHistory {
    /// Full chunks never change again, so a rebuild can hold them without the feed's lock.
    sealed: Vec<Arc<Chunk>>,
    tail: Chunk,
    chunk_rows: usize,
    capacity_bytes: usize,
    rows: u64,
    last_identity: Option<(i64, u32)>,
}

/// An immutable view of the history at one moment.
#[derive(Debug, Clone)]
pub struct HistorySnapshot {
    sealed: Vec<Arc<Chunk>>,
    tail: Chunk,
    chunk_rows: usize,
    rows: u64,
}

impl HistorySnapshot {
    pub fn rows(&self) -> u64 {
        self.rows
    }

    pub fn iter_from(&self, start: u64) -> impl Iterator<Item = Trade> + '_ {
        iter_chunks(&self.sealed, &self.tail, self.chunk_rows, start)
    }
}

impl Default for TradeHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl TradeHistory {
    pub fn new() -> Self {
        Self::with_limits(CHUNK_ROWS, DEFAULT_CAPACITY_BYTES)
    }

    pub fn with_limits(chunk_rows: usize, capacity_bytes: usize) -> Self {
        Self {
            sealed: Vec::new(),
            tail: Chunk::default(),
            chunk_rows: chunk_rows.max(1),
            capacity_bytes,
            rows: 0,
            last_identity: None,
        }
    }

    pub fn rows(&self) -> u64 {
        self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0
    }

    fn chunk_count(&self) -> usize {
        self.sealed.len() + usize::from(self.tail.len() > 0 || self.tail.time_msc.capacity() > 0)
    }

    /// Bytes held, counted by allocated chunk like the cap is.
    pub fn allocated_bytes(&self) -> usize {
        self.chunk_count() * self.chunk_rows * ROW_BYTES
    }

    pub fn capacity_bytes(&self) -> usize {
        self.capacity_bytes
    }

    pub fn clear(&mut self) {
        self.sealed = Vec::new();
        self.tail = Chunk::default();
        self.rows = 0;
        self.last_identity = None;
    }

    pub fn snapshot(&self) -> HistorySnapshot {
        HistorySnapshot {
            sealed: self.sealed.clone(),
            tail: self.tail.clone(),
            chunk_rows: self.chunk_rows,
            rows: self.rows,
        }
    }

    /// Appends rows in source order.
    ///
    /// A row is a replay when its identity `(time_msc, occurrence)` does not advance past the
    /// last stored one; two prints with equal price, volume and flags but different occurrence
    /// are both kept. The batch is stored whole or not at all.
    pub fn append(&mut self, batch: &TradeColumns) -> Result<Appended, HistoryError> {
        let mut fresh = Vec::with_capacity(batch.len());
        let mut last = self.last_identity;
        for i in 0..batch.len() {
            let identity = (batch.time_msc[i], batch.occurrence[i]);
            if last.is_some_and(|l| identity <= l) {
                continue;
            }
            last = Some(identity);
            fresh.push(i);
        }
        let duplicates = batch.len() - fresh.len();
        let free_rows = if self.chunk_count() == 0 {
            0
        } else {
            self.chunk_rows - self.tail.len()
        };
        let new_chunks = fresh
            .len()
            .saturating_sub(free_rows)
            .div_ceil(self.chunk_rows);
        if (self.chunk_count() + new_chunks) * self.chunk_rows * ROW_BYTES > self.capacity_bytes {
            return Err(HistoryError::CapacityExceeded {
                capacity_bytes: self.capacity_bytes,
            });
        }
        for &i in &fresh {
            if self.tail.time_msc.capacity() == 0 {
                self.tail = Chunk::with_capacity(self.chunk_rows);
            } else if self.tail.len() >= self.chunk_rows {
                let full = std::mem::replace(&mut self.tail, Chunk::with_capacity(self.chunk_rows));
                self.sealed.push(Arc::new(full));
            }
            self.tail.time_msc.push(batch.time_msc[i]);
            self.tail.price.push(batch.price[i]);
            self.tail.volume.push(batch.volume[i]);
            self.tail.raw_flags.push(batch.raw_flags[i]);
            self.tail.occurrence.push(batch.occurrence[i]);
        }
        self.rows += fresh.len() as u64;
        self.last_identity = last;
        Ok(Appended {
            stored: fresh.len(),
            duplicates,
        })
    }

    pub fn get(&self, index: u64) -> Option<Trade> {
        if index >= self.rows {
            return None;
        }
        let c = (index / self.chunk_rows as u64) as usize;
        let i = (index % self.chunk_rows as u64) as usize;
        Some(match self.sealed.get(c) {
            Some(chunk) => chunk.trade(i),
            None => self.tail.trade(i),
        })
    }

    /// Trades from row `start` to the end, oldest first.
    pub fn iter_from(&self, start: u64) -> impl Iterator<Item = Trade> + '_ {
        iter_chunks(&self.sealed, &self.tail, self.chunk_rows, start)
    }

    /// Trades from the newest back to the oldest.
    pub fn iter_rev(&self) -> impl Iterator<Item = Trade> + '_ {
        std::iter::once(&self.tail)
            .chain(self.sealed.iter().rev().map(|c| c.as_ref()))
            .flat_map(|chunk| (0..chunk.len()).rev().map(move |i| chunk.trade(i)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(items: &[(i64, u32)]) -> TradeColumns {
        let mut c = TradeColumns::default();
        for &(t, occ) in items {
            c.push(t, 100.0, 2.0, 32, occ);
        }
        c
    }

    #[test]
    fn equal_prints_with_distinct_occurrence_are_all_kept() {
        let mut h = TradeHistory::new();
        let out = h.append(&rows(&[(1000, 0), (1000, 1), (1000, 2)])).unwrap();
        assert_eq!(
            out,
            Appended {
                stored: 3,
                duplicates: 0
            }
        );
        assert_eq!(h.rows(), 3);
    }

    #[test]
    fn replayed_identities_are_dropped_not_the_prints_themselves() {
        let mut h = TradeHistory::new();
        h.append(&rows(&[(1000, 0), (1000, 1)])).unwrap();
        let out = h.append(&rows(&[(1000, 1), (1000, 2), (1001, 0)])).unwrap();
        assert_eq!(
            out,
            Appended {
                stored: 2,
                duplicates: 1
            }
        );
        let times: Vec<_> = h.iter_from(0).map(|t| (t.time_msc, t.occurrence)).collect();
        assert_eq!(times, vec![(1000, 0), (1000, 1), (1000, 2), (1001, 0)]);
    }

    #[test]
    fn chunks_are_reused_across_reads_in_both_directions() {
        let mut h = TradeHistory::with_limits(4, usize::MAX);
        let items: Vec<_> = (0..10).map(|i| (1000 + i, 0)).collect();
        h.append(&rows(&items)).unwrap();
        assert_eq!(h.iter_from(0).count(), 10);
        assert_eq!(
            h.iter_from(6).map(|t| t.time_msc).collect::<Vec<_>>(),
            vec![1006, 1007, 1008, 1009]
        );
        assert_eq!(h.iter_rev().next().unwrap().time_msc, 1009);
        assert_eq!(h.get(5).unwrap().time_msc, 1005);
        assert!(h.get(10).is_none());
    }

    #[test]
    fn a_batch_past_the_cap_is_refused_whole() {
        let mut h = TradeHistory::with_limits(4, 2 * 4 * ROW_BYTES);
        h.append(&rows(&[(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)]))
            .unwrap();
        let err = h
            .append(&rows(&[(6, 0), (7, 0), (8, 0), (9, 0), (10, 0)]))
            .unwrap_err();
        assert!(matches!(err, HistoryError::CapacityExceeded { .. }));
        assert_eq!(h.rows(), 5, "nothing of a refused batch is stored");
        assert!(h.append(&rows(&[(6, 0), (7, 0), (8, 0)])).is_ok());
    }

    #[test]
    fn clear_releases_everything() {
        let mut h = TradeHistory::with_limits(4, usize::MAX);
        h.append(&rows(&[(1, 0), (2, 0)])).unwrap();
        h.clear();
        assert_eq!((h.rows(), h.allocated_bytes()), (0, 0));
        assert!(h.append(&rows(&[(1, 0)])).is_ok());
    }
}
