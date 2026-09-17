use std::time::Instant;

use cxx_qt_lib::QString;
use q_buffers::frame::{BarColumns, TimeLabel};

#[derive(Debug, Clone)]
pub enum BarDelivery {
    Completed(BarColumns),
    Forming(BarColumns),
}

impl PartialEq for BarDelivery {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BarDelivery::Completed(a), BarDelivery::Completed(b))
            | (BarDelivery::Forming(a), BarDelivery::Forming(b)) => {
                a.time == b.time
                    && a.open
                        .iter()
                        .zip(&b.open)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.high
                        .iter()
                        .zip(&b.high)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.low
                        .iter()
                        .zip(&b.low)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
                    && a.close
                        .iter()
                        .zip(&b.close)
                        .all(|(x, y)| x.to_bits() == y.to_bits())
            }
            _ => false,
        }
    }
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, connection_state)]
        #[qproperty(QString, last_error)]
        #[qproperty(i64, data_age_ms)]
        #[qproperty(i64, applied)]
        #[qproperty(i64, dropped)]
        #[qproperty(i64, gaps_closed)]
        #[qproperty(i64, resnapshots)]
        type BarFeed = super::BarFeedRust;
    }

    impl cxx_qt::Threading for BarFeed {}
}

pub struct BarFeedRust {
    pub connection_state: QString,
    pub last_error: QString,
    pub data_age_ms: i64,
    pub applied: i64,
    pub dropped: i64,
    pub gaps_closed: i64,
    pub resnapshots: i64,

    pub series: q_qt::BarSeriesRust,
    pub last_applied_instant: Option<Instant>,
}

impl BarFeedRust {
    pub fn new(symbol: &str, timeframe: &str) -> Self {
        Self {
            connection_state: QString::from("connecting"),
            last_error: QString::from(""),
            data_age_ms: -1,
            applied: 0,
            dropped: 0,
            gaps_closed: 0,
            resnapshots: 0,
            series: q_qt::BarSeriesRust::new(symbol, timeframe, 500_000),
            last_applied_instant: None,
        }
    }

    pub fn set_connection_state(&mut self, state: &str) {
        self.connection_state = QString::from(state);
    }

    pub fn set_last_error(&mut self, error: &str) {
        self.last_error = QString::from(error);
    }

    pub fn update_counters(&mut self, dropped: u64, gaps_closed: u64, resnapshots: u64) {
        self.dropped = dropped as i64;
        self.gaps_closed = gaps_closed as i64;
        self.resnapshots = resnapshots as i64;
    }

    pub fn update_data_age(&mut self) {
        if let Some(inst) = self.last_applied_instant {
            self.data_age_ms = inst.elapsed().as_millis() as i64;
        } else {
            self.data_age_ms = -1;
        }
    }

    pub fn apply_delivery(&mut self, delivery: BarDelivery) {
        match delivery {
            BarDelivery::Completed(bars) => {
                let count = bars.time.len() as i64;
                if self.series.ingest_completed(bars).is_ok() {
                    self.applied += count;
                    self.last_applied_instant = Some(Instant::now());
                    self.data_age_ms = 0;
                }
            }
            BarDelivery::Forming(bars) => {
                if self.series.ingest_forming(bars).is_ok() {
                    self.applied += 1;
                    self.last_applied_instant = Some(Instant::now());
                    self.data_age_ms = 0;
                }
            }
        }
    }

    pub fn apply_deliveries(&mut self, deliveries: impl IntoIterator<Item = BarDelivery>) {
        for delivery in deliveries {
            self.apply_delivery(delivery);
        }
    }
}

impl Default for BarFeedRust {
    fn default() -> Self {
        Self::new("DEFAULT", "1m")
    }
}

pub fn make_bar_columns(t: i64, open: f64, high: f64, low: f64, close: f64) -> BarColumns {
    BarColumns {
        time: vec![t],
        open: vec![open],
        high: vec![high],
        low: vec![low],
        close: vec![close],
        tick_volume: None,
        spread: None,
        real_volume: None,
        label: TimeLabel::Utc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bar(t: i64, close: f64) -> BarColumns {
        make_bar_columns(t, 10.0, close + 1.0, 9.0, close)
    }

    #[test]
    fn test_bar_feed_initial_state() {
        let feed = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed.connection_state.to_string(), "connecting");
        assert_eq!(feed.last_error.to_string(), "");
        assert_eq!(feed.data_age_ms, -1);
        assert_eq!(feed.applied, 0);
        assert_eq!(feed.dropped, 0);
        assert_eq!(feed.gaps_closed, 0);
        assert_eq!(feed.resnapshots, 0);
        assert_eq!(feed.series.bar_count, 0);
        assert_eq!(feed.series.revision, 0);
        assert_eq!(feed.series.symbol.to_string(), "PETR4");
        assert_eq!(feed.series.timeframe.to_string(), "1m");
    }

    #[test]
    fn test_bar_feed_drain_completed_advances_series_and_counters() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        let deliveries: Vec<_> = (1..=5)
            .map(|i| BarDelivery::Completed(test_bar(i * 60, 10.0 + i as f64)))
            .collect();

        feed.apply_deliveries(deliveries);

        assert_eq!(feed.applied, 5);
        assert_eq!(feed.series.bar_count, 5);
        assert_eq!(feed.series.revision, 5);
        assert_eq!(feed.series.last_time, 300);
        assert_eq!(feed.series.last_price.to_bits(), 15.0f64.to_bits());
        assert_eq!(feed.data_age_ms, 0);
    }

    #[test]
    fn test_bar_feed_drain_forming_advances_series() {
        let mut feed = BarFeedRust::new("PETR4", "1m");

        feed.apply_delivery(BarDelivery::Completed(test_bar(60, 10.0)));
        assert_eq!(feed.series.bar_count, 1);
        assert_eq!(feed.series.revision, 1);
        assert!(!feed.series.has_forming);

        feed.apply_delivery(BarDelivery::Forming(test_bar(120, 10.5)));
        assert_eq!(feed.applied, 2);
        assert_eq!(feed.series.bar_count, 1); // Completed count doesn't change
        assert!(feed.series.has_forming);
        assert_eq!(feed.series.revision, 2);
        assert_eq!(feed.series.last_price.to_bits(), 10.5f64.to_bits());
    }

    #[test]
    fn test_bar_feed_forming_coalescing_and_completed_in_order() {
        let mut feed = BarFeedRust::new("PETR4", "1m");

        let deliveries = vec![
            BarDelivery::Completed(test_bar(60, 10.0)),
            BarDelivery::Forming(test_bar(120, 19.99)),
        ];

        feed.apply_deliveries(deliveries);

        assert_eq!(feed.applied, 2);
        assert_eq!(feed.series.bar_count, 1);
        assert!(feed.series.has_forming);
        assert_eq!(feed.series.revision, 2);
        assert_eq!(feed.series.last_price.to_bits(), 19.99f64.to_bits());
    }

    #[test]
    fn test_bar_feed_setters_and_counters() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        feed.set_connection_state("live");
        assert_eq!(feed.connection_state.to_string(), "live");

        feed.set_last_error("stream_unavailable");
        assert_eq!(feed.last_error.to_string(), "stream_unavailable");

        feed.update_counters(7, 3, 2);
        assert_eq!(feed.dropped, 7);
        assert_eq!(feed.gaps_closed, 3);
        assert_eq!(feed.resnapshots, 2);
    }

    #[test]
    fn test_bar_feed_update_data_age() {
        let mut feed = BarFeedRust::new("PETR4", "1m");
        assert_eq!(feed.data_age_ms, -1);
        feed.update_data_age();
        assert_eq!(feed.data_age_ms, -1);

        feed.apply_delivery(BarDelivery::Completed(test_bar(60, 10.0)));
        assert_eq!(feed.data_age_ms, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        feed.update_data_age();
        assert!(feed.data_age_ms >= 4);
    }
}
