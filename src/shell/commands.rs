//! Command registry (Q-052): commands are data. Every window renders the same registry;
//! enablement is evaluated from global state, so a command disabled by the kill switch is
//! disabled in every window at once. Two commands sharing a shortcut is a startup error.

use crate::execution::enablement::{self, CommandKind, Enablement, Health};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Works from whichever window has focus.
    App,
    /// Works only while a panel of this kind has focus.
    Panel(&'static str),
}

/// What the kill switch does to a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillSwitch {
    Ignored,
    /// Disabled while the kill switch is engaged.
    DisabledWhenOn,
    /// Disabled while the kill switch is not engaged.
    DisabledWhenOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    /// Qt key sequence, e.g. `Ctrl+Shift+K`.
    pub shortcut: &'static str,
    pub scope: Scope,
    /// The §8.1 enablement rule of the execution command this triggers, if any.
    pub rule: Option<CommandKind>,
    pub kill_switch: KillSwitch,
}

const fn cmd(
    id: &'static str,
    label: &'static str,
    shortcut: &'static str,
    scope: Scope,
    rule: Option<CommandKind>,
    kill_switch: KillSwitch,
) -> Command {
    Command {
        id,
        label,
        shortcut,
        scope,
        rule,
        kill_switch,
    }
}

pub const STANDARD: &[Command] = &[
    cmd(
        "palette.open",
        "Command palette",
        "Ctrl+Shift+P",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "window.open-market",
        "New market window",
        "Ctrl+Shift+1",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "window.open-operations",
        "New operations window",
        "Ctrl+Shift+2",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "window.merge-all",
        "Merge all windows into this one",
        "Ctrl+Shift+M",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "window.split-all",
        "Split merged windows out again",
        "Ctrl+Shift+S",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "window.close",
        "Close window",
        "Ctrl+W",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "selection.toggle-detach",
        "Detach or reattach selection",
        "Ctrl+Shift+D",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "kill-switch.set",
        "Engage kill switch",
        "Ctrl+Shift+K",
        Scope::App,
        Some(CommandKind::KillSwitchSet),
        KillSwitch::DisabledWhenOn,
    ),
    cmd(
        "kill-switch.clear",
        "Clear kill switch",
        "Ctrl+Alt+K",
        Scope::App,
        Some(CommandKind::KillSwitchClear),
        KillSwitch::DisabledWhenOff,
    ),
    cmd(
        "account.create",
        "Create account",
        "Ctrl+Shift+A",
        Scope::App,
        Some(CommandKind::CreateAccount),
        KillSwitch::Ignored,
    ),
    cmd(
        "deployment.create",
        "Deploy strategy",
        "Ctrl+Shift+N",
        Scope::App,
        Some(CommandKind::CreateDeployment),
        KillSwitch::DisabledWhenOn,
    ),
    cmd(
        "deployment.flatten",
        "Flatten selected deployment",
        "Ctrl+Shift+F",
        Scope::App,
        Some(CommandKind::Flatten),
        KillSwitch::Ignored,
    ),
    cmd(
        "detail.next-tab",
        "Next detail tab",
        "Ctrl+]",
        Scope::Panel("detail"),
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "detail.previous-tab",
        "Previous detail tab",
        "Ctrl+[",
        Scope::Panel("detail"),
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "chart.focus-symbol",
        "Focus chart symbol",
        "Ctrl+Shift+C",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
    cmd(
        "chart.follow-deployment",
        "Follow selected deployment",
        "Ctrl+Shift+L",
        Scope::App,
        None,
        KillSwitch::Ignored,
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateShortcut {
        shortcut: String,
        first: String,
        second: String,
    },
    DuplicateId(String),
    MissingLabel(String),
    MissingShortcut(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateShortcut {
                shortcut,
                first,
                second,
            } => write!(
                f,
                "shortcut {shortcut} is bound to both `{first}` and `{second}`"
            ),
            Self::DuplicateId(id) => write!(f, "command id `{id}` is registered twice"),
            Self::MissingLabel(id) => write!(f, "command `{id}` has no label"),
            Self::MissingShortcut(id) => write!(f, "command `{id}` has no shortcut"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Debug, Clone)]
pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    /// Validates and builds a registry. Shortcuts compare case-insensitively.
    pub fn new(commands: &[Command]) -> Result<Self, RegistryError> {
        for (i, c) in commands.iter().enumerate() {
            if c.label.is_empty() {
                return Err(RegistryError::MissingLabel(c.id.into()));
            }
            if c.shortcut.is_empty() {
                return Err(RegistryError::MissingShortcut(c.id.into()));
            }
            for other in &commands[..i] {
                if other.id == c.id {
                    return Err(RegistryError::DuplicateId(c.id.into()));
                }
                if other.shortcut.eq_ignore_ascii_case(c.shortcut) {
                    return Err(RegistryError::DuplicateShortcut {
                        shortcut: c.shortcut.into(),
                        first: other.id.into(),
                        second: c.id.into(),
                    });
                }
            }
        }
        Ok(Self {
            commands: commands.to_vec(),
        })
    }

    /// The registry every window shares. A collision here is a programming error caught at
    /// startup, never a silent precedence rule.
    pub fn standard() -> Result<Self, RegistryError> {
        Self::new(STANDARD)
    }

    /// The process-wide registry. Built once; a collision panics with the offending pair.
    pub fn shared() -> &'static CommandRegistry {
        static SHARED: std::sync::OnceLock<CommandRegistry> = std::sync::OnceLock::new();
        SHARED.get_or_init(|| {
            CommandRegistry::standard().unwrap_or_else(|e| panic!("invalid command registry: {e}"))
        })
    }

    pub fn all(&self) -> &[Command] {
        &self.commands
    }

    pub fn get(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|c| c.id == id)
    }

    /// Evaluated from global state only: the same answer whichever window asks.
    pub fn enablement(&self, id: &str, health: &Health, kill_switch: bool) -> Enablement {
        let Some(command) = self.get(id) else {
            return Enablement::Disabled {
                reason: "unknown command".into(),
            };
        };
        match (command.kill_switch, kill_switch) {
            (KillSwitch::DisabledWhenOn, true) => {
                return Enablement::Disabled {
                    reason: "Kill switch engaged".into(),
                }
            }
            (KillSwitch::DisabledWhenOff, false) => {
                return Enablement::Disabled {
                    reason: "Kill switch not engaged".into(),
                }
            }
            _ => {}
        }
        match command.rule {
            Some(kind) => enablement::enabled(kind, health),
            None => Enablement::Enabled,
        }
    }

    pub fn to_json(&self) -> String {
        let rows: Vec<_> = self
            .commands
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "label": c.label,
                    "shortcut": c.shortcut,
                    "scope": match c.scope { Scope::App => "app", Scope::Panel(_) => "panel" },
                    "panel": match c.scope { Scope::App => "", Scope::Panel(k) => k },
                })
            })
            .collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::enablement::WorkerStatus;

    fn healthy() -> Health {
        Health {
            api_reachable: true,
            postgres_available: true,
            worker_status: WorkerStatus::Active,
            worker_heartbeat_age_s: 1.0,
            edge_reachable: true,
            edge_mt5_connected: true,
        }
    }

    #[test]
    fn standard_registry_is_valid_and_enumerable() {
        let reg = CommandRegistry::standard().expect("standard registry");
        assert!(reg.all().len() >= 12);
        assert!(reg
            .all()
            .iter()
            .all(|c| !c.label.is_empty() && !c.shortcut.is_empty()));
        let json: serde_json::Value = serde_json::from_str(&reg.to_json()).unwrap();
        assert_eq!(json.as_array().unwrap().len(), reg.all().len());
    }

    #[test]
    fn duplicate_shortcut_is_a_startup_error() {
        let a = cmd("a", "A", "Ctrl+K", Scope::App, None, KillSwitch::Ignored);
        let b = cmd(
            "b",
            "B",
            "ctrl+k",
            Scope::Panel("detail"),
            None,
            KillSwitch::Ignored,
        );
        match CommandRegistry::new(&[a, b]) {
            Err(RegistryError::DuplicateShortcut { first, second, .. }) => {
                assert_eq!((first.as_str(), second.as_str()), ("a", "b"));
            }
            other => panic!("expected a shortcut collision, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_id_and_missing_fields_are_rejected() {
        let a = cmd("a", "A", "Ctrl+1", Scope::App, None, KillSwitch::Ignored);
        let dup = cmd("a", "A2", "Ctrl+2", Scope::App, None, KillSwitch::Ignored);
        assert_eq!(
            CommandRegistry::new(&[a, dup]).unwrap_err(),
            RegistryError::DuplicateId("a".into())
        );
        let no_label = cmd("x", "", "Ctrl+3", Scope::App, None, KillSwitch::Ignored);
        assert!(matches!(
            CommandRegistry::new(&[no_label]),
            Err(RegistryError::MissingLabel(_))
        ));
    }

    #[test]
    fn kill_switch_disables_the_same_commands_for_every_caller() {
        let reg = CommandRegistry::standard().unwrap();
        let h = healthy();
        assert!(reg.enablement("deployment.create", &h, false).is_enabled());
        assert!(!reg.enablement("deployment.create", &h, true).is_enabled());
        assert!(!reg.enablement("kill-switch.set", &h, true).is_enabled());
        assert!(reg.enablement("kill-switch.clear", &h, true).is_enabled());
        assert!(!reg.enablement("kill-switch.clear", &h, false).is_enabled());
    }

    #[test]
    fn section_8_1_rules_still_apply() {
        let reg = CommandRegistry::standard().unwrap();
        let mut h = healthy();
        h.worker_status = WorkerStatus::Offline;
        assert!(!reg.enablement("deployment.create", &h, false).is_enabled());
        h.api_reachable = false;
        assert!(!reg.enablement("kill-switch.set", &h, false).is_enabled());
        assert!(!reg.enablement("nope", &h, false).is_enabled());
    }
}
