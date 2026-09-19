//! The execution half of the stream client: six topic state machines, one shared snapshot.
//!
//! Every execution topic follows §4.2 independently, but they all re-read the single
//! execution snapshot, which is applied to all six with each topic's own watermark.

use crate::contracts_stream::ExecutionSnapshot;
use crate::execution::store::ExecutionHandle;
use crate::stream::client::ClientShared;
use crate::stream::execution::{
    decode_execution_entry, snapshot_watermark, ExecPayload, ExecutionEvent,
};
use crate::stream::policy::EXECUTION_TOPICS;
use crate::stream::topic_state::{on_event, Action, Event, Phase, TopicState};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

/// A re-snapshot that keeps asking for another one is given up on and retried with backoff.
const MAX_SNAPSHOT_ROUNDS: usize = 5;

pub struct Net<'a> {
    pub http: &'a reqwest::Client,
    pub api_base: &'a str,
    pub shared: &'a ClientShared,
    pub is_shutdown: &'a AtomicBool,
}

type ExecAction = Action<ExecPayload>;

pub struct ExecSession {
    states: HashMap<String, TopicState<ExecPayload>>,
    handle: ExecutionHandle,
    retry_attempt: u32,
    retry_at: Option<Instant>,
}

fn is_execution_topic(topic: &str) -> bool {
    EXECUTION_TOPICS.contains(&topic)
}

impl ExecSession {
    pub fn new(handle: ExecutionHandle) -> Self {
        let states = EXECUTION_TOPICS
            .iter()
            .map(|t| (t.to_string(), TopicState::unfiltered(t)))
            .collect();
        Self {
            states,
            handle,
            retry_attempt: 0,
            retry_at: None,
        }
    }

    pub fn handles(topic: &str) -> bool {
        is_execution_topic(topic)
    }

    /// When a failed snapshot should be retried, if one is pending.
    pub fn retry_at(&self) -> Option<Instant> {
        self.retry_at
    }

    /// The stream was lost: keep the last state and mark it unconfirmed (§8.1).
    pub fn lost(&self) {
        self.handle
            .mutate(|s| s.mark_unconfirmed(SystemTime::now()));
    }

    /// Handles the `subscribed` ack, then reads the snapshot once for all six topics.
    pub async fn on_subscribed(
        &mut self,
        net: &Net<'_>,
        topics: &serde_json::Map<String, serde_json::Value>,
    ) {
        let mut any = false;
        for topic in EXECUTION_TOPICS {
            let Some(t) = topics.get(topic) else { continue };
            let Some(state) = self.states.get_mut(topic) else {
                continue;
            };
            let epoch = t
                .get("epoch")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let last_seq = t.get("last_seq").and_then(|v| v.as_i64()).unwrap_or(0);
            on_event(state, Event::Subscribed { epoch, last_seq });
            any = true;
        }
        if any {
            self.request_snapshot(net, false).await;
        }
    }

    pub async fn on_envelope(
        &mut self,
        net: &Net<'_>,
        topic: &str,
        epoch: &str,
        seq: i64,
        payload: &serde_json::Value,
    ) {
        let event = match decode_execution_entry(topic, payload) {
            Ok(e) => e,
            Err(e) => {
                // An event we cannot apply exactly is replaced by the state that includes it.
                net.shared.inc_dropped();
                net.shared.set_last_error(&e.to_string());
                self.request_snapshot(net, true).await;
                return;
            }
        };
        let Some(state) = self.states.get_mut(topic) else {
            net.shared.inc_dropped();
            return;
        };
        let actions = on_event(
            state,
            Event::Entry {
                epoch: epoch.to_string(),
                seq,
                symbol: String::new(),
                timeframe: String::new(),
                payload: ExecPayload::Event(event),
            },
        );
        self.drive(net, topic, actions).await;
    }

    pub async fn on_lagging(&mut self, net: &Net<'_>, topic: &str, from_seq: i64) {
        if let Some(state) = self.states.get_mut(topic) {
            let actions = on_event(state, Event::Lagging { from_seq });
            self.drive(net, topic, actions).await;
        }
    }

    pub async fn on_cursor_expired(&mut self, net: &Net<'_>, topic: &str) {
        if let Some(state) = self.states.get_mut(topic) {
            let actions = on_event(state, Event::CursorExpired);
            self.drive(net, topic, actions).await;
        }
    }

    pub async fn on_epoch_changed(&mut self, net: &Net<'_>, topic: &str, new_epoch: &str) {
        let Some(state) = self.states.get_mut(topic) else {
            return;
        };
        // A restart announces the new epoch on every topic. The first re-snapshot already
        // moved all six to it, so the rest cost nothing.
        if state.epoch.as_deref() == Some(new_epoch) && state.phase == Phase::Live {
            return;
        }
        let actions = on_event(
            state,
            Event::EpochChanged {
                new_epoch: new_epoch.to_string(),
            },
        );
        self.drive(net, topic, actions).await;
    }

    /// Retries a failed snapshot once its backoff has elapsed.
    pub async fn retry(&mut self, net: &Net<'_>) {
        self.retry_at = None;
        self.request_snapshot(net, true).await;
    }

    async fn drive(&mut self, net: &Net<'_>, topic: &str, actions: Vec<ExecAction>) {
        if self.process(net, topic, actions).await {
            self.request_snapshot(net, true).await;
        }
    }

    /// Reads the execution snapshot and applies it to all six topics, repeating while a
    /// topic still asks for one. A failure leaves the store as it was, unconfirmed.
    async fn request_snapshot(&mut self, net: &Net<'_>, resnapshot: bool) {
        // A failed snapshot already has a retry on the clock; events that ask for another
        // one meanwhile wait for it instead of hammering the API and inflating the backoff.
        if self.retry_at.is_some() {
            return;
        }
        let mut resnapshot = resnapshot;
        for _ in 0..MAX_SNAPSHOT_ROUNDS {
            match self.snapshot_once(net, resnapshot).await {
                Some(false) => return,
                Some(true) => resnapshot = true,
                None => return,
            }
        }
        self.fail(net, "execution snapshot did not settle");
    }

    fn fail(&mut self, net: &Net<'_>, reason: &str) {
        self.handle
            .mutate(|s| s.mark_unconfirmed(SystemTime::now()));
        net.shared.set_last_error(reason);
        self.retry_attempt += 1;
        let backoff_ms = (50u64 * (1u64 << (self.retry_attempt - 1).min(10))).min(30_000);
        self.retry_at = Some(Instant::now() + Duration::from_millis(backoff_ms));
    }

    /// `None`: the read failed and a retry is scheduled. `Some(more)`: applied, and
    /// `more` says a topic asked for another snapshot while its buffer was released.
    async fn snapshot_once(&mut self, net: &Net<'_>, resnapshot: bool) -> Option<bool> {
        if net.is_shutdown.load(Ordering::SeqCst) {
            return None;
        }
        let url = format!("{}/api/v1/stream/execution/snapshot", net.api_base);
        net.shared.inc_rest_calls();
        if resnapshot {
            net.shared.inc_resnapshots();
        }
        let resp = match net.http.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                self.fail(net, &e.to_string());
                return None;
            }
        };
        if !resp.status().is_success() {
            let reason = if resp.status().as_u16() == 503 {
                "execution_snapshot_unavailable".to_string()
            } else {
                format!("execution snapshot status {}", resp.status())
            };
            self.fail(net, &reason);
            return None;
        }
        let snap: ExecutionSnapshot = match resp.json().await {
            Ok(s) => s,
            Err(e) => {
                self.fail(net, &format!("execution snapshot: {e}"));
                return None;
            }
        };
        let mut marks = Vec::new();
        for topic in EXECUTION_TOPICS {
            match snapshot_watermark(&snap, topic) {
                Ok(m) => marks.push((topic, m)),
                Err(e) => {
                    self.fail(net, &e.to_string());
                    return None;
                }
            }
        }
        self.retry_attempt = 0;
        self.retry_at = None;
        let snap = Arc::new(snap);
        let mut more = false;
        for (topic, (epoch, seq)) in marks {
            let Some(state) = self.states.get_mut(topic) else {
                continue;
            };
            let actions = on_event(
                state,
                Event::Snapshot {
                    epoch,
                    seq,
                    payload: ExecPayload::Snapshot(snap.clone()),
                },
            );
            more |= self.process(net, topic, actions).await;
        }
        Some(more)
    }

    /// Runs one topic's actions. Returns true if a re-snapshot is wanted.
    async fn process(&mut self, net: &Net<'_>, topic: &str, actions: Vec<ExecAction>) -> bool {
        let mut queue: VecDeque<ExecAction> = actions.into();
        let mut need_snapshot = false;
        while let Some(action) = queue.pop_front() {
            if net.is_shutdown.load(Ordering::SeqCst) {
                return false;
            }
            match action {
                Action::Apply(payload) => self.apply(topic, payload),
                Action::Drop(_) => net.shared.inc_dropped(),
                Action::ReSnapshot => need_snapshot = true,
                Action::FetchHistory { from_seq } => {
                    match self.fetch_history(net, topic, from_seq).await {
                        Some(more) => queue.extend(more),
                        None => need_snapshot = true,
                    }
                }
            }
        }
        self.sync_watermark(topic);
        need_snapshot
    }

    fn apply(&self, topic: &str, payload: ExecPayload) {
        match payload {
            ExecPayload::Snapshot(snap) => {
                self.handle.mutate(|s| s.apply_snapshot(&snap, &[topic]));
            }
            ExecPayload::Event(event) => {
                self.handle.mutate(|s| s.apply(event));
            }
        }
    }

    fn sync_watermark(&self, topic: &str) {
        if let Some(state) = self.states.get(topic) {
            if let (Some(epoch), Some(seq)) = (&state.epoch, state.last_applied_seq) {
                self.handle.mutate(|s| s.set_watermark(topic, epoch, seq));
            }
        }
    }

    /// Fills a gap from history by sequence, page by page. `None` means the history is
    /// gone (or useless) and the topic has asked for a re-snapshot.
    async fn fetch_history(
        &mut self,
        net: &Net<'_>,
        topic: &str,
        from_seq: i64,
    ) -> Option<Vec<ExecAction>> {
        let mut out = Vec::new();
        let mut from = from_seq;
        loop {
            let state = self.states.get_mut(topic)?;
            let epoch = state.epoch.clone()?;
            let url = format!(
                "{}/api/v1/stream/{topic}/history?epoch={epoch}&from_seq={from}&limit=500",
                net.api_base
            );
            net.shared.inc_rest_calls();
            let page = match net.http.get(&url).send().await {
                Ok(r) if r.status().is_success() => r
                    .json::<crate::stream::client::HistoryPageResponse>()
                    .await
                    .ok(),
                _ => None,
            };
            let decoded = page.and_then(|p| {
                let entries: Option<Vec<(i64, ExecPayload)>> = p
                    .entries
                    .iter()
                    .map(|e| {
                        decode_execution_entry(topic, &e.payload)
                            .ok()
                            .map(|ev: ExecutionEvent| (e.seq, ExecPayload::Event(ev)))
                    })
                    .collect();
                entries.map(|entries| (p.epoch, entries, p.next_seq))
            });
            let state = self.states.get_mut(topic)?;
            let Some((page_epoch, entries, next_seq)) = decoded else {
                on_event(state, Event::HistoryExpired);
                return None;
            };
            let before = state.last_applied_seq;
            out.extend(on_event(
                state,
                Event::HistoryPage {
                    epoch: page_epoch,
                    entries,
                    next_seq,
                },
            ));
            net.shared.inc_gaps_closed();
            if !matches!(state.phase, Phase::AwaitingHistory { .. }) {
                return Some(out);
            }
            // Still a gap: another page is needed, but only if this one made progress.
            if state.last_applied_seq == before {
                on_event(state, Event::HistoryExpired);
                return None;
            }
            from = state.last_applied_seq.unwrap_or(from) + 1;
        }
    }
}
