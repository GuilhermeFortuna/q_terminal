//! Pure §8.1 command enablement for the operations workspace (Q-048).

const WORKER_STALE_S: f64 = 30.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Health {
    pub api_reachable: bool,
    pub postgres_available: bool,
    pub worker_status: WorkerStatus,
    pub worker_heartbeat_age_s: f64,
    pub edge_reachable: bool,
    pub edge_mt5_connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkerStatus {
    #[default]
    Unknown,
    Active,
    Offline,
}

impl WorkerStatus {
    pub fn from_api(status: &str) -> Self {
        match status {
            "active" => Self::Active,
            _ => Self::Offline,
        }
    }

    pub fn is_active(self, heartbeat_age_s: f64) -> bool {
        self == Self::Active && heartbeat_age_s <= WORKER_STALE_S
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    CreateAccount,
    CreateDeployment,
    Start,
    Pause,
    Stop,
    Flatten,
    KillSwitchSet,
    KillSwitchClear,
    Resolve,
}

impl CommandKind {
    pub const ALL: [Self; 9] = [
        Self::CreateAccount,
        Self::CreateDeployment,
        Self::Start,
        Self::Pause,
        Self::Stop,
        Self::Flatten,
        Self::KillSwitchSet,
        Self::KillSwitchClear,
        Self::Resolve,
    ];

    pub fn parse_kind(s: &str) -> Option<Self> {
        match s {
            "create_account" => Some(Self::CreateAccount),
            "create_deployment" => Some(Self::CreateDeployment),
            "start" => Some(Self::Start),
            "pause" => Some(Self::Pause),
            "stop" => Some(Self::Stop),
            "flatten" => Some(Self::Flatten),
            "kill_switch_set" => Some(Self::KillSwitchSet),
            "kill_switch_clear" => Some(Self::KillSwitchClear),
            "resolve" => Some(Self::Resolve),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Enablement {
    Enabled,
    Disabled { reason: String },
}

impl Enablement {
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Enabled => None,
            Self::Disabled { reason } => Some(reason.as_str()),
        }
    }
}

pub fn enabled(cmd: CommandKind, health: &Health) -> Enablement {
    if !health.api_reachable {
        return Enablement::Disabled {
            reason: "API unreachable".into(),
        };
    }

    let worker_up = health
        .worker_status
        .is_active(health.worker_heartbeat_age_s);
    let edge_ok = health.edge_reachable && health.edge_mt5_connected;

    match cmd {
        CommandKind::KillSwitchSet | CommandKind::KillSwitchClear => Enablement::Enabled,
        CommandKind::Flatten => {
            if edge_ok {
                Enablement::Enabled
            } else {
                Enablement::Disabled {
                    reason: "MT5 edge or terminal unavailable".into(),
                }
            }
        }
        CommandKind::Start | CommandKind::CreateDeployment => {
            if !health.postgres_available {
                return Enablement::Disabled {
                    reason: "Postgres unavailable".into(),
                };
            }
            if !worker_up {
                return Enablement::Disabled {
                    reason: "Execution worker offline".into(),
                };
            }
            Enablement::Enabled
        }
        CommandKind::CreateAccount | CommandKind::Resolve => {
            if !health.postgres_available {
                return Enablement::Disabled {
                    reason: "Postgres unavailable".into(),
                };
            }
            Enablement::Enabled
        }
        CommandKind::Pause | CommandKind::Stop => {
            if !health.postgres_available {
                return Enablement::Disabled {
                    reason: "Postgres unavailable".into(),
                };
            }
            Enablement::Enabled
        }
    }
}

pub fn health_from_ops(
    api_offline: bool,
    postgres_available: bool,
    worker_status: &str,
    worker_heartbeat_age_s: f64,
    edge_reachable: bool,
    edge_mt5_connected: bool,
) -> Health {
    Health {
        api_reachable: !api_offline,
        postgres_available,
        worker_status: WorkerStatus::from_api(worker_status),
        worker_heartbeat_age_s,
        edge_reachable,
        edge_mt5_connected,
    }
}
