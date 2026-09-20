#![allow(clippy::float_cmp)]

pub use q_terminal::{
    bridge, chart_bridge, chart_target, config, contracts_stream, execution, execution_controls,
    execution_models, history, ops_session, ops_status, startup, stream,
};

use execution::enablement::{enabled, CommandKind, Enablement, Health, WorkerStatus};

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    health: Health,
}

#[derive(Clone, Copy)]
struct Expectation {
    cmd: CommandKind,
    enabled: bool,
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "healthy",
        health: Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: true,
            edge_mt5_connected: true,
        },
    },
    Scenario {
        name: "api_unreachable",
        health: Health {
            api_reachable: false,
            postgres_available: false,
            worker_status: WorkerStatus::Unknown,
            worker_heartbeat_age_s: 0.0,
            edge_reachable: false,
            edge_mt5_connected: false,
        },
    },
    Scenario {
        name: "postgres_down_edge_reachable",
        health: Health {
            api_reachable: true,
            postgres_available: false,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: true,
            edge_mt5_connected: true,
        },
    },
    Scenario {
        name: "postgres_down_edge_unreachable",
        health: Health {
            api_reachable: true,
            postgres_available: false,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: false,
            edge_mt5_connected: false,
        },
    },
    Scenario {
        name: "worker_down",
        health: Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Offline,
            worker_heartbeat_age_s: 120.0,
            edge_reachable: true,
            edge_mt5_connected: true,
        },
    },
    Scenario {
        name: "edge_down",
        health: Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: false,
            edge_mt5_connected: false,
        },
    },
    Scenario {
        name: "terminal_down",
        health: Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: true,
            edge_mt5_connected: false,
        },
    },
    Scenario {
        name: "redis_down_stream_disconnected",
        health: Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: true,
            edge_mt5_connected: true,
        },
    },
];

fn expectations_for(scenario: &Scenario) -> Vec<Expectation> {
    match scenario.name {
        "healthy" | "redis_down_stream_disconnected" => CommandKind::ALL
            .iter()
            .map(|cmd| Expectation {
                cmd: *cmd,
                enabled: true,
            })
            .collect(),
        "api_unreachable" => CommandKind::ALL
            .iter()
            .map(|cmd| Expectation {
                cmd: *cmd,
                enabled: false,
            })
            .collect(),
        "postgres_down_edge_reachable" => vec![
            Expectation {
                cmd: CommandKind::Start,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateDeployment,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateAccount,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Resolve,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Pause,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Stop,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Flatten,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchSet,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchClear,
                enabled: true,
            },
        ],
        "postgres_down_edge_unreachable" => vec![
            Expectation {
                cmd: CommandKind::Start,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateDeployment,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateAccount,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Resolve,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Pause,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Stop,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::Flatten,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::KillSwitchSet,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchClear,
                enabled: true,
            },
        ],
        "worker_down" => vec![
            Expectation {
                cmd: CommandKind::Start,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateDeployment,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::CreateAccount,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Resolve,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Pause,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Stop,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Flatten,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchSet,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchClear,
                enabled: true,
            },
        ],
        "edge_down" | "terminal_down" => vec![
            Expectation {
                cmd: CommandKind::Start,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::CreateDeployment,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::CreateAccount,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Resolve,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Pause,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Stop,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::Flatten,
                enabled: false,
            },
            Expectation {
                cmd: CommandKind::KillSwitchSet,
                enabled: true,
            },
            Expectation {
                cmd: CommandKind::KillSwitchClear,
                enabled: true,
            },
        ],
        other => panic!("missing expectations for scenario {other}"),
    }
}

/// Acceptance criterion 4: full §8.1 enablement matrix from the spec.
#[test]
fn test_enablement_matrix_matches_spec() {
    for scenario in SCENARIOS {
        for exp in expectations_for(scenario) {
            let result = enabled(exp.cmd, &scenario.health);
            let actual = result.is_enabled();
            assert_eq!(
                actual, exp.enabled,
                "scenario={} cmd={:?} got {:?}",
                scenario.name, exp.cmd, result
            );
            if !exp.enabled {
                assert!(
                    matches!(result, Enablement::Disabled { .. }),
                    "disabled command must carry a reason: {:?}",
                    result
                );
            }
        }
    }
}
