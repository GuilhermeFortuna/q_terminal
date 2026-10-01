//! Versioned workspace TOML schema and migrations (Q-053).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::identity::ScreenFingerprint;
use crate::display::placement::GeometryIntent;
use crate::shell::layout::Node;

pub const CURRENT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub schema_version: u32,
    pub name: String,
    pub windows: Vec<WorkspaceWindow>,
    pub selection: WorkspaceSelection,
    #[serde(default, deserialize_with = "deserialize_study_sets")]
    pub study_sets: HashMap<String, Vec<WorkspaceStudy>>,
    #[serde(default, deserialize_with = "deserialize_chart_preferences")]
    pub chart_preferences: HashMap<String, WorkspaceChartPreferences>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceChartPreferences {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub timeframe: String,
    #[serde(default = "default_chart_mode")]
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followed_deployment_id: Option<String>,
    #[serde(
        default,
        alias = "manual_symbol",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_manual_symbol: Option<String>,
    #[serde(
        default,
        alias = "manual_timeframe",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_manual_timeframe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_bars: Option<i32>,
}

fn default_chart_mode() -> String {
    "following".to_string()
}

fn deserialize_chart_preferences<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, WorkspaceChartPreferences>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, toml::Value>::deserialize(deserializer)?;
    let mut output = HashMap::new();
    for (panel, val) in raw {
        if let Ok(mut prefs) = val.try_into::<WorkspaceChartPreferences>() {
            if let Some(bars) = prefs.visible_bars {
                if bars <= 0 || bars > 10000 {
                    eprintln!("workspace: dropped invalid visible_bars ({bars}) for panel {panel}");
                    prefs.visible_bars = None;
                }
            }
            if !prefs.symbol.is_empty() && !crate::chart_target::is_valid_symbol(&prefs.symbol) {
                eprintln!(
                    "workspace: dropped invalid symbol ('{}') for panel {panel}",
                    prefs.symbol
                );
                prefs.symbol = String::new();
            }
            if !prefs.timeframe.is_empty()
                && !crate::chart_target::is_valid_timeframe(&prefs.timeframe)
            {
                eprintln!(
                    "workspace: dropped invalid timeframe ('{}') for panel {panel}",
                    prefs.timeframe
                );
                prefs.timeframe = String::new();
            }
            if prefs.mode != "following" && prefs.mode != "manual" {
                eprintln!(
                    "workspace: dropped invalid mode ('{}') for panel {panel}",
                    prefs.mode
                );
                prefs.mode = default_chart_mode();
            }
            if let Some(ref sym) = prefs.last_manual_symbol {
                if !sym.is_empty() && !crate::chart_target::is_valid_symbol(sym) {
                    eprintln!(
                        "workspace: dropped invalid last_manual_symbol ('{}') for panel {panel}",
                        sym
                    );
                    prefs.last_manual_symbol = None;
                }
            }
            if let Some(ref tf) = prefs.last_manual_timeframe {
                if !tf.is_empty() && !crate::chart_target::is_valid_timeframe(tf) {
                    eprintln!(
                        "workspace: dropped invalid last_manual_timeframe ('{}') for panel {panel}",
                        tf
                    );
                    prefs.last_manual_timeframe = None;
                }
            }
            output.insert(panel, prefs);
        } else {
            eprintln!("workspace: dropped invalid chart_preferences for panel {panel}");
        }
    }
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceStudy {
    pub kind: String,
    pub period: i64,
    pub source: String,
    pub num_std: f64,
    #[serde(default = "default_visible")]
    pub visible: bool,
    pub palette_index: u8,
}

fn default_visible() -> bool {
    true
}

fn deserialize_study_sets<'de, D>(
    deserializer: D,
) -> Result<HashMap<String, Vec<WorkspaceStudy>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = HashMap::<String, Vec<toml::Value>>::deserialize(deserializer)?;
    let mut output = HashMap::new();
    for (panel, entries) in raw {
        let valid = entries
            .into_iter()
            .filter_map(|entry| {
                let parsed: Result<WorkspaceStudy, _> = entry.try_into();
                match parsed {
                    Ok(study)
                        if ["sma", "ema", "bollinger", "vwap", "rsi", "atr"]
                            .contains(&study.kind.as_str())
                            && ["close", "open", "high", "low", "hlc3"]
                                .contains(&study.source.as_str())
                            && (1..=1000).contains(&study.period)
                            && study.num_std.is_finite()
                            && (0.1..=10.0).contains(&study.num_std)
                            && study.palette_index < 8 =>
                    {
                        Some(study)
                    }
                    _ => {
                        eprintln!("workspace: dropped invalid study entry for panel {panel}");
                        None
                    }
                }
            })
            .take(8)
            .collect::<Vec<_>>();
        if !valid.is_empty() {
            output.insert(panel, valid);
        }
    }
    Ok(output)
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
        0 | 1 => {
            let mut raw = raw;
            if let Some(table) = raw.as_table_mut() {
                table.insert(
                    "schema_version".into(),
                    toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
                );
                table.insert(
                    "study_sets".into(),
                    toml::Value::Table(toml::map::Map::new()),
                );
                table.insert(
                    "chart_preferences".into(),
                    toml::Value::Table(toml::map::Map::new()),
                );
            }
            raw
        }
        2 => {
            let mut raw = raw;
            if let Some(table) = raw.as_table_mut() {
                table.insert(
                    "schema_version".into(),
                    toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
                );
                table.insert(
                    "chart_preferences".into(),
                    toml::Value::Table(toml::map::Map::new()),
                );
            }
            raw
        }
        3 => raw,
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
        study_sets: HashMap::new(),
        chart_preferences: HashMap::new(),
    }
}

pub fn default_chart() -> WorkspaceFile {
    use crate::display::identity::fixtures;

    WorkspaceFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        name: "Chart".into(),
        windows: vec![WorkspaceWindow {
            composition: "chart".into(),
            root: Node::tabs(&["chart"]),
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
        study_sets: HashMap::new(),
        chart_preferences: HashMap::new(),
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
        study_sets: HashMap::new(),
        chart_preferences: HashMap::new(),
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

    #[test]
    fn version_one_workspace_migrates_without_studies() {
        let mut raw: toml::Value = toml::from_str(&serialize_workspace(&default_single())).unwrap();
        raw["schema_version"] = toml::Value::Integer(1);
        raw.as_table_mut().unwrap().remove("study_sets");
        raw.as_table_mut().unwrap().remove("chart_preferences");
        let loaded = parse_workspace(&toml::to_string(&raw).unwrap()).unwrap();
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(loaded.study_sets.is_empty());
        assert!(loaded.chart_preferences.is_empty());
    }

    #[test]
    fn version_two_workspace_migrates_preserving_studies() {
        let mut raw: toml::Value = toml::from_str(&serialize_workspace(&default_single())).unwrap();
        raw["schema_version"] = toml::Value::Integer(2);
        raw.as_table_mut().unwrap().remove("chart_preferences");
        let mut studies = toml::map::Map::new();
        studies.insert(
            "chart".into(),
            toml::Value::Array(vec![toml::Value::Table({
                let mut s = toml::map::Map::new();
                s.insert("kind".into(), toml::Value::String("ema".into()));
                s.insert("period".into(), toml::Value::Integer(21));
                s.insert("source".into(), toml::Value::String("close".into()));
                s.insert("num_std".into(), toml::Value::Float(2.0));
                s.insert("visible".into(), toml::Value::Boolean(true));
                s.insert("palette_index".into(), toml::Value::Integer(3));
                s
            })]),
        );
        raw["study_sets"] = toml::Value::Table(studies);
        let loaded = parse_workspace(&toml::to_string(&raw).unwrap()).unwrap();
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.study_sets["chart"].len(), 1);
        assert_eq!(loaded.study_sets["chart"][0].kind, "ema");
        assert!(loaded.chart_preferences.is_empty());
    }

    #[test]
    fn study_sets_round_trip_and_invalid_studies_are_dropped() {
        let mut file = default_single();
        file.study_sets.insert(
            "chart".into(),
            vec![WorkspaceStudy {
                kind: "ema".into(),
                period: 21,
                source: "close".into(),
                num_std: 2.0,
                visible: true,
                palette_index: 3,
            }],
        );
        let loaded = parse_workspace(&serialize_workspace(&file)).unwrap();
        assert_eq!(loaded.study_sets["chart"][0], file.study_sets["chart"][0]);

        let mut raw: toml::Value = toml::from_str(&serialize_workspace(&file)).unwrap();
        raw["study_sets"]["chart"][0]["kind"] = toml::Value::String("unknown".into());
        let loaded = parse_workspace(&toml::to_string(&raw).unwrap()).unwrap();
        assert!(loaded.study_sets.is_empty());
    }

    #[test]
    fn chart_preferences_round_trip_and_invalid_settings_dropped() {
        let mut file = default_single();
        file.chart_preferences.insert(
            "chart".into(),
            WorkspaceChartPreferences {
                symbol: "PETR4".into(),
                timeframe: "1m".into(),
                mode: "following".into(),
                followed_deployment_id: Some("dep-42".into()),
                last_manual_symbol: Some("VALE3".into()),
                last_manual_timeframe: Some("5m".into()),
                visible_bars: Some(150),
            },
        );
        let text = serialize_workspace(&file);
        let loaded = parse_workspace(&text).unwrap();
        assert_eq!(
            loaded.chart_preferences["chart"],
            file.chart_preferences["chart"]
        );

        let mut raw: toml::Value = toml::from_str(&text).unwrap();
        raw["chart_preferences"]["chart"]["visible_bars"] = toml::Value::Integer(-10);
        raw["chart_preferences"]["chart"]["mode"] = toml::Value::String("invalid_mode".into());
        let loaded = parse_workspace(&toml::to_string(&raw).unwrap()).unwrap();
        let chart_prefs = &loaded.chart_preferences["chart"];
        assert_eq!(chart_prefs.visible_bars, None);
        assert_eq!(chart_prefs.mode, "following");
        assert_eq!(chart_prefs.symbol, "PETR4");
    }
}
