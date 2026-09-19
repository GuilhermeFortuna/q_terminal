//! A scriptable model of the backend's execution stream, for the fake server (Q-046).
//!
//! It keeps the durable log of every execution topic and derives the execution snapshot
//! from that log, independently of the terminal's store. The snapshot is the oracle the
//! tests compare the store against.

use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

const ACCOUNT: &str = "22222222-2222-2222-2222-222222222222";

/// An RFC 3339 UTC timestamp that increases with `n`, so `updated_at` orders like `seq`.
pub fn ts(n: i64) -> String {
    let n = n.max(0);
    format!(
        "2026-09-18T{:02}:{:02}:{:02}Z",
        (n / 3600) % 24,
        (n / 60) % 60,
        n % 60
    )
}

pub fn deployment(id: &str, lifecycle: &str) -> Value {
    json!({"entity": "deployment", "id": id, "deployment_id": id, "account_id": ACCOUNT,
           "broker_mode": "paper", "config_hash": "abc", "lifecycle": lifecycle,
           "live_activation_enabled": false, "name": format!("dep-{id}"),
           "strategy_name": "momentum_alpha", "strategy_version": 1,
           "symbol": "PETR4", "timeframe": "1m", "updated_at": ts(0)})
}

pub fn decision(id: &str, deployment_id: &str) -> Value {
    json!({"entity": "decision", "id": id, "deployment_id": deployment_id,
           "account_id": ACCOUNT, "bar_close_time": ts(0), "config_hash": "abc",
           "outcome": "order_filled", "signal_action": "buy", "requested_quantity": "1.0",
           "strategy_name": "momentum_alpha", "strategy_version": 1, "symbol": "PETR4",
           "timeframe": "1m", "updated_at": ts(0)})
}

pub fn order(id: &str, deployment_id: &str, status: &str) -> Value {
    json!({"entity": "order", "id": id, "deployment_id": deployment_id,
           "account_id": ACCOUNT, "intent_id": format!("intent-{id}"),
           "broker_mode": "paper", "order_type": "market", "side": "buy",
           "quantity": "1.0", "status": status, "reconciliation_state": "not_required",
           "updated_at": ts(0)})
}

pub fn fill(id: &str, deployment_id: &str, quantity_after: &str, open: bool) -> Value {
    json!({"entity": "fill", "id": id, "deployment_id": deployment_id,
           "account_id": ACCOUNT, "order_id": format!("order-{id}"), "broker_mode": "paper",
           "external_fill_id": format!("ext-{id}"), "side": "buy", "quantity": "1.0",
           "price": "10.00", "fee": "0.10", "slippage": "0.01", "filled_at": ts(0),
           "position_after": {"id": format!("pos-{deployment_id}"),
               "deployment_id": deployment_id, "side": "long",
               "quantity": quantity_after, "average_entry_price": "10.00",
               "is_open": open, "updated_at": ts(0)},
           "updated_at": ts(0)})
}

pub fn ledger(id: &str, balance: &str) -> Value {
    json!({"entity": "ledger", "id": id, "account_id": ACCOUNT, "entry_type": "fee",
           "amount": "-0.10", "balance_after": balance, "updated_at": ts(0),
           "account_after": {"id": ACCOUNT, "name": "paper", "currency": "BRL",
               "initial_balance": "1000.00", "cash_balance": balance,
               "realized_pnl": "0.00", "updated_at": ts(0)}})
}

pub fn risk_rejection(id: &str, deployment_id: &str) -> Value {
    json!({"entity": "risk", "kind": "risk_rejection", "id": id,
           "deployment_id": deployment_id, "account_id": ACCOUNT,
           "rejection_code": "insufficient_equity", "message": "no margin",
           "updated_at": ts(0)})
}

pub fn kill_switch(id: &str, enabled: bool) -> Value {
    json!({"entity": "risk", "kind": "kill_switch", "id": id, "deployment_id": null,
           "account_id": null, "kill_switch_enabled": enabled,
           "kill_switch_reason": "manual", "updated_by": "ops", "updated_at": ts(0)})
}

#[derive(Debug, Default)]
pub struct ExecFake {
    log: HashMap<String, Vec<(i64, Value)>>,
    limit: usize,
}

fn str_of(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

impl ExecFake {
    pub fn new() -> Self {
        Self {
            log: HashMap::new(),
            limit: 50,
        }
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    pub fn last_seq(&self, topic: &str) -> i64 {
        self.log
            .get(topic)
            .and_then(|l| l.last())
            .map(|(s, _)| *s)
            .unwrap_or(0)
    }

    /// Appends to the topic's log, stamping `updated_at` from the assigned sequence.
    pub fn append(&mut self, topic: &str, mut payload: Value) -> (i64, Value) {
        let seq = self.last_seq(topic) + 1;
        let stamp = ts(seq);
        payload["updated_at"] = json!(stamp);
        if topic == "fills" {
            payload["filled_at"] = json!(stamp);
            payload["position_after"]["updated_at"] = json!(stamp);
        }
        if topic == "ledger" {
            payload["account_after"]["updated_at"] = json!(stamp);
        }
        self.log
            .entry(topic.to_string())
            .or_default()
            .push((seq, payload.clone()));
        (seq, payload)
    }

    pub fn entry(&self, topic: &str, seq: i64) -> Option<Value> {
        self.log
            .get(topic)?
            .iter()
            .find(|(s, _)| *s == seq)
            .map(|(_, v)| v.clone())
    }

    pub fn history(&self, topic: &str, from_seq: i64, limit: usize) -> Vec<(i64, Value)> {
        self.log
            .get(topic)
            .map(|l| {
                l.iter()
                    .filter(|(s, _)| *s >= from_seq)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn has_topic(topic: &str) -> bool {
        crate::stream::policy::EXECUTION_TOPICS.contains(&topic)
    }

    /// The latest version of each entity on a topic, ordered by the sequence that last
    /// touched it.
    fn latest(&self, topic: &str, id_of: impl Fn(&Value) -> String) -> Vec<(i64, Value)> {
        let mut by_id: BTreeMap<String, (i64, Value)> = BTreeMap::new();
        for (seq, v) in self.log.get(topic).map(|l| l.as_slice()).unwrap_or(&[]) {
            by_id.insert(id_of(v), (*seq, v.clone()));
        }
        let mut out: Vec<_> = by_id.into_values().collect();
        out.sort_by_key(|(s, _)| *s);
        out
    }

    fn recent(&self, topic: &str, key: impl Fn(&Value) -> String) -> Vec<Value> {
        let mut per_dep: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for (_, v) in self.latest(topic, |v| str_of(v, "id")) {
            per_dep.entry(key(&v)).or_default().push(v);
        }
        let mut out = Vec::new();
        for (_, list) in per_dep {
            let skip = list.len().saturating_sub(self.limit);
            out.extend(list.into_iter().skip(skip));
        }
        out
    }

    /// The execution snapshot the backend would serve now.
    pub fn snapshot(&self, epoch: &str) -> Value {
        let deployments: Vec<Value> = self
            .latest("deployments", |v| str_of(v, "id"))
            .into_iter()
            .map(|(_, v)| v)
            .collect();
        let accounts: Vec<Value> = self
            .latest("ledger", |v| str_of(&v["account_after"], "id"))
            .into_iter()
            .map(|(_, v)| v["account_after"].clone())
            .collect();
        let positions: Vec<Value> = self
            .latest("fills", |v| str_of(v, "deployment_id"))
            .into_iter()
            .map(|(_, v)| v["position_after"].clone())
            .filter(|p| p["is_open"].as_bool().unwrap_or(false))
            .collect();
        let control = self
            .latest("risk", |v| str_of(v, "kind"))
            .into_iter()
            .find(|(_, v)| v["kind"] == "kill_switch")
            .map(|(_, v)| {
                json!({"kill_switch_enabled": v["kill_switch_enabled"],
                       "kill_switch_reason": v["kill_switch_reason"],
                       "updated_by": v["updated_by"], "updated_at": v["updated_at"]})
            })
            .unwrap_or_else(|| {
                json!({"kill_switch_enabled": false, "kill_switch_reason": null,
                       "updated_by": null, "updated_at": "2026-09-18T09:00:00Z"})
            });
        let mut watermark = serde_json::Map::new();
        for topic in crate::stream::policy::EXECUTION_TOPICS {
            watermark.insert(
                topic.to_string(),
                json!({"epoch": epoch, "seq": self.last_seq(topic)}),
            );
        }
        json!({
            "deployments": deployments,
            "accounts": accounts,
            "positions": positions,
            "orders": self.recent("orders", |v| str_of(v, "deployment_id")),
            "recent": {
                "decisions": self.recent("decisions", |v| str_of(v, "deployment_id")),
                "fills": self.recent("fills", |v| str_of(v, "deployment_id")),
                "risk": self.recent("risk", |v| str_of(v, "deployment_id")),
            },
            "control": control,
            "limits": {"recent_decisions": self.limit, "recent_fills": self.limit,
                       "recent_orders": self.limit, "recent_risk": self.limit},
            "watermark": Value::Object(watermark),
        })
    }
}
