use crate::config::Config;
use crate::execution::store::ExecutionHandle;
use crate::stream::arrow_decode::decode_arrow_bars;
use crate::stream::base64::b64_decode;
use crate::stream::exec_session::{ExecSession, Net};
use crate::stream::frame::{classify_text, split_binary, ControlFrame, ServerFrame};
use crate::stream::policy::EXECUTION_TOPICS;
use crate::stream::sink::BarSink;
use crate::stream::topic_state::{on_event, Action, Event, TopicState};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Live,
    Reconnecting { attempt: u32 },
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counters {
    pub applied: u64,
    pub dropped: u64,
    pub gaps_closed: u64,
    pub resnapshots: u64,
    pub rest_calls: u64,
}

#[derive(Debug, Deserialize)]
pub struct LatestResponse {
    pub topic: String,
    pub entries: HashMap<String, StreamEntryPayload>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryPageResponse {
    pub topic: String,
    pub epoch: String,
    pub entries: Vec<StreamEntryPayload>,
    pub next_seq: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct StreamEntryPayload {
    pub seq: i64,
    pub epoch: String,
    pub payload_kind: String,
    pub payload: serde_json::Value,
}

const BAR_TOPICS: [&str; 2] = ["bars.forming", "bars.completed"];

pub struct ClientShared {
    connection_state: RwLock<ConnectionState>,
    counters: RwLock<Counters>,
    last_error: RwLock<String>,
    last_applied_instant: RwLock<Option<Instant>>,
    change_listener: RwLock<Option<Arc<dyn Fn() + Send + Sync + 'static>>>,
}

impl Default for ClientShared {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientShared {
    pub fn new() -> Self {
        Self {
            connection_state: RwLock::new(ConnectionState::Connecting),
            counters: RwLock::new(Counters::default()),
            last_error: RwLock::new(String::new()),
            last_applied_instant: RwLock::new(None),
            change_listener: RwLock::new(None),
        }
    }

    pub fn set_listener(&self, listener: Arc<dyn Fn() + Send + Sync + 'static>) {
        let mut guard = self.change_listener.write().unwrap();
        *guard = Some(listener);
    }

    fn notify_change(&self) {
        let listener = self.change_listener.read().unwrap().clone();
        if let Some(l) = listener {
            l();
        }
    }

    pub fn set_state(&self, state: ConnectionState) {
        {
            let mut guard = self.connection_state.write().unwrap();
            *guard = state;
        }
        self.notify_change();
    }

    pub fn set_last_error(&self, err: &str) {
        {
            let mut guard = self.last_error.write().unwrap();
            *guard = err.to_string();
        }
        self.notify_change();
    }

    pub fn inc_applied(&self) {
        {
            let mut guard = self.counters.write().unwrap();
            guard.applied += 1;
        }
        self.notify_change();
    }

    pub fn inc_dropped(&self) {
        {
            let mut guard = self.counters.write().unwrap();
            guard.dropped += 1;
        }
        self.notify_change();
    }

    pub fn inc_gaps_closed(&self) {
        {
            let mut guard = self.counters.write().unwrap();
            guard.gaps_closed += 1;
        }
        self.notify_change();
    }

    pub fn inc_resnapshots(&self) {
        {
            let mut guard = self.counters.write().unwrap();
            guard.resnapshots += 1;
        }
        self.notify_change();
    }

    pub fn inc_rest_calls(&self) {
        {
            let mut guard = self.counters.write().unwrap();
            guard.rest_calls += 1;
        }
        self.notify_change();
    }

    pub fn record_applied(&self) {
        let mut guard = self.last_applied_instant.write().unwrap();
        *guard = Some(Instant::now());
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state.read().unwrap().clone()
    }

    pub fn counters(&self) -> Counters {
        *self.counters.read().unwrap()
    }

    pub fn last_error(&self) -> String {
        self.last_error.read().unwrap().clone()
    }
}

pub struct StreamClient {
    shared: Arc<ClientShared>,
    shutdown_tx: Option<broadcast::Sender<()>>,
    runtime_thread: Option<std::thread::JoinHandle<()>>,
    is_shutdown: Arc<AtomicBool>,
}

/// Where the client delivers what it learns. Without an execution handle the client
/// subscribes to the bar topics only.
#[derive(Clone)]
pub struct Sinks {
    pub bars: BarSink,
    pub execution: Option<ExecutionHandle>,
}

impl StreamClient {
    pub fn start(config: Config, sink: BarSink) -> Self {
        Self::start_with(
            config,
            Sinks {
                bars: sink,
                execution: None,
            },
        )
    }

    pub fn start_with(config: Config, sinks: Sinks) -> Self {
        crate::stream::policy::assert_topic_policies(crate::stream::policy::KNOWN_TOPICS)
            .expect("topic policy assertion failed");

        let shared = Arc::new(ClientShared::new());
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let is_shutdown = Arc::new(AtomicBool::new(false));

        let shared_clone = shared.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let is_shutdown_clone = is_shutdown.clone();

        let runtime_thread = std::thread::Builder::new()
            .name("stream-client".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .thread_stack_size(8 * 1024 * 1024)
                    .enable_all()
                    .build()
                    .expect("failed to build tokio runtime");

                rt.block_on(async move {
                    run_client_loop(config, sinks, shared_clone, shutdown_rx, is_shutdown_clone)
                        .await;
                });
            })
            .expect("spawn stream client thread");

        Self {
            shared,
            shutdown_tx: Some(shutdown_tx),
            runtime_thread: Some(runtime_thread),
            is_shutdown,
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.shared.connection_state.read().unwrap().clone()
    }

    pub fn shared(&self) -> Arc<ClientShared> {
        self.shared.clone()
    }

    pub fn set_listener(&self, listener: Arc<dyn Fn() + Send + Sync + 'static>) {
        self.shared.set_listener(listener);
    }

    pub fn counters(&self) -> Counters {
        *self.shared.counters.read().unwrap()
    }

    pub fn last_error(&self) -> String {
        self.shared.last_error.read().unwrap().clone()
    }

    pub fn data_age_ms(&self) -> i64 {
        if let Some(inst) = *self.shared.last_applied_instant.read().unwrap() {
            inst.elapsed().as_millis() as i64
        } else {
            -1
        }
    }

    pub fn shutdown(mut self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.runtime_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StreamClient {
    fn drop(&mut self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.runtime_thread.take() {
            let _ = handle.join();
        }
    }
}

async fn fetch_and_apply_snapshot(
    topic: &str,
    state: &mut TopicState,
    sink: &BarSink,
    shared: &ClientShared,
    http: &reqwest::Client,
    api_base: &str,
    is_shutdown: &AtomicBool,
) {
    if is_shutdown.load(Ordering::SeqCst) {
        return;
    }
    let url = format!("{api_base}/api/v1/stream/{topic}/latest");
    shared.inc_rest_calls();
    let resp = match http.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return,
    };
    let prev_seq = state.last_applied_seq;
    if resp.status().is_success() {
        if let Ok(latest) = resp.json::<LatestResponse>().await {
            for entry in latest.entries.values() {
                if is_shutdown.load(Ordering::SeqCst) {
                    return;
                }
                if Some(entry.seq) == prev_seq {
                    return;
                }
                if let Some(payload_str) = entry.payload.as_str() {
                    if let Ok(bytes) = b64_decode(payload_str) {
                        if let Ok(bars) = decode_arrow_bars(&bytes) {
                            let actions = on_event(
                                state,
                                Event::Snapshot {
                                    epoch: entry.epoch.clone(),
                                    seq: entry.seq,
                                    payload: bars,
                                },
                            );
                            handle_actions(
                                state,
                                actions,
                                topic,
                                sink,
                                shared,
                                http,
                                api_base,
                                is_shutdown,
                            )
                            .await;
                            return;
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_actions(
    state: &mut TopicState,
    actions: Vec<Action>,
    topic: &str,
    sink: &BarSink,
    shared: &ClientShared,
    http: &reqwest::Client,
    api_base: &str,
    is_shutdown: &AtomicBool,
) {
    for action in actions {
        if is_shutdown.load(Ordering::SeqCst) {
            return;
        }
        match action {
            Action::Apply(bars) => {
                if topic == "bars.completed" {
                    sink.deliver_completed(bars);
                } else if topic == "bars.forming" {
                    sink.deliver_forming(bars);
                }
                shared.inc_applied();
                shared.record_applied();
            }
            Action::Drop(_) => {
                shared.inc_dropped();
            }
            Action::FetchHistory { from_seq } => {
                if let Some(epoch) = &state.epoch {
                    let url = format!(
                        "{api_base}/api/v1/stream/{topic}/history?epoch={epoch}&from_seq={from_seq}&limit=500"
                    );
                    shared.inc_rest_calls();
                    let resp = http.get(&url).send().await;
                    match resp {
                        Ok(r) if r.status().as_u16() == 410 => {
                            let next_acts = on_event(state, Event::HistoryExpired);
                            shared.inc_resnapshots();
                            Box::pin(handle_actions(
                                state,
                                next_acts,
                                topic,
                                sink,
                                shared,
                                http,
                                api_base,
                                is_shutdown,
                            ))
                            .await;
                        }
                        Ok(r) if r.status().is_success() => {
                            if let Ok(hist) = r.json::<HistoryPageResponse>().await {
                                let mut decoded_entries = Vec::new();
                                for entry in hist.entries {
                                    if let Some(p_str) = entry.payload.as_str() {
                                        if let Ok(b) = b64_decode(p_str) {
                                            if let Ok(bars) = decode_arrow_bars(&b) {
                                                decoded_entries.push((entry.seq, bars));
                                            }
                                        }
                                    }
                                }
                                let next_acts = on_event(
                                    state,
                                    Event::HistoryPage {
                                        epoch: hist.epoch,
                                        entries: decoded_entries,
                                        next_seq: hist.next_seq,
                                    },
                                );
                                shared.inc_gaps_closed();
                                Box::pin(handle_actions(
                                    state,
                                    next_acts,
                                    topic,
                                    sink,
                                    shared,
                                    http,
                                    api_base,
                                    is_shutdown,
                                ))
                                .await;
                            }
                        }
                        _ => {
                            let next_acts = on_event(state, Event::HistoryExpired);
                            shared.inc_resnapshots();
                            Box::pin(handle_actions(
                                state,
                                next_acts,
                                topic,
                                sink,
                                shared,
                                http,
                                api_base,
                                is_shutdown,
                            ))
                            .await;
                        }
                    }
                }
            }
            Action::ReSnapshot => {
                shared.inc_resnapshots();
                Box::pin(fetch_and_apply_snapshot(
                    topic,
                    state,
                    sink,
                    shared,
                    http,
                    api_base,
                    is_shutdown,
                ))
                .await;
            }
        }
    }
}

async fn run_client_loop(
    config: Config,
    sinks: Sinks,
    shared: Arc<ClientShared>,
    mut shutdown_rx: broadcast::Receiver<()>,
    is_shutdown: Arc<AtomicBool>,
) {
    let http = reqwest::Client::new();
    let mut attempt = 0u32;
    let sink = sinks.bars.clone();

    let ws_base = if config.api_base.starts_with("https://") {
        config.api_base.replacen("https://", "wss://", 1)
    } else if config.api_base.starts_with("http://") {
        config.api_base.replacen("http://", "ws://", 1)
    } else {
        format!("ws://{}", config.api_base)
    };
    let ws_url = format!("{ws_base}/api/v1/stream");

    loop {
        if is_shutdown.load(Ordering::SeqCst) {
            break;
        }

        if attempt > 0 {
            let backoff_ms = (50u64 * (1u64 << (attempt - 1).min(10))).min(30_000);
            shared.set_state(ConnectionState::Reconnecting { attempt });

            tokio::select! {
                _ = shutdown_rx.recv() => {
                    break;
                }
                _ = tokio::time::sleep(Duration::from_millis(backoff_ms)) => {}
            }
        }

        if is_shutdown.load(Ordering::SeqCst) {
            break;
        }

        shared.set_state(ConnectionState::Connecting);

        let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((stream, _)) => stream,
            Err(e) => {
                let is_503 = match &e {
                    tokio_tungstenite::tungstenite::Error::Http(resp) => {
                        resp.status().as_u16() == 503
                    }
                    _ => false,
                };
                if is_503 {
                    shared.set_state(ConnectionState::Unavailable);
                    shared.set_last_error("stream_unavailable");
                } else {
                    shared.set_last_error(&e.to_string());
                }
                attempt += 1;
                continue;
            }
        };

        attempt = 0;
        let mut states: HashMap<String, TopicState> = HashMap::new();
        states.insert(
            "bars.forming".to_string(),
            TopicState::new("bars.forming", true, &config.symbol, &config.timeframe),
        );
        states.insert(
            "bars.completed".to_string(),
            TopicState::new("bars.completed", false, &config.symbol, &config.timeframe),
        );

        let mut exec = sinks.execution.clone().map(ExecSession::new);
        let mut topics: Vec<&str> = BAR_TOPICS.to_vec();
        if exec.is_some() {
            topics.extend(EXECUTION_TOPICS);
        }

        let (mut ws_sink, mut ws_source) = ws_stream.split();

        let sub_msg = json!({ "topics": topics });
        if ws_sink
            .send(Message::Text(sub_msg.to_string().into()))
            .await
            .is_err()
        {
            attempt += 1;
            continue;
        }

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    return;
                }
                _ = async {
                    match exec.as_ref().and_then(|e| e.retry_at()) {
                        Some(at) => tokio::time::sleep_until(tokio::time::Instant::from_std(at)).await,
                        None => std::future::pending().await,
                    }
                } => {
                    if let Some(e) = exec.as_mut() {
                        e.retry(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }).await;
                    }
                }
                msg_opt = ws_source.next() => {
                    let msg = match msg_opt {
                        Some(Ok(m)) => m,
                        _ => {
                            break;
                        }
                    };

                    match msg {
                        Message::Text(text) => {
                            match classify_text(&text) {
                                Ok(ServerFrame::Control(control)) => {
                                    match control {
                                        ControlFrame::Subscribed(sub) => {
                                            if let Some(obj) = sub.topics.as_object() {
                                                for topic in BAR_TOPICS {
                                                    let Some(t) = obj.get(topic) else { continue };
                                                    let Some(state) = states.get_mut(topic) else { continue };
                                                    let ep = t.get("epoch").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                    let l_seq = t.get("last_seq").and_then(|v| v.as_i64()).unwrap_or(0);
                                                    on_event(state, Event::Subscribed { epoch: ep, last_seq: l_seq });
                                                    fetch_and_apply_snapshot(topic, state, &sink, &shared, &http, &config.api_base, &is_shutdown).await;
                                                }
                                                if let Some(e) = exec.as_mut() {
                                                    e.on_subscribed(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }, obj).await;
                                                }
                                                shared.set_state(ConnectionState::Live);
                                            }
                                        }
                                        ControlFrame::Rejected(rej) => {
                                            shared.set_last_error(&rej.reason);
                                        }
                                        ControlFrame::CursorExpired(ce) => {
                                            if let Some(e) = exec.as_mut().filter(|_| ExecSession::handles(&ce.topic)) {
                                                e.on_cursor_expired(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }, &ce.topic).await;
                                            } else if let Some(state) = states.get_mut(&ce.topic) {
                                                let acts = on_event(state, Event::CursorExpired);
                                                handle_actions(state, acts, &ce.topic, &sink, &shared, &http, &config.api_base, &is_shutdown).await;
                                            }
                                        }
                                        ControlFrame::Lagging(lag) => {
                                            if let Some(e) = exec.as_mut().filter(|_| ExecSession::handles(&lag.topic)) {
                                                e.on_lagging(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }, &lag.topic, lag.from_seq).await;
                                            } else if let Some(state) = states.get_mut(&lag.topic) {
                                                let acts = on_event(state, Event::Lagging { from_seq: lag.from_seq });
                                                handle_actions(state, acts, &lag.topic, &sink, &shared, &http, &config.api_base, &is_shutdown).await;
                                            }
                                        }
                                        ControlFrame::EpochChanged(ec) => {
                                            if let Some(e) = exec.as_mut().filter(|_| ExecSession::handles(&ec.topic)) {
                                                e.on_epoch_changed(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }, &ec.topic, &ec.new_epoch).await;
                                            } else if let Some(state) = states.get_mut(&ec.topic) {
                                                let acts = on_event(state, Event::EpochChanged { new_epoch: ec.new_epoch });
                                                handle_actions(state, acts, &ec.topic, &sink, &shared, &http, &config.api_base, &is_shutdown).await;
                                            }
                                        }
                                    }
                                }
                                Ok(ServerFrame::Envelope(header)) if exec.is_some() && ExecSession::handles(&header.topic) => {
                                    let payload = serde_json::from_str::<serde_json::Value>(&text)
                                        .ok()
                                        .and_then(|v| v.get("payload").cloned())
                                        .unwrap_or(serde_json::Value::Null);
                                    if let Some(e) = exec.as_mut() {
                                        e.on_envelope(&Net { http: &http, api_base: &config.api_base, shared: &shared, is_shutdown: &is_shutdown }, &header.topic, &header.epoch, header.seq, &payload).await;
                                    }
                                }
                                _ => {
                                    shared.inc_dropped();
                                }
                            }
                        }
                        Message::Binary(bytes) => {
                            match split_binary(&bytes) {
                                Ok(bin_frame) => {
                                    let header = bin_frame.header;
                                    let (sym, tf) = match &header.key {
                                        Some(serde_json::Value::Object(obj)) => {
                                            let s = obj.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                                            let t = obj.get("timeframe").and_then(|v| v.as_str()).unwrap_or("");
                                            (s.to_string(), t.to_string())
                                        }
                                        Some(serde_json::Value::String(s)) => {
                                            let parts: Vec<&str> = s.split(':').collect();
                                            if parts.len() == 2 {
                                                (parts[0].to_string(), parts[1].to_string())
                                            } else {
                                                (s.clone(), "".to_string())
                                            }
                                        }
                                        _ => ("".to_string(), "".to_string()),
                                    };

                                    match decode_arrow_bars(bin_frame.arrow) {
                                        Ok(bars) => {
                                            let event = Event::Entry {
                                                epoch: header.epoch,
                                                seq: header.seq,
                                                symbol: sym,
                                                timeframe: tf,
                                                payload: bars,
                                            };
                                            match states.get_mut(&header.topic) {
                                                Some(state) => {
                                                    let acts = on_event(state, event);
                                                    handle_actions(state, acts, &header.topic, &sink, &shared, &http, &config.api_base, &is_shutdown).await;
                                                }
                                                None => shared.inc_dropped(),
                                            }
                                        }
                                        Err(_) => {
                                            shared.inc_dropped();
                                        }
                                    }
                                }
                                Err(_) => {
                                    shared.inc_dropped();
                                }
                            }
                        }
                        Message::Close(_) => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(e) = &exec {
            e.lost();
        }
        attempt += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::fake_server::FakeServer;
    use crate::stream::topic_state::BarColumns;

    fn dummy_bar(t: i64, close: f64) -> BarColumns {
        BarColumns::single(t, 10.0, 11.0, 9.0, close, 100.0)
    }

    #[tokio::test]
    async fn test_shutdown_thread_joins_and_delivery_lands_nowhere() {
        let server = FakeServer::start().await;
        server
            .set_snapshot("bars.completed", 10, dummy_bar(60, 10.0))
            .await;
        server
            .set_snapshot("bars.forming", 10, dummy_bar(120, 10.5))
            .await;

        let config = Config {
            api_base: server.api_base(),
            symbol: "PETR4".to_string(),
            timeframe: "1m".to_string(),
            operator: "operator".to_string(),
        };

        let sink = BarSink::new();
        let client = StreamClient::start(config, sink.clone());

        for _ in 0..50 {
            if client.connection_state() == ConnectionState::Live {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Drain anything delivered during snapshot
        let _ = sink.drain();

        // Shut down client - runtime thread joins cleanly
        client.shutdown();

        // Sending more bars to the server now should land nowhere in sink
        server
            .send_bar("bars.completed", 11, "PETR4", "1m", dummy_bar(180, 11.0))
            .await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        let post_shutdown_deliveries = sink.drain();
        assert!(
            post_shutdown_deliveries.is_empty(),
            "deliveries after shutdown must land nowhere"
        );

        server.shutdown().await;
    }
}
