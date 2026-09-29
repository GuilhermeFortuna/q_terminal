#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use serde_json::Value;
use uuid::Uuid;

use crate::execution::commands::{
    CatalogDeploymentParams, Command, CommandClient, CommandError, ManualFill,
};
use crate::execution::enablement::{self, CommandKind, Health};
use crate::execution::store::ExecutionHandle;

static PENDING_CONTROL_HANDLE: std::sync::Mutex<Option<ExecutionHandle>> =
    std::sync::Mutex::new(None);

pub fn stage_control_handle(handle: ExecutionHandle) {
    *PENDING_CONTROL_HANDLE.lock().unwrap() = Some(handle);
}

pub const STREAM_SETTLE_BOUND_MS: u64 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPhase {
    InFlight,
    AwaitingStream,
    PendingStream,
    Settled,
    Refused,
}

#[derive(Debug, Clone)]
enum Settlement {
    DeploymentLifecycle {
        deployment_id: String,
        lifecycle: String,
    },
    DeploymentPendingAction {
        deployment_id: String,
        action: String,
    },
    DeploymentEdited {
        deployment_id: String,
        expected_revision: i64,
    },
    KillSwitch {
        enabled: bool,
    },
    OrderReconciled {
        order_id: String,
    },
    AccountCreated {
        name: String,
    },
    DeploymentCreated {
        name: String,
    },
}

#[derive(Debug, Clone)]
struct PendingAction {
    #[allow(dead_code)]
    key: Uuid,
    kind: CommandKind,
    phase: ActionPhase,
    started_at: Instant,
    #[allow(dead_code)]
    baseline_revision: u64,
    settlement: Settlement,
    status_code: u16,
    error_code: String,
    error_message: String,
    response_id: String,
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, api_base)]
        #[qproperty(QString, operator_name)]
        #[qproperty(QString, saved_runs_json)]
        #[qproperty(QString, strategy_catalog_json)]
        #[qproperty(QString, symbol_search_results_json)]
        #[qproperty(QString, deployment_detail_json)]
        #[qproperty(QString, performance_summary_json)]
        #[qproperty(QString, performance_marks_json)]
        type ExecutionControls = super::ExecutionControlsRust;

        #[qinvokable]
        fn setup(self: Pin<&mut ExecutionControls>, api_base: QString, operator_name: QString);

        #[qinvokable]
        fn update_health(
            self: Pin<&mut ExecutionControls>,
            api_offline: bool,
            postgres_available: bool,
            worker_status: QString,
            worker_heartbeat_age_s: f64,
            edge_reachable: bool,
            edge_mt5_connected: bool,
        );

        #[qinvokable]
        fn is_command_enabled(self: Pin<&mut ExecutionControls>, kind: QString) -> bool;

        #[qinvokable]
        fn shell_command_enabled(
            self: Pin<&mut ExecutionControls>,
            id: QString,
            kill_switch: bool,
        ) -> bool;

        #[qinvokable]
        fn shell_command_reason(
            self: Pin<&mut ExecutionControls>,
            id: QString,
            kill_switch: bool,
        ) -> QString;

        #[qinvokable]
        fn command_disabled_reason(self: Pin<&mut ExecutionControls>, kind: QString) -> QString;

        #[qinvokable]
        fn request(self: Pin<&mut ExecutionControls>, kind: QString, args_json: QString)
            -> QString;

        #[qinvokable]
        fn action_phase(self: Pin<&mut ExecutionControls>, action_id: QString) -> QString;

        #[qinvokable]
        fn action_message(self: Pin<&mut ExecutionControls>, action_id: QString) -> QString;

        #[qinvokable]
        fn action_status_code(self: Pin<&mut ExecutionControls>, action_id: QString) -> i32;

        #[qinvokable]
        fn action_error_code(self: Pin<&mut ExecutionControls>, action_id: QString) -> QString;

        #[qinvokable]
        fn action_response_id(self: Pin<&mut ExecutionControls>, action_id: QString) -> QString;

        #[qinvokable]
        fn is_deployment_editable(
            self: Pin<&mut ExecutionControls>,
            deployment_id: QString,
        ) -> bool;

        #[qinvokable]
        fn deployment_edit_disabled_reason(
            self: Pin<&mut ExecutionControls>,
            deployment_id: QString,
        ) -> QString;

        #[qinvokable]
        fn fetch_saved_runs(self: Pin<&mut ExecutionControls>);

        #[qinvokable]
        fn fetch_strategy_catalog(self: Pin<&mut ExecutionControls>);

        #[qinvokable]
        fn search_symbols(self: Pin<&mut ExecutionControls>, query: QString);

        #[qinvokable]
        fn fetch_deployment_detail(self: Pin<&mut ExecutionControls>, deployment_id: QString);

        #[qinvokable]
        fn fetch_performance(
            self: Pin<&mut ExecutionControls>,
            deployment_id: QString,
            bar_close_time: QString,
        );

        #[qinvokable]
        fn clear_performance(self: Pin<&mut ExecutionControls>);

        #[qinvokable]
        fn bind_store(self: Pin<&mut ExecutionControls>);
    }

    impl cxx_qt::Threading for ExecutionControls {}
}

pub struct ExecutionControlsRust {
    pub api_base: QString,
    pub operator_name: QString,
    pub saved_runs_json: QString,
    pub strategy_catalog_json: QString,
    pub symbol_search_results_json: QString,
    pub deployment_detail_json: QString,
    pub performance_summary_json: QString,
    pub performance_marks_json: QString,
    performance_gen: Arc<AtomicU64>,
    client: Option<CommandClient>,
    handle: Option<ExecutionHandle>,
    health: Health,
    actions: HashMap<String, PendingAction>,
    inflight_kinds: HashMap<CommandKind, String>,
    dirty: Arc<AtomicBool>,
}

impl Default for ExecutionControlsRust {
    fn default() -> Self {
        Self {
            api_base: QString::default(),
            operator_name: QString::from("operator"),
            saved_runs_json: QString::from("[]"),
            strategy_catalog_json: QString::from("{\"strategies\":[]}"),
            symbol_search_results_json: QString::from("[]"),
            deployment_detail_json: QString::from("{}"),
            performance_summary_json: QString::from("{}"),
            performance_marks_json: QString::from("[]"),
            performance_gen: Arc::new(AtomicU64::new(0)),
            client: None,
            handle: None,
            health: Health {
                api_reachable: true,
                postgres_available: true,
                worker_status: enablement::WorkerStatus::Active,
                worker_heartbeat_age_s: 0.0,
                edge_reachable: true,
                edge_mt5_connected: true,
            },
            actions: HashMap::new(),
            inflight_kinds: HashMap::new(),
            dirty: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ExecutionControlsRust {
    pub fn set_handle(&mut self, handle: ExecutionHandle) {
        let dirty = self.dirty.clone();
        handle.set_listener(Arc::new(move || {
            dirty.store(true, Ordering::Release);
        }));
        self.handle = Some(handle);
        self.dirty.store(true, Ordering::Release);
    }

    fn current_health(&self) -> Health {
        self.health
    }

    fn parse_kind(kind: &str) -> Option<CommandKind> {
        CommandKind::parse_kind(kind)
    }

    fn build_command(&self, kind: CommandKind, args: &Value, actor: &str) -> Option<Command> {
        match kind {
            CommandKind::CreateAccount => Some(Command::CreateAccount {
                name: args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("paper")
                    .to_string(),
                initial_balance: args
                    .get("initial_balance")
                    .and_then(|v| v.as_str())
                    .unwrap_or("100000")
                    .to_string(),
                currency: args
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .unwrap_or("BRL")
                    .to_string(),
            }),
            CommandKind::CreateDeployment => {
                let paper_account_id = args
                    .get("paper_account_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let broker_mode = args
                    .get("broker_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("paper");
                if paper_account_id.is_empty() || name.is_empty() {
                    return None;
                }
                let run_id = args
                    .get("source_backtest_run_id")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let catalog = if let Some(cat) = args.get("catalog") {
                    let strategy_name = cat
                        .get("strategy_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let symbol = cat
                        .get("symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let timeframe = cat
                        .get("timeframe")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if strategy_name.is_empty() || symbol.is_empty() || timeframe.is_empty() {
                        return None;
                    }
                    Some(CatalogDeploymentParams {
                        strategy_name,
                        strategy_params: cat
                            .get("strategy_params")
                            .cloned()
                            .unwrap_or(serde_json::json!({})),
                        exit_params: cat
                            .get("exit_params")
                            .cloned()
                            .unwrap_or(serde_json::json!({})),
                        symbol,
                        timeframe,
                        sizing_config: cat
                            .get("sizing_config")
                            .cloned()
                            .unwrap_or(serde_json::json!({"quantity": "1"})),
                        risk_config: cat
                            .get("risk_config")
                            .cloned()
                            .unwrap_or(serde_json::json!({})),
                        paper_cost_config: cat.get("paper_cost_config").cloned().unwrap_or(
                            serde_json::json!({
                                "point_value": "1",
                                "slippage_points": "0",
                                "cost_per_contract": "0",
                                "cost_bps": "0"
                            }),
                        ),
                    })
                } else {
                    None
                };

                if run_id.is_none() && catalog.is_none() {
                    return None;
                }

                Some(Command::CreateDeployment {
                    paper_account_id: paper_account_id.to_string(),
                    name: name.to_string(),
                    broker_mode: broker_mode.to_string(),
                    source_backtest_run_id: run_id,
                    catalog,
                })
            }
            CommandKind::EditDeployment => {
                let deployment_id = args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let expected_revision = args
                    .get("expected_revision")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if deployment_id.is_empty() || expected_revision < 1 {
                    return None;
                }
                let configuration = args
                    .get("configuration")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                Some(Command::EditDeployment {
                    deployment_id: deployment_id.to_string(),
                    expected_revision,
                    actor: actor.to_string(),
                    configuration,
                })
            }
            CommandKind::Start | CommandKind::Pause | CommandKind::Stop | CommandKind::Flatten => {
                let deployment_id = args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if deployment_id.is_empty() {
                    return None;
                }
                let action = match kind {
                    CommandKind::Start => "start",
                    CommandKind::Pause => "pause",
                    CommandKind::Stop => "stop",
                    CommandKind::Flatten => "flatten",
                    _ => return None,
                };
                Some(Command::Lifecycle {
                    deployment_id: deployment_id.to_string(),
                    action: action.to_string(),
                    confirm: kind == CommandKind::Flatten || kind == CommandKind::Stop,
                    actor: actor.to_string(),
                })
            }
            CommandKind::KillSwitchSet => Some(Command::KillSwitch {
                enabled: true,
                confirm: true,
                reason: args
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                actor: actor.to_string(),
            }),
            CommandKind::KillSwitchClear => Some(Command::KillSwitch {
                enabled: false,
                confirm: true,
                reason: None,
                actor: actor.to_string(),
            }),
            CommandKind::Resolve => {
                let order_id = args.get("order_id").and_then(|v| v.as_str()).unwrap_or("");
                let outcome = args.get("outcome").and_then(|v| v.as_str()).unwrap_or("");
                let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                if order_id.is_empty() || outcome.is_empty() || reason.is_empty() {
                    return None;
                }
                let fill = if outcome == "filled" {
                    let price = args.get("price").and_then(|v| v.as_str()).unwrap_or("");
                    let quantity = args.get("quantity").and_then(|v| v.as_str()).unwrap_or("");
                    let filled_at = args.get("filled_at").and_then(|v| v.as_str()).unwrap_or("");
                    if price.is_empty() || quantity.is_empty() || filled_at.is_empty() {
                        return None;
                    }
                    Some(ManualFill {
                        price: price.to_string(),
                        quantity: quantity.to_string(),
                        filled_at: filled_at.to_string(),
                        fee: args.get("fee").and_then(|v| v.as_str()).map(String::from),
                        external_fill_id: args
                            .get("external_fill_id")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                } else {
                    None
                };
                Some(Command::Resolve {
                    order_id: order_id.to_string(),
                    outcome: outcome.to_string(),
                    actor: actor.to_string(),
                    reason: reason.to_string(),
                    fill,
                })
            }
        }
    }

    fn settlement_for(kind: CommandKind, args: &Value) -> Option<Settlement> {
        match kind {
            CommandKind::CreateAccount => Some(Settlement::AccountCreated {
                name: args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }),
            CommandKind::CreateDeployment => Some(Settlement::DeploymentCreated {
                name: args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }),
            CommandKind::EditDeployment => {
                let deployment_id = args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let expected_revision = args
                    .get("expected_revision")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                Some(Settlement::DeploymentEdited {
                    deployment_id,
                    expected_revision,
                })
            }
            CommandKind::Start => Some(Settlement::DeploymentLifecycle {
                deployment_id: args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                lifecycle: "running".to_string(),
            }),
            CommandKind::Pause => Some(Settlement::DeploymentLifecycle {
                deployment_id: args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                lifecycle: "paused".to_string(),
            }),
            CommandKind::Stop => Some(Settlement::DeploymentLifecycle {
                deployment_id: args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                lifecycle: "stopped".to_string(),
            }),
            CommandKind::Flatten => Some(Settlement::DeploymentPendingAction {
                deployment_id: args
                    .get("deployment_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                action: "flatten".to_string(),
            }),
            CommandKind::KillSwitchSet => Some(Settlement::KillSwitch { enabled: true }),
            CommandKind::KillSwitchClear => Some(Settlement::KillSwitch { enabled: false }),
            CommandKind::Resolve => Some(Settlement::OrderReconciled {
                order_id: args
                    .get("order_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            }),
        }
    }

    fn is_settled(
        store: &crate::execution::store::ExecutionStore,
        settlement: &Settlement,
    ) -> bool {
        let data = store.data();
        match settlement {
            Settlement::DeploymentEdited {
                deployment_id,
                expected_revision,
            } => data
                .deployments
                .get(deployment_id)
                .map(|d| d.config_revision.unwrap_or(0) > *expected_revision)
                .unwrap_or(false),
            Settlement::DeploymentLifecycle {
                deployment_id,
                lifecycle,
            } => data
                .deployments
                .get(deployment_id)
                .map(|d| d.lifecycle == *lifecycle)
                .unwrap_or(false),
            Settlement::DeploymentPendingAction {
                deployment_id,
                action,
            } => data
                .deployments
                .get(deployment_id)
                .and_then(|d| d.pending_action.as_ref())
                .map(|p| p == action)
                .unwrap_or(false),
            Settlement::KillSwitch { enabled } => data
                .control
                .as_ref()
                .map(|c| c.kill_switch_enabled == *enabled)
                .unwrap_or(false),
            Settlement::OrderReconciled { order_id } => data.orders.values().any(|ring| {
                ring.iter().any(|o| {
                    o.id == *order_id
                        && o.reconciliation_state != "unknown"
                        && o.status != "unknown"
                })
            }),
            Settlement::AccountCreated { name } => data.accounts.values().any(|a| a.name == *name),
            Settlement::DeploymentCreated { name } => {
                data.deployments.values().any(|d| d.name == *name)
            }
        }
    }

    fn tick_actions(&mut self) {
        let handle = match &self.handle {
            Some(h) => h.clone(),
            None => return,
        };
        let now = Instant::now();
        for action in self.actions.values_mut() {
            if matches!(
                action.phase,
                ActionPhase::Settled | ActionPhase::Refused | ActionPhase::InFlight
            ) {
                continue;
            }
            let settled = handle.read(|store| Self::is_settled(store, &action.settlement));
            if settled {
                action.phase = ActionPhase::Settled;
                continue;
            }
            let elapsed = now.duration_since(action.started_at).as_millis() as u64;
            if elapsed >= STREAM_SETTLE_BOUND_MS
                && matches!(action.phase, ActionPhase::AwaitingStream)
            {
                action.phase = ActionPhase::PendingStream;
            }
        }
        self.inflight_kinds.retain(|_, id| {
            self.actions
                .get(id)
                .map(|a| a.phase == ActionPhase::InFlight)
                .unwrap_or(false)
        });
    }

    fn phase_label(action: &PendingAction) -> &'static str {
        match action.phase {
            ActionPhase::InFlight => "in_flight",
            ActionPhase::AwaitingStream => "awaiting_stream",
            ActionPhase::PendingStream => "pending_stream",
            ActionPhase::Settled => "settled",
            ActionPhase::Refused => "refused",
        }
    }

    fn message_for(action: &PendingAction) -> String {
        match action.phase {
            ActionPhase::InFlight => "Sending…".to_string(),
            ActionPhase::AwaitingStream => "Accepted; waiting for stream".to_string(),
            ActionPhase::PendingStream => "Applied; waiting for the stream".to_string(),
            ActionPhase::Settled => "Confirmed on stream".to_string(),
            ActionPhase::Refused => {
                if !action.error_code.is_empty() {
                    format!("{}: {}", action.error_code, action.error_message)
                } else {
                    action.error_message.clone()
                }
            }
        }
    }
}

impl ffi::ExecutionControls {
    pub fn setup(mut self: Pin<&mut Self>, api_base: QString, operator_name: QString) {
        let base = api_base.to_string();
        self.as_mut().set_api_base(api_base);
        self.as_mut().set_operator_name(operator_name);
        self.as_mut().rust_mut().client = Some(CommandClient::new(&base));
    }

    pub fn update_health(
        mut self: Pin<&mut Self>,
        api_offline: bool,
        postgres_available: bool,
        worker_status: QString,
        worker_heartbeat_age_s: f64,
        edge_reachable: bool,
        edge_mt5_connected: bool,
    ) {
        self.as_mut().rust_mut().health = enablement::health_from_ops(
            api_offline,
            postgres_available,
            &worker_status.to_string(),
            worker_heartbeat_age_s,
            edge_reachable,
            edge_mt5_connected,
        );
    }

    pub fn is_command_enabled(self: Pin<&mut Self>, kind: QString) -> bool {
        let kind = ExecutionControlsRust::parse_kind(&kind.to_string());
        kind.map(|k| enablement::enabled(k, &self.rust().current_health()).is_enabled())
            .unwrap_or(false)
    }

    /// Shell command enablement (Q-052), from the one health snapshot every window shares.
    pub fn shell_command_enabled(self: Pin<&mut Self>, id: QString, kill_switch: bool) -> bool {
        crate::shell::commands::CommandRegistry::shared()
            .enablement(&id.to_string(), &self.rust().current_health(), kill_switch)
            .is_enabled()
    }

    pub fn shell_command_reason(self: Pin<&mut Self>, id: QString, kill_switch: bool) -> QString {
        let e = crate::shell::commands::CommandRegistry::shared().enablement(
            &id.to_string(),
            &self.rust().current_health(),
            kill_switch,
        );
        QString::from(e.reason().unwrap_or(""))
    }

    pub fn command_disabled_reason(self: Pin<&mut Self>, kind: QString) -> QString {
        let kind_str = kind.to_string();
        let health = self.rust().current_health();
        let reason = match ExecutionControlsRust::parse_kind(&kind_str) {
            Some(k) => match enablement::enabled(k, &health) {
                enablement::Enablement::Disabled { reason } => reason,
                enablement::Enablement::Enabled => String::new(),
            },
            None => String::new(),
        };
        QString::from(reason)
    }

    pub fn request(mut self: Pin<&mut Self>, kind: QString, args_json: QString) -> QString {
        let kind = match ExecutionControlsRust::parse_kind(&kind.to_string()) {
            Some(k) => k,
            None => return QString::from(""),
        };
        if self.rust().inflight_kinds.contains_key(&kind) {
            return QString::from("");
        }
        if !enablement::enabled(kind, &self.rust().current_health()).is_enabled() {
            return QString::from("");
        }
        let args: Value = serde_json::from_str(&args_json.to_string()).unwrap_or(Value::Null);
        let actor = self.rust().operator_name.to_string();
        let cmd = match self.rust().build_command(kind, &args, &actor) {
            Some(c) => c,
            None => return QString::from(""),
        };
        let settlement = match ExecutionControlsRust::settlement_for(kind, &args) {
            Some(s) => s,
            None => return QString::from(""),
        };
        let action_id = Uuid::new_v4().to_string();
        let key = Uuid::new_v4();
        let baseline_revision = self
            .rust()
            .handle
            .as_ref()
            .map(|h| h.read(|s| s.revision()))
            .unwrap_or(0);
        let api_base = self.rust().api_base.to_string();
        let qt_thread = self.qt_thread();

        self.as_mut().rust_mut().actions.insert(
            action_id.clone(),
            PendingAction {
                key,
                kind,
                phase: ActionPhase::InFlight,
                started_at: Instant::now(),
                baseline_revision,
                settlement,
                status_code: 0,
                error_code: String::new(),
                error_message: String::new(),
                response_id: String::new(),
            },
        );
        self.as_mut()
            .rust_mut()
            .inflight_kinds
            .insert(kind, action_id.clone());

        let action_id_for_thread = action_id.clone();
        let _ = std::thread::Builder::new()
            .name("execution-command".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let result = client.send(key, &cmd).await;
                    let _ = qt_thread.queue(move |mut controls| {
                        let mut rust = controls.as_mut().rust_mut();
                        let Some(action) = rust.actions.get_mut(&action_id_for_thread) else {
                            return;
                        };
                        let kind = action.kind;
                        match result {
                            Ok(outcome) if outcome.status >= 200 && outcome.status < 300 => {
                                action.phase = ActionPhase::AwaitingStream;
                                action.status_code = outcome.status;
                                action.response_id = outcome
                                    .body
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                            }
                            Ok(outcome) => {
                                action.phase = ActionPhase::Refused;
                                action.status_code = outcome.status;
                                let detail = outcome.body.get("detail");
                                action.error_code = outcome
                                    .body
                                    .get("code")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("command_rejected")
                                    .to_string();
                                action.error_message = outcome
                                    .body
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .map(String::from)
                                    .or_else(|| detail.and_then(Value::as_str).map(String::from))
                                    .or_else(|| {
                                        detail.and_then(Value::as_array).map(|items| {
                                            items
                                                .iter()
                                                .map(|item| {
                                                    let field = item
                                                        .get("loc")
                                                        .and_then(Value::as_array)
                                                        .and_then(|loc| loc.last())
                                                        .and_then(Value::as_str)
                                                        .unwrap_or("request");
                                                    let message = item
                                                        .get("msg")
                                                        .and_then(Value::as_str)
                                                        .unwrap_or("invalid value");
                                                    format!("{field}: {message}")
                                                })
                                                .collect::<Vec<_>>()
                                                .join("; ")
                                        })
                                    })
                                    .unwrap_or_else(|| "command rejected".to_string());
                            }
                            Err(CommandError::Transport(msg)) => {
                                action.phase = ActionPhase::Refused;
                                action.error_code = "transport_error".to_string();
                                action.error_message = msg;
                            }
                            Err(CommandError::InvalidResponse(msg)) => {
                                action.phase = ActionPhase::Refused;
                                action.error_code = "invalid_response".to_string();
                                action.error_message = msg;
                            }
                        }
                        rust.inflight_kinds.remove(&kind);
                    });
                });
            });

        QString::from(action_id)
    }

    pub fn action_phase(mut self: Pin<&mut Self>, action_id: QString) -> QString {
        self.as_mut().rust_mut().tick_actions();
        let id = action_id.to_string();
        let phase = self
            .rust()
            .actions
            .get(&id)
            .map(ExecutionControlsRust::phase_label)
            .unwrap_or("");
        QString::from(phase)
    }

    pub fn action_message(mut self: Pin<&mut Self>, action_id: QString) -> QString {
        self.as_mut().rust_mut().tick_actions();
        let id = action_id.to_string();
        let msg = self
            .rust()
            .actions
            .get(&id)
            .map(ExecutionControlsRust::message_for)
            .unwrap_or_default();
        QString::from(&msg)
    }

    pub fn bind_store(mut self: Pin<&mut Self>) {
        if let Some(handle) = PENDING_CONTROL_HANDLE.lock().unwrap().take() {
            self.as_mut().rust_mut().set_handle(handle);
        }
    }

    pub fn fetch_saved_runs(self: Pin<&mut Self>) {
        let api_base = self.rust().api_base.to_string();
        let qt_thread = self.qt_thread();
        let _ = std::thread::Builder::new()
            .name("fetch-saved-runs".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let runs = client.fetch_saved_runs().await.unwrap_or_default();
                    let json = serde_json::to_string(&runs).unwrap_or_else(|_| "[]".to_string());
                    let _ = qt_thread.queue(move |mut controls| {
                        controls.as_mut().set_saved_runs_json(QString::from(&json));
                    });
                });
            });
    }

    pub fn action_status_code(mut self: Pin<&mut Self>, action_id: QString) -> i32 {
        self.as_mut().rust_mut().tick_actions();
        let id = action_id.to_string();
        self.rust()
            .actions
            .get(&id)
            .map(|a| a.status_code as i32)
            .unwrap_or(0)
    }

    pub fn action_error_code(mut self: Pin<&mut Self>, action_id: QString) -> QString {
        self.as_mut().rust_mut().tick_actions();
        let id = action_id.to_string();
        let code = self
            .rust()
            .actions
            .get(&id)
            .map(|a| a.error_code.as_str())
            .unwrap_or("");
        QString::from(code)
    }

    pub fn action_response_id(self: Pin<&mut Self>, action_id: QString) -> QString {
        self.rust()
            .actions
            .get(&action_id.to_string())
            .map(|action| QString::from(&action.response_id))
            .unwrap_or_default()
    }

    pub fn deployment_edit_disabled_reason(
        self: Pin<&mut Self>,
        deployment_id: QString,
    ) -> QString {
        let dep_id = deployment_id.to_string();
        if dep_id.is_empty() {
            return QString::from("No deployment selected");
        }
        let health = self.rust().current_health();
        if !health.api_reachable {
            return QString::from("API unreachable");
        }
        if !health.postgres_available {
            return QString::from("Postgres unavailable");
        }
        let handle = match &self.rust().handle {
            Some(h) => h.clone(),
            None => return QString::from("Store unavailable"),
        };
        let reason = handle.read(|store| {
            let data = store.data();
            let dep = match data.deployments.get(&dep_id) {
                Some(d) => d,
                None => return "Deployment not found".to_string(),
            };
            if dep.broker_mode == "mt5_live" {
                return "Live deployments cannot be edited in dev profile".to_string();
            }
            let lc = dep.lifecycle.as_str();
            if lc != "draft" && lc != "paused" {
                return format!(
                    "Cannot edit deployment in '{lc}' lifecycle (must be draft or paused)"
                );
            }
            if let Some(pos) = data.positions.get(&dep_id) {
                let q = pos.quantity.as_str();
                if !q.is_empty() && q != "0" && q != "0.0" {
                    return "Cannot edit deployment with an open position (must be flat)"
                        .to_string();
                }
            }
            if let Some(pa) = dep.pending_action.as_ref().and_then(|v| v.as_str()) {
                if !pa.is_empty() {
                    return format!("Cannot edit while action '{pa}' is pending");
                }
            }
            let unk_count = data
                .orders
                .get(&dep_id)
                .map(|ring| {
                    ring.iter()
                        .filter(|o| {
                            o.reconciliation_state == "unknown"
                                || o.reconciliation_state == "failed"
                        })
                        .count()
                })
                .unwrap_or(0);
            if unk_count > 0 {
                return "Cannot edit deployment with unresolved unknown orders".to_string();
            }
            String::new()
        });
        QString::from(&reason)
    }

    pub fn is_deployment_editable(self: Pin<&mut Self>, deployment_id: QString) -> bool {
        self.deployment_edit_disabled_reason(deployment_id)
            .to_string()
            .is_empty()
    }

    pub fn fetch_strategy_catalog(self: Pin<&mut Self>) {
        let api_base = self.rust().api_base.to_string();
        let qt_thread = self.qt_thread();
        let _ = std::thread::Builder::new()
            .name("fetch-strategy-catalog".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let catalog = client
                        .fetch_strategy_catalog()
                        .await
                        .unwrap_or_else(|_| serde_json::json!({"strategies": []}));
                    let json = serde_json::to_string(&catalog)
                        .unwrap_or_else(|_| "{\"strategies\":[]}".to_string());
                    let _ = qt_thread.queue(move |mut controls| {
                        controls
                            .as_mut()
                            .set_strategy_catalog_json(QString::from(&json));
                    });
                });
            });
    }

    pub fn search_symbols(self: Pin<&mut Self>, query: QString) {
        let api_base = self.rust().api_base.to_string();
        let q = query.to_string();
        let qt_thread = self.qt_thread();
        let _ = std::thread::Builder::new()
            .name("search-symbols".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let results = client.search_symbols(&q).await.unwrap_or_default();
                    let json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string());
                    let _ = qt_thread.queue(move |mut controls| {
                        controls
                            .as_mut()
                            .set_symbol_search_results_json(QString::from(&json));
                    });
                });
            });
    }

    pub fn fetch_deployment_detail(self: Pin<&mut Self>, deployment_id: QString) {
        let api_base = self.rust().api_base.to_string();
        let dep_id = deployment_id.to_string();
        let qt_thread = self.qt_thread();
        let _ = std::thread::Builder::new()
            .name("fetch-deployment-detail".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let detail = client
                        .fetch_deployment(&dep_id)
                        .await
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let json = serde_json::to_string(&detail).unwrap_or_else(|_| "{}".to_string());
                    let _ = qt_thread.queue(move |mut controls| {
                        controls
                            .as_mut()
                            .set_deployment_detail_json(QString::from(&json));
                    });
                });
            });
    }

    pub fn clear_performance(mut self: Pin<&mut Self>) {
        self.rust().performance_gen.fetch_add(1, Ordering::SeqCst);
        self.as_mut()
            .set_performance_summary_json(QString::from("{}"));
        self.as_mut()
            .set_performance_marks_json(QString::from("[]"));
    }

    pub fn fetch_performance(
        self: Pin<&mut Self>,
        deployment_id: QString,
        bar_close_time: QString,
    ) {
        let dep_id = deployment_id.to_string();
        if dep_id.is_empty() {
            return;
        }
        let bar_target = bar_close_time.to_string();
        let api_base = self.rust().api_base.to_string();
        let gen = self.rust().performance_gen.clone();
        let my_gen = gen.fetch_add(1, Ordering::SeqCst) + 1;
        let qt_thread = self.qt_thread();

        let _ = std::thread::Builder::new()
            .name("fetch-performance".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                rt.block_on(async move {
                    let client = CommandClient::new(&api_base);
                    let mut attempts = 0;
                    let max_attempts = if !bar_target.is_empty() { 3 } else { 1 };
                    let mut summary_val = serde_json::json!({});
                    let mut marks_val = serde_json::json!([]);

                    while attempts < max_attempts {
                        if gen.load(Ordering::SeqCst) != my_gen {
                            return;
                        }
                        if let Ok(sum) = client.fetch_performance(&dep_id).await {
                            summary_val = sum;
                        }
                        if let Ok(m_resp) = client.fetch_performance_marks(&dep_id, 50, 0).await {
                            let items = m_resp
                                .get("items")
                                .cloned()
                                .unwrap_or(serde_json::json!([]));
                            marks_val = items;
                        }

                        if bar_target.is_empty() {
                            break;
                        }

                        let has_matching_mark = marks_val
                            .as_array()
                            .map(|arr| {
                                arr.first()
                                    .and_then(|m| m.get("bar_close_time"))
                                    .and_then(|v| v.as_str())
                                    == Some(&bar_target)
                            })
                            .unwrap_or(false);

                        if has_matching_mark {
                            break;
                        }

                        attempts += 1;
                        if attempts < max_attempts {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    }

                    if gen.load(Ordering::SeqCst) != my_gen {
                        return;
                    }

                    let sum_str =
                        serde_json::to_string(&summary_val).unwrap_or_else(|_| "{}".to_string());
                    let marks_str =
                        serde_json::to_string(&marks_val).unwrap_or_else(|_| "[]".to_string());

                    let _ = qt_thread.queue(move |mut controls| {
                        controls
                            .as_mut()
                            .set_performance_summary_json(QString::from(&sum_str));
                        controls
                            .as_mut()
                            .set_performance_marks_json(QString::from(&marks_str));
                    });
                });
            });
    }
}
