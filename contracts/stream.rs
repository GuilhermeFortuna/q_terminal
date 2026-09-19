// GENERATED FILE - DO NOT EDIT. Source schemas: schema/stream/control/cursor-expired.schema.json, schema/stream/control/epoch-changed.schema.json, schema/stream/control/lagging.schema.json, schema/stream/control/rejected.schema.json, schema/stream/control/subscribe.schema.json, schema/stream/control/subscribed.schema.json, schema/stream/envelope.schema.json, schema/stream/payloads/execution-common.schema.json, schema/stream/payloads/execution-decision.schema.json, schema/stream/payloads/execution-deployment.schema.json, schema/stream/payloads/execution-fill.schema.json, schema/stream/payloads/execution-ledger.schema.json, schema/stream/payloads/execution-order.schema.json, schema/stream/payloads/execution-risk.schema.json, schema/stream/payloads/job-progress.schema.json, schema/stream/payloads/job-terminal.schema.json, schema/stream/replay/execution-snapshot.schema.json, schema/stream/replay/history-expired.schema.json, schema/stream/replay/history-page.schema.json, schema/stream/replay/latest.schema.json, schema/stream/replay/watermark.schema.json
use serde::{Deserialize, Serialize};

pub type BrokerMode = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CursorExpiredFrame {
    pub cursor: Option<String>,
    pub topic: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

pub type Decimal = String;

pub type DecisionOutcome = String;

pub type DeploymentLifecycle = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpochChangedFrame {
    pub new_epoch: String,
    pub previous_epoch: Option<String>,
    pub topic: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionAccount {
    pub cash_balance: Decimal,
    pub created_at: Option<String>,
    pub currency: String,
    pub id: UUID,
    pub initial_balance: Decimal,
    pub name: String,
    pub realized_pnl: Decimal,
    pub risk_config: Option<serde_json::Value>,
    pub sizing_config: Option<serde_json::Value>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCommon {
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionControl {
    pub kill_switch_enabled: bool,
    pub kill_switch_reason: Option<serde_json::Value>,
    pub updated_at: String,
    pub updated_by: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDecisionState {
    pub account_id: UUID,
    pub bar_close_time: String,
    pub compiled_config: Option<serde_json::Value>,
    pub config_hash: String,
    pub context: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub deployment_id: UUID,
    pub entity: String,
    pub id: UUID,
    pub outcome: DecisionOutcome,
    pub reason: Option<serde_json::Value>,
    pub requested_quantity: Option<Decimal>,
    pub risk_config: Option<serde_json::Value>,
    pub signal_action: SignalAction,
    pub sizing_config: Option<serde_json::Value>,
    pub strategy_name: String,
    pub strategy_version: i64,
    pub symbol: String,
    pub timeframe: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionDeploymentState {
    pub account_id: UUID,
    pub broker_mode: BrokerMode,
    pub compiled_config: Option<serde_json::Value>,
    pub config_hash: String,
    pub created_at: Option<String>,
    pub deployment_id: UUID,
    pub entity: String,
    pub id: UUID,
    pub last_bar_close_time: Option<serde_json::Value>,
    pub lifecycle: DeploymentLifecycle,
    pub live_activation_enabled: bool,
    pub name: String,
    pub paper_account_id: Option<UUID>,
    pub pending_action: Option<serde_json::Value>,
    pub pending_action_requested_at: Option<serde_json::Value>,
    pub risk_config: Option<serde_json::Value>,
    pub sizing_config: Option<serde_json::Value>,
    pub started_at: Option<serde_json::Value>,
    pub stopped_at: Option<serde_json::Value>,
    pub strategy_name: String,
    pub strategy_version: i64,
    pub symbol: String,
    pub timeframe: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionFillEvent {
    pub account_id: UUID,
    pub broker_mode: BrokerMode,
    pub created_at: Option<String>,
    pub deployment_id: UUID,
    pub details: Option<serde_json::Value>,
    pub entity: String,
    pub external_fill_id: String,
    pub fee: Decimal,
    pub filled_at: String,
    pub id: UUID,
    pub order_id: UUID,
    pub position_after: ExecutionPosition,
    pub price: Decimal,
    pub quantity: Decimal,
    pub quote_ask: Option<Decimal>,
    pub quote_bid: Option<Decimal>,
    pub quote_timestamp: Option<serde_json::Value>,
    pub side: ExecutionSide,
    pub slippage: Decimal,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionLedgerEvent {
    pub account_after: ExecutionAccount,
    pub account_id: UUID,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub created_at: Option<String>,
    pub deployment_id: Option<UUID>,
    pub description: Option<serde_json::Value>,
    pub entity: String,
    pub entry_type: LedgerEntryType,
    pub fill_id: Option<UUID>,
    pub id: UUID,
    pub paper_account_id: Option<UUID>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionOrderState {
    pub account_id: UUID,
    pub broker_mode: BrokerMode,
    pub completed_at: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub decision_id: Option<UUID>,
    pub deployment_id: UUID,
    pub details: Option<serde_json::Value>,
    pub entity: String,
    pub external_order_id: Option<serde_json::Value>,
    pub id: UUID,
    pub intent_committed_at: Option<serde_json::Value>,
    pub intent_id: UUID,
    pub order_type: ExecutionOrderType,
    pub quantity: Decimal,
    pub reconciled_at: Option<serde_json::Value>,
    pub reconciled_by: Option<serde_json::Value>,
    pub reconciliation_attempted_at: Option<serde_json::Value>,
    pub reconciliation_detail: Option<serde_json::Value>,
    pub reconciliation_error: Option<serde_json::Value>,
    pub reconciliation_state: ReconciliationState,
    pub rejection_reason: Option<serde_json::Value>,
    pub side: ExecutionSide,
    pub status: ExecutionOrderStatus,
    pub submitted_at: Option<serde_json::Value>,
    pub updated_at: String,
}

pub type ExecutionOrderStatus = String;

pub type ExecutionOrderType = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionPosition {
    pub average_entry_price: Option<Decimal>,
    pub closed_at: Option<serde_json::Value>,
    pub deployment_id: UUID,
    pub id: UUID,
    pub is_open: bool,
    pub opened_at: Option<serde_json::Value>,
    pub quantity: Decimal,
    pub side: PositionSide,
    pub updated_at: String,
}

pub type ExecutionRiskEvent = serde_json::Value;

pub type ExecutionSide = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub accounts: Vec<ExecutionAccount>,
    pub control: ExecutionControl,
    pub deployments: Vec<ExecutionDeploymentState>,
    pub limits: serde_json::Value,
    pub orders: Vec<ExecutionOrderState>,
    pub positions: Vec<ExecutionPosition>,
    pub recent: serde_json::Value,
    pub watermark: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryExpiredResponse {
    pub oldest_available_seq: Option<serde_json::Value>,
    pub requested_from_seq: i64,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryPageResponse {
    pub entries: Vec<StreamEnvelope>,
    pub epoch: String,
    pub next_seq: serde_json::Value,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobProgressPayload {
    pub job_id: String,
    pub kind: String,
    pub message: Option<String>,
    pub progress: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobTerminalPayload {
    pub error: Option<String>,
    pub finished_at: String,
    pub job_id: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaggingFrame {
    pub from_seq: i64,
    pub topic: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatestValuesResponse {
    pub entries: serde_json::Value,
    pub topic: String,
}

pub type LedgerEntryType = String;

pub type PositionSide = String;

pub type ReconciliationState = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RejectedFrame {
    pub detail: Option<String>,
    pub reason: String,
    pub topic: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

pub type RiskRejectionCode = String;

pub type SignalAction = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotWatermark {
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamEnvelope {
    pub epoch: String,
    pub key: Option<serde_json::Value>,
    pub origin_ts: String,
    pub payload: serde_json::Value,
    pub payload_kind: String,
    pub payload_schema: String,
    pub producer_id: String,
    pub schema_major: i64,
    pub seq: i64,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeFrame {
    pub cursors: Option<serde_json::Value>,
    pub topics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribedFrame {
    pub topics: serde_json::Value,
    #[serde(rename = "type")]
    pub r#type: String,
}

pub type UUID = String;
