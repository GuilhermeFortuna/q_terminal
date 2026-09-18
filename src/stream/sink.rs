use crate::stream::topic_state::BarColumns;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum BarDelivery {
    Completed(BarColumns),
    Forming(BarColumns),
}

struct BarSinkInner {
    completed: VecDeque<BarColumns>,
    forming: Option<BarColumns>,
}

#[derive(Clone)]
pub struct BarSink {
    inner: Arc<Mutex<BarSinkInner>>,
}

impl Default for BarSink {
    fn default() -> Self {
        Self::new()
    }
}

impl BarSink {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BarSinkInner {
                completed: VecDeque::new(),
                forming: None,
            })),
        }
    }

    pub fn deliver_completed(&self, bars: BarColumns) {
        let mut inner = self.inner.lock().unwrap();
        inner.completed.push_back(bars);
    }

    pub fn deliver_forming(&self, bars: BarColumns) {
        let mut inner = self.inner.lock().unwrap();
        inner.forming = Some(bars);
    }

    pub fn drain(&self) -> Vec<BarDelivery> {
        let mut inner = self.inner.lock().unwrap();
        let mut result =
            Vec::with_capacity(inner.completed.len() + if inner.forming.is_some() { 1 } else { 0 });
        while let Some(c) = inner.completed.pop_front() {
            result.push(BarDelivery::Completed(c));
        }
        if let Some(f) = inner.forming.take() {
            result.push(BarDelivery::Forming(f));
        }
        result
    }

    pub fn drain_into(&self, feed: &mut crate::bridge::bar_feed::BarFeedRust) {
        for delivery in self.drain() {
            feed.apply_delivery(delivery.into());
        }
    }
}

impl From<crate::stream::topic_state::BarColumns> for q_buffers::frame::BarColumns {
    fn from(c: crate::stream::topic_state::BarColumns) -> Self {
        Self {
            time: c.time,
            open: c.open,
            high: c.high,
            low: c.low,
            close: c.close,
            tick_volume: None,
            spread: None,
            real_volume: None,
            label: q_buffers::frame::TimeLabel::Utc,
        }
    }
}

impl From<BarDelivery> for crate::bridge::bar_feed::BarDelivery {
    fn from(d: BarDelivery) -> Self {
        match d {
            BarDelivery::Completed(b) => Self::Completed(b.into()),
            BarDelivery::Forming(b) => Self::Forming(b.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_bar(t: i64, close: f64) -> BarColumns {
        BarColumns::single(t, 10.0, 11.0, 9.0, close, 100.0)
    }

    #[test]
    fn test_thousand_forming_deliveries_leave_one_bar_the_newest() {
        let sink = BarSink::new();

        for i in 0..1000 {
            sink.deliver_forming(dummy_bar(100, i as f64));
        }

        let deliveries = sink.drain();
        assert_eq!(
            deliveries.len(),
            1,
            "1,000 forming deliveries between two drains must leave exactly one bar"
        );
        match &deliveries[0] {
            BarDelivery::Forming(bars) => {
                assert_eq!(
                    bars.close[0].to_bits(),
                    999.0f64.to_bits(),
                    "must be the newest bar"
                );
            }
            BarDelivery::Completed(_) => panic!("expected Forming delivery"),
        }

        // Second drain is empty
        assert!(sink.drain().is_empty());
    }

    #[test]
    fn test_thousand_completed_deliveries_leave_thousand_in_order() {
        let sink = BarSink::new();

        for i in 0..1000 {
            sink.deliver_completed(dummy_bar(i, i as f64));
        }

        let deliveries = sink.drain();
        assert_eq!(
            deliveries.len(),
            1000,
            "1,000 completed deliveries must leave 1,000 in order"
        );
        for (i, delivery) in deliveries.iter().enumerate() {
            match delivery {
                BarDelivery::Completed(bars) => {
                    assert_eq!(bars.time[0], i as i64);
                    assert_eq!(bars.close[0].to_bits(), (i as f64).to_bits());
                }
                BarDelivery::Forming(_) => panic!("expected Completed delivery"),
            }
        }

        // Second drain is empty
        assert!(sink.drain().is_empty());
    }

    #[test]
    fn test_completed_bar_and_superseding_forming_bar_drain_in_order() {
        let sink = BarSink::new();

        let completed = dummy_bar(100, 10.0);
        let forming = dummy_bar(101, 10.5);

        sink.deliver_completed(completed.clone());
        sink.deliver_forming(forming.clone());

        let deliveries = sink.drain();
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0], BarDelivery::Completed(completed));
        assert_eq!(deliveries[1], BarDelivery::Forming(forming));

        // Second drain is empty
        assert!(sink.drain().is_empty());
    }

    #[test]
    fn test_drain_into_bar_feed() {
        let sink = BarSink::new();
        let mut feed = crate::bridge::bar_feed::BarFeedRust::new("PETR4", "1m");
        feed.history_controller().open_gate();

        sink.deliver_completed(dummy_bar(60, 10.0));
        sink.deliver_forming(dummy_bar(120, 10.5));

        sink.drain_into(&mut feed);

        assert_eq!(feed.applied, 2);
        assert_eq!(feed.series.bar_count, 1);
        assert!(feed.series.has_forming);
        assert_eq!(feed.series.revision, 2);
        assert_eq!(feed.series.last_price.to_bits(), 10.5f64.to_bits());
        assert_eq!(feed.data_age_ms, 0);
    }
}
