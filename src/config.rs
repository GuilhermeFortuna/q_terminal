use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Read once at startup from $XDG_CONFIG_HOME/q_terminal/config.toml, with
/// every field overridable by an environment variable. No default address is
/// compiled into a binary that talks to a machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub api_base: String,
    pub symbol: String,
    pub timeframe: String,
    pub operator: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingField(String),
    Io(String),
    Parse(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingField(field) => write!(f, "missing configuration field: {field}"),
            ConfigError::Io(err) => write!(f, "failed to read configuration file: {err}"),
            ConfigError::Parse(err) => write!(f, "failed to parse configuration file: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn default_config_path(get_env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(xdg) = get_env("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join("q_terminal")
                    .join("config.toml"),
            );
        }
    }
    if let Some(home) = get_env("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Some(
                PathBuf::from(trimmed)
                    .join(".config")
                    .join("q_terminal")
                    .join("config.toml"),
            );
        }
    }
    None
}

fn parse_toml_str(content: &str) -> Result<HashMap<String, String>, ConfigError> {
    let mut map = HashMap::new();
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Skip table headers like [stream]
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            let mut val = v.trim();
            // Strip comments if any outside quotes
            if let Some(idx) = val.find('#') {
                // Check if hash is inside quotes
                let prefix = &val[..idx];
                let single_quotes = prefix.chars().filter(|&c| c == '\'').count();
                let double_quotes = prefix.chars().filter(|&c| c == '"').count();
                if single_quotes % 2 == 0 && double_quotes % 2 == 0 {
                    val = prefix.trim();
                }
            }
            // Strip outer quotes
            let cleaned_val = if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                if val.len() >= 2 {
                    &val[1..val.len() - 1]
                } else {
                    val
                }
            } else {
                val
            };
            map.insert(key, cleaned_val.to_string());
        } else {
            return Err(ConfigError::Parse(format!(
                "invalid syntax on line {}: {}",
                line_no + 1,
                line
            )));
        }
    }
    Ok(map)
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_with(|k| std::env::var(k).ok())
    }

    pub fn load_with(get_env: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let path = default_config_path(&get_env);
        Self::load_from_path_and_env(path.as_deref(), get_env)
    }

    pub fn load_from_path_and_env(
        path: Option<&Path>,
        get_env: impl Fn(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let mut file_values = HashMap::new();
        if let Some(p) = path {
            if p.exists() {
                let content = std::fs::read_to_string(p)
                    .map_err(|e| ConfigError::Io(format!("{}: {}", p.display(), e)))?;
                file_values = parse_toml_str(&content)?;
            }
        }

        // Resolving api_base / address
        let api_base = get_env("Q_TERMINAL_API_BASE")
            .or_else(|| get_env("API_BASE"))
            .or_else(|| get_env("Q_TERMINAL_ADDRESS"))
            .or_else(|| get_env("ADDRESS"))
            .or_else(|| file_values.get("api_base").cloned())
            .or_else(|| file_values.get("address").cloned())
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ConfigError::MissingField("api_base".to_string()))?;

        // Resolving symbol
        let symbol = get_env("Q_TERMINAL_SYMBOL")
            .or_else(|| get_env("SYMBOL"))
            .or_else(|| file_values.get("symbol").cloned())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ConfigError::MissingField("symbol".to_string()))?;

        // Resolving timeframe
        let timeframe = get_env("Q_TERMINAL_TIMEFRAME")
            .or_else(|| get_env("TIMEFRAME"))
            .or_else(|| file_values.get("timeframe").cloned())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ConfigError::MissingField("timeframe".to_string()))?;

        let operator = get_env("Q_TERMINAL_OPERATOR")
            .or_else(|| file_values.get("operator").cloned())
            .or_else(|| get_env("USER"))
            .or_else(|| get_env("USERNAME"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "operator".to_string());

        Ok(Self {
            api_base,
            symbol,
            timeframe,
            operator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "q_terminal_test_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn test_complete_file_loads() {
        let dir = temp_test_dir("complete_file");
        let config_path = dir.join("config.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "# Q terminal config").unwrap();
        writeln!(file, "api_base = \"http://127.0.0.1:8000/\"").unwrap();
        writeln!(file, "symbol = \"PETR4\"").unwrap();
        writeln!(file, "timeframe = \"1m\"").unwrap();

        let config = Config::load_from_path_and_env(Some(&config_path), |_| None).unwrap();
        // Trailing slash stripped
        assert_eq!(config.api_base, "http://127.0.0.1:8000");
        assert_eq!(config.symbol, "PETR4");
        assert_eq!(config.timeframe, "1m");
        assert_eq!(config.operator, "operator");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_operator_from_env() {
        let dir = temp_test_dir("operator_env");
        let config_path = dir.join("config.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "api_base = \"http://127.0.0.1:8000\"").unwrap();
        writeln!(file, "symbol = \"PETR4\"").unwrap();
        writeln!(file, "timeframe = \"1m\"").unwrap();

        let env_map = |key: &str| match key {
            "Q_TERMINAL_OPERATOR" => Some("desk-alpha".into()),
            _ => None,
        };
        let config = Config::load_from_path_and_env(Some(&config_path), env_map).unwrap();
        assert_eq!(config.operator, "desk-alpha");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_missing_file_with_env_vars_loads() {
        let dir = temp_test_dir("missing_file");
        let nonexistent_path = dir.join("nonexistent.toml");

        let env_map = |key: &str| match key {
            "API_BASE" => Some("http://127.0.0.1:9000".into()),
            "SYMBOL" => Some("VALE3".into()),
            "TIMEFRAME" => Some("5m".into()),
            _ => None,
        };

        let config = Config::load_from_path_and_env(Some(&nonexistent_path), env_map).unwrap();
        assert_eq!(config.api_base, "http://127.0.0.1:9000");
        assert_eq!(config.symbol, "VALE3");
        assert_eq!(config.timeframe, "5m");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_missing_address_is_error_naming_field() {
        let dir = temp_test_dir("missing_address");
        let config_path = dir.join("config.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "symbol = \"PETR4\"").unwrap();
        writeln!(file, "timeframe = \"1m\"").unwrap();

        let err = Config::load_from_path_and_env(Some(&config_path), |_| None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("api_base") || msg.contains("address"),
            "Error message should name the field, got: {msg}"
        );
        match err {
            ConfigError::MissingField(field) => {
                assert!(field.contains("api_base") || field.contains("address"));
            }
            other => panic!("expected MissingField, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_no_address_baked_in() {
        // With no config file and no environment variables, loading must fail because no address is baked in
        let err = Config::load_from_path_and_env(None, |_| None).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("api_base") || msg.contains("address"),
            "Error message should name the missing address field, got: {msg}"
        );
    }

    #[test]
    fn test_env_var_overrides_file() {
        let dir = temp_test_dir("env_override");
        let config_path = dir.join("config.toml");
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "api_base = \"http://127.0.0.1:8000\"").unwrap();
        writeln!(file, "symbol = \"PETR4\"").unwrap();
        writeln!(file, "timeframe = \"1m\"").unwrap();

        let env_map = |key: &str| match key {
            "Q_TERMINAL_SYMBOL" => Some("ITUB4".into()),
            _ => None,
        };

        let config = Config::load_from_path_and_env(Some(&config_path), env_map).unwrap();
        assert_eq!(config.symbol, "ITUB4");
        assert_eq!(config.api_base, "http://127.0.0.1:8000");
        assert_eq!(config.timeframe, "1m");
        let _ = std::fs::remove_dir_all(dir);
    }
}
