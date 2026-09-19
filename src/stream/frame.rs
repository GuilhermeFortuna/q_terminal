use crate::contracts_stream::{
    CursorExpiredFrame, EpochChangedFrame, LaggingFrame, SubscribedFrame,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedFrame {
    pub reason: String,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeHeader {
    pub topic: String,
    pub schema_major: i64,
    pub seq: i64,
    pub epoch: String,
    pub producer_id: String,
    pub origin_ts: String,
    pub payload_kind: String,
    pub payload_schema: String,
    #[serde(default)]
    pub key: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct BinaryFrame<'a> {
    pub header: EnvelopeHeader,
    pub arrow: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFrame {
    Subscribed(SubscribedFrame),
    Rejected(RejectedFrame),
    CursorExpired(CursorExpiredFrame),
    Lagging(LaggingFrame),
    EpochChanged(EpochChangedFrame),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerFrame {
    Control(ControlFrame),
    Envelope(EnvelopeHeader),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    Truncated(String),
    InvalidUtf8(String),
    InvalidJson(String),
    MissingField(String),
    NotAnObject,
    UnknownControlType(String),
    SchemaMismatch {
        topic: String,
        expected: String,
        actual: String,
    },
    InvalidPayloadKind(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::Truncated(msg) => write!(f, "truncated frame: {msg}"),
            FrameError::InvalidUtf8(msg) => write!(f, "invalid utf-8: {msg}"),
            FrameError::InvalidJson(msg) => write!(f, "invalid json: {msg}"),
            FrameError::MissingField(msg) => write!(f, "missing field: {msg}"),
            FrameError::NotAnObject => write!(f, "frame is not a JSON object"),
            FrameError::UnknownControlType(t) => write!(f, "unknown control type: {t}"),
            FrameError::SchemaMismatch {
                topic,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "schema mismatch for topic {topic}: expected {expected}, got {actual}"
                )
            }
            FrameError::InvalidPayloadKind(k) => write!(f, "invalid payload kind: {k}"),
        }
    }
}

impl std::error::Error for FrameError {}

pub fn topic_expected_payload_schema(topic: &str) -> Option<&'static str> {
    match topic {
        "bars.forming" | "bars.completed" => Some("schema/api/arrow/bars.schema.json"),
        "quotes" => Some("schema/api/arrow/ticks.schema.json"),
        "jobs.progress" => Some("schema/stream/payloads/job-progress.schema.json"),
        "jobs.terminal" => Some("schema/stream/payloads/job-terminal.schema.json"),
        "decisions" => Some("schema/stream/payloads/execution-decision.schema.json"),
        "orders" => Some("schema/stream/payloads/execution-order.schema.json"),
        "fills" => Some("schema/stream/payloads/execution-fill.schema.json"),
        "risk" => Some("schema/stream/payloads/execution-risk.schema.json"),
        "ledger" => Some("schema/stream/payloads/execution-ledger.schema.json"),
        "deployments" => Some("schema/stream/payloads/execution-deployment.schema.json"),
        _ => None,
    }
}

pub fn split_binary(bytes: &[u8]) -> Result<BinaryFrame<'_>, FrameError> {
    if bytes.len() < 4 {
        return Err(FrameError::Truncated(format!(
            "frame length {} < 4 bytes",
            bytes.len()
        )));
    }
    let header_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if bytes.len() < 4 + header_len {
        return Err(FrameError::Truncated(format!(
            "frame length {} < required 4 + header_len {}",
            bytes.len(),
            header_len
        )));
    }
    let header_slice = &bytes[4..4 + header_len];
    let header_json =
        std::str::from_utf8(header_slice).map_err(|e| FrameError::InvalidUtf8(e.to_string()))?;
    let header: EnvelopeHeader =
        serde_json::from_str(header_json).map_err(|e| FrameError::InvalidJson(e.to_string()))?;

    if let Some(expected) = topic_expected_payload_schema(&header.topic) {
        if header.payload_schema != expected {
            return Err(FrameError::SchemaMismatch {
                topic: header.topic.clone(),
                expected: expected.to_string(),
                actual: header.payload_schema.clone(),
            });
        }
    }

    let arrow = &bytes[4 + header_len..];
    Ok(BinaryFrame { header, arrow })
}

pub fn make_binary_frame(header: &EnvelopeHeader, arrow_bytes: &[u8]) -> Vec<u8> {
    let header_json = serde_json::to_string(header).expect("valid header json");
    let header_bytes = header_json.as_bytes();
    let header_len = header_bytes.len() as u32;

    let mut buf = Vec::with_capacity(4 + header_bytes.len() + arrow_bytes.len());
    buf.extend_from_slice(&header_len.to_le_bytes());
    buf.extend_from_slice(header_bytes);
    buf.extend_from_slice(arrow_bytes);
    buf
}

pub fn classify_text(text: &str) -> Result<ServerFrame, FrameError> {
    let val: serde_json::Value =
        serde_json::from_str(text).map_err(|e| FrameError::InvalidJson(e.to_string()))?;
    let obj = val.as_object().ok_or(FrameError::NotAnObject)?;

    if let Some(type_val) = obj.get("type") {
        let type_str = type_val
            .as_str()
            .ok_or_else(|| FrameError::UnknownControlType(type_val.to_string()))?;
        match type_str {
            "subscribed" => {
                let frame: SubscribedFrame = serde_json::from_value(val)
                    .map_err(|e| FrameError::InvalidJson(e.to_string()))?;
                Ok(ServerFrame::Control(ControlFrame::Subscribed(frame)))
            }
            "rejected" => {
                let frame: RejectedFrame = serde_json::from_value(val)
                    .map_err(|e| FrameError::InvalidJson(e.to_string()))?;
                Ok(ServerFrame::Control(ControlFrame::Rejected(frame)))
            }
            "cursor_expired" => {
                let frame: CursorExpiredFrame = serde_json::from_value(val)
                    .map_err(|e| FrameError::InvalidJson(e.to_string()))?;
                Ok(ServerFrame::Control(ControlFrame::CursorExpired(frame)))
            }
            "lagging" => {
                let frame: LaggingFrame = serde_json::from_value(val)
                    .map_err(|e| FrameError::InvalidJson(e.to_string()))?;
                Ok(ServerFrame::Control(ControlFrame::Lagging(frame)))
            }
            "epoch_changed" => {
                let frame: EpochChangedFrame = serde_json::from_value(val)
                    .map_err(|e| FrameError::InvalidJson(e.to_string()))?;
                Ok(ServerFrame::Control(ControlFrame::EpochChanged(frame)))
            }
            other => Err(FrameError::UnknownControlType(other.to_string())),
        }
    } else {
        let header: EnvelopeHeader =
            serde_json::from_value(val).map_err(|e| FrameError::InvalidJson(e.to_string()))?;
        if let Some(expected) = topic_expected_payload_schema(&header.topic) {
            if header.payload_schema != expected {
                return Err(FrameError::SchemaMismatch {
                    topic: header.topic.clone(),
                    expected: expected.to_string(),
                    actual: header.payload_schema.clone(),
                });
            }
        }
        Ok(ServerFrame::Envelope(header))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_header_json(topic: &str, schema: &str, seq: Option<i64>) -> String {
        let mut obj = serde_json::json!({
            "topic": topic,
            "schema_major": 1,
            "epoch": "epoch-1",
            "producer_id": "test-producer",
            "origin_ts": "2026-09-17T12:00:00Z",
            "payload_kind": "arrow_ipc",
            "payload_schema": schema,
            "key": {"symbol": "PETR4", "timeframe": "1m"}
        });
        if let Some(s) = seq {
            obj["seq"] = serde_json::json!(s);
        }
        obj.to_string()
    }

    fn make_binary_frame_bytes(header_json: &str, arrow_bytes: &[u8]) -> Vec<u8> {
        let header_bytes = header_json.as_bytes();
        let header_len = header_bytes.len() as u32;
        let mut bytes = Vec::with_capacity(4 + header_bytes.len() + arrow_bytes.len());
        bytes.extend_from_slice(&header_len.to_le_bytes());
        bytes.extend_from_slice(header_bytes);
        bytes.extend_from_slice(arrow_bytes);
        bytes
    }

    #[test]
    fn test_split_binary_well_formed() {
        let header_json = make_valid_header_json(
            "bars.forming",
            "schema/api/arrow/bars.schema.json",
            Some(42),
        );
        let arrow_data = b"arrow_ipc_stream_bytes";
        let frame_bytes = make_binary_frame_bytes(&header_json, arrow_data);

        let binary_frame = split_binary(&frame_bytes).expect("should split well-formed frame");
        assert_eq!(binary_frame.header.topic, "bars.forming");
        assert_eq!(binary_frame.header.seq, 42);
        assert_eq!(binary_frame.header.schema_major, 1);
        assert_eq!(binary_frame.arrow, arrow_data);
    }

    #[test]
    fn test_split_binary_three_bytes() {
        let bytes = [1, 2, 3];
        let err = split_binary(&bytes).expect_err("3-byte frame must fail");
        assert!(matches!(err, FrameError::Truncated(_)));
    }

    #[test]
    fn test_split_binary_header_len_past_end() {
        let bytes = [100, 0, 0, 0, 1, 2, 3];
        let err = split_binary(&bytes).expect_err("header_len past end must fail");
        assert!(matches!(err, FrameError::Truncated(_)));
    }

    #[test]
    fn test_split_binary_non_json_header() {
        let not_json = b"this is not json at all";
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(not_json.len() as u32).to_le_bytes());
        bytes.extend_from_slice(not_json);
        bytes.extend_from_slice(b"arrow_data");

        let err = split_binary(&bytes).expect_err("non-json header must fail");
        assert!(matches!(err, FrameError::InvalidJson(_)));
    }

    #[test]
    fn test_split_binary_missing_seq() {
        let header_json =
            make_valid_header_json("bars.forming", "schema/api/arrow/bars.schema.json", None);
        let bytes = make_binary_frame_bytes(&header_json, b"arrow_data");

        let err = split_binary(&bytes).expect_err("missing seq must fail");
        assert!(matches!(
            err,
            FrameError::MissingField(_) | FrameError::InvalidJson(_)
        ));
    }

    #[test]
    fn test_split_binary_schema_not_topics_rejected() {
        // bars.forming should have bars.schema.json, not ticks.schema.json
        let header_json = make_valid_header_json(
            "bars.forming",
            "schema/api/arrow/ticks.schema.json",
            Some(1),
        );
        let bytes = make_binary_frame_bytes(&header_json, b"arrow_data");

        let err = split_binary(&bytes).expect_err("schema mismatch must be rejected");
        assert!(matches!(err, FrameError::SchemaMismatch { .. }));
    }

    #[test]
    fn test_classify_text_five_control_values() {
        // 1. Subscribed
        let subscribed_json = r#"{"type": "subscribed", "topics": {"bars.forming": {"cursor": "0", "epoch": "1", "last_seq": 10}}}"#;
        match classify_text(subscribed_json).expect("subscribed") {
            ServerFrame::Control(ControlFrame::Subscribed(_)) => {}
            other => panic!("expected Subscribed, got {:?}", other),
        }

        // 2. Rejected
        let rejected_json = r#"{"type": "rejected", "reason": "stream_unavailable", "topic": "bars.completed", "detail": "unavailable"}"#;
        match classify_text(rejected_json).expect("rejected") {
            ServerFrame::Control(ControlFrame::Rejected(f)) => {
                assert_eq!(f.reason, "stream_unavailable");
                assert_eq!(f.topic.as_deref(), Some("bars.completed"));
            }
            other => panic!("expected Rejected, got {:?}", other),
        }

        // 3. CursorExpired
        let cursor_expired_json =
            r#"{"type": "cursor_expired", "topic": "bars.completed", "cursor": "0-0"}"#;
        match classify_text(cursor_expired_json).expect("cursor_expired") {
            ServerFrame::Control(ControlFrame::CursorExpired(f)) => {
                assert_eq!(f.topic, "bars.completed");
            }
            other => panic!("expected CursorExpired, got {:?}", other),
        }

        // 4. Lagging
        let lagging_json = r#"{"type": "lagging", "topic": "bars.completed", "from_seq": 100}"#;
        match classify_text(lagging_json).expect("lagging") {
            ServerFrame::Control(ControlFrame::Lagging(f)) => {
                assert_eq!(f.topic, "bars.completed");
                assert_eq!(f.from_seq, 100);
            }
            other => panic!("expected Lagging, got {:?}", other),
        }

        // 5. EpochChanged
        let epoch_changed_json = r#"{"type": "epoch_changed", "topic": "bars.completed", "new_epoch": "2", "previous_epoch": "1"}"#;
        match classify_text(epoch_changed_json).expect("epoch_changed") {
            ServerFrame::Control(ControlFrame::EpochChanged(f)) => {
                assert_eq!(f.topic, "bars.completed");
                assert_eq!(f.new_epoch, "2");
            }
            other => panic!("expected EpochChanged, got {:?}", other),
        }
    }

    #[test]
    fn test_classify_text_without_type_is_envelope() {
        let envelope_json =
            make_valid_header_json("bars.forming", "schema/api/arrow/bars.schema.json", Some(5));
        match classify_text(&envelope_json).expect("envelope") {
            ServerFrame::Envelope(header) => {
                assert_eq!(header.topic, "bars.forming");
                assert_eq!(header.seq, 5);
            }
            other => panic!("expected Envelope, got {:?}", other),
        }
    }

    #[test]
    fn test_classify_text_schema_not_topics_rejected() {
        let envelope_json = make_valid_header_json(
            "bars.forming",
            "schema/api/arrow/ticks.schema.json",
            Some(5),
        );
        let err = classify_text(&envelope_json).expect_err("schema mismatch must be rejected");
        assert!(matches!(err, FrameError::SchemaMismatch { .. }));
    }
}
