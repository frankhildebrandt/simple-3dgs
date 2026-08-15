//! Tauri IPC: pipeline, archive library, and app config.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::app_config::{self, AppConfig};
use crate::archive::{ArchiveEntry, ArchiveLibrary};
use crate::colmap_log::CameraSnapshot;
use crate::colmap_pose::{self, SparsePreview};
use crate::error::PipelineError;
use crate::frame_log::FrameSnapshot;
use crate::html_export;
use crate::manifest::{self, ProjectManifest};
use crate::pipeline::{InputKind, PipelineConfig, PipelineEvents};
use crate::project::{self, ProjectLayout, Stage};
use crate::settings::PipelineSettings;
use crate::sidecar::{CancelFlag, ProcessRunner};
use crate::train_log::TrainSnapshot;
use base64::Engine;
use tauri::ipc::{InvokeBody, Request, Response};

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

fn default_until() -> String {
    "train".into()
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
    #[serde(default = "default_until")]
    pub until: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    #[serde(default)]
    pub title: Option<String>,
    pub source_path: String,
    pub source_kind: String,
    #[serde(default)]
    pub settings: PipelineSettings,
    #[serde(default)]
    pub temp: bool,
    #[serde(default)]
    pub projects_dir: Option<String>,
    #[serde(default)]
    pub project_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    #[serde(flatten)]
    pub manifest: ProjectManifest,
    pub dir: String,
    pub frame_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ply_path: Option<String>,
    pub has_frames: bool,
    pub has_cameras: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFrame {
    pub name: String,
    pub path: String,
    pub index: usize,
    pub sharpness: f32,
    pub motion: f32,
    pub selected: bool,
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
    pub completed_stage: String,
    pub project_dir: String,
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

    fn frame_stats(&mut self, stats: &FrameSnapshot) {
        let _ = self.app.emit("pipeline-frame-stats", stats);
    }

    fn camera_stats(&mut self, stats: &CameraSnapshot) {
        let _ = self.app.emit("pipeline-camera-stats", stats);
    }

    fn frame_preview(&mut self, path: &Path) {
        let _ = self.app.emit(
            "pipeline-frame-preview",
            path.to_string_lossy().into_owned(),
        );
    }

    fn sparse_preview(&mut self, preview: &SparsePreview) {
        let _ = self.app.emit("pipeline-sparse", preview);
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
        until: parse_until(&request.until),
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
                completed_stage: outcome.completed_stage.as_str().into(),
                project_dir: outcome.project_dir.to_string_lossy().into_owned(),
            };
            if outcome.completed_stage == Stage::Train && outcome.ply.is_file() {
                let _ = app.emit("pipeline-complete", payload.ply_path.clone());
            } else {
                let _ = app.emit("pipeline-stage", payload.completed_stage.clone());
            }
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
pub fn save_config(app: AppHandle, mut config: AppConfig) -> Result<AppConfig, String> {
    if config.archive_dir.trim().is_empty() {
        return Err("Choose an archive folder.".into());
    }
    let archive = PathBuf::from(&config.archive_dir);
    std::fs::create_dir_all(&archive)
        .map_err(|err| PipelineError::from_io_path(err, &archive).to_string())?;
    if !config.temp_project {
        if config.projects_dir.as_deref().unwrap_or("").trim().is_empty()
            && config.project_dir.as_deref().unwrap_or("").trim().is_empty()
        {
            let documents = app.path().document_dir().map_err(|err| err.to_string())?;
            config.projects_dir = Some(
                manifest::default_projects_dir(&documents)
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        if let Some(dir) = config.projects_dir.as_deref() {
            let path = PathBuf::from(dir);
            std::fs::create_dir_all(&path)
                .map_err(|err| PipelineError::from_io_path(err, &path).to_string())?;
        }
    }
    app_config::save(&config_dir(&app)?, &config).map_err(|err| err.to_string())?;
    ArchiveLibrary::open(&archive).map_err(|err| err.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn list_archive(app: AppHandle) -> Result<Vec<ArchiveEntry>, String> {
    library(&app)?.list().map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_archive(app: AppHandle, id: String) -> Result<ArchiveEntry, String> {
    library(&app)?.get(&id).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn rename_archive(app: AppHandle, id: String, title: String) -> Result<ArchiveEntry, String> {
    let entry = library(&app)?
        .rename(&id, &title)
        .map_err(|err| err.to_string())?;
    emit_archive_changed(&app, &entry.meta.id);
    Ok(entry)
}

#[tauri::command]
pub fn delete_archive(app: AppHandle, id: String) -> Result<(), String> {
    library(&app)?.remove(&id).map_err(|err| err.to_string())?;
    emit_archive_changed(&app, &id);
    Ok(())
}

#[tauri::command]
pub fn set_archive_poster(
    app: AppHandle,
    id: String,
    jpeg_base64: String,
) -> Result<ArchiveEntry, String> {
    let jpeg = base64::engine::general_purpose::STANDARD
        .decode(jpeg_base64.trim())
        .map_err(|err| format!("Invalid preview image: {err}"))?;
    let entry = library(&app)?
        .set_poster(&id, &jpeg)
        .map_err(|err| err.to_string())?;
    emit_archive_changed(&app, &entry.meta.id);
    Ok(entry)
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

#[tauri::command]
pub fn spz_cache_fresh(app: AppHandle, id: String) -> Result<bool, String> {
    library(&app)?
        .spz_is_fresh(&id)
        .map_err(|err| err.to_string())
}

/// Receives SPZ bytes as a raw IPC body; archive id is in the `id` header.
#[tauri::command]
pub fn cache_archive_spz(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let id = request
        .headers()
        .get("id")
        .ok_or_else(|| "Missing archive id.".to_string())?
        .to_str()
        .map_err(|err| err.to_string())?
        .to_string();
    let bytes = match request.body() {
        InvokeBody::Raw(bytes) => bytes.as_slice(),
        InvokeBody::Json(_) => {
            return Err("SPZ cache expects a binary payload.".into());
        }
    };
    library(&app)?
        .write_spz(&id, bytes)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn export_spz(app: AppHandle, id: String, dest_path: String) -> Result<(), String> {
    library(&app)?
        .export_spz(&id, Path::new(&dest_path))
        .map_err(|err| err.to_string())
}

/// Reads a local PLY or SPZ as a raw IPC body so the viewer can skip `asset:` fetch.
#[tauri::command]
pub fn read_splat_file(path: String) -> Result<Response, String> {
    Ok(Response::new(read_splat_bytes(Path::new(&path))?))
}

/// Loads splat bytes from disk; empty, missing, and TCC-denied files fail with a stable message.
fn read_splat_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|err| {
        if err.kind() == io::ErrorKind::PermissionDenied {
            PipelineError::from_io_path(err, path).to_string()
        } else {
            format!("Cannot read splat: {err}")
        }
    })?;
    if bytes.is_empty() {
        return Err("Cannot read splat: file is empty.".into());
    }
    Ok(bytes)
}

#[tauri::command]
pub fn drop_archive_ply(app: AppHandle, id: String) -> Result<ArchiveEntry, String> {
    let entry = library(&app)?
        .drop_uncompressed_ply(&id)
        .map_err(|err| err.to_string())?;
    emit_archive_changed(&app, &entry.meta.id);
    Ok(entry)
}

fn parse_until(value: &str) -> Stage {
    match value {
        "frames" => Stage::Frames,
        "colmap" => Stage::Colmap,
        _ => Stage::Train,
    }
}

#[tauri::command]
pub fn create_project(app: AppHandle, request: CreateProjectRequest) -> Result<ProjectEntry, String> {
    let source = PathBuf::from(&request.source_path);
    let title = request
        .title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| manifest::title_from_source(&source));
    let mut manifest = ProjectManifest::new(
        title,
        request.source_path,
        request.source_kind,
        request.settings,
        request.temp,
    );
    let root = if let Some(dir) = request.project_dir.filter(|s| !s.trim().is_empty()) {
        PathBuf::from(dir)
    } else if request.temp {
        let archive = PathBuf::from(load_config(&app)?.archive_dir);
        ArchiveLibrary::open(&archive)
            .map_err(|err| err.to_string())?
            .scratch_dir(&source)
    } else {
        let projects = resolve_projects_dir(&app, request.projects_dir.as_deref())?;
        projects.join(&manifest.id)
    };
    fs::create_dir_all(&root).map_err(|err| PipelineError::from_io_path(err, &root).to_string())?;
    let layout = ProjectLayout::new(&root);
    layout.create().map_err(|err| err.to_string())?;
    if request.temp {
        manifest.id = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(manifest.id);
    }
    manifest
        .save(&manifest::project_file(&root))
        .map_err(|err| err.to_string())?;
    project_entry(&root)
}

#[tauri::command]
pub fn open_project(path: String) -> Result<ProjectEntry, String> {
    let root = PathBuf::from(path);
    if !manifest::project_file(&root).is_file() {
        return Err("That folder is not a Simple 3DGS project.".into());
    }
    project_entry(&root)
}

#[tauri::command]
pub fn list_projects(app: AppHandle, projects_dir: Option<String>) -> Result<Vec<ProjectEntry>, String> {
    let root = resolve_projects_dir(&app, projects_dir.as_deref())?;
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    let read = fs::read_dir(&root).map_err(|err| PipelineError::from_io_path(err, &root).to_string())?;
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() && manifest::project_file(&path).is_file() {
            if let Ok(project) = project_entry(&path) {
                entries.push(project);
            }
        }
    }
    entries.sort_by(|a, b| b.manifest.updated_at.cmp(&a.manifest.updated_at));
    Ok(entries)
}

#[tauri::command]
pub fn list_project_frames(project_dir: String) -> Result<Vec<ProjectFrame>, String> {
    let root = PathBuf::from(project_dir);
    let layout = ProjectLayout::new(&root);
    let manifest = manifest::FrameManifest::load(&manifest::frames_file(&root))
        .map_err(|err| err.to_string())?;
    let frames = if manifest.frames.is_empty() {
        crate::keyframes::list_stills(&layout.frames_dir())
            .map_err(|err| err.to_string())?
            .into_iter()
            .enumerate()
            .map(|(index, path)| ProjectFrame {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: path.to_string_lossy().into_owned(),
                index,
                sharpness: 0.0,
                motion: 0.0,
                selected: true,
            })
            .collect()
    } else {
        manifest
            .frames
            .into_iter()
            .map(|frame| ProjectFrame {
                path: layout.frames_dir().join(&frame.name).to_string_lossy().into_owned(),
                name: frame.name,
                index: frame.index,
                sharpness: frame.sharpness,
                motion: frame.motion,
                selected: frame.selected,
            })
            .collect()
    };
    Ok(frames)
}

#[tauri::command]
pub fn get_sparse_preview(project_dir: String) -> Result<SparsePreview, String> {
    let layout = ProjectLayout::new(PathBuf::from(project_dir));
    Ok(colmap_pose::sparse_preview(&layout.sparse_model_dir()).unwrap_or_default())
}

fn resolve_projects_dir(app: &AppHandle, override_dir: Option<&str>) -> Result<PathBuf, String> {
    if let Some(dir) = override_dir.filter(|s| !s.trim().is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let config = load_config(app)?;
    if let Some(dir) = config.projects_dir.filter(|s| !s.trim().is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let documents = app.path().document_dir().map_err(|err| err.to_string())?;
    Ok(manifest::default_projects_dir(&documents))
}

fn project_entry(root: &Path) -> Result<ProjectEntry, String> {
    let mut manifest = ProjectManifest::load(&manifest::project_file(root)).map_err(|err| err.to_string())?;
    let layout = ProjectLayout::new(root);
    if layout.is_complete(Stage::Train) {
        manifest.stage = "train".into();
    } else if layout.is_complete(Stage::Colmap) {
        manifest.stage = "colmap".into();
    } else if layout.is_complete(Stage::Frames) {
        manifest.stage = "frames".into();
    }
    let ply = layout.output_ply();
    Ok(ProjectEntry {
        manifest,
        dir: root.to_string_lossy().into_owned(),
        frame_count: project::count_frames(&layout.frames_dir()).unwrap_or(0) as u32,
        ply_path: ply.is_file().then(|| ply.to_string_lossy().into_owned()),
        has_frames: layout.is_complete(Stage::Frames),
        has_cameras: layout.is_complete(Stage::Colmap),
    })
}

/// Loads config.json and creates the archive folder. Call from the macOS main thread (app setup) so TCC can prompt.
pub(crate) fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let dir = config_dir(app)?;
    let documents = app.path().document_dir().map_err(|err| err.to_string())?;
    let default_archive = app_config::default_archive_dir(&documents);
    let mut config =
        app_config::load_or_init(&dir, &default_archive).map_err(|err| err.to_string())?;
    let fallback = app
        .path()
        .app_data_dir()
        .map_err(|err| err.to_string())?
        .join("archive");
    // Touch the archive on the main thread so macOS can show a TCC prompt.
    let resolved = app_config::resolve_archive_dir(Path::new(&config.archive_dir), &fallback)
        .map_err(|err| err.to_string())?;
    let resolved_str = resolved.to_string_lossy().into_owned();
    if config.archive_dir != resolved_str {
        config.archive_dir = resolved_str;
        app_config::save(&dir, &config).map_err(|err| err.to_string())?;
    }
    if config
        .projects_dir
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        let projects = manifest::default_projects_dir(&documents);
        let _ = fs::create_dir_all(&projects);
        config.projects_dir = Some(projects.to_string_lossy().into_owned());
        app_config::save(&dir, &config).map_err(|err| err.to_string())?;
    }
    Ok(config)
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|err| err.to_string())
}

fn library(app: &AppHandle) -> Result<ArchiveLibrary, String> {
    let config = load_config(app)?;
    ArchiveLibrary::open(&config.archive_dir).map_err(|err| err.to_string())
}

fn emit_archive_changed(app: &AppHandle, id: &str) {
    let _ = app.emit("archive-changed", id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_splat_bytes_rejects_missing_and_empty_files() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone.ply");
        assert!(read_splat_bytes(&missing)
            .unwrap_err()
            .starts_with("Cannot read splat:"));

        let empty = dir.path().join("empty.ply");
        fs::write(&empty, b"").unwrap();
        assert_eq!(
            read_splat_bytes(&empty).unwrap_err(),
            "Cannot read splat: file is empty."
        );

        let ply = dir.path().join("scene.ply");
        fs::write(&ply, b"ply\n").unwrap();
        assert_eq!(read_splat_bytes(&ply).unwrap(), b"ply\n");
    }

    #[cfg(unix)]
    #[test]
    fn read_splat_bytes_names_permission_denied_paths() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let ply = dir.path().join("locked.ply");
        fs::write(&ply, b"ply\n").unwrap();
        let mut perms = fs::metadata(&ply).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&ply, perms).unwrap();

        let err = read_splat_bytes(&ply).unwrap_err();
        let mut perms = fs::metadata(&ply).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&ply, perms).unwrap();

        assert!(err.contains("locked.ply"), "{err}");
        assert!(err.contains("Files and Folders"), "{err}");
    }
}
