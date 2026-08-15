//! Project folder layout, stage markers, and resume helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::PipelineError;

pub const MIN_FRAMES: usize = 8;
pub const OUTPUT_PLY: &str = "scene.ply";
pub const VIEW_JSON: &str = "view.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Frames,
    Colmap,
    Train,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frames => "frames",
            Self::Colmap => "colmap",
            Self::Train => "train",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectLayout {
    root: PathBuf,
}

impl ProjectLayout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn input_dir(&self) -> PathBuf {
        self.root.join("input")
    }

    pub fn frames_dir(&self) -> PathBuf {
        self.root.join("frames")
    }

    pub fn colmap_dir(&self) -> PathBuf {
        self.root.join("colmap")
    }

    pub fn database_path(&self) -> PathBuf {
        self.colmap_dir().join("database.db")
    }

    /// Room global SfM copy of `database.db`; calibrator writes in-place here.
    pub fn database_global_path(&self) -> PathBuf {
        self.colmap_dir().join("database_global.db")
    }

    pub fn sparse_dir(&self) -> PathBuf {
        self.colmap_dir().join("sparse")
    }

    pub fn sparse_model_dir(&self) -> PathBuf {
        self.sparse_dir().join("0")
    }

    pub fn dataset_dir(&self) -> PathBuf {
        self.root.join("dataset")
    }

    pub fn dataset_images_dir(&self) -> PathBuf {
        self.dataset_dir().join("images")
    }

    pub fn dataset_sparse_dir(&self) -> PathBuf {
        self.dataset_dir().join("sparse").join("0")
    }

    pub fn train_dir(&self) -> PathBuf {
        self.root.join("train")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join("output")
    }

    pub fn output_ply(&self) -> PathBuf {
        self.output_dir().join(OUTPUT_PLY)
    }

    fn marker_path(&self, stage: Stage) -> PathBuf {
        self.root
            .join("markers")
            .join(format!("{}.done", stage.as_str()))
    }

    fn archive_marker(&self) -> PathBuf {
        self.root.join("markers").join("archive.done")
    }

    /// Archive entry id written after a successful ingest, if any.
    pub fn archived_id(&self) -> Option<String> {
        fs::read_to_string(self.archive_marker())
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn mark_archived(&self, id: &str) -> Result<(), PipelineError> {
        fs::create_dir_all(self.root.join("markers"))?;
        fs::write(self.archive_marker(), format!("{id}\n"))?;
        Ok(())
    }

    /// Creates the empty project tree. Existing files are left in place.
    pub fn create(&self) -> Result<(), PipelineError> {
        for dir in [
            self.input_dir(),
            self.frames_dir(),
            self.colmap_dir(),
            self.sparse_dir(),
            self.dataset_dir(),
            self.train_dir(),
            self.output_dir(),
            self.root.join("markers"),
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    pub fn is_complete(&self, stage: Stage) -> bool {
        self.marker_path(stage).is_file()
    }

    pub fn mark_complete(&self, stage: Stage) -> Result<(), PipelineError> {
        fs::create_dir_all(self.root.join("markers"))?;
        fs::write(self.marker_path(stage), b"ok\n")?;
        Ok(())
    }

    pub fn clear_from(&self, stage: Stage) -> Result<(), PipelineError> {
        let stages = [Stage::Frames, Stage::Colmap, Stage::Train];
        let start = stages.iter().position(|s| *s == stage).unwrap_or(0);
        for later in &stages[start..] {
            let marker = self.marker_path(*later);
            if marker.exists() {
                fs::remove_file(marker)?;
            }
        }
        let archived = self.archive_marker();
        if archived.exists() {
            fs::remove_file(archived)?;
        }
        match stage {
            Stage::Frames => {
                remove_dir_contents(&self.frames_dir())?;
                remove_dir_contents(&self.dataset_dir())?;
                self.remove_databases()?;
                remove_dir_contents(&self.sparse_dir())?;
                let ply = self.output_ply();
                if ply.exists() {
                    fs::remove_file(ply)?;
                }
            }
            Stage::Colmap => {
                self.remove_databases()?;
                remove_dir_contents(&self.sparse_dir())?;
                remove_dir_contents(&self.dataset_dir())?;
                let ply = self.output_ply();
                if ply.exists() {
                    fs::remove_file(ply)?;
                }
            }
            Stage::Train => {
                let ply = self.output_ply();
                if ply.exists() {
                    fs::remove_file(ply)?;
                }
            }
        }
        Ok(())
    }

    /// Deletes both the matching database and the Room global-SfM copy.
    fn remove_databases(&self) -> Result<(), PipelineError> {
        for db in [self.database_path(), self.database_global_path()] {
            if db.exists() {
                fs::remove_file(db)?;
            }
        }
        Ok(())
    }
}

/// Counts JPEG/PNG stills used as COLMAP input.
pub fn count_frames(dir: &Path) -> Result<usize, PipelineError> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() && is_image(&path) {
            count += 1;
        }
    }
    Ok(count)
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
        Some(ext) if ext == "jpg" || ext == "jpeg" || ext == "png"
    )
}

/// First JPEG/PNG in `dir` by name, used as an archive poster.
pub fn first_frame(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let mut paths: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_image(path))
        .collect();
    paths.sort();
    paths.into_iter().next()
}

/// Copies or hard-links stills into `dataset/images` and the sparse model into `dataset/sparse/0`.
pub fn assemble_dataset(layout: &ProjectLayout) -> Result<(), PipelineError> {
    let images = layout.dataset_images_dir();
    let sparse = layout.dataset_sparse_dir();
    fs::create_dir_all(&images)?;
    fs::create_dir_all(&sparse)?;
    remove_dir_contents(&images)?;
    remove_dir_contents(&sparse)?;

    for entry in fs::read_dir(layout.frames_dir())? {
        let src = entry?.path();
        if src.is_file() && is_image(&src) {
            let dest = images.join(
                src.file_name()
                    .ok_or_else(|| PipelineError::message("frame is missing a file name"))?,
            );
            link_or_copy(&src, &dest)?;
        }
    }

    let model = layout.sparse_model_dir();
    if !model.is_dir() {
        return Err(PipelineError::message(
            "COLMAP did not write sparse/0. Reconstruction failed.",
        ));
    }
    for name in ["cameras.bin", "images.bin", "points3D.bin"] {
        let src = model.join(name);
        if !src.is_file() {
            return Err(PipelineError::message(format!(
                "COLMAP sparse model is missing {name}."
            )));
        }
        fs::copy(&src, sparse.join(name))?;
    }
    crate::colmap_pose::write_output_view(layout);
    Ok(())
}

/// Newest `*.ply` in `dir` by mtime. `None` if the folder is empty or missing.
pub fn newest_ply(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let mut best: Option<(SystemTime, PathBuf)> = None;
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ply") {
            continue;
        }
        let meta = fs::metadata(&path).ok()?;
        if !meta.is_file() || meta.len() == 0 {
            continue;
        }
        let modified = meta.modified().ok()?;
        if best.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, path)| path)
}

/// Like [`newest_ply`], but skips files still being written.
pub fn newest_ready_ply(dir: &Path) -> Option<PathBuf> {
    let path = newest_ply(dir)?;
    let meta = fs::metadata(&path).ok()?;
    let age = meta.modified().ok()?.elapsed().ok()?;
    (age >= Duration::from_millis(250)).then_some(path)
}

/// Picks `scene.ply` or the newest `*.ply` in `export_dir` and copies it to the canonical output.
pub fn finalize_ply(export_dir: &Path, dest: &Path) -> Result<(), PipelineError> {
    if dest.is_file() {
        return Ok(());
    }
    let preferred = export_dir.join(OUTPUT_PLY);
    let src = if preferred.is_file() && preferred != dest {
        preferred
    } else {
        newest_ply(export_dir)
            .ok_or_else(|| PipelineError::message("Brush finished but no .ply was exported."))?
    };
    if src == dest {
        return Ok(());
    }
    fs::create_dir_all(dest.parent().unwrap_or(export_dir))?;
    fs::copy(src, dest)?;
    Ok(())
}

pub(crate) fn link_or_copy(src: &Path, dest: &Path) -> Result<(), PipelineError> {
    if dest.exists() {
        fs::remove_file(dest)?;
    }
    match fs::hard_link(src, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(src, dest)?;
            Ok(())
        }
    }
}

fn remove_dir_contents(dir: &Path) -> Result<(), PipelineError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_and_markers_roundtrip() {
        let dir = tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path());
        layout.create().unwrap();
        assert!(layout.frames_dir().is_dir());
        assert!(!layout.is_complete(Stage::Frames));
        layout.mark_complete(Stage::Frames).unwrap();
        assert!(layout.is_complete(Stage::Frames));
        layout.clear_from(Stage::Frames).unwrap();
        assert!(!layout.is_complete(Stage::Frames));
    }

    #[test]
    fn count_frames_ignores_non_images() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.jpg"), b"x").unwrap();
        fs::write(dir.path().join("b.PNG"), b"x").unwrap();
        fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        assert_eq!(count_frames(dir.path()).unwrap(), 2);
    }

    #[test]
    fn first_frame_is_sorted_image() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("b.jpg"), b"x").unwrap();
        fs::write(dir.path().join("a.png"), b"x").unwrap();
        fs::write(dir.path().join("z.txt"), b"x").unwrap();
        let first = first_frame(dir.path()).unwrap();
        assert_eq!(first.file_name().unwrap(), "a.png");
    }

    #[test]
    fn assemble_dataset_copies_sparse_bins() {
        let dir = tempdir().unwrap();
        let layout = ProjectLayout::new(dir.path());
        layout.create().unwrap();
        fs::write(layout.frames_dir().join("frame_00001.jpg"), b"img").unwrap();
        fs::create_dir_all(layout.sparse_model_dir()).unwrap();
        for name in ["cameras.bin", "images.bin", "points3D.bin"] {
            fs::write(layout.sparse_model_dir().join(name), name.as_bytes()).unwrap();
        }
        assemble_dataset(&layout).unwrap();
        assert!(layout
            .dataset_images_dir()
            .join("frame_00001.jpg")
            .is_file());
        assert!(layout.dataset_sparse_dir().join("cameras.bin").is_file());
    }

    #[test]
    fn finalize_ply_picks_newest_when_scene_missing() {
        let dir = tempdir().unwrap();
        let export = dir.path().join("export");
        fs::create_dir_all(&export).unwrap();
        fs::write(export.join("export_1000.ply"), b"old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(export.join("export_5000.ply"), b"new").unwrap();
        let dest = dir.path().join("output").join(OUTPUT_PLY);
        finalize_ply(&export, &dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new");
    }
}
