//! Scripted command routes for the fake server (Q-048).

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

type CommandResponse = Option<(u16, Vec<(String, String)>, String)>;

#[derive(Debug, Clone)]
pub struct CommandLogEntry {
    pub method: String,
    pub path: String,
    pub key: Uuid,
    pub body: Value,
}

#[derive(Debug, Clone, Default)]
pub enum CommandScript {
    #[default]
    Accept,
    Refuse {
        status: u16,
        code: String,
        message: String,
    },
    InProgressFirst,
}

#[derive(Debug, Default)]
pub struct CommandFakeState {
    pub script: CommandScript,
    pub transport_failures_remaining: usize,
    pub delay_accept_ms: u64,
    keys: HashMap<Uuid, StoredCommand>,
    log: Vec<CommandLogEntry>,
    accounts_created: usize,
    deployments_created: usize,
}

#[derive(Debug, Clone)]
struct StoredCommand {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    complete: bool,
}

impl CommandFakeState {
    pub fn refuse_lifecycle(code: &str) -> Self {
        Self {
            script: CommandScript::Refuse {
                status: 409,
                code: code.to_string(),
                message: format!("cannot transition: {code}"),
            },
            ..Default::default()
        }
    }

    pub fn set_transport_failures(&mut self, n: usize) {
        self.transport_failures_remaining = n;
    }

    pub fn should_fail_transport(&mut self) -> bool {
        if self.transport_failures_remaining > 0 {
            self.transport_failures_remaining -= 1;
            return true;
        }
        false
    }

    pub fn log(&self) -> &[CommandLogEntry] {
        &self.log
    }

    pub fn handle(
        &mut self,
        method: &str,
        path: &str,
        key: Option<Uuid>,
        body: &str,
    ) -> CommandResponse {
        if method == "GET" && path.contains("/backtests") {
            let body = json!({
                "items": [
                    {
                        "id": "run-001",
                        "strategy_name": "MACrossover",
                        "symbol": "WIN$N",
                        "timeframe": "1m",
                        "saved_at": "2026-09-18T12:00:00Z"
                    }
                ]
            })
            .to_string();
            return Some((200, vec![], body));
        }

        if method == "GET" && path.ends_with("/strategy-catalog") {
            let body = json!({
                "strategies": [
                    {
                        "name": "MovingAverageCross",
                        "label": "Moving Average Cross",
                        "description": "Dual moving average crossover trend strategy",
                        "source_kind": "builtin",
                        "strategy_type": "candle",
                        "base_strategy_name": "MovingAverageCross",
                        "default_timeframe": "15m",
                        "default_symbol": "WIN$N",
                        "params": [
                            {
                                "name": "fast_period",
                                "label": "Fast Period",
                                "type": "int",
                                "default": 10,
                                "min": 2.0,
                                "max": 100.0,
                                "step": 1.0,
                                "choices": null,
                                "hint": "Lookback for fast moving average",
                                "exit_group": null
                            },
                            {
                                "name": "slow_period",
                                "label": "Slow Period",
                                "type": "int",
                                "default": 30,
                                "min": 5.0,
                                "max": 200.0,
                                "step": 1.0,
                                "choices": null,
                                "hint": "Lookback for slow moving average",
                                "exit_group": null
                            },
                            {
                                "name": "stop_loss_pct",
                                "label": "Stop Loss (%)",
                                "type": "float",
                                "default": 1.5,
                                "min": 0.1,
                                "max": 10.0,
                                "step": 0.1,
                                "choices": null,
                                "hint": "Stop loss percentage",
                                "exit_group": "risk"
                            }
                        ]
                    },
                    {
                        "name": "CustomMACrossover",
                        "label": "Custom MA Wrapper",
                        "description": "User saved custom MA wrapper",
                        "source_kind": "custom",
                        "strategy_type": "candle",
                        "base_strategy_name": "MovingAverageCross",
                        "params": [
                            {
                                "name": "fast_period",
                                "label": "Fast Period",
                                "type": "int",
                                "default": 8,
                                "min": 2.0,
                                "max": 50.0,
                                "step": 1.0,
                                "choices": null,
                                "hint": "Fast lookback",
                                "exit_group": null
                            }
                        ]
                    },
                    {
                        "name": "TickScalper",
                        "label": "Tick Scalper",
                        "description": "High frequency tick scalper",
                        "source_kind": "builtin",
                        "strategy_type": "tick",
                        "base_strategy_name": "TickScalper",
                        "params": []
                    }
                ]
            })
            .to_string();
            return Some((200, vec![], body));
        }

        if method == "GET" && path.contains("/symbols/search") {
            let body = json!([
                {
                    "symbol": "WIN$N",
                    "name": "Mini Ibovespa Futuro",
                    "exchange": "B3",
                    "assetClass": "FUTURES"
                },
                {
                    "symbol": "PETR4",
                    "name": "Petroleo Brasileiro SA",
                    "exchange": "B3",
                    "assetClass": "EQUITY"
                },
                {
                    "symbol": "VALE3",
                    "name": "Vale SA",
                    "exchange": "B3",
                    "assetClass": "EQUITY"
                }
            ])
            .to_string();
            return Some((200, vec![], body));
        }

        if method == "GET" && path.contains("/performance/marks") {
            let body = json!({
                "items": [
                    {
                        "bar_close_time": "2026-09-27T12:00:00Z",
                        "config_revision": 1,
                        "mark_status": "marked",
                        "quote_bid": "100.50",
                        "quote_ask": "100.55",
                        "quote_timestamp": "2026-09-27T12:00:01Z",
                        "quote_source": "bbo",
                        "mark_price": "100.50",
                        "realized_pnl": "150.00",
                        "fees": "12.50",
                        "unrealized_pnl": "50.00",
                        "equity_delta": "37.50"
                    }
                ],
                "total": 1,
                "limit": 50,
                "offset": 0
            })
            .to_string();
            return Some((200, vec![], body));
        }

        if method == "GET" && path.ends_with("/performance") {
            let body = json!({
                "deployment_id": "dep-0001",
                "realized_pnl": "150.00",
                "unrealized_pnl": "50.00",
                "fees": "12.50",
                "net_pnl": "187.50",
                "closed_trade_count": 5,
                "win_count": 3,
                "win_rate": "0.60",
                "mark_status": "marked",
                "marked_at": "2026-09-27T12:00:00Z",
                "config_revision": 1
            })
            .to_string();
            return Some((200, vec![], body));
        }

        if method == "GET"
            && path.contains("/api/v1/execution/deployments/")
            && !path.ends_with("/actions")
            && !path.ends_with("/chart")
            && !path.ends_with("/performance")
            && !path.contains("/performance/")
            && !path.ends_with("/orders")
            && !path.ends_with("/fills")
            && !path.ends_with("/decisions")
            && !path.ends_with("/risk-events")
            && !path.ends_with("/ledger")
        {
            let id = path.rsplit('/').next().unwrap_or("dep-0001");
            let body = json!({
                "id": id,
                "paper_account_id": "acc-0001",
                "name": "Moving Average Cross WIN$N",
                "broker_mode": "paper",
                "lifecycle": "paused",
                "strategy_name": "MovingAverageCross",
                "strategy_version": 1,
                "config_hash": "abcdef1234567890",
                "config_revision": 1,
                "source_kind": "builtin",
                "symbol": "WIN$N",
                "timeframe": "15m",
                "live_activation_enabled": false,
                "created_at": "2026-09-27T10:00:00Z",
                "updated_at": "2026-09-27T10:00:00Z",
                "compiled_config": {
                    "strategy_params": {
                        "fast_period": 10,
                        "slow_period": 30
                    },
                    "exit_params": {
                        "stop_loss_pct": 1.5
                    }
                },
                "sizing_config": {
                    "quantity": "1"
                },
                "risk_config": {
                    "max_daily_loss": "5000"
                },
                "paper_cost_config": {
                    "point_value": "1.0",
                    "slippage_points": "0.0",
                    "cost_per_contract": "0.0",
                    "cost_bps": "0.0"
                },
                "unknown_order_count": 0
            })
            .to_string();
            return Some((200, vec![], body));
        }

        let key = key?;
        let parsed_body: Value = serde_json::from_str(body).unwrap_or(json!({}));

        if let Some(stored) = self.keys.get(&key) {
            if stored.complete {
                let mut headers = stored.headers.clone();
                headers.push(("Idempotency-Replayed".into(), "true".into()));
                return Some((stored.status, headers, stored.body.clone()));
            }
        }

        self.log.push(CommandLogEntry {
            method: method.to_string(),
            path: path.to_string(),
            key,
            body: parsed_body.clone(),
        });

        if matches!(self.script, CommandScript::InProgressFirst) && !self.keys.contains_key(&key) {
            self.keys.insert(
                key,
                StoredCommand {
                    status: 409,
                    headers: vec![("Retry-After".into(), "1".into())],
                    body: json!({"code": "idempotency_in_progress"}).to_string(),
                    complete: false,
                },
            );
            self.script = CommandScript::Accept;
            return Some((
                409,
                vec![("Retry-After".into(), "1".into())],
                json!({"code": "idempotency_in_progress"}).to_string(),
            ));
        }

        if let CommandScript::Refuse {
            status,
            code,
            message,
        } = &self.script
        {
            let body = json!({"code": code, "message": message}).to_string();
            self.keys.insert(
                key,
                StoredCommand {
                    status: *status,
                    headers: vec![],
                    body: body.clone(),
                    complete: true,
                },
            );
            return Some((*status, vec![], body));
        }

        let (status, response_body) = if path.ends_with("/accounts") {
            self.accounts_created += 1;
            (
                200,
                json!({
                    "id": format!("acc-{:04}", self.accounts_created),
                    "name": parsed_body.get("name").cloned().unwrap_or(json!("paper")),
                    "currency": "BRL",
                    "initial_balance": "10000",
                    "cash_balance": "10000",
                }),
            )
        } else if path.contains("/configuration") {
            let rev = parsed_body
                .get("expected_revision")
                .and_then(|v| v.as_i64())
                .unwrap_or(1);
            (
                200,
                json!({
                    "id": "dep-0001",
                    "name": "edited-deployment",
                    "lifecycle": "paused",
                    "broker_mode": "paper",
                    "config_revision": rev + 1,
                    "compiled_config": {},
                    "sizing_config": parsed_body.get("sizing_config").cloned().unwrap_or(json!({"quantity": "1"})),
                    "risk_config": parsed_body.get("risk_config").cloned().unwrap_or(json!({})),
                    "paper_cost_config": parsed_body.get("paper_cost_config").cloned().unwrap_or(json!({})),
                }),
            )
        } else if path.ends_with("/deployments") {
            self.deployments_created += 1;
            let cat = parsed_body.get("catalog");
            let sym = cat
                .and_then(|c| c.get("symbol"))
                .and_then(|s| s.as_str())
                .unwrap_or("WIN$N");
            let tf = cat
                .and_then(|c| c.get("timeframe"))
                .and_then(|s| s.as_str())
                .unwrap_or("15m");
            (
                200,
                json!({
                    "id": format!("dep-{:04}", self.deployments_created),
                    "name": parsed_body.get("name").cloned().unwrap_or(json!("new")),
                    "lifecycle": "stopped",
                    "broker_mode": parsed_body.get("broker_mode").cloned().unwrap_or(json!("paper")),
                    "symbol": sym,
                    "timeframe": tf,
                    "config_revision": 1,
                }),
            )
        } else if path.contains("/actions") {
            let action = parsed_body
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("start");
            (
                200,
                json!({
                    "accepted": true,
                    "lifecycle": if action == "stop" { "stopped" } else { "running" },
                    "pending_action": if action == "flatten" { json!("flatten") } else { json!(null) },
                    "message": format!("{action} accepted"),
                }),
            )
        } else if path.ends_with("/kill-switch") {
            (
                200,
                json!({
                    "accepted": true,
                    "kill_switch": {
                        "enabled": parsed_body.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                    },
                }),
            )
        } else if path.contains("/resolve") {
            (
                200,
                json!({
                    "accepted": true,
                    "resolution": parsed_body.get("outcome").cloned().unwrap_or(json!("not_filled")),
                    "message": "resolved",
                }),
            )
        } else if path.contains("/backtests") {
            (
                200,
                json!({
                    "items": [
                        {
                            "id": "run-001",
                            "strategy_name": "MACrossover",
                            "symbol": "WIN$N",
                            "timeframe": "1m",
                            "saved_at": "2026-09-18T12:00:00Z"
                        }
                    ]
                }),
            )
        } else {
            (404, json!({"code": "not_found"}))
        };

        let body_str = response_body.to_string();
        self.keys.insert(
            key,
            StoredCommand {
                status,
                headers: vec![],
                body: body_str.clone(),
                complete: true,
            },
        );
        Some((status, vec![], body_str))
    }
}

pub struct CommandCallCounter(pub AtomicUsize);

impl CommandCallCounter {
    pub fn inc(&self) -> usize {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn get(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

pub fn parse_idempotency_key(headers_block: &str) -> Option<Uuid> {
    for line in headers_block.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("idempotency-key:") {
            let val = line.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            return Uuid::parse_str(val).ok();
        }
    }
    None
}

pub fn parse_body_from_request(request: &str) -> String {
    if let Some(idx) = request.find("\r\n\r\n") {
        return request[idx + 4..].to_string();
    }
    if let Some(idx) = request.find("\n\n") {
        return request[idx + 2..].to_string();
    }
    String::new()
}
