//! REST command client with idempotency-key retries (Q-048 / Q-044).

use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ManualFill {
    pub price: String,
    pub quantity: String,
    pub filled_at: String,
    pub fee: Option<String>,
    pub external_fill_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Command {
    CreateAccount {
        name: String,
        initial_balance: String,
        currency: String,
    },
    CreateDeployment {
        paper_account_id: String,
        name: String,
        broker_mode: String,
        source_backtest_run_id: String,
    },
    Lifecycle {
        deployment_id: String,
        action: String,
        confirm: bool,
        actor: String,
    },
    KillSwitch {
        enabled: bool,
        confirm: bool,
        reason: Option<String>,
        actor: String,
    },
    Resolve {
        order_id: String,
        outcome: String,
        actor: String,
        reason: String,
        fill: Option<ManualFill>,
    },
}

#[derive(Debug, Clone)]
pub struct CommandOutcome {
    pub key: Uuid,
    pub status: u16,
    pub body: Value,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub enum CommandError {
    Transport(String),
    InvalidResponse(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(msg) => write!(f, "transport error: {msg}"),
            Self::InvalidResponse(msg) => write!(f, "invalid response: {msg}"),
        }
    }
}

impl std::error::Error for CommandError {}

pub struct CommandClient {
    client: reqwest::Client,
    api_base: String,
}

impl CommandClient {
    pub fn new(api_base: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base: api_base.trim_end_matches('/').to_string(),
        }
    }

    pub async fn send(
        &self,
        action_key: Uuid,
        cmd: &Command,
    ) -> Result<CommandOutcome, CommandError> {
        let (method, path, body) = route_for(cmd);
        let url = format!("{}/{path}", self.api_base);
        let body_str = body.to_string();

        let mut attempt = 0u32;
        while attempt < 3 {
            attempt += 1;
            let resp = match self
                .client
                .request(method.clone(), &url)
                .header("Idempotency-Key", action_key.to_string())
                .header("Content-Type", "application/json")
                .body(body_str.clone())
                .send()
                .await
            {
                Ok(r) => r,
                Err(err) => {
                    if attempt < 3 {
                        tokio::time::sleep(backoff(attempt)).await;
                        continue;
                    }
                    return Err(CommandError::Transport(err.to_string()));
                }
            };

            let status = resp.status().as_u16();
            let replayed = resp
                .headers()
                .get("idempotency-replayed")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            let text = resp.text().await.unwrap_or_default();
            if status == 409 {
                let err_code = serde_json::from_str::<Value>(&text)
                    .ok()
                    .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(String::from))
                    .unwrap_or_default();
                if err_code == "idempotency_in_progress" && attempt < 3 {
                    let wait = retry_after.unwrap_or(1).min(2);
                    tokio::time::sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                let body_json = parse_body(&text);
                return Ok(CommandOutcome {
                    key: action_key,
                    status,
                    body: body_json,
                    replayed: false,
                });
            }
            let body_json = parse_body(&text);
            return Ok(CommandOutcome {
                key: action_key,
                status,
                body: body_json,
                replayed,
            });
        }

        Err(CommandError::Transport(
            "exhausted idempotency retries".to_string(),
        ))
    }

    pub async fn fetch_saved_runs(&self) -> Result<Vec<Value>, CommandError> {
        let url = format!(
            "{}/api/v1/backtests?saved_only=true&limit=200",
            self.api_base
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CommandError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(CommandError::InvalidResponse(text));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| CommandError::InvalidResponse(e.to_string()))?;
        let items = body
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(items)
    }
}

fn route_for(cmd: &Command) -> (reqwest::Method, String, Value) {
    match cmd {
        Command::CreateAccount {
            name,
            initial_balance,
            currency,
        } => (
            reqwest::Method::POST,
            "api/v1/execution/accounts".to_string(),
            json!({
                "name": name,
                "initial_balance": initial_balance,
                "currency": currency,
            }),
        ),
        Command::CreateDeployment {
            paper_account_id,
            name,
            broker_mode,
            source_backtest_run_id,
        } => (
            reqwest::Method::POST,
            "api/v1/execution/deployments".to_string(),
            json!({
                "paper_account_id": paper_account_id,
                "name": name,
                "broker_mode": broker_mode,
                "source_backtest_run_id": source_backtest_run_id,
            }),
        ),
        Command::Lifecycle {
            deployment_id,
            action,
            confirm,
            actor,
        } => (
            reqwest::Method::POST,
            format!("api/v1/execution/deployments/{deployment_id}/actions"),
            json!({
                "action": action,
                "confirm": confirm,
                "actor": actor,
            }),
        ),
        Command::KillSwitch {
            enabled,
            confirm,
            reason,
            actor,
        } => (
            reqwest::Method::PUT,
            "api/v1/execution/kill-switch".to_string(),
            json!({
                "enabled": enabled,
                "confirm": confirm,
                "reason": reason,
                "updated_by": actor,
            }),
        ),
        Command::Resolve {
            order_id,
            outcome,
            actor,
            reason,
            fill,
        } => {
            let mut body = json!({
                "outcome": outcome,
                "actor": actor,
                "reason": reason,
            });
            if let Some(f) = fill {
                body["price"] = json!(f.price);
                body["quantity"] = json!(f.quantity);
                body["filled_at"] = json!(f.filled_at);
                if let Some(fee) = &f.fee {
                    body["fee"] = json!(fee);
                }
                if let Some(ext) = &f.external_fill_id {
                    body["external_fill_id"] = json!(ext);
                }
            }
            (
                reqwest::Method::POST,
                format!("api/v1/execution/orders/{order_id}/resolve"),
                body,
            )
        }
    }
}

fn parse_body(text: &str) -> Value {
    if text.is_empty() {
        return Value::Null;
    }
    serde_json::from_str(text).unwrap_or_else(|_| json!({ "detail": text }))
}

fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(100 * attempt as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::fake_commands::CommandFakeState;
    use crate::stream::fake_server::FakeServer;

    #[tokio::test]
    async fn test_send_uses_one_key_per_action() {
        let server = FakeServer::start().await;
        let client = CommandClient::new(&server.api_base());
        let key = Uuid::new_v4();
        let cmd = Command::CreateAccount {
            name: "test".into(),
            initial_balance: "10000".into(),
            currency: "BRL".into(),
        };
        let out = client.send(key, &cmd).await.unwrap();
        assert_eq!(out.key, key);
        assert_eq!(out.status, 200);
        let log = server.command_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].key, key);
    }

    #[tokio::test]
    async fn test_transport_retry_reuses_key() {
        let server = FakeServer::start().await;
        server.set_command_transport_failures(1).await;
        let client = CommandClient::new(&server.api_base());
        let key = Uuid::new_v4();
        let cmd = Command::KillSwitch {
            enabled: true,
            confirm: true,
            reason: Some("test".into()),
            actor: "ops".into(),
        };
        let out = client.send(key, &cmd).await.unwrap();
        assert_eq!(out.key, key);
        let log = server.command_log();
        assert_eq!(log.len(), 1, "only the successful attempt is logged");
        assert_eq!(log[0].key, key);
    }

    #[tokio::test]
    async fn test_refused_command_surfaces_backend_error() {
        let server = FakeServer::start().await;
        server
            .set_command_script(CommandFakeState::refuse_lifecycle("illegal_transition"))
            .await;
        let client = CommandClient::new(&server.api_base());
        let key = Uuid::new_v4();
        let cmd = Command::Lifecycle {
            deployment_id: "11111111-1111-1111-1111-111111111111".into(),
            action: "start".into(),
            confirm: false,
            actor: "ops".into(),
        };
        let out = client.send(key, &cmd).await.unwrap();
        assert_eq!(out.status, 409);
        assert_eq!(
            out.body.get("code").and_then(|v| v.as_str()),
            Some("illegal_transition")
        );
    }
}
