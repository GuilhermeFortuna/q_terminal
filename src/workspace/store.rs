//! Workspace persistence under the XDG config directory (Q-053).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::schema::{
    default_chart, default_single, default_trading, parse_workspace, serialize_workspace,
    LoadError, WorkspaceFile, CURRENT_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InjectedWriteFailure {
    #[default]
    None,
    Write,
    Sync,
    Rename,
}

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
    failure_injector: InjectedWriteFailure,
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
            failure_injector: InjectedWriteFailure::None,
        };
        store.ensure_defaults();
        store.last_used = store.read_last_used();
        store
    }

    pub fn set_injected_failure(&mut self, failure: InjectedWriteFailure) {
        self.failure_injector = failure;
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

    pub fn unused_recovery_name(&self) -> String {
        let base = "Recovery";
        if !self.path_for(base).exists() {
            return base.to_string();
        }
        let mut n = 2;
        loop {
            let candidate = format!("{base} {n}");
            if !self.path_for(&candidate).exists() {
                return candidate;
            }
            n += 1;
        }
    }

    pub fn load_last_or_default(&mut self) -> (WorkspaceFile, bool) {
        let fallback = default_chart();
        let name = match self.last_used.as_ref() {
            Some(n) => n.clone(),
            None => {
                return match self.load("Chart") {
                    Ok(file) => (file, false),
                    Err(LoadError::FutureVersion { found, current }) => {
                        let mut rec = fallback;
                        rec.name = self.unused_recovery_name();
                        self.reports.push(WorkspaceReport {
                            name: "Chart".to_string(),
                            message: format!(
                                "workspace schema {found} is newer than supported {current}; using recovery workspace '{}'",
                                rec.name
                            ),
                        });
                        (rec, true)
                    }
                    Err(_) => (fallback, false),
                };
            }
        };
        match self.load(&name) {
            Ok(file) => (file, false),
            Err(LoadError::FutureVersion { found, current }) => {
                let mut rec = fallback;
                rec.name = self.unused_recovery_name();
                self.reports.push(WorkspaceReport {
                    name: name.clone(),
                    message: format!(
                        "could not restore '{name}': workspace schema {found} is newer than supported {current}; using recovery workspace '{}'",
                        rec.name
                    ),
                });
                (rec, true)
            }
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
        atomic_write(&self.dir, &path, &text, self.failure_injector)
            .map_err(|e| LoadError::Io(e.to_string()))?;
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
        let defaults = [default_chart(), default_trading(), default_single()];
        for file in defaults {
            let path = self.path_for(&file.name);
            if !path.exists() {
                let text = serialize_workspace(&file);
                let _ = atomic_write(&self.dir, &path, &text, InjectedWriteFailure::None);
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
        atomic_write(&self.dir, &self.state_path, &text, self.failure_injector)
            .map_err(|e| LoadError::Io(e.to_string()))?;
        Ok(())
    }

    /// Renames a corrupt file aside instead of deleting it. Future-version files are preserved unchanged.
    pub fn quarantine(&mut self, name: &str, reason: &str) -> Result<(), LoadError> {
        let path = self.path_for(name);
        if !path.exists() {
            return Ok(());
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(raw) = toml::from_str::<toml::Value>(&content) {
                if let Some(v) = raw.get("schema_version").and_then(|v| v.as_integer()) {
                    if v as u32 > CURRENT_SCHEMA_VERSION {
                        return Ok(());
                    }
                }
            }
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

fn atomic_write(
    dir: &Path,
    dest_path: &Path,
    content: &str,
    failure_injector: InjectedWriteFailure,
) -> Result<(), std::io::Error> {
    use std::io::Write;

    if failure_injector == InjectedWriteFailure::Write {
        return Err(std::io::Error::other("injected write failure"));
    }

    let tmp_path = dir.join(format!(
        ".tmp-{}-{}",
        dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("ws"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let res = (|| -> Result<(), std::io::Error> {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;

        if failure_injector == InjectedWriteFailure::Sync {
            return Err(std::io::Error::other("injected sync failure"));
        }
        file.sync_all()?;
        drop(file);

        if failure_injector == InjectedWriteFailure::Rename {
            return Err(std::io::Error::other("injected rename failure"));
        }
        fs::rename(&tmp_path, dest_path)?;

        if let Ok(parent) = fs::File::open(dir) {
            let _ = parent.sync_all();
        }
        Ok(())
    })();

    if res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    res
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
        assert!(store.path_for("Chart").exists());
        assert!(store.path_for("Trading").exists());
        assert!(store.path_for("Single monitor").exists());
        let chart = store.load("Chart").unwrap();
        assert_eq!(chart.windows.len(), 1);
        assert_eq!(chart.windows[0].root.panels(), vec!["chart"]);
        let trading = store.load("Trading").unwrap();
        assert_eq!(trading.windows.len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn fresh_config_selects_chart_when_last_used_absent() {
        let dir = temp_dir("fresh_chart");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let (file, fallback) = store.load_last_or_default();
        assert_eq!(file.name, "Chart");
        assert!(!fallback);
        assert_eq!(file.windows.len(), 1);
        assert_eq!(file.windows[0].root.panels(), vec!["chart"]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recorded_last_used_is_restored() {
        let dir = temp_dir("recorded");
        let store = WorkspaceStore::open_dir(dir.clone());
        store.set_last_used("Trading").unwrap();
        let mut store2 = WorkspaceStore::open_dir(dir.clone());
        let (file, fallback) = store2.load_last_or_default();
        assert_eq!(file.name, "Trading");
        assert!(!fallback);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unreadable_last_used_falls_back_to_chart() {
        let dir = temp_dir("unreadable");
        let store = WorkspaceStore::open_dir(dir.clone());
        store.set_last_used("Corrupt").unwrap();
        let bad_path = store.path_for("Corrupt");
        fs::write(bad_path, "invalid toml [[[").unwrap();
        let mut store2 = WorkspaceStore::open_dir(dir.clone());
        let (file, fallback) = store2.load_last_or_default();
        assert_eq!(file.name, "Chart");
        assert!(fallback);
        assert!(store2.reports().iter().any(|r| r.name == "Corrupt"));
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

    #[test]
    fn future_version_is_preserved_and_uses_recovery_workspace() {
        let dir = temp_dir("future_preservation");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let future_content = "schema_version = 99\nname = \"FutureWs\"\nwindows = []\nselection = { global = \"\", detached = {} }\n";
        let future_path = store.path_for("FutureWs");
        fs::write(&future_path, future_content).unwrap();
        store.set_last_used("FutureWs").unwrap();

        let (recovery, fallback) = store.load_last_or_default();
        assert!(fallback);
        assert_eq!(recovery.name, "Recovery");
        // Verify future version file is UNCHANGED
        let disk_content = fs::read_to_string(&future_path).unwrap();
        assert_eq!(disk_content, future_content);

        // Saving recovery workspace does NOT touch future file
        store.save(&recovery).unwrap();
        assert!(store.path_for("Recovery").exists());
        let disk_content2 = fs::read_to_string(&future_path).unwrap();
        assert_eq!(disk_content2, future_content);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn injected_write_failure_preserves_valid_file() {
        let dir = temp_dir("injected_write");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let orig = store.load("Trading").unwrap();
        let orig_text = fs::read_to_string(store.path_for("Trading")).unwrap();

        let mut edited = orig.clone();
        edited.name = "Trading".into();
        edited.windows.clear(); // changed

        store.set_injected_failure(InjectedWriteFailure::Write);
        let res = store.save(&edited);
        assert!(res.is_err());

        // File on disk must be untouched
        let on_disk = fs::read_to_string(store.path_for("Trading")).unwrap();
        assert_eq!(on_disk, orig_text);

        // Retrying without failure succeeds
        store.set_injected_failure(InjectedWriteFailure::None);
        let mut valid_edit = orig;
        valid_edit.windows[0].composition = "custom".into();
        assert!(store.save(&valid_edit).is_ok());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn injected_sync_failure_preserves_valid_file() {
        let dir = temp_dir("injected_sync");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let orig_text = fs::read_to_string(store.path_for("Trading")).unwrap();

        let mut edited = store.load("Trading").unwrap();
        edited.windows.clear();

        store.set_injected_failure(InjectedWriteFailure::Sync);
        assert!(store.save(&edited).is_err());

        let on_disk = fs::read_to_string(store.path_for("Trading")).unwrap();
        assert_eq!(on_disk, orig_text);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn injected_rename_failure_preserves_valid_file() {
        let dir = temp_dir("injected_rename");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let orig_text = fs::read_to_string(store.path_for("Trading")).unwrap();

        let mut edited = store.load("Trading").unwrap();
        edited.windows.clear();

        store.set_injected_failure(InjectedWriteFailure::Rename);
        assert!(store.save(&edited).is_err());

        let on_disk = fs::read_to_string(store.path_for("Trading")).unwrap();
        assert_eq!(on_disk, orig_text);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn quarantine_preserves_future_version_file() {
        let dir = temp_dir("future_quarantine");
        let mut store = WorkspaceStore::open_dir(dir.clone());
        let future_content = "schema_version = 99\nname = \"Future\"\nwindows = []\n";
        let future_path = store.path_for("Future");
        fs::write(&future_path, future_content).unwrap();

        store.quarantine("Future", "newer version").unwrap();
        // File must still exist with original name, not moved to .bad-*
        assert!(future_path.exists());
        let count = dir
            .read_dir()
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .contains(".bad-")
            })
            .count();
        assert_eq!(count, 0);

        let _ = fs::remove_dir_all(dir);
    }
}
