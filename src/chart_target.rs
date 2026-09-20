//! Makes the chart follow the selected deployment (Q-049).
//!
//! `select` is called on the Qt thread with the selected deployment's symbol and timeframe
//! (or `None`). When the target changes it bumps a generation, retargets the feed (which
//! drops the previous target's bars and history load) and then the stream client's bar
//! filters. Bars and history results carry the generation they were produced for, so
//! nothing of the previous target reaches the new chart, even if it was in flight.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::chart_bridge;
use crate::stream::client::Retargeter;
use crate::stream::sink::{BarDelivery, BarSink};

pub struct ChartTarget {
    pub symbol: String,
    pub timeframe: String,
    pub generation: u64,
}

pub struct ChartTargeter {
    feed: usize,
    fallback: (String, String),
    current: Mutex<(String, String)>,
    generation: AtomicU64,
    retargeter: Retargeter,
}

impl ChartTargeter {
    /// `fallback` is the configured symbol and timeframe, shown with no deployment selected.
    pub fn new(
        feed: *mut chart_bridge::BarFeed,
        fallback: (String, String),
        retargeter: Retargeter,
    ) -> Arc<Self> {
        Arc::new(Self {
            feed: feed as usize,
            current: Mutex::new(fallback.clone()),
            fallback,
            generation: AtomicU64::new(0),
            retargeter,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Retargets to `deployment`'s symbol and timeframe, or to the fallback when `None`.
    /// Returns the new target when it changed.
    pub fn select(&self, deployment: Option<(String, String)>) -> Option<ChartTarget> {
        let (symbol, timeframe) = deployment.unwrap_or_else(|| self.fallback.clone());
        {
            let mut cur = self.current.lock().unwrap();
            if *cur == (symbol.clone(), timeframe.clone()) {
                return None;
            }
            *cur = (symbol.clone(), timeframe.clone());
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let feed = self.feed as *mut chart_bridge::BarFeed;
        unsafe {
            chart_bridge::feed_retarget(feed, &symbol, &timeframe, generation as i64);
        }
        self.retargeter.retarget(&symbol, &timeframe, generation);
        Some(ChartTarget {
            symbol,
            timeframe,
            generation,
        })
    }
}

/// Installs the sink listener that posts stamped bars to the feed on the Qt thread.
pub fn install_bar_drainer(sink: &BarSink, feed: *mut chart_bridge::BarFeed) {
    let feed_addr = feed as usize;
    let drain_sink = sink.clone();
    sink.set_listener(Arc::new(move || {
        let feed = feed_addr as *mut chart_bridge::BarFeed;
        let (generation, deliveries) = drain_sink.drain_stamped();
        let generation = generation as i64;
        for delivery in deliveries {
            match delivery {
                BarDelivery::Completed(cols) => {
                    for i in 0..cols.time.len() {
                        unsafe {
                            chart_bridge::post_feed_completed_bar(
                                feed,
                                generation,
                                cols.time[i],
                                cols.open[i],
                                cols.high[i],
                                cols.low[i],
                                cols.close[i],
                            );
                        }
                    }
                }
                BarDelivery::Forming(cols) => {
                    if let Some(&t) = cols.time.first() {
                        unsafe {
                            chart_bridge::post_feed_forming_bar(
                                feed,
                                generation,
                                t,
                                cols.open[0],
                                cols.high[0],
                                cols.low[0],
                                cols.close[0],
                            );
                        }
                    }
                }
            }
        }
    }));
}
