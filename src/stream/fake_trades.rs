//! The trade snapshot and history routes of the fake backend, scripted per symbol.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::contracts_stream::TradeSnapshotResponse;
use crate::stream::trade_arrow::encode_arrow_trades;
use crate::trades::feed::SourceContext;
use crate::trades::history::TradeColumns;

/// A session as the backend would freeze it: the descriptor and the rows of each page.
#[derive(Clone)]
pub struct FakeSnapshot {
    pub descriptor: TradeSnapshotResponse,
    pub context: SourceContext,
    pub pages: Vec<TradeColumns>,
}

#[derive(Clone)]
enum Script {
    Ready(Vec<FakeSnapshot>),
    /// Answer 202 this many times, then serve the snapshot.
    Pending(usize, Box<FakeSnapshot>),
    Unavailable,
    NotFound,
}

#[derive(Default)]
struct State {
    scripts: HashMap<String, Script>,
    by_id: HashMap<String, FakeSnapshot>,
    expired: HashSet<String>,
    snapshot_requests: Vec<String>,
    history_requests: Vec<String>,
}

pub struct FakeResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl FakeResponse {
    fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: body.to_string().into_bytes(),
        }
    }

    pub fn head(&self) -> String {
        let reason = match self.status {
            200 => "OK",
            202 => "Accepted",
            404 => "Not Found",
            410 => "Gone",
            _ => "Service Unavailable",
        };
        let mut head = format!(
            "HTTP/1.1 {} {reason}\r\nContent-Length: {}\r\nConnection: close",
            self.status,
            self.body.len()
        );
        for (k, v) in &self.headers {
            head.push_str(&format!("\r\n{k}: {v}"));
        }
        head.push_str("\r\n\r\n");
        head
    }
}

#[derive(Default)]
pub struct TradeFake {
    state: Mutex<State>,
    page_delay_ms: AtomicU64,
}

fn query_value(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}

impl TradeFake {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_snapshot(&self, snapshot: FakeSnapshot) {
        self.set_snapshots(vec![snapshot]);
    }

    /// Each snapshot request is answered with the next descriptor; the last one repeats.
    pub fn set_snapshots(&self, snapshots: Vec<FakeSnapshot>) {
        let Some(symbol) = snapshots.first().map(|s| s.descriptor.symbol.clone()) else {
            return;
        };
        let mut s = self.state();
        for snap in &snapshots {
            s.by_id
                .insert(snap.descriptor.snapshot_id.clone(), snap.clone());
        }
        s.scripts.insert(symbol, Script::Ready(snapshots));
    }

    /// The symbol answers 202 `times` times before serving `snapshot`.
    pub fn set_pending(&self, times: usize, snapshot: FakeSnapshot) {
        let mut s = self.state();
        s.by_id
            .insert(snapshot.descriptor.snapshot_id.clone(), snapshot.clone());
        s.scripts.insert(
            snapshot.descriptor.symbol.clone(),
            Script::Pending(times, Box::new(snapshot)),
        );
    }

    pub fn set_unavailable(&self, symbol: &str) {
        self.state()
            .scripts
            .insert(symbol.into(), Script::Unavailable);
    }

    pub fn set_not_found(&self, symbol: &str) {
        self.state().scripts.insert(symbol.into(), Script::NotFound);
    }

    /// The token now answers 410 on every page.
    pub fn expire(&self, snapshot_id: &str) {
        self.state().expired.insert(snapshot_id.into());
    }

    pub fn set_page_delay_ms(&self, ms: u64) {
        self.page_delay_ms.store(ms, Ordering::SeqCst);
    }

    pub fn page_delay_ms(&self) -> u64 {
        self.page_delay_ms.load(Ordering::SeqCst)
    }

    /// Symbols asked for a snapshot, in order.
    pub fn snapshot_requests(&self) -> Vec<String> {
        self.state().snapshot_requests.clone()
    }

    pub fn snapshot_requests_for(&self, symbol: &str) -> usize {
        self.state()
            .snapshot_requests
            .iter()
            .filter(|s| *s == symbol)
            .count()
    }

    /// Snapshot ids of the pages asked for, in order.
    pub fn history_requests(&self) -> Vec<String> {
        self.state().history_requests.clone()
    }

    pub fn respond(&self, path_and_query: &str) -> Option<FakeResponse> {
        let (path, query) = path_and_query
            .split_once('?')
            .unwrap_or((path_and_query, ""));
        match path {
            "/api/v1/market/trades/snapshot" => {
                let symbol = query_value(query, "symbol")?;
                Some(self.snapshot(&symbol))
            }
            "/api/v1/market/trades/history" => {
                let id = query_value(query, "snapshot_id")?;
                let cursor = query_value(query, "cursor");
                Some(self.page(&id, cursor.as_deref()))
            }
            _ => None,
        }
    }

    fn snapshot(&self, symbol: &str) -> FakeResponse {
        let mut s = self.state();
        s.snapshot_requests.push(symbol.to_string());
        match s.scripts.get(symbol).cloned() {
            Some(Script::Ready(mut queue)) => {
                let snap = if queue.len() > 1 {
                    let first = queue.remove(0);
                    s.scripts.insert(symbol.into(), Script::Ready(queue));
                    first
                } else {
                    queue[0].clone()
                };
                FakeResponse::json(
                    200,
                    serde_json::to_value(&snap.descriptor).unwrap_or_default(),
                )
            }
            Some(Script::Pending(0, snap)) => {
                s.scripts
                    .insert(symbol.into(), Script::Ready(vec![(*snap).clone()]));
                FakeResponse::json(
                    200,
                    serde_json::to_value(&snap.descriptor).unwrap_or_default(),
                )
            }
            Some(Script::Pending(left, snap)) => {
                s.scripts
                    .insert(symbol.into(), Script::Pending(left - 1, snap));
                let mut resp = FakeResponse::json(
                    202,
                    serde_json::json!({"status": "backfill_pending", "status_token": "backfill-1"}),
                );
                resp.headers.push(("Retry-After".into(), "1".into()));
                resp
            }
            Some(Script::NotFound) | None => FakeResponse::json(
                404,
                serde_json::json!({"code": "unknown_symbol", "message": "unknown symbol"}),
            ),
            Some(Script::Unavailable) => FakeResponse::json(
                503,
                serde_json::json!({"code": "trade_source_unavailable", "message": "unavailable"}),
            ),
        }
    }

    fn page(&self, id: &str, cursor: Option<&str>) -> FakeResponse {
        let mut s = self.state();
        s.history_requests.push(id.to_string());
        if s.expired.contains(id) {
            return FakeResponse::json(
                410,
                serde_json::json!({"code": "snapshot_expired", "message": "expired"}),
            );
        }
        let Some(snap) = s.by_id.get(id) else {
            return FakeResponse::json(
                404,
                serde_json::json!({"code": "snapshot_not_found", "message": "unknown"}),
            );
        };
        let index = cursor
            .and_then(|c| c.strip_prefix("cursor-"))
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or(0);
        let rows = snap.pages.get(index).cloned().unwrap_or_default();
        let next = (index + 1 < snap.pages.len()).then(|| format!("cursor-{}", index + 1));
        let d = &snap.descriptor;
        let body = encode_arrow_trades(&snap.context, &rows).unwrap_or_default();
        FakeResponse {
            status: 200,
            headers: vec![
                (
                    "Content-Type".into(),
                    "application/vnd.apache.arrow.stream".into(),
                ),
                ("X-Q-Trade-Snapshot-Id".into(), d.snapshot_id.clone()),
                (
                    "X-Q-Trade-Source-Generation".into(),
                    d.source_generation.clone(),
                ),
                ("X-Q-Trade-Symbol".into(), d.symbol.clone()),
                ("X-Q-Trade-Volume-Field".into(), d.volume_field.clone()),
                ("X-Q-Trade-Volume-Unit".into(), d.volume_unit.clone()),
                ("X-Q-Trade-Page-Count".into(), rows.len().to_string()),
                (
                    "X-Q-Trade-Frozen-Epoch".into(),
                    d.frozen_watermark.epoch.clone(),
                ),
                (
                    "X-Q-Trade-Frozen-Seq".into(),
                    d.frozen_watermark.seq.to_string(),
                ),
                ("X-Q-Trade-Next-Cursor".into(), next.unwrap_or_default()),
            ],
            body,
        }
    }
}
