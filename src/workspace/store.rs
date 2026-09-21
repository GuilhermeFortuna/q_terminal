//! Workspace persistence under the XDG config directory (Q-053).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::schema::{
    default_single, default_trading, parse_workspace, serialize_workspace, LoadError, WorkspaceFile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReport {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct WorkspaceStore {
    dir: PathBuf,
    state_path: PathBuf,
    last_used: Option<String>,
    reports: Vec<WorkspaceReport>,
}

impl WorkspaceStore {
    pub fn open_default() -> Self {
        Self::open_dir(default_workspaces_dir())
    }

    pub fn open_dir(dir: PathBuf) -> Self {
        let state_path = dir.join("state.toml");
        let mut store = Self {
            dir,
            state_path,
            last_used: None,
            reports: Vec::new(),
        };
        store.ensure_defaults();
        store.last_used = store.read_last_used();
        store
    }

    pub fn reports(&self) -> &[WorkspaceReport] {
        &self.reports
    }

    pub fn workspaces_dir(&self) -> &Path {
        &self.dir
    }

    pub fn list(&self) -> Vec<String> {
        let mut names = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml")
                    && path.file_name() != Some(self.state_path.file_name().unwrap())
                {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(file) = parse_workspace(&content) {
                            names.push(file.name);
                        }
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    pub fn load(&mut self, name: &str) -> Result<WorkspaceFile, LoadError> {
        let path = self.path_for(name);
        let content = fs::read_to_string(&path).map_err(|e| LoadError::Io(e.to_string()))?;
        parse_workspace(&content)
    }

    pub fn load_last_or_default(&mut self) -> (WorkspaceFile, bool) {
        let fallback = default_single();
        let name = self
            .last_used
            .clone()
            .or_else(|| self.list().first().cloned())
            .unwrap_or_else(|| fallback.name.clone());
        match self.load(&name) {
            Ok(file) => (file, false),
            Err(err) => {
                self.reports.push(WorkspaceReport {
                    name: name.clone(),
                    message: format!("could not restore '{name}': {err}; using default"),
                });
                (fallback, true)
            }
        }
    }

    pub fn save(&self, file: &WorkspaceFile) -> Result<(), LoadError> {
        fs::create_dir_all(&self.dir).map_err(|e| LoadError::Io(e.to_string()))?;
        let path = self.path_for(&file.name);
        let text = serialize_workspace(file);
        fs::write(path, text).map_err(|e| LoadError::Io(e.to_string()))?;
        self.write_last_used(&file.name)?;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), LoadError> {
        let path = self.path_for(name);
        if path.exists() {
            fs::remove_file(path).map_err(|e| LoadError::Io(e.to_string()))?;
        }
        if self.last_used.as_deref() == Some(name) {
            self.last_used = None;
            let _ = fs::remove_file(&self.state_path);
        }
        Ok(())
    }

    pub fn duplicate(&mut self, from: &str, to: &str) -> Result<(), LoadError> {
        let file = self.load(from)?;
        let mut copy = file;
        copy.name = to.to_string();
        self.save(&copy)
    }

    pub fn set_last_used(&self, name: &str) -> Result<(), LoadError> {
        self.write_last_used(name)
    }

    pub fn path_for(&self, name: &str) -> PathBuf {
        let slug = slugify(name);
        self.dir.join(format!("{slug}.toml"))
    }

    fn ensure_defaults(&mut self) {
        let _ = fs::create_dir_all(&self.dir);
        let defaults = [default_trading(), default_single()];
        for file in defaults {
            let path = self.path_for(&file.name);
            if !path.exists() {
                let _ = self.save(&file);
            }
        }
    }

    fn read_last_used(&self) -> Option<String> {
        if !self.state_path.exists() {
            return None;
        }
        let content = fs::read_to_string(&self.state_path).ok()?;
        let map: HashMap<String, String> = toml::from_str(&content).ok()?;
        map.get("last_workspace").cloned()
    }

    fn write_last_used(&self, name: &str) -> Result<(), LoadError> {
        fs::create_dir_all(&self.dir).map_err(|e| LoadError::Io(e.to_string()))?;
        let text = format!("last_workspace = \"{}\"\n", name.replace('"', "\\\""));
        fs::write(&self.state_path, text).map_err(|e| LoadError::Io(e.to_string()))?;
        Ok(())
    }

    /// Renames a corrupt file aside instead of deleting it.
    pub fn quarantine(&mut self, name: &str, reason: &str) -> Result<(), LoadError> {
        let path = self.path_for(name);
        if !path.exists() {
            return Ok(());
        }
        let quarantined = self.dir.join(format!(
            "{}.bad-{}",
            path.file_stem().unwrap().to_string_lossy(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ));
        fs::rename(&path, quarantined).map_err(|e| LoadError::Io(e.to_string()))?;
        self.reports.push(WorkspaceReport {
            name: name.to_string(),
            message: format!("quarantined unreadable workspace: {reason}"),
        });
        Ok(())
    }
}

pub fn default_workspaces_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("q_terminal").join("workspaces");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("q_terminal")
            .join("workspaces");
    }
    PathBuf::from(".config/q_terminal/workspaces")
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::schema::CURRENT_SCHEMA_VERSION;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "q_terminal_ws_{}_{}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&path);
        path
    }

    #[test]
    fn defaults_created_once() {
        let dir = temp_dir("defaults");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        assert!(store.path_for("Trading").exists());
        assert!(store.path_for("Single monitor").exists());
        let trading = store.load("Trading").unwrap();
        assert_eq!(trading.windows.len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_reload_round_trip() {
        let dir = temp_dir("roundtrip");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let file = store.load("Single monitor").unwrap();
        store.save(&file).unwrap();
        let again = store.load("Single monitor").unwrap();
        assert_eq!(again.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(again, file);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_file_quarantined_not_deleted() {
        let dir = temp_dir("corrupt");
        let store = WorkspaceStore::open_dir(dir.clone());
        let bad = dir.join("broken.toml");
        fs::write(&bad, "not valid").unwrap();
        let mut store = store;
        store.quarantine("broken", "parse").unwrap();
        assert!(!bad.exists());
        assert!(dir.read_dir().unwrap().count() >= 1);
        let _ = fs::remove_dir_all(dir);
    }
}
