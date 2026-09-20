//! Makes the chart follow the selected deployment (Q-049).
//!
//! `select` is called on the Qt thread with the selected deployment's symbol and timeframe
//! (or `None`). When the target changes it bumps a generation, retargets the feed (which
//! drops the previous target's bars and history load) and then the stream client's bar
//! filters. Bars and history results carry the generation they were produced for, so
//! nothing of the previous target reaches the new chart, even if it was in flight.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::chart_bridge;
use crate::stream::client::Retargeter;
use crate::stream::sink::{BarDelivery, BarSink};

pub struct ChartTarget {
    pub symbol: String,
    pub timeframe: String,
    pub generation: u64,
}

/// Fetches the selected deployment's indicator overlays from the backend's chart route:
/// once on selection and once per completed bar, never on a timer.
pub struct OverlayFetcher {
    api_base: String,
    feed: usize,
    deployment: Mutex<Option<String>>,
    requests: AtomicUsize,
}

impl OverlayFetcher {
    pub fn new(api_base: &str, feed: *mut chart_bridge::BarFeed) -> Arc<Self> {
        Arc::new(Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            feed: feed as usize,
            deployment: Mutex::new(None),
            requests: AtomicUsize::new(0),
        })
    }

    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    pub fn set_deployment(&self, id: Option<String>) {
        *self.deployment.lock().unwrap() = id;
    }

    /// Requests overlays for the selected deployment on behalf of target `generation`.
    /// The result is posted to the feed, which drops it if the target has moved on.
    pub fn request(self: &Arc<Self>, generation: u64) {
        let Some(id) = self.deployment.lock().unwrap().clone() else {
            return;
        };
        self.requests.fetch_add(1, Ordering::SeqCst);
        let this = self.clone();
        let _ = std::thread::Builder::new()
            .name("overlay-fetch".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                let url = format!("{}/api/v1/execution/deployments/{id}/chart", this.api_base);
                let body = rt.block_on(async {
                    let resp = reqwest::get(&url).await.ok()?.error_for_status().ok()?;
                    resp.text().await.ok()
                });
                if let Some(body) = body {
                    let feed = this.feed as *mut chart_bridge::BarFeed;
                    unsafe { chart_bridge::post_feed_overlays(feed, generation as i64, &body) };
                }
            });
    }
}

pub struct ChartTargeter {
    feed: usize,
    fetcher: Arc<OverlayFetcher>,
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
        fetcher: Arc<OverlayFetcher>,
    ) -> Arc<Self> {
        Arc::new(Self {
            feed: feed as usize,
            fetcher,
            current: Mutex::new(fallback.clone()),
            fallback,
            generation: AtomicU64::new(0),
            retargeter,
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Follows `deployment` (id, symbol, timeframe), or the configured symbol when `None`:
    /// retargets the feed and the stream when the symbol or timeframe changed, and fetches
    /// the deployment's overlays when the deployment changed. Returns the new target when
    /// it changed.
    pub fn select(&self, deployment: Option<(String, String, String)>) -> Option<ChartTarget> {
        let (id, symbol, timeframe) = match deployment {
            Some((id, s, t)) => (Some(id), s, t),
            None => (None, self.fallback.0.clone(), self.fallback.1.clone()),
        };
        let deployment_changed = {
            let mut dep = self.fetcher.deployment.lock().unwrap();
            let changed = *dep != id;
            *dep = id;
            changed
        };
        let retargeted = {
            let mut cur = self.current.lock().unwrap();
            if *cur == (symbol.clone(), timeframe.clone()) {
                None
            } else {
                *cur = (symbol.clone(), timeframe.clone());
                Some(())
            }
        };
        if retargeted.is_none() {
            if deployment_changed {
                self.fetcher.request(self.generation());
            }
            return None;
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let feed = self.feed as *mut chart_bridge::BarFeed;
        unsafe {
            chart_bridge::feed_retarget(feed, &symbol, &timeframe, generation as i64);
        }
        self.retargeter.retarget(&symbol, &timeframe, generation);
        self.fetcher.request(generation);
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
