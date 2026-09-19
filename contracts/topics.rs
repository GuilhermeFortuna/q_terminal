// GENERATED FILE - DO NOT EDIT. Source schemas: schema/stream/topics.yaml
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicPolicy {
    pub name: &'static str,
    pub topic_class: &'static str,
    pub retention_duration: &'static str,
    pub retention_entries: usize,
    pub coalesce_key: &'static [&'static str],
    pub on_overflow: &'static str,
    pub replay: &'static str,
    pub payload_schema: &'static str,
}

pub const TOPICS: &[TopicPolicy] = &[
    TopicPolicy {
        name: "bars.completed",
        topic_class: "ephemeral",
        retention_duration: "P1D",
        retention_entries: 100000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "retention_only",
        payload_schema: "schema/api/arrow/bars.schema.json",
    },
    TopicPolicy {
        name: "bars.forming",
        topic_class: "ephemeral",
        retention_duration: "PT1H",
        retention_entries: 10000,
        coalesce_key: &["symbol", "timeframe"],
        on_overflow: "coalesce",
        replay: "retention_only",
        payload_schema: "schema/api/arrow/bars.schema.json",
    },
    TopicPolicy {
        name: "decisions",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-decision.schema.json",
    },
    TopicPolicy {
        name: "deployments",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-deployment.schema.json",
    },
    TopicPolicy {
        name: "fills",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-fill.schema.json",
    },
    TopicPolicy {
        name: "jobs.progress",
        topic_class: "ephemeral",
        retention_duration: "P1D",
        retention_entries: 50000,
        coalesce_key: &["kind", "job_id"],
        on_overflow: "coalesce",
        replay: "retention_only",
        payload_schema: "schema/stream/payloads/job-progress.schema.json",
    },
    TopicPolicy {
        name: "jobs.terminal",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/job-terminal.schema.json",
    },
    TopicPolicy {
        name: "ledger",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-ledger.schema.json",
    },
    TopicPolicy {
        name: "orders",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-order.schema.json",
    },
    TopicPolicy {
        name: "quotes",
        topic_class: "ephemeral",
        retention_duration: "PT1H",
        retention_entries: 100000,
        coalesce_key: &["symbol"],
        on_overflow: "coalesce",
        replay: "retention_only",
        payload_schema: "schema/api/arrow/ticks.schema.json",
    },
    TopicPolicy {
        name: "risk",
        topic_class: "durable",
        retention_duration: "P1D",
        retention_entries: 200000,
        coalesce_key: &[],
        on_overflow: "lag",
        replay: "unbounded",
        payload_schema: "schema/stream/payloads/execution-risk.schema.json",
    },
];

impl TopicPolicy {
    pub fn get(name: &str) -> Option<&'static TopicPolicy> {
        TOPICS.iter().find(|t| t.name == name)
    }
}
