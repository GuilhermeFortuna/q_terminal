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
        } else if path.ends_with("/deployments") {
            self.deployments_created += 1;
            (
                200,
                json!({
                    "id": format!("dep-{:04}", self.deployments_created),
                    "name": parsed_body.get("name").cloned().unwrap_or(json!("new")),
                    "lifecycle": "stopped",
                    "broker_mode": parsed_body.get("broker_mode").cloned().unwrap_or(json!("paper")),
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
