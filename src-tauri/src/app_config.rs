//! Persisted app settings: archive path, UI mode, and default project handling.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;

const CONFIG_FILE: &str = "config.json";

fn default_true() -> bool {
    true
}

/// Last reconstruct/archive chrome tab. Settings is not persisted as a mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiMode {
    #[default]
    Easy,
    Expert,
    Archive,
}

/// On-disk config. `archive_dir` is required once the user (or default) has picked it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub archive_dir: String,
    #[serde(default)]
    pub ui_mode: UiMode,
    #[serde(default = "default_true")]
    pub temp_project: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_dir: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            archive_dir: String::new(),
            ui_mode: UiMode::Easy,
            temp_project: true,
            project_dir: None,
        }
    }
}

impl AppConfig {
    pub fn with_archive_dir(archive_dir: impl Into<String>) -> Self {
        Self {
            archive_dir: archive_dir.into(),
            ..Self::default()
        }
    }
}

/// Loads `config.json` from the app config directory, or writes `default_archive`.
pub fn load_or_init(config_dir: &Path, default_archive: &Path) -> Result<AppConfig, PipelineError> {
    fs::create_dir_all(config_dir)?;
    let path = config_dir.join(CONFIG_FILE);
    if path.is_file() {
        let parsed: AppConfig = serde_json::from_slice(&fs::read(&path)?)
            .map_err(|err| PipelineError::message(format!("Invalid config.json: {err}")))?;
        if !parsed.archive_dir.trim().is_empty() {
            return Ok(parsed);
        }
    }
    let config = AppConfig::with_archive_dir(default_archive.to_string_lossy());
    save(config_dir, &config)?;
    Ok(config)
}

pub fn save(config_dir: &Path, config: &AppConfig) -> Result<(), PipelineError> {
    fs::create_dir_all(config_dir)?;
    fs::write(
        config_dir.join(CONFIG_FILE),
        serde_json::to_vec_pretty(config)
            .map_err(|err| PipelineError::message(format!("Could not write config: {err}")))?,
    )?;
    Ok(())
}

/// `~/Documents/Simple 3DGS/archive` when `documents` is the user Documents folder.
pub fn default_archive_dir(documents: &Path) -> PathBuf {
    documents.join("Simple 3DGS").join("archive")
}

/// Creates `preferred`. On TCC/EPERM, creates `fallback` instead so the app still starts.
pub fn resolve_archive_dir(preferred: &Path, fallback: &Path) -> Result<PathBuf, PipelineError> {
    match fs::create_dir_all(preferred) {
        Ok(()) => Ok(preferred.to_path_buf()),
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
            fs::create_dir_all(fallback)?;
            Ok(fallback.to_path_buf())
        }
        Err(err) => Err(PipelineError::from_io_path(err, preferred)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_writes_default_then_roundtrips() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let archive = dir
            .path()
            .join("Documents")
            .join("Simple 3DGS")
            .join("archive");
        let first = load_or_init(&config_dir, &archive).unwrap();
        assert_eq!(first.archive_dir, archive.to_string_lossy());
        assert_eq!(first.ui_mode, UiMode::Easy);
        assert!(first.temp_project);
        assert_eq!(first.project_dir, None);
        assert!(config_dir.join("config.json").is_file());
        let again = load_or_init(&config_dir, &dir.path().join("other")).unwrap();
        assert_eq!(again, first);
    }

    #[test]
    fn save_overwrites_archive_dir() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        load_or_init(&config_dir, &dir.path().join("a")).unwrap();
        save(
            &config_dir,
            &AppConfig::with_archive_dir(dir.path().join("b").to_string_lossy()),
        )
        .unwrap();
        let loaded = load_or_init(&config_dir, &dir.path().join("a")).unwrap();
        assert!(loaded.archive_dir.ends_with("b"));
    }

    #[test]
    fn old_config_json_defaults_new_fields() {
        let parsed: AppConfig = serde_json::from_str(r#"{"archiveDir":"/tmp/a"}"#).unwrap();
        assert_eq!(parsed.archive_dir, "/tmp/a");
        assert_eq!(parsed.ui_mode, UiMode::Easy);
        assert!(parsed.temp_project);
        assert_eq!(parsed.project_dir, None);
    }

    #[test]
    fn save_writes_ui_mode_and_temp_project() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let config = AppConfig {
            archive_dir: "/tmp/archive".into(),
            ui_mode: UiMode::Expert,
            temp_project: false,
            project_dir: Some("/tmp/project".into()),
        };
        save(&config_dir, &config).unwrap();
        let raw = fs::read_to_string(config_dir.join("config.json")).unwrap();
        assert!(raw.contains("\"uiMode\": \"expert\""));
        assert!(raw.contains("\"tempProject\": false"));
        assert!(raw.contains("\"projectDir\": \"/tmp/project\""));
        let loaded = load_or_init(&config_dir, Path::new("/tmp/other")).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn default_path_nests_under_documents() {
        let path = default_archive_dir(Path::new("/Users/frank/Documents"));
        assert_eq!(
            path,
            Path::new("/Users/frank/Documents/Simple 3DGS/archive")
        );
    }

    #[cfg(unix)]
    #[test]
    fn resolve_archive_falls_back_when_preferred_is_not_writable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        let mut perms = fs::metadata(&locked).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&locked, perms).unwrap();

        let preferred = locked.join("archive");
        let fallback = dir.path().join("fallback");
        let resolved = resolve_archive_dir(&preferred, &fallback).unwrap();
        assert_eq!(resolved, fallback);
        assert!(fallback.is_dir());

        let mut perms = fs::metadata(&locked).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&locked, perms).unwrap();
    }

    #[test]
    fn resolve_archive_keeps_writable_preferred() {
        let dir = tempdir().unwrap();
        let preferred = dir.path().join("documents").join("archive");
        let fallback = dir.path().join("fallback");
        let resolved = resolve_archive_dir(&preferred, &fallback).unwrap();
        assert_eq!(resolved, preferred);
        assert!(preferred.is_dir());
        assert!(!fallback.exists());
    }
}
