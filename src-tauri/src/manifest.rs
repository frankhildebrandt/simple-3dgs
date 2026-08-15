//! On-disk manifests: project.json, frames.json, cameras.json.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::PipelineError;
use crate::project::Stage;
use crate::settings::PipelineSettings;

pub const PROJECT_FILE: &str = "project.json";
pub const FRAMES_FILE: &str = "frames.json";
pub const CAMERAS_FILE: &str = "cameras.json";
pub const IMAGE_LIST_FILE: &str = "image_list.txt";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub id: String,
    pub title: String,
    pub source_path: String,
    pub source_kind: String,
    pub settings: PipelineSettings,
    pub stage: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub temp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameEntry {
    pub name: String,
    pub index: usize,
    pub sharpness: f32,
    pub motion: f32,
    #[serde(default = "default_true")]
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FrameManifest {
    pub frames: Vec<FrameEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CameraEntry {
    pub name: String,
    pub position: [f64; 3],
    pub quaternion: [f64; 4],
    pub registered: bool,
    #[serde(default = "default_true")]
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CameraManifest {
    pub cameras: Vec<CameraEntry>,
}

fn default_true() -> bool {
    true
}

impl ProjectManifest {
    pub fn new(
        title: impl Into<String>,
        source_path: impl Into<String>,
        source_kind: impl Into<String>,
        settings: PipelineSettings,
        temp: bool,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            source_path: source_path.into(),
            source_kind: source_kind.into(),
            settings,
            stage: "idle".into(),
            created_at: now.clone(),
            updated_at: now,
            temp,
        }
    }

    pub fn load(path: &Path) -> Result<Self, PipelineError> {
        let bytes = fs::read(path).map_err(|err| PipelineError::from_io_path(err, path))?;
        serde_json::from_slice(&bytes)
            .map_err(|err| PipelineError::message(format!("Invalid project.json: {err}")))
    }

    pub fn save(&self, path: &Path) -> Result<(), PipelineError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|err| PipelineError::message(format!("project.json: {err}")))?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn touch_stage(&mut self, stage: Stage) {
        self.stage = stage.as_str().into();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

impl FrameManifest {
    pub fn load(path: &Path) -> Result<Self, PipelineError> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|err| PipelineError::from_io_path(err, path))?;
        serde_json::from_slice(&bytes)
            .map_err(|err| PipelineError::message(format!("Invalid frames.json: {err}")))
    }

    pub fn save(&self, path: &Path) -> Result<(), PipelineError> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|err| PipelineError::message(format!("frames.json: {err}")))?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn selected_names(&self) -> Vec<&str> {
        self.frames
            .iter()
            .filter(|frame| frame.selected)
            .map(|frame| frame.name.as_str())
            .collect()
    }
}

impl CameraManifest {
    pub fn load(path: &Path) -> Result<Self, PipelineError> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(|err| PipelineError::from_io_path(err, path))?;
        serde_json::from_slice(&bytes)
            .map_err(|err| PipelineError::message(format!("Invalid cameras.json: {err}")))
    }

    pub fn save(&self, path: &Path) -> Result<(), PipelineError> {
        let json = serde_json::to_vec_pretty(self)
            .map_err(|err| PipelineError::message(format!("cameras.json: {err}")))?;
        fs::write(path, json)?;
        Ok(())
    }
}

/// Writes one selected image name per line for COLMAP `--image_list_path`.
pub fn write_image_list(path: &Path, names: &[&str]) -> Result<(), PipelineError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = String::new();
    for name in names {
        body.push_str(name);
        body.push('\n');
    }
    fs::write(path, body)?;
    Ok(())
}

/// Title from a source path stem, falling back to "Untitled".
pub fn title_from_source(source: &Path) -> String {
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['_', '-'], " "))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Untitled".into())
}

/// Default folder for named projects, next to the archive.
pub fn default_projects_dir(documents: &Path) -> PathBuf {
    documents.join("Simple 3DGS").join("projects")
}

pub fn project_file(root: &Path) -> PathBuf {
    root.join(PROJECT_FILE)
}

pub fn frames_file(root: &Path) -> PathBuf {
    root.join(FRAMES_FILE)
}

pub fn cameras_file(root: &Path) -> PathBuf {
    root.join(CAMERAS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;
    use tempfile::tempdir;

    #[test]
    fn project_roundtrip() {
        let dir = tempdir().unwrap();
        let path = project_file(dir.path());
        let mut manifest = ProjectManifest::new(
            "Kitchen",
            "/tmp/clip.mov",
            "video",
            PipelineSettings::from_preset(Preset::Fast),
            false,
        );
        manifest.save(&path).unwrap();
        let loaded = ProjectManifest::load(&path).unwrap();
        assert_eq!(loaded.title, "Kitchen");
        assert_eq!(loaded.source_kind, "video");
        manifest.touch_stage(Stage::Frames);
        manifest.save(&path).unwrap();
        assert_eq!(ProjectManifest::load(&path).unwrap().stage, "frames");
    }

    #[test]
    fn frames_selected_names_and_image_list() {
        let dir = tempdir().unwrap();
        let manifest = FrameManifest {
            frames: vec![
                FrameEntry {
                    name: "frame_00001.jpg".into(),
                    index: 0,
                    sharpness: 12.0,
                    motion: 1.0,
                    selected: true,
                },
                FrameEntry {
                    name: "frame_00002.jpg".into(),
                    index: 4,
                    sharpness: 8.0,
                    motion: 3.0,
                    selected: false,
                },
                FrameEntry {
                    name: "frame_00003.jpg".into(),
                    index: 9,
                    sharpness: 20.0,
                    motion: 2.0,
                    selected: true,
                },
            ],
        };
        let path = frames_file(dir.path());
        manifest.save(&path).unwrap();
        let loaded = FrameManifest::load(&path).unwrap();
        assert_eq!(loaded.selected_names(), vec!["frame_00001.jpg", "frame_00003.jpg"]);
        let list = dir.path().join(IMAGE_LIST_FILE);
        write_image_list(&list, &loaded.selected_names()).unwrap();
        let text = fs::read_to_string(&list).unwrap();
        assert_eq!(text, "frame_00001.jpg\nframe_00003.jpg\n");
    }

    #[test]
    fn cameras_roundtrip_defaults_included() {
        let dir = tempdir().unwrap();
        let path = cameras_file(dir.path());
        let manifest = CameraManifest {
            cameras: vec![CameraEntry {
                name: "frame_00001.jpg".into(),
                position: [1.0, 2.0, 3.0],
                quaternion: [0.0, 0.0, 0.0, 1.0],
                registered: true,
                included: true,
            }],
        };
        manifest.save(&path).unwrap();
        let loaded = CameraManifest::load(&path).unwrap();
        assert!(loaded.cameras[0].included);
        assert_eq!(loaded.cameras[0].position[1], 2.0);
    }

    #[test]
    fn title_from_video_stem() {
        assert_eq!(title_from_source(Path::new("/tmp/Kitchen_orbit.mov")), "Kitchen orbit");
        assert_eq!(title_from_source(Path::new("/")), "Untitled");
    }
}
