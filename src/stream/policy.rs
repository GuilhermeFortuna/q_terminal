#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPolicy {
    pub name: &'static str,
    pub topic_class: &'static str,
    pub coalesce_key: &'static [&'static str],
}

pub const EXECUTION_TOPICS: [&str; 6] = [
    "decisions",
    "orders",
    "fills",
    "risk",
    "ledger",
    "deployments",
];

pub const KNOWN_TOPICS: &[TopicPolicy] = &[
    TopicPolicy {
        name: "bars.forming",
        topic_class: "ephemeral",
        coalesce_key: &["symbol", "timeframe"],
    },
    TopicPolicy {
        name: "bars.completed",
        topic_class: "ephemeral",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "decisions",
        topic_class: "durable",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "orders",
        topic_class: "durable",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "fills",
        topic_class: "durable",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "risk",
        topic_class: "durable",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "ledger",
        topic_class: "durable",
        coalesce_key: &[],
    },
    TopicPolicy {
        name: "deployments",
        topic_class: "durable",
        coalesce_key: &[],
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    MissingTopic(String),
    ClassMismatch {
        topic: String,
        expected: &'static str,
        actual: String,
    },
    CoalesceKeyMismatch {
        topic: String,
        expected: Vec<&'static str>,
        actual: Vec<String>,
    },
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTopic(t) => write!(f, "missing expected topic in policy: {t}"),
            Self::ClassMismatch {
                topic,
                expected,
                actual,
            } => write!(
                f,
                "topic {topic} class mismatch: expected {expected}, got {actual}"
            ),
            Self::CoalesceKeyMismatch {
                topic,
                expected,
                actual,
            } => write!(
                f,
                "topic {topic} coalesce key mismatch: expected {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl std::error::Error for PolicyError {}

pub fn assert_topic_policies<'a, I>(policies: I) -> Result<(), PolicyError>
where
    I: IntoIterator<Item = &'a TopicPolicy>,
{
    let policy_list: Vec<&'a TopicPolicy> = policies.into_iter().collect();

    // Verify bars.forming
    let forming = policy_list
        .iter()
        .find(|p| p.name == "bars.forming")
        .ok_or_else(|| PolicyError::MissingTopic("bars.forming".to_string()))?;

    if forming.topic_class != "ephemeral" {
        return Err(PolicyError::ClassMismatch {
            topic: "bars.forming".to_string(),
            expected: "ephemeral",
            actual: forming.topic_class.to_string(),
        });
    }

    if forming.coalesce_key != ["symbol", "timeframe"] {
        return Err(PolicyError::CoalesceKeyMismatch {
            topic: "bars.forming".to_string(),
            expected: vec!["symbol", "timeframe"],
            actual: forming.coalesce_key.iter().map(|s| s.to_string()).collect(),
        });
    }

    // Verify bars.completed
    let completed = policy_list
        .iter()
        .find(|p| p.name == "bars.completed")
        .ok_or_else(|| PolicyError::MissingTopic("bars.completed".to_string()))?;

    if completed.topic_class != "ephemeral" {
        return Err(PolicyError::ClassMismatch {
            topic: "bars.completed".to_string(),
            expected: "ephemeral",
            actual: completed.topic_class.to_string(),
        });
    }

    if !completed.coalesce_key.is_empty() {
        return Err(PolicyError::CoalesceKeyMismatch {
            topic: "bars.completed".to_string(),
            expected: vec![],
            actual: completed
                .coalesce_key
                .iter()
                .map(|s| s.to_string())
                .collect(),
        });
    }

    // Execution events are never coalesced or dropped on the client.
    for topic in EXECUTION_TOPICS {
        let p = policy_list
            .iter()
            .find(|p| p.name == topic)
            .ok_or_else(|| PolicyError::MissingTopic(topic.to_string()))?;
        if p.topic_class != "durable" {
            return Err(PolicyError::ClassMismatch {
                topic: topic.to_string(),
                expected: "durable",
                actual: p.topic_class.to_string(),
            });
        }
        if !p.coalesce_key.is_empty() {
            return Err(PolicyError::CoalesceKeyMismatch {
                topic: topic.to_string(),
                expected: vec![],
                actual: p.coalesce_key.iter().map(|s| s.to_string()).collect(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_topics_policy_assertion_passes() {
        assert!(assert_topic_policies(KNOWN_TOPICS).is_ok());
    }

    #[test]
    fn test_policy_missing_topic_fails() {
        let bad = &[TopicPolicy {
            name: "bars.forming",
            topic_class: "ephemeral",
            coalesce_key: &["symbol", "timeframe"],
        }];
        let res = assert_topic_policies(bad);
        assert_eq!(
            res,
            Err(PolicyError::MissingTopic("bars.completed".to_string()))
        );
    }

    #[test]
    fn test_policy_class_mismatch_fails() {
        let bad = &[
            TopicPolicy {
                name: "bars.forming",
                topic_class: "durable",
                coalesce_key: &["symbol", "timeframe"],
            },
            TopicPolicy {
                name: "bars.completed",
                topic_class: "ephemeral",
                coalesce_key: &[],
            },
        ];
        let res = assert_topic_policies(bad);
        assert_eq!(
            res,
            Err(PolicyError::ClassMismatch {
                topic: "bars.forming".to_string(),
                expected: "ephemeral",
                actual: "durable".to_string(),
            })
        );
    }

    #[test]
    fn test_policy_coalesce_key_mismatch_fails() {
        let bad = &[
            TopicPolicy {
                name: "bars.forming",
                topic_class: "ephemeral",
                coalesce_key: &["symbol"],
            },
            TopicPolicy {
                name: "bars.completed",
                topic_class: "ephemeral",
                coalesce_key: &[],
            },
        ];
        let res = assert_topic_policies(bad);
        assert_eq!(
            res,
            Err(PolicyError::CoalesceKeyMismatch {
                topic: "bars.forming".to_string(),
                expected: vec!["symbol", "timeframe"],
                actual: vec!["symbol".to_string()],
            })
        );
    }

    #[test]
    fn test_coalescing_execution_topic_fails() {
        let mut topics: Vec<TopicPolicy> = KNOWN_TOPICS.to_vec();
        for t in topics.iter_mut() {
            if t.name == "fills" {
                t.coalesce_key = &["deployment_id"];
            }
        }
        assert_eq!(
            assert_topic_policies(&topics),
            Err(PolicyError::CoalesceKeyMismatch {
                topic: "fills".to_string(),
                expected: vec![],
                actual: vec!["deployment_id".to_string()],
            })
        );
    }

    #[test]
    fn test_missing_or_ephemeral_execution_topic_fails() {
        let without: Vec<TopicPolicy> = KNOWN_TOPICS
            .iter()
            .filter(|t| t.name != "ledger")
            .cloned()
            .collect();
        assert_eq!(
            assert_topic_policies(&without),
            Err(PolicyError::MissingTopic("ledger".to_string()))
        );

        let mut topics: Vec<TopicPolicy> = KNOWN_TOPICS.to_vec();
        for t in topics.iter_mut() {
            if t.name == "orders" {
                t.topic_class = "ephemeral";
            }
        }
        assert!(matches!(
            assert_topic_policies(&topics),
            Err(PolicyError::ClassMismatch { .. })
        ));
    }
}
