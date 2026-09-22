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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartMode {
    Following,
    Manual,
}

impl ChartMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Following => "following",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartSelection {
    pub mode: ChartMode,
    pub requested_symbol: String,
    pub requested_timeframe: String,
    pub committed_symbol: String,
    pub committed_timeframe: String,
}

pub fn is_valid_timeframe(timeframe: &str) -> bool {
    let tf = timeframe.trim();
    if tf.is_empty() {
        return false;
    }
    let first = tf.chars().next().unwrap();
    if first.is_ascii_alphabetic() && tf.len() > 1 && tf[1..].chars().all(|c| c.is_ascii_digit()) {
        let code = first.to_ascii_uppercase();
        if matches!(code, 'S' | 'M' | 'H' | 'D' | 'W') {
            return tf[1..].parse::<i64>().map(|n| n > 0).unwrap_or(false);
        }
    }
    let last = tf.chars().last().unwrap();
    if last.is_ascii_alphabetic()
        && tf.len() > 1
        && tf[..tf.len() - 1].chars().all(|c| c.is_ascii_digit())
    {
        let code = last.to_ascii_uppercase();
        if matches!(code, 'S' | 'M' | 'H' | 'D' | 'W') {
            return tf[..tf.len() - 1]
                .parse::<i64>()
                .map(|n| n > 0)
                .unwrap_or(false);
        }
    }
    false
}

pub fn is_valid_symbol(symbol: &str) -> bool {
    let sym = symbol.trim();
    if sym.is_empty() {
        return false;
    }
    sym.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '$' | '_'))
}

pub fn validate_target(symbol: &str, timeframe: &str) -> Result<(String, String), String> {
    let sym = symbol.trim();
    let tf = timeframe.trim();
    if sym.is_empty() {
        return Err(format!(
            "Invalid chart target '{symbol} · {timeframe}': symbol cannot be empty"
        ));
    }
    if !is_valid_symbol(sym) {
        return Err(format!(
            "Invalid chart target '{symbol} · {timeframe}': invalid symbol characters"
        ));
    }
    if tf.is_empty() {
        return Err(format!(
            "Invalid chart target '{symbol} · {timeframe}': timeframe cannot be empty"
        ));
    }
    if !is_valid_timeframe(tf) {
        return Err(format!(
            "Invalid chart target '{symbol} · {timeframe}': unsupported timeframe format"
        ));
    }
    Ok((sym.to_uppercase(), tf.to_string()))
}

pub struct ChartTargeter {
    feed: usize,
    fetcher: Arc<OverlayFetcher>,
    fallback: (String, String),
    mode: Mutex<ChartMode>,
    following_target: Mutex<Option<(String, String, String, String)>>,
    requested: Mutex<(String, String)>,
    committed: Mutex<(String, String)>,
    generation: AtomicU64,
    retargeter: Retargeter,
    recent_symbols: Mutex<Vec<String>>,
    on_restore_rows: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl ChartTargeter {
    /// `fallback` is the configured symbol and timeframe, shown with no deployment selected.
    pub fn new(
        feed: *mut chart_bridge::BarFeed,
        fallback: (String, String),
        retargeter: Retargeter,
        fetcher: Arc<OverlayFetcher>,
    ) -> Arc<Self> {
        let recents = vec![fallback.0.clone()];
        Arc::new(Self {
            feed: feed as usize,
            fetcher,
            mode: Mutex::new(ChartMode::Following),
            following_target: Mutex::new(None),
            requested: Mutex::new(fallback.clone()),
            committed: Mutex::new(fallback.clone()),
            fallback,
            generation: AtomicU64::new(0),
            retargeter,
            recent_symbols: Mutex::new(recents),
            on_restore_rows: Mutex::new(None),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn mode(&self) -> ChartMode {
        *self.mode.lock().unwrap()
    }

    pub fn is_manual(&self) -> bool {
        self.mode() == ChartMode::Manual
    }

    pub fn is_following(&self) -> bool {
        self.mode() == ChartMode::Following
    }

    pub fn selection(&self) -> ChartSelection {
        let mode = self.mode();
        let (req_sym, req_tf) = self.requested.lock().unwrap().clone();
        let (com_sym, com_tf) = self.committed.lock().unwrap().clone();
        ChartSelection {
            mode,
            requested_symbol: req_sym,
            requested_timeframe: req_tf,
            committed_symbol: com_sym,
            committed_timeframe: com_tf,
        }
    }

    pub fn following_target(&self) -> Option<(String, String, String, String)> {
        self.following_target.lock().unwrap().clone()
    }

    pub fn recent_symbols(&self) -> Vec<String> {
        self.recent_symbols.lock().unwrap().clone()
    }

    pub fn add_recent_symbol(&self, symbol: &str) {
        let sym = symbol.trim().to_uppercase();
        if sym.is_empty() {
            return;
        }
        let mut recents = self.recent_symbols.lock().unwrap();
        recents.retain(|s| s != &sym);
        recents.insert(0, sym);
        if recents.len() > 10 {
            recents.truncate(10);
        }
    }

    pub fn set_on_restore_rows(&self, cb: Arc<dyn Fn() + Send + Sync>) {
        *self.on_restore_rows.lock().unwrap() = Some(cb);
    }

    /// Requests a manual chart target. Switches to Manual mode, clears overlays/markers,
    /// and retargets if the committed pair changes.
    pub fn request_manual(
        &self,
        symbol: &str,
        timeframe: &str,
    ) -> Result<Option<ChartTarget>, String> {
        let (sym, tf) = validate_target(symbol, timeframe)?;
        *self.mode.lock().unwrap() = ChartMode::Manual;
        *self.requested.lock().unwrap() = (sym.clone(), tf.clone());

        // Manual mode clears deployment overlays and markers
        self.fetcher.set_deployment(None);
        let feed = self.feed as *mut chart_bridge::BarFeed;
        unsafe {
            chart_bridge::feed_set_execution_rows(feed, "[]", "[]");
        }

        let mut committed = self.committed.lock().unwrap();
        if *committed == (sym.clone(), tf.clone()) {
            return Ok(None);
        }
        *committed = (sym.clone(), tf.clone());
        drop(committed);

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        unsafe {
            chart_bridge::feed_retarget(feed, &sym, &tf, generation as i64);
        }
        self.retargeter.retarget(&sym, &tf, generation);
        Ok(Some(ChartTarget {
            symbol: sym,
            timeframe: tf,
            generation,
        }))
    }

    /// Switches back to Following mode, targeting the current global deployment selection
    /// or falling back to the configured pair.
    pub fn follow_deployment(&self) -> Option<ChartTarget> {
        *self.mode.lock().unwrap() = ChartMode::Following;
        let dep = self.following_target.lock().unwrap().clone();
        let (id, _name, symbol, timeframe) = match dep {
            Some((id, name, s, t)) => (Some(id), name, s, t),
            None => (
                None,
                String::new(),
                self.fallback.0.clone(),
                self.fallback.1.clone(),
            ),
        };
        *self.requested.lock().unwrap() = (symbol.clone(), timeframe.clone());

        self.fetcher.set_deployment(id.clone());

        let mut committed = self.committed.lock().unwrap();
        let retargeted = if *committed == (symbol.clone(), timeframe.clone()) {
            false
        } else {
            *committed = (symbol.clone(), timeframe.clone());
            true
        };
        drop(committed);

        if !retargeted {
            if let Some(ref cb) = *self.on_restore_rows.lock().unwrap() {
                cb();
            }
            if id.is_some() {
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
        if let Some(ref cb) = *self.on_restore_rows.lock().unwrap() {
            cb();
        }
        if id.is_some() {
            self.fetcher.request(generation);
        }
        Some(ChartTarget {
            symbol,
            timeframe,
            generation,
        })
    }

    /// Follows `deployment` (id, symbol, timeframe), or the configured symbol when `None`:
    /// retargets the feed and the stream when the symbol or timeframe changed, and fetches
    /// the deployment's overlays when the deployment changed.
    ///
    /// When in Manual mode, stores the deployment but does NOT retarget the chart.
    pub fn select(
        &self,
        deployment: Option<(String, String, String, String)>,
    ) -> Option<ChartTarget> {
        *self.following_target.lock().unwrap() = deployment.clone();
        let mode = *self.mode.lock().unwrap();
        if mode == ChartMode::Manual {
            return None;
        }

        let (id, _name, symbol, timeframe) = match deployment {
            Some((id, name, s, t)) => (Some(id), name, s, t),
            None => (
                None,
                String::new(),
                self.fallback.0.clone(),
                self.fallback.1.clone(),
            ),
        };
        *self.requested.lock().unwrap() = (symbol.clone(), timeframe.clone());

        let deployment_changed = {
            let mut dep = self.fetcher.deployment.lock().unwrap();
            let changed = *dep != id;
            *dep = id.clone();
            changed
        };
        let retargeted = {
            let mut committed = self.committed.lock().unwrap();
            if *committed == (symbol.clone(), timeframe.clone()) {
                None
            } else {
                *committed = (symbol.clone(), timeframe.clone());
                Some(())
            }
        };
        if retargeted.is_none() {
            if deployment_changed && id.is_some() {
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
        if id.is_some() {
            self.fetcher.request(generation);
        }
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
