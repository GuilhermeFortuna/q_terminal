use crate::stream::arrow_decode::encode_arrow_bars;
use crate::stream::base64::b64_encode;
use crate::stream::frame::{make_binary_frame, EnvelopeHeader};
use crate::stream::topic_state::BarColumns;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;

pub enum ServerCommand {
    SendBinary(Vec<u8>),
    SendText(String),
    Close,
}

#[derive(Clone)]
pub struct SnapshotEntry {
    pub seq: i64,
    pub epoch: String,
    pub bars: BarColumns,
}

pub type HistoryMap = HashMap<String, Vec<(i64, BarColumns)>>;

pub struct FakeServer {
    addr: SocketAddr,
    deny_503: Arc<AtomicBool>,
    history_expired: Arc<AtomicBool>,
    reject_forming: Arc<AtomicBool>,
    epoch: Arc<RwLock<String>>,
    snapshots: Arc<RwLock<HashMap<String, SnapshotEntry>>>,
    history: Arc<RwLock<HistoryMap>>,
    catalog_json: Arc<RwLock<Option<String>>>,
    rest_calls: Arc<std::sync::atomic::AtomicUsize>,
    command_tx: mpsc::Sender<ServerCommand>,
    shutdown_tx: mpsc::Sender<()>,
}

impl FakeServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let deny_503 = Arc::new(AtomicBool::new(false));
        let history_expired = Arc::new(AtomicBool::new(false));
        let reject_forming = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(RwLock::new("epoch-1".to_string()));
        let snapshots = Arc::new(RwLock::new(HashMap::<String, SnapshotEntry>::new()));
        let history = Arc::new(RwLock::new(HashMap::<String, Vec<(i64, BarColumns)>>::new()));
        let catalog_json = Arc::new(RwLock::new(None::<String>));
        let rest_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<ServerCommand>(100);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let ws_sender_holder: Arc<Mutex<Option<mpsc::Sender<Message>>>> =
            Arc::new(Mutex::new(None));
        let ws_sender_clone = ws_sender_holder.clone();

        // Background task forwarding ServerCommands to active WebSocket
        let ws_fwd_holder = ws_sender_holder.clone();
        tokio::spawn(async move {
            while let Some(cmd) = cmd_rx.recv().await {
                let sender = {
                    let guard = ws_fwd_holder.lock().await;
                    guard.clone()
                };
                if let Some(tx) = sender {
                    match cmd {
                        ServerCommand::SendBinary(b) => {
                            let _ = tx.send(Message::Binary(b.into())).await;
                        }
                        ServerCommand::SendText(t) => {
                            let _ = tx.send(Message::Text(t.into())).await;
                        }
                        ServerCommand::Close => {
                            let _ = tx.send(Message::Close(None)).await;
                        }
                    }
                }
            }
        });

        // Server connection accept loop
        let d503 = deny_503.clone();
        let hexp = history_expired.clone();
        let rform = reject_forming.clone();
        let ep_arc = epoch.clone();
        let snap_arc = snapshots.clone();
        let hist_arc = history.clone();
        let cat_arc = catalog_json.clone();
        let rest_cnt = rest_calls.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    res = listener.accept() => {
                        if let Ok((mut socket, _)) = res {
                            let d503 = d503.clone();
                            let hexp = hexp.clone();
                            let rform = rform.clone();
                            let ep_arc = ep_arc.clone();
                            let snap_arc = snap_arc.clone();
                            let hist_arc = hist_arc.clone();
                            let cat_arc = cat_arc.clone();
                            let rest_cnt = rest_cnt.clone();
                            let ws_sender_clone = ws_sender_clone.clone();

                            tokio::spawn(async move {
                                // Peek without consuming to inspect headers
                                let mut buf = [0u8; 4096];
                                let n = match socket.peek(&mut buf).await {
                                    Ok(n) if n > 0 => n,
                                    _ => return,
                                };
                                let request_text = String::from_utf8_lossy(&buf[..n]);
                                let first_line = request_text.lines().next().unwrap_or("").to_string();
                                let is_ws = request_text.contains("Upgrade: websocket") || request_text.contains("upgrade: websocket");

                                if d503.load(Ordering::SeqCst) {
                                    let mut discard = [0u8; 4096];
                                    let _ = socket.read(&mut discard).await;
                                    let resp = "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: 33\r\nConnection: close\r\n\r\n{\"code\":\"stream_unavailable\"}";
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                    let _ = socket.shutdown().await;
                                    return;
                                }

                                if is_ws {
                                    let mut ws_stream = match tokio_tungstenite::accept_async(socket).await {
                                        Ok(ws) => ws,
                                        Err(_) => return,
                                    };

                                    // Wait for SubscribeFrame
                                    if let Some(Ok(Message::Text(sub_text))) = ws_stream.next().await {
                                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sub_text) {
                                            let cur_ep = ep_arc.read().await.clone();
                                            let mut topics_obj = serde_json::Map::new();

                                            if let Some(topics) = v.get("topics").and_then(|t| t.as_array()) {
                                                for top in topics {
                                                    if let Some(top_str) = top.as_str() {
                                                        topics_obj.insert(
                                                            top_str.to_string(),
                                                            json!({
                                                                "cursor": "0-0",
                                                                "epoch": cur_ep,
                                                                "last_seq": 100
                                                            }),
                                                        );
                                                    }
                                                }
                                            }

                                            let ack = json!({
                                                "type": "subscribed",
                                                "topics": topics_obj
                                            });
                                            let _ = ws_stream.send(Message::Text(ack.to_string().into())).await;

                                            if rform.load(Ordering::SeqCst) {
                                                let rej = json!({
                                                    "type": "rejected",
                                                    "topic": "bars.forming",
                                                    "reason": "unsupported_timeframe"
                                                });
                                                let _ = ws_stream.send(Message::Text(rej.to_string().into())).await;
                                            }
                                        }
                                    }

                                    // Split ws_stream
                                    let (mut ws_sink, mut ws_source) = ws_stream.split();
                                    let (tx, mut rx) = mpsc::channel::<Message>(100);
                                    {
                                        let mut guard = ws_sender_clone.lock().await;
                                        *guard = Some(tx);
                                    }

                                    // Forward rx to ws_sink
                                    let fwd_handle = tokio::spawn(async move {
                                        while let Some(msg) = rx.recv().await {
                                            if ws_sink.send(msg).await.is_err() {
                                                break;
                                            }
                                        }
                                    });

                                    // Drain incoming messages until close
                                    while let Some(msg) = ws_source.next().await {
                                        if let Ok(Message::Close(_)) = msg {
                                            break;
                                        }
                                    }

                                    fwd_handle.abort();
                                    let mut guard = ws_sender_clone.lock().await;
                                    *guard = None;
                                    return;
                                }

                                // Consume REST request bytes
                                let _ = socket.read(&mut buf[..n]).await;

                                rest_cnt.fetch_add(1, Ordering::SeqCst);

                                // Handle REST requests
                                let parts: Vec<&str> = first_line.split_whitespace().collect();
                                if parts.len() < 2 {
                                    return;
                                }
                                let path_and_query = parts[1];
                                let path = path_and_query.split('?').next().unwrap_or("");

                                if path.contains("/api/v1/catalog/datasets") {
                                    let cat_guard = cat_arc.read().await;
                                    if let Some(body) = cat_guard.as_ref() {
                                        let resp = format!(
                                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                            body.len(),
                                            body
                                        );
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        let _ = socket.shutdown().await;
                                        return;
                                    }
                                }

                                if path.ends_with("/latest") {
                                    let topic = path.trim_start_matches("/api/v1/stream/").trim_end_matches("/latest");
                                    let snap_guard = snap_arc.read().await;
                                    let cur_ep = ep_arc.read().await.clone();

                                    let mut entries = serde_json::Map::new();
                                    if let Some(snap) = snap_guard.get(topic) {
                                        let arrow_bytes = encode_arrow_bars(&snap.bars).unwrap();
                                        let b64 = b64_encode(&arrow_bytes);

                                        let entry_val = json!({
                                            "topic": topic,
                                            "schema_major": 1,
                                            "seq": snap.seq,
                                            "epoch": cur_ep,
                                            "producer_id": "fake",
                                            "origin_ts": "2026-09-17T00:00:00Z",
                                            "payload_kind": "arrow_ipc",
                                            "payload_schema": "schema/api/arrow/bars.schema.json",
                                            "key": { "symbol": "PETR4", "timeframe": "1m" },
                                            "payload": b64
                                        });
                                        entries.insert("PETR4:1m".to_string(), entry_val);
                                    }

                                    let resp_body = json!({
                                        "topic": topic,
                                        "entries": entries
                                    }).to_string();

                                    let resp = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                        resp_body.len(),
                                        resp_body
                                    );
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                    let _ = socket.shutdown().await;
                                } else if path.ends_with("/history") {
                                    let topic = path.trim_start_matches("/api/v1/stream/").trim_end_matches("/history");

                                    if hexp.load(Ordering::SeqCst) {
                                        let err_body = json!({
                                            "topic": topic,
                                            "requested_from_seq": 1,
                                            "oldest_available_seq": 5
                                        }).to_string();
                                        let resp = format!(
                                            "HTTP/1.1 410 Gone\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                            err_body.len(),
                                            err_body
                                        );
                                        let _ = socket.write_all(resp.as_bytes()).await;
                                        let _ = socket.shutdown().await;
                                        return;
                                    }

                                    let cur_ep = ep_arc.read().await.clone();
                                    let hist_guard = hist_arc.read().await;

                                    let mut entries_arr = Vec::new();
                                    if let Some(list) = hist_guard.get(topic) {
                                        for (seq, bars) in list {
                                            let arrow_bytes = encode_arrow_bars(bars).unwrap();
                                            let b64 = b64_encode(&arrow_bytes);
                                            entries_arr.push(json!({
                                                "topic": topic,
                                                "schema_major": 1,
                                                "seq": *seq,
                                                "epoch": cur_ep,
                                                "producer_id": "fake",
                                                "origin_ts": "2026-09-17T00:00:00Z",
                                                "payload_kind": "arrow_ipc",
                                                "payload_schema": "schema/api/arrow/bars.schema.json",
                                                "key": { "symbol": "PETR4", "timeframe": "1m" },
                                                "payload": b64
                                            }));
                                        }
                                    }

                                    let resp_body = json!({
                                        "topic": topic,
                                        "epoch": cur_ep,
                                        "entries": entries_arr,
                                        "next_seq": null
                                    }).to_string();

                                    let resp = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                        resp_body.len(),
                                        resp_body
                                    );
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                    let _ = socket.shutdown().await;
                                } else {
                                    let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                                    let _ = socket.write_all(resp.as_bytes()).await;
                                    let _ = socket.shutdown().await;
                                }
                            });
                        }
                    }
                }
            }
        });

        Self {
            addr,
            deny_503,
            history_expired,
            reject_forming,
            epoch,
            snapshots,
            history,
            catalog_json,
            rest_calls,
            command_tx: cmd_tx,
            shutdown_tx,
        }
    }

    pub fn api_base(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    pub async fn set_catalog(&self, catalog_body: &str) {
        let mut guard = self.catalog_json.write().await;
        *guard = Some(catalog_body.to_string());
    }

    pub fn rest_calls(&self) -> usize {
        self.rest_calls.load(Ordering::SeqCst)
    }

    pub fn set_deny_503(&self, val: bool) {
        self.deny_503.store(val, Ordering::SeqCst);
    }

    pub fn set_history_expired(&self, val: bool) {
        self.history_expired.store(val, Ordering::SeqCst);
    }

    pub fn set_reject_forming(&self, val: bool) {
        self.reject_forming.store(val, Ordering::SeqCst);
    }

    pub async fn set_epoch(&self, new_epoch: &str) {
        let mut guard = self.epoch.write().await;
        *guard = new_epoch.to_string();
    }

    pub async fn set_snapshot(&self, topic: &str, seq: i64, bars: BarColumns) {
        let cur_ep = self.epoch.read().await.clone();
        let mut guard = self.snapshots.write().await;
        guard.insert(
            topic.to_string(),
            SnapshotEntry {
                seq,
                epoch: cur_ep,
                bars,
            },
        );
    }

    pub async fn add_history_entry(&self, topic: &str, seq: i64, bars: BarColumns) {
        let mut guard = self.history.write().await;
        guard
            .entry(topic.to_string())
            .or_default()
            .push((seq, bars));
    }

    pub async fn send_bar(
        &self,
        topic: &str,
        seq: i64,
        symbol: &str,
        timeframe: &str,
        bars: BarColumns,
    ) {
        self.set_snapshot(topic, seq, bars.clone()).await;
        let cur_ep = self.epoch.read().await.clone();
        let header = EnvelopeHeader {
            topic: topic.to_string(),
            schema_major: 1,
            seq,
            epoch: cur_ep,
            producer_id: "fake".to_string(),
            origin_ts: "2026-09-17T00:00:00Z".to_string(),
            payload_kind: "arrow_ipc".to_string(),
            payload_schema: "schema/api/arrow/bars.schema.json".to_string(),
            key: Some(json!({ "symbol": symbol, "timeframe": timeframe })),
        };
        let arrow_bytes = encode_arrow_bars(&bars).unwrap();
        let frame_bytes = make_binary_frame(&header, &arrow_bytes);
        let _ = self
            .command_tx
            .send(ServerCommand::SendBinary(frame_bytes))
            .await;
    }

    pub async fn send_lagging(&self, topic: &str, from_seq: i64) {
        let msg = json!({
            "type": "lagging",
            "topic": topic,
            "from_seq": from_seq
        });
        let _ = self
            .command_tx
            .send(ServerCommand::SendText(msg.to_string()))
            .await;
    }

    pub async fn send_cursor_expired(&self, topic: &str) {
        let msg = json!({
            "type": "cursor_expired",
            "topic": topic
        });
        let _ = self
            .command_tx
            .send(ServerCommand::SendText(msg.to_string()))
            .await;
    }

    pub async fn send_epoch_changed(&self, topic: &str, new_epoch: &str) {
        let prev_ep = self.epoch.read().await.clone();
        {
            let mut guard = self.epoch.write().await;
            *guard = new_epoch.to_string();
        }
        let msg = json!({
            "type": "epoch_changed",
            "topic": topic,
            "new_epoch": new_epoch,
            "previous_epoch": prev_ep
        });
        let _ = self
            .command_tx
            .send(ServerCommand::SendText(msg.to_string()))
            .await;
    }

    pub async fn close_client(&self) {
        let _ = self.command_tx.send(ServerCommand::Close).await;
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}
