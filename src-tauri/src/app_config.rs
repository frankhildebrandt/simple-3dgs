//! Persisted app settings: the user-chosen archive directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;

const CONFIG_FILE: &str = "config.json";

/// On-disk config. `archive_dir` is required once the user (or default) has picked it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub archive_dir: String,
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
    let config = AppConfig {
        archive_dir: default_archive.to_string_lossy().into_owned(),
    };
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
        assert!(config_dir.join("config.json").is_file());
        let again = load_or_init(&config_dir, &dir.path().join("other")).unwrap();
        assert_eq!(again.archive_dir, first.archive_dir);
    }

    #[test]
    fn save_overwrites_archive_dir() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        load_or_init(&config_dir, &dir.path().join("a")).unwrap();
        save(
            &config_dir,
            &AppConfig {
                archive_dir: dir.path().join("b").to_string_lossy().into_owned(),
            },
        )
        .unwrap();
        let loaded = load_or_init(&config_dir, &dir.path().join("a")).unwrap();
        assert!(loaded.archive_dir.ends_with("b"));
    }

    #[test]
    fn default_path_nests_under_documents() {
        let path = default_archive_dir(Path::new("/Users/frank/Documents"));
        assert_eq!(
            path,
            Path::new("/Users/frank/Documents/Simple 3DGS/archive")
        );
    }
}
