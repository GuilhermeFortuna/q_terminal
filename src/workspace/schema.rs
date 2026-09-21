//! Versioned workspace TOML schema and migrations (Q-053).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::identity::ScreenFingerprint;
use crate::display::placement::GeometryIntent;
use crate::shell::layout::Node;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub schema_version: u32,
    pub name: String,
    pub windows: Vec<WorkspaceWindow>,
    pub selection: WorkspaceSelection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceWindow {
    pub composition: String,
    pub root: Node,
    pub display: ScreenFingerprint,
    pub geometry: GeometryIntent,
    pub detached: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkspaceSelection {
    pub global: String,
    pub detached: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Io(String),
    Parse(String),
    FutureVersion { found: u32, current: u32 },
    Truncated,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "workspace io error: {e}"),
            Self::Parse(e) => write!(f, "workspace parse error: {e}"),
            Self::FutureVersion { found, current } => {
                write!(
                    f,
                    "workspace schema {found} is newer than supported {current}"
                )
            }
            Self::Truncated => write!(f, "workspace file is truncated or incomplete"),
        }
    }
}

impl std::error::Error for LoadError {}

pub fn migrate(raw: toml::Value) -> Result<WorkspaceFile, LoadError> {
    let version = raw
        .get("schema_version")
        .and_then(|v| v.as_integer())
        .map(|v| v as u32)
        .ok_or(LoadError::Truncated)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(LoadError::FutureVersion {
            found: version,
            current: CURRENT_SCHEMA_VERSION,
        });
    }
    let migrated = match version {
        0 | 1 => raw,
        _ => {
            return Err(LoadError::FutureVersion {
                found: version,
                current: CURRENT_SCHEMA_VERSION,
            })
        }
    };
    let file: WorkspaceFile = migrated
        .try_into()
        .map_err(|e: toml::de::Error| LoadError::Parse(e.to_string()))?;
    if file.name.is_empty() || file.windows.is_empty() {
        return Err(LoadError::Truncated);
    }
    Ok(file)
}

pub fn parse_workspace(content: &str) -> Result<WorkspaceFile, LoadError> {
    if content.trim().is_empty() {
        return Err(LoadError::Truncated);
    }
    let raw: toml::Value = toml::from_str(content).map_err(|e| LoadError::Parse(e.to_string()))?;
    migrate(raw)
}

pub fn serialize_workspace(file: &WorkspaceFile) -> String {
    let mut out = file.clone();
    out.schema_version = CURRENT_SCHEMA_VERSION;
    toml::to_string_pretty(&out).unwrap_or_default()
}

pub fn default_trading() -> WorkspaceFile {
    use crate::display::identity::fixtures;
    use crate::shell::layout::composition;

    WorkspaceFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        name: "Trading".into(),
        windows: vec![
            WorkspaceWindow {
                composition: "market".into(),
                root: composition("market").expect("market"),
                display: fixtures::asus_vg328().fingerprint,
                geometry: GeometryIntent {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                detached: false,
            },
            WorkspaceWindow {
                composition: "operations".into(),
                root: composition("operations").expect("operations"),
                display: fixtures::samsung_odyssey_g5().fingerprint,
                geometry: GeometryIntent {
                    x: 1920,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
                detached: false,
            },
        ],
        selection: WorkspaceSelection::default(),
    }
}

pub fn default_single() -> WorkspaceFile {
    use crate::display::identity::fixtures;
    use crate::shell::layout::composition;

    WorkspaceFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        name: "Single monitor".into(),
        windows: vec![WorkspaceWindow {
            composition: "merged".into(),
            root: composition("merged").expect("merged"),
            display: fixtures::asus_vg328().fingerprint,
            geometry: GeometryIntent {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            detached: false,
        }],
        selection: WorkspaceSelection::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_serialises_schema_version() {
        let file = default_single();
        let text = serialize_workspace(&file);
        let loaded = parse_workspace(&text).unwrap();
        assert_eq!(loaded, file);
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn future_version_is_rejected() {
        let text = "schema_version = 99\nname = \"x\"\nwindows = []\nselection = { global = \"\", detached = {} }\n";
        let err = parse_workspace(text).unwrap_err();
        assert!(matches!(err, LoadError::FutureVersion { .. }));
    }

    #[test]
    fn malformed_file_is_reported() {
        let err = parse_workspace("not valid [[[").unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)));
    }

    #[test]
    fn truncated_file_is_reported() {
        let err = parse_workspace("schema_version = 1").unwrap_err();
        assert!(matches!(err, LoadError::Truncated | LoadError::Parse(_)));
    }
}
