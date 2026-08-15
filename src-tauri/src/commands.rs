//! Tauri IPC: pipeline, archive library, and app config.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_config::{self, AppConfig};
use crate::archive::{ArchiveEntry, ArchiveLibrary};
use crate::error::PipelineError;
use crate::html_export;
use crate::pipeline::{InputKind, PipelineConfig, PipelineEvents};
use crate::project::Stage;
use crate::settings::PipelineSettings;
use crate::sidecar::{CancelFlag, ProcessRunner};
use crate::train_log::TrainSnapshot;

pub struct AppState {
    pub cancel: CancelFlag,
    pub running: Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cancel: CancelFlag::new(),
            running: Mutex::new(false),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPipelineRequest {
    pub source_path: String,
    pub source_kind: String,
    #[serde(default)]
    pub project_dir: Option<String>,
    #[serde(default)]
    pub archive_dir: Option<String>,
    #[serde(default = "default_true")]
    pub temp_project: bool,
    #[serde(default)]
    pub settings: PipelineSettings,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub stage: String,
    pub percent: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResult {
    pub ply_path: String,
    pub archive_id: Option<String>,
    pub archive_error: Option<String>,
}

struct TauriEvents {
    app: AppHandle,
}

impl PipelineEvents for TauriEvents {
    fn progress(&mut self, stage: Stage, percent: u8, message: &str) {
        let _ = self.app.emit(
            "pipeline-progress",
            ProgressPayload {
                stage: stage.as_str().into(),
                percent,
                message: message.into(),
            },
        );
    }

    fn log(&mut self, line: &str) {
        let _ = self.app.emit("pipeline-log", line.to_string());
    }

    fn preview(&mut self, path: &Path) {
        let _ = self
            .app
            .emit("pipeline-preview", path.to_string_lossy().into_owned());
    }

    fn train_stats(&mut self, stats: &TrainSnapshot) {
        let _ = self.app.emit("pipeline-train-stats", stats);
    }
}

#[tauri::command]
pub async fn start_pipeline(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartPipelineRequest,
) -> Result<RunResult, String> {
    {
        let mut running = state.running.lock().map_err(|err| err.to_string())?;
        if *running {
            return Err("A reconstruction is already running.".into());
        }
        *running = true;
    }
    state.cancel.reset();

    let kind = match request.source_kind.as_str() {
        "images" => InputKind::Images,
        _ => InputKind::Video,
    };
    let archive_dir = match request.archive_dir.filter(|s| !s.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(load_config(&app)?.archive_dir),
    };
    let config = PipelineConfig {
        project_dir: request
            .project_dir
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
        archive_dir,
        temp_project: request.temp_project,
        source: request.source_path.into(),
        kind,
        settings: request.settings.sanitized(),
        force: request.force,
    };
    let cancel = state.cancel.clone();
    let worker_app = app.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut runner = ProcessRunner::new(cancel.clone());
        let mut events = TauriEvents { app: worker_app };
        crate::pipeline::run_pipeline(&config, &mut runner, &cancel, &mut events)
    })
    .await
    .map_err(|err| err.to_string())?;

    if let Ok(mut running) = state.running.lock() {
        *running = false;
    }

    match result {
        Ok(outcome) => {
            let payload = RunResult {
                ply_path: outcome.ply.to_string_lossy().into_owned(),
                archive_id: outcome.archive_id,
                archive_error: outcome.archive_error,
            };
            let _ = app.emit("pipeline-complete", payload.ply_path.clone());
            Ok(payload)
        }
        Err(PipelineError::Cancelled) => {
            let _ = app.emit("pipeline-error", "Cancelled");
            Err("Cancelled".into())
        }
        Err(err) => {
            let message = err.to_string();
            let _ = app.emit("pipeline-error", message.clone());
            Err(message)
        }
    }
}

#[tauri::command]
pub fn cancel_pipeline(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel.cancel();
    Ok(())
}

#[tauri::command]
pub fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    load_config(&app)
}

#[tauri::command]
pub fn set_archive_dir(app: AppHandle, path: String) -> Result<AppConfig, String> {
    if path.trim().is_empty() {
        return Err("Choose an archive folder.".into());
    }
    let archive = PathBuf::from(&path);
    std::fs::create_dir_all(&archive).map_err(|err| err.to_string())?;
    let config = AppConfig {
        archive_dir: archive.to_string_lossy().into_owned(),
    };
    app_config::save(&config_dir(&app)?, &config).map_err(|err| err.to_string())?;
    ArchiveLibrary::open(&archive).map_err(|err| err.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn list_archive(app: AppHandle) -> Result<Vec<ArchiveEntry>, String> {
    library(&app)?.list().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn import_3dgs(app: AppHandle, path: String) -> Result<ArchiveEntry, String> {
    library(&app)?
        .import_3dgs(Path::new(&path))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn export_3dgs(app: AppHandle, id: String, dest_path: String) -> Result<(), String> {
    library(&app)?
        .export_3dgs(&id, Path::new(&dest_path))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn export_html(app: AppHandle, id: String, dest_dir: String) -> Result<(), String> {
    let lib = library(&app)?;
    let entry = lib.get(&id).map_err(|err| err.to_string())?;
    html_export::export_html(&entry, Path::new(&dest_dir)).map_err(|err| err.to_string())
}

fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let documents = app.path().document_dir().map_err(|err| err.to_string())?;
    let default_archive = app_config::default_archive_dir(&documents);
    app_config::load_or_init(&config_dir(app)?, &default_archive).map_err(|err| err.to_string())
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|err| err.to_string())
}

fn library(app: &AppHandle) -> Result<ArchiveLibrary, String> {
    let config = load_config(app)?;
    ArchiveLibrary::open(&config.archive_dir).map_err(|err| err.to_string())
}
