//! Decoding of the six execution topics' envelope payloads (Q-039).

use crate::contracts_stream::{
    ExecutionDecisionState, ExecutionDeploymentState, ExecutionFillEvent, ExecutionLedgerEvent,
    ExecutionOrderState, ExecutionSnapshot,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionEvent {
    Deployment(ExecutionDeploymentState),
    Decision(ExecutionDecisionState),
    Order(ExecutionOrderState),
    Fill(ExecutionFillEvent),
    /// Risk events carry a `kind` discriminator (`risk_rejection`, `kill_switch`); the
    /// contract leaves them untyped, so the store reads only the fields it needs.
    Risk(serde_json::Value),
    Ledger(ExecutionLedgerEvent),
}

/// What a topic state machine hands to the store: the shared snapshot, or one event.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecPayload {
    Snapshot(Arc<ExecutionSnapshot>),
    Event(ExecutionEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnknownTopic(String),
    Invalid { topic: String, detail: String },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTopic(t) => write!(f, "not an execution topic: {t}"),
            Self::Invalid { topic, detail } => {
                write!(f, "invalid {topic} payload: {detail}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

pub fn decode_execution_entry(
    topic: &str,
    payload: &serde_json::Value,
) -> Result<ExecutionEvent, DecodeError> {
    fn parse<T: serde::de::DeserializeOwned>(
        topic: &str,
        payload: &serde_json::Value,
    ) -> Result<T, DecodeError> {
        serde_json::from_value(payload.clone()).map_err(|e| DecodeError::Invalid {
            topic: topic.to_string(),
            detail: e.to_string(),
        })
    }
    match topic {
        "deployments" => parse(topic, payload).map(ExecutionEvent::Deployment),
        "decisions" => parse(topic, payload).map(ExecutionEvent::Decision),
        "orders" => parse(topic, payload).map(ExecutionEvent::Order),
        "fills" => parse(topic, payload).map(ExecutionEvent::Fill),
        "ledger" => parse(topic, payload).map(ExecutionEvent::Ledger),
        "risk" => {
            if payload.is_object() {
                Ok(ExecutionEvent::Risk(payload.clone()))
            } else {
                Err(DecodeError::Invalid {
                    topic: topic.to_string(),
                    detail: "risk payload is not an object".to_string(),
                })
            }
        }
        other => Err(DecodeError::UnknownTopic(other.to_string())),
    }
}

/// The per-topic `(epoch, seq)` watermarks of a snapshot.
pub fn snapshot_watermark(
    snap: &ExecutionSnapshot,
    topic: &str,
) -> Result<(String, i64), DecodeError> {
    let bad = |detail: &str| DecodeError::Invalid {
        topic: topic.to_string(),
        detail: detail.to_string(),
    };
    let w = snap
        .watermark
        .get(topic)
        .ok_or_else(|| bad("snapshot has no watermark for the topic"))?;
    let epoch = w
        .get("epoch")
        .and_then(|v| v.as_str())
        .ok_or_else(|| bad("watermark epoch"))?;
    let seq = w
        .get("seq")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| bad("watermark seq"))?;
    Ok((epoch.to_string(), seq))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_topic_and_bad_payload_are_errors() {
        assert!(matches!(
            decode_execution_entry("quotes", &json!({})),
            Err(DecodeError::UnknownTopic(_))
        ));
        assert!(matches!(
            decode_execution_entry("orders", &json!({"id": 1})),
            Err(DecodeError::Invalid { .. })
        ));
        assert!(matches!(
            decode_execution_entry("risk", &json!("nope")),
            Err(DecodeError::Invalid { .. })
        ));
        assert!(decode_execution_entry("risk", &json!({"kind": "kill_switch"})).is_ok());
    }
}
