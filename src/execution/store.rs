//! The terminal's exact, live, in-memory copy of execution state (Q-046).
//!
//! The store replaces entities by identifier with the state an event carries. It does no
//! position or balance arithmetic, and it keeps decimals as the strings the contract uses.

use crate::contracts_stream::{
    ExecutionAccount, ExecutionControl, ExecutionDecisionState, ExecutionDeploymentState,
    ExecutionFillEvent, ExecutionOrderState, ExecutionPosition, ExecutionSnapshot,
};
use crate::stream::execution::ExecutionEvent;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::SystemTime;

/// Ring capacity used until the first snapshot declares the real limits.
const DEFAULT_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub recent_decisions: usize,
    pub recent_fills: usize,
    pub recent_orders: usize,
    pub recent_risk: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            recent_decisions: DEFAULT_LIMIT,
            recent_fills: DEFAULT_LIMIT,
            recent_orders: DEFAULT_LIMIT,
            recent_risk: DEFAULT_LIMIT,
        }
    }
}

impl Limits {
    fn from_snapshot(snap: &ExecutionSnapshot) -> Self {
        let read = |key: &str| {
            snap.limits
                .get(key)
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(DEFAULT_LIMIT)
        };
        Self {
            recent_decisions: read("recent_decisions"),
            recent_fills: read("recent_fills"),
            recent_orders: read("recent_orders"),
            recent_risk: read("recent_risk"),
        }
    }
}

/// Key of risk events that belong to no deployment (kill switch changes).
pub const GLOBAL_KEY: &str = "";

/// The entities the store holds. Two stores are equal exactly when their `ExecutionData` are.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionData {
    pub deployments: BTreeMap<String, ExecutionDeploymentState>,
    pub accounts: BTreeMap<String, ExecutionAccount>,
    /// Open positions by deployment id. A closed position is not kept.
    pub positions: BTreeMap<String, ExecutionPosition>,
    /// The rings below hold oldest first, per deployment id.
    pub orders: BTreeMap<String, VecDeque<ExecutionOrderState>>,
    pub decisions: BTreeMap<String, VecDeque<ExecutionDecisionState>>,
    pub fills: BTreeMap<String, VecDeque<ExecutionFillEvent>>,
    pub risk: BTreeMap<String, VecDeque<serde_json::Value>>,
    pub control: Option<ExecutionControl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub deployments: usize,
    pub accounts: usize,
    pub positions: usize,
    pub orders: usize,
    pub decisions: usize,
    pub fills: usize,
    pub risk: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ExecutionStore {
    data: ExecutionData,
    limits: Limits,
    revision: u64,
    confirmed: bool,
    last_confirmed_at: Option<SystemTime>,
    watermarks: BTreeMap<String, (String, i64)>,
}

fn total<T>(map: &BTreeMap<String, VecDeque<T>>) -> usize {
    map.values().map(|r| r.len()).sum()
}

/// Replaces the entry for `key`; true when that changed the map.
fn upsert<V: PartialEq>(map: &mut BTreeMap<String, V>, key: String, value: V) -> bool {
    if map.get(&key) == Some(&value) {
        return false;
    }
    map.insert(key, value);
    true
}

/// Inserts or replaces `item` in its deployment's ring, newest last, and trims the oldest.
fn ring_upsert<T: Clone + PartialEq>(
    map: &mut BTreeMap<String, VecDeque<T>>,
    key: &str,
    item: T,
    limit: usize,
    id_of: impl Fn(&T) -> String,
) -> bool {
    if limit == 0 {
        return false;
    }
    let id = id_of(&item);
    let ring = map.entry(key.to_string()).or_default();
    if let Some(pos) = ring.iter().position(|e| id_of(e) == id) {
        if ring[pos] == item {
            return false;
        }
        ring.remove(pos);
    }
    ring.push_back(item);
    while ring.len() > limit {
        ring.pop_front();
    }
    true
}

/// Groups a snapshot collection into per-deployment rings, ordered by `updated_at`.
fn build_rings<T: Clone>(
    items: &[T],
    limit: usize,
    key_of: impl Fn(&T) -> String,
    updated_at: impl Fn(&T) -> String,
) -> BTreeMap<String, VecDeque<T>> {
    let mut grouped: BTreeMap<String, Vec<T>> = BTreeMap::new();
    for item in items {
        grouped.entry(key_of(item)).or_default().push(item.clone());
    }
    let mut out = BTreeMap::new();
    for (key, mut list) in grouped {
        list.sort_by_key(|a| updated_at(a));
        let skip = list.len().saturating_sub(limit);
        let ring: VecDeque<T> = list.into_iter().skip(skip).collect();
        if !ring.is_empty() {
            out.insert(key, ring);
        }
    }
    out
}

fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn non_null(v: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    v.get(key).filter(|x| !x.is_null()).cloned()
}

fn control_from_risk(v: &serde_json::Value) -> Option<ExecutionControl> {
    if v.get("kind").and_then(|k| k.as_str()) != Some("kill_switch") {
        return None;
    }
    Some(ExecutionControl {
        kill_switch_enabled: v
            .get("kill_switch_enabled")
            .and_then(|b| b.as_bool())
            .unwrap_or(false),
        kill_switch_reason: non_null(v, "kill_switch_reason"),
        updated_at: str_field(v, "updated_at"),
        updated_by: non_null(v, "updated_by"),
    })
}

impl ExecutionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(&self) -> &ExecutionData {
        &self.data
    }

    /// Increases on every applied change, so a view can redraw at most once per frame.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// True once a snapshot has been applied and the stream has not been lost since.
    pub fn is_confirmed(&self) -> bool {
        self.confirmed
    }

    /// When the state was last known to match the backend, once it is unconfirmed.
    pub fn last_confirmed_at(&self) -> Option<SystemTime> {
        self.last_confirmed_at
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// The `(epoch, seq)` last applied per topic.
    pub fn watermarks(&self) -> &BTreeMap<String, (String, i64)> {
        &self.watermarks
    }

    pub fn set_watermark(&mut self, topic: &str, epoch: &str, seq: i64) {
        self.watermarks
            .insert(topic.to_string(), (epoch.to_string(), seq));
    }

    pub fn counts(&self) -> Counts {
        Counts {
            deployments: self.data.deployments.len(),
            accounts: self.data.accounts.len(),
            positions: self.data.positions.len(),
            orders: total(&self.data.orders),
            decisions: total(&self.data.decisions),
            fills: total(&self.data.fills),
            risk: total(&self.data.risk),
        }
    }

    /// The lines `--headless-report --execution` prints: entity counts, the kill switch and
    /// the `(epoch, seq)` last applied on each topic.
    pub fn report_lines(&self) -> Vec<String> {
        let c = self.counts();
        let mut lines = vec![
            format!("confirmed: {}", self.confirmed),
            format!("deployments: {}", c.deployments),
            format!("accounts: {}", c.accounts),
            format!("positions: {}", c.positions),
            format!("orders: {}", c.orders),
            format!("decisions: {}", c.decisions),
            format!("fills: {}", c.fills),
            format!("risk: {}", c.risk),
            format!(
                "kill_switch: {}",
                match &self.data.control {
                    Some(c) if c.kill_switch_enabled => "on",
                    Some(_) => "off",
                    None => "unknown",
                }
            ),
        ];
        for (topic, (epoch, seq)) in &self.watermarks {
            lines.push(format!("seq {topic}: {seq} ({epoch})"));
        }
        lines
    }

    /// Keeps the last state and marks it with the time it was last confirmed (§8.1).
    pub fn mark_unconfirmed(&mut self, since: SystemTime) {
        if self.confirmed {
            self.confirmed = false;
            self.last_confirmed_at = Some(since);
        }
    }

    /// Replaces the entities owned by `topics` with the snapshot's. Position state is
    /// owned by `fills`, accounts by `ledger` and the kill switch by `risk`.
    pub fn apply_snapshot(&mut self, snap: &ExecutionSnapshot, topics: &[&str]) {
        let limits = Limits::from_snapshot(snap);
        if limits != self.limits {
            self.limits = limits;
        }
        #[derive(serde::Deserialize, Default)]
        struct Recent {
            #[serde(default)]
            decisions: Vec<ExecutionDecisionState>,
            #[serde(default)]
            fills: Vec<ExecutionFillEvent>,
            #[serde(default)]
            risk: Vec<serde_json::Value>,
        }
        let recent: Recent = serde_json::from_value(snap.recent.clone()).unwrap_or_default();

        let before = self.data.clone();
        for topic in topics {
            match *topic {
                "deployments" => {
                    self.data.deployments = snap
                        .deployments
                        .iter()
                        .map(|d| (d.id.clone(), d.clone()))
                        .collect();
                }
                "ledger" => {
                    self.data.accounts = snap
                        .accounts
                        .iter()
                        .map(|a| (a.id.clone(), a.clone()))
                        .collect();
                }
                "orders" => {
                    self.data.orders = build_rings(
                        &snap.orders,
                        limits.recent_orders,
                        |o| o.deployment_id.clone(),
                        |o| o.updated_at.clone(),
                    );
                }
                "decisions" => {
                    self.data.decisions = build_rings(
                        &recent.decisions,
                        limits.recent_decisions,
                        |d| d.deployment_id.clone(),
                        |d| d.updated_at.clone(),
                    );
                }
                "fills" => {
                    self.data.fills = build_rings(
                        &recent.fills,
                        limits.recent_fills,
                        |f| f.deployment_id.clone(),
                        |f| f.updated_at.clone(),
                    );
                    self.data.positions = snap
                        .positions
                        .iter()
                        .filter(|p| p.is_open)
                        .map(|p| (p.deployment_id.clone(), p.clone()))
                        .collect();
                }
                "risk" => {
                    self.data.risk = build_rings(
                        &recent.risk,
                        limits.recent_risk,
                        |r| str_field(r, "deployment_id"),
                        |r| str_field(r, "updated_at"),
                    );
                    self.data.control = Some(snap.control.clone());
                }
                _ => {}
            }
        }
        if self.data != before {
            self.revision += 1;
        }
        self.confirmed = true;
        self.last_confirmed_at = Some(SystemTime::now());
    }

    /// Applies one event by replacing the entity it names. Returns whether anything changed.
    pub fn apply(&mut self, event: ExecutionEvent) -> bool {
        let limits = self.limits;
        let changed = match event {
            ExecutionEvent::Deployment(d) => upsert(&mut self.data.deployments, d.id.clone(), d),
            ExecutionEvent::Decision(d) => ring_upsert(
                &mut self.data.decisions,
                &d.deployment_id.clone(),
                d,
                limits.recent_decisions,
                |x| x.id.clone(),
            ),
            ExecutionEvent::Order(o) => ring_upsert(
                &mut self.data.orders,
                &o.deployment_id.clone(),
                o,
                limits.recent_orders,
                |x| x.id.clone(),
            ),
            ExecutionEvent::Fill(f) => {
                let position = f.position_after.clone();
                let key = position.deployment_id.clone();
                let position_changed = if position.is_open {
                    upsert(&mut self.data.positions, key, position)
                } else {
                    self.data.positions.remove(&key).is_some()
                };
                let ring_changed = ring_upsert(
                    &mut self.data.fills,
                    &f.deployment_id.clone(),
                    f,
                    limits.recent_fills,
                    |x| x.id.clone(),
                );
                position_changed || ring_changed
            }
            ExecutionEvent::Risk(r) => {
                let control_changed = match control_from_risk(&r) {
                    Some(c) if self.data.control.as_ref() != Some(&c) => {
                        self.data.control = Some(c);
                        true
                    }
                    _ => false,
                };
                let key = str_field(&r, "deployment_id");
                let ring_changed =
                    ring_upsert(&mut self.data.risk, &key, r, limits.recent_risk, |x| {
                        str_field(x, "id")
                    });
                control_changed || ring_changed
            }
            ExecutionEvent::Ledger(l) => {
                let account = l.account_after;
                upsert(&mut self.data.accounts, account.id.clone(), account)
            }
        };
        if changed {
            self.revision += 1;
        }
        changed
    }
}

type Listener = Arc<dyn Fn() + Send + Sync + 'static>;

/// The store shared between the stream thread and readers, with a change listener.
#[derive(Clone, Default)]
pub struct ExecutionHandle {
    store: Arc<Mutex<ExecutionStore>>,
    listener: Arc<RwLock<Option<Listener>>>,
}

impl ExecutionHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_listener(&self, listener: Listener) {
        *self.listener.write().unwrap() = Some(listener);
    }

    /// Runs `f` on the store, then notifies the listener if the revision or the
    /// confirmation state moved.
    pub fn mutate<R>(&self, f: impl FnOnce(&mut ExecutionStore) -> R) -> R {
        let (result, moved) = {
            let mut store = self.store.lock().unwrap();
            let before = (store.revision, store.confirmed);
            let result = f(&mut store);
            (result, before != (store.revision, store.confirmed))
        };
        if moved {
            let listener = self.listener.read().unwrap().clone();
            if let Some(l) = listener {
                l();
            }
        }
        result
    }

    pub fn read<R>(&self, f: impl FnOnce(&ExecutionStore) -> R) -> R {
        f(&self.store.lock().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::execution::decode_execution_entry;
    use serde_json::json;

    const DEP: &str = "11111111-1111-1111-1111-111111111111";
    const ACC: &str = "22222222-2222-2222-2222-222222222222";

    fn position(qty: &str, open: bool, ts: &str) -> serde_json::Value {
        json!({"id": "p1", "deployment_id": DEP, "side": "long", "quantity": qty,
               "average_entry_price": "10.0", "is_open": open, "opened_at": null,
               "closed_at": null, "updated_at": ts})
    }

    fn account(balance: &str, ts: &str) -> serde_json::Value {
        json!({"id": ACC, "name": "paper", "currency": "BRL", "initial_balance": "1000.0",
               "cash_balance": balance, "realized_pnl": "0", "risk_config": null,
               "sizing_config": null, "created_at": null, "updated_at": ts})
    }

    fn fill(id: &str, qty_after: &str, open: bool, ts: &str) -> ExecutionEvent {
        decode_execution_entry(
            "fills",
            &json!({"entity": "fill", "id": id, "deployment_id": DEP, "account_id": ACC,
                    "order_id": "o1", "broker_mode": "paper", "external_fill_id": id,
                    "side": "buy", "quantity": "1", "price": "10", "fee": "0",
                    "slippage": "0", "quote_bid": null, "quote_ask": null,
                    "quote_timestamp": null, "filled_at": ts, "details": null,
                    "position_after": position(qty_after, open, ts),
                    "created_at": null, "updated_at": ts}),
        )
        .unwrap()
    }

    fn ledger(balance: &str, ts: &str) -> ExecutionEvent {
        decode_execution_entry(
            "ledger",
            &json!({"entity": "ledger", "id": format!("l-{ts}"), "account_id": ACC,
                    "deployment_id": DEP, "paper_account_id": null, "fill_id": null,
                    "entry_type": "fee", "amount": "-1", "balance_after": balance,
                    "description": null, "account_after": account(balance, ts),
                    "created_at": null, "updated_at": ts}),
        )
        .unwrap()
    }

    fn order(id: &str, status: &str, ts: &str) -> ExecutionEvent {
        decode_execution_entry(
            "orders",
            &json!({"entity": "order", "id": id, "deployment_id": DEP, "account_id": ACC,
                    "decision_id": null, "intent_id": "i1", "broker_mode": "paper",
                    "order_type": "market", "side": "buy", "quantity": "1",
                    "status": status, "reconciliation_state": "not_required",
                    "details": null, "created_at": null, "updated_at": ts,
                    "completed_at": null, "external_order_id": null,
                    "intent_committed_at": null, "reconciled_at": null,
                    "reconciled_by": null, "reconciliation_attempted_at": null,
                    "reconciliation_detail": null, "reconciliation_error": null,
                    "rejection_reason": null, "submitted_at": null}),
        )
        .unwrap()
    }

    fn kill_switch(enabled: bool, ts: &str) -> ExecutionEvent {
        ExecutionEvent::Risk(
            json!({"entity": "risk", "kind": "kill_switch", "id": format!("k-{ts}"),
            "deployment_id": null, "account_id": null, "kill_switch_enabled": enabled,
            "kill_switch_reason": "manual", "updated_by": "ops", "updated_at": ts}),
        )
    }

    fn empty_snapshot(limit: u64) -> ExecutionSnapshot {
        serde_json::from_value(json!({
            "deployments": [], "accounts": [], "positions": [], "orders": [],
            "recent": {"decisions": [], "fills": [], "risk": []},
            "control": {"kill_switch_enabled": false, "kill_switch_reason": null,
                        "updated_by": null, "updated_at": "2026-09-18T09:00:00Z"},
            "limits": {"recent_decisions": limit, "recent_fills": limit,
                       "recent_orders": limit, "recent_risk": limit},
            "watermark": {}
        }))
        .unwrap()
    }

    #[test]
    fn replace_by_id_and_revision_only_on_change() {
        let mut store = ExecutionStore::new();
        assert!(store.apply(order("o1", "submitted", "2026-09-18T10:00:00Z")));
        assert_eq!(store.revision(), 1);
        assert!(store.apply(order("o1", "filled", "2026-09-18T10:00:01Z")));
        assert_eq!(store.revision(), 2);
        assert_eq!(store.counts().orders, 1, "same id replaces");
        assert!(!store.apply(order("o1", "filled", "2026-09-18T10:00:01Z")));
        assert_eq!(store.revision(), 2, "an identical event changes nothing");
    }

    #[test]
    fn fill_sets_position_from_post_fill_state_and_close_removes_it() {
        let mut store = ExecutionStore::new();
        store.apply(fill("f1", "5", true, "2026-09-18T10:00:00Z"));
        assert_eq!(store.data().positions[DEP].quantity, "5");
        store.apply(fill("f2", "3", true, "2026-09-18T10:00:01Z"));
        assert_eq!(store.data().positions[DEP].quantity, "3");
        store.apply(fill("f3", "0", false, "2026-09-18T10:00:02Z"));
        assert!(store.data().positions.is_empty());
        assert_eq!(store.counts().fills, 3);
    }

    #[test]
    fn ledger_entry_sets_account_from_post_entry_balances() {
        let mut store = ExecutionStore::new();
        store.apply(ledger("999", "2026-09-18T10:00:00Z"));
        assert_eq!(store.data().accounts[ACC].cash_balance, "999");
        store.apply(ledger("998", "2026-09-18T10:00:01Z"));
        assert_eq!(store.data().accounts[ACC].cash_balance, "998");
    }

    #[test]
    fn kill_switch_event_updates_control() {
        let mut store = ExecutionStore::new();
        store.apply(kill_switch(true, "2026-09-18T10:00:00Z"));
        assert!(store.data().control.as_ref().unwrap().kill_switch_enabled);
        store.apply(kill_switch(false, "2026-09-18T10:00:01Z"));
        assert!(!store.data().control.as_ref().unwrap().kill_switch_enabled);
    }

    #[test]
    fn rings_are_bounded_by_the_snapshot_limits() {
        let mut store = ExecutionStore::new();
        store.apply_snapshot(&empty_snapshot(3), &["orders", "fills"]);
        assert_eq!(store.limits().recent_orders, 3);
        for i in 0..6 {
            store.apply(order(
                &format!("o{i}"),
                "filled",
                &format!("2026-09-18T10:00:0{i}Z"),
            ));
            store.apply(fill(
                &format!("f{i}"),
                "1",
                true,
                &format!("2026-09-18T10:00:0{i}Z"),
            ));
        }
        assert_eq!(store.counts().orders, 3);
        assert_eq!(store.counts().fills, 3);
        let ids: Vec<_> = store.data().orders[DEP]
            .iter()
            .map(|o| o.id.clone())
            .collect();
        assert_eq!(ids, ["o3", "o4", "o5"], "the oldest are dropped");
    }

    #[test]
    fn snapshot_replaces_only_the_listed_topics() {
        let mut store = ExecutionStore::new();
        store.apply(order("o1", "filled", "2026-09-18T10:00:00Z"));
        store.apply(ledger("999", "2026-09-18T10:00:00Z"));
        store.apply_snapshot(&empty_snapshot(50), &["orders"]);
        assert_eq!(store.counts().orders, 0);
        assert_eq!(store.counts().accounts, 1, "ledger owns accounts");
        store.apply_snapshot(&empty_snapshot(50), &["ledger"]);
        assert_eq!(store.counts().accounts, 0);
        assert!(store.is_confirmed());
    }

    #[test]
    fn unconfirmed_keeps_state_and_records_time() {
        let mut store = ExecutionStore::new();
        store.apply_snapshot(&empty_snapshot(50), &["orders"]);
        store.apply(order("o1", "filled", "2026-09-18T10:00:00Z"));
        let t = SystemTime::now();
        store.mark_unconfirmed(t);
        assert!(!store.is_confirmed());
        assert_eq!(store.last_confirmed_at(), Some(t));
        assert_eq!(store.counts().orders, 1);
    }
}
