//! Cheap deterministic stage fingerprints and a read-only reuse plan.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::error::PipelineError;
use crate::manifest::{ProjectManifest, StageFingerprints};
use crate::project::{count_frames, ProjectLayout, Stage, MIN_FRAMES};
use crate::settings::PipelineSettings;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StagePlan {
    pub stage: String,
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PipelinePlan {
    pub stages: Vec<StagePlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidate_from: Option<String>,
    #[serde(skip)]
    pub expected: StageFingerprints,
}

pub fn fresh(expected: StageFingerprints) -> PipelinePlan {
    PipelinePlan {
        stages: [Stage::Frames, Stage::Colmap, Stage::Train]
            .into_iter()
            .map(|stage| StagePlan {
                stage: stage.as_str().into(),
                action: "run".into(),
                reason: "No matching project output".into(),
            })
            .collect(),
        invalidate_from: None,
        expected,
    }
}

pub fn fingerprints(
    source: &Path,
    source_kind: &str,
    settings: PipelineSettings,
) -> Result<StageFingerprints, PipelineError> {
    let settings = settings.sanitized();
    let source_stamp = source_stamp(source, source_kind)?;
    let source = digest_json(&source_stamp)?;
    let frames = digest_json(&serde_json::json!({
        "source": source,
        "fps": settings.fps,
        "maxImageSize": settings.max_image_size,
        "startSeconds": settings.start_seconds,
        "durationSeconds": settings.duration_seconds,
        "frameFormat": settings.frame_format,
        "jpegQuality": settings.jpeg_quality,
        "maxFrames": settings.max_frames,
        "extractMode": settings.extract_mode,
        "extractQuality": settings.extract_quality,
        "extract": settings.extract_knobs(),
    }))?;
    let colmap = digest_json(&serde_json::json!({
        "frames": frames,
        "captureMode": settings.capture_mode,
        "matchOverlap": settings.match_overlap,
        "colmap": settings.colmap_knobs(),
    }))?;
    let train = digest_json(&serde_json::json!({
        "colmap": colmap,
        "trainSteps": settings.train_steps,
        "maxSplats": settings.max_splats,
        "brush": settings.brush_knobs(),
    }))?;
    Ok(StageFingerprints {
        version: 1,
        source: Some(source),
        frames: Some(frames),
        colmap: Some(colmap),
        train: Some(train),
    })
}

pub fn build(
    layout: &ProjectLayout,
    manifest: Option<&ProjectManifest>,
    source: &Path,
    source_kind: &str,
    settings: PipelineSettings,
) -> Result<PipelinePlan, PipelineError> {
    let expected = fingerprints(source, source_kind, settings)?;
    let legacy_expected = manifest.and_then(|saved| {
        (same_path(Path::new(&saved.source_path), source) && saved.source_kind == source_kind)
            .then(|| fingerprints(source, source_kind, saved.settings).ok())
            .flatten()
    });
    let saved = manifest.and_then(|m| m.fingerprints.as_ref());
    let valid = [
        layout.is_complete(Stage::Frames)
            && count_frames(&layout.frames_dir()).unwrap_or(0) >= MIN_FRAMES,
        layout.is_complete(Stage::Colmap) && layout.sparse_model_dir().join("images.bin").is_file(),
        layout.is_complete(Stage::Train) && layout.output_ply().is_file(),
    ];
    let stages = [Stage::Frames, Stage::Colmap, Stage::Train];
    let expected_values = [
        expected.frames.as_ref(),
        expected.colmap.as_ref(),
        expected.train.as_ref(),
    ];
    let saved_values = [
        saved.and_then(|v| v.frames.as_ref()),
        saved.and_then(|v| v.colmap.as_ref()),
        saved.and_then(|v| v.train.as_ref()),
    ];
    let legacy_values = [
        legacy_expected.as_ref().and_then(|v| v.frames.as_ref()),
        legacy_expected.as_ref().and_then(|v| v.colmap.as_ref()),
        legacy_expected.as_ref().and_then(|v| v.train.as_ref()),
    ];
    let mut invalid_from: Option<usize> = None;
    let mut plans = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        let fingerprint_matches = saved_values[index] == expected_values[index]
            || (saved_values[index].is_none() && legacy_values[index] == expected_values[index]);
        if invalid_from.is_none() {
            let stale_fingerprint = valid[index] && !fingerprint_matches;
            let broken_chain = !valid[index]
                && stages[index..]
                    .iter()
                    .copied()
                    .any(|candidate| layout.is_complete(candidate));
            if stale_fingerprint || broken_chain {
                invalid_from = Some(index);
            }
        }
        let (action, reason) = if invalid_from.is_some_and(|start| index == start) {
            ("invalidate", "Source or relevant settings changed")
        } else if invalid_from.is_some_and(|start| index > start) {
            ("run", "Depends on an invalidated earlier stage")
        } else if valid[index] && fingerprint_matches {
            ("reuse", "Existing output matches this run")
        } else {
            ("run", "No complete matching output")
        };
        plans.push(StagePlan {
            stage: stage.as_str().into(),
            action: action.into(),
            reason: reason.into(),
        });
    }
    Ok(PipelinePlan {
        stages: plans,
        invalidate_from: invalid_from.map(|index| stages[index].as_str().into()),
        expected,
    })
}

fn source_stamp(path: &Path, kind: &str) -> Result<serde_json::Value, PipelineError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if kind == "images" {
        let mut files = Vec::new();
        for entry in fs::read_dir(path).map_err(|err| PipelineError::from_io_path(err, path))? {
            let candidate = entry?.path();
            if candidate.is_file() && crate::project::is_image(&candidate) {
                files.push(file_stamp(&candidate));
            }
        }
        files.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(serde_json::json!({ "kind": kind, "path": canonical, "files": files }))
    } else {
        Ok(serde_json::json!({ "kind": kind, "path": canonical, "file": file_stamp(path) }))
    }
}

fn file_stamp(path: &Path) -> (String, u64, u128) {
    let meta = fs::metadata(path).ok();
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        meta.as_ref().map(|m| m.len()).unwrap_or(0),
        modified,
    )
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.canonicalize().ok() == right.canonicalize().ok() || left == right
}

fn digest_json(value: &impl Serialize) -> Result<String, PipelineError> {
    let bytes = serde_json::to_vec(value).map_err(|err| PipelineError::message(err.to_string()))?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("v1-{hash:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;
    use tempfile::tempdir;

    fn completed_project(
        root: &Path,
        source: &Path,
        settings: PipelineSettings,
        with_fingerprints: bool,
    ) -> (ProjectLayout, ProjectManifest) {
        let layout = ProjectLayout::new(root);
        layout.create().unwrap();
        for index in 0..MIN_FRAMES {
            fs::write(layout.frames_dir().join(format!("{index}.jpg")), b"x").unwrap();
        }
        fs::create_dir_all(layout.sparse_model_dir()).unwrap();
        fs::write(layout.sparse_model_dir().join("images.bin"), b"x").unwrap();
        fs::write(layout.output_ply(), b"ply").unwrap();
        for stage in [Stage::Frames, Stage::Colmap, Stage::Train] {
            layout.mark_complete(stage).unwrap();
        }
        let mut manifest =
            ProjectManifest::new("test", source.to_string_lossy(), "video", settings, false);
        if with_fingerprints {
            manifest.fingerprints = Some(fingerprints(source, "video", settings).unwrap());
        }
        (layout, manifest)
    }

    #[test]
    fn viewer_only_change_keeps_all_stage_fingerprints() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let base = PipelineSettings::from_preset(Preset::Fast).sanitized();
        let mut changed = base;
        let mut viewer = changed.viewer_knobs();
        viewer.move_speed += 1.0;
        changed.viewer = Some(viewer);
        assert_eq!(
            fingerprints(&source, "video", base).unwrap(),
            fingerprints(&source, "video", changed).unwrap()
        );
    }

    #[test]
    fn train_change_only_changes_train_fingerprint() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let base = PipelineSettings::from_preset(Preset::Fast);
        let mut changed = base;
        changed.train_steps += 1;
        let a = fingerprints(&source, "video", base).unwrap();
        let b = fingerprints(&source, "video", changed).unwrap();
        assert_eq!(a.frames, b.frames);
        assert_eq!(a.colmap, b.colmap);
        assert_ne!(a.train, b.train);
    }

    #[test]
    fn frame_setting_changes_every_dependent_fingerprint() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let base = PipelineSettings::from_preset(Preset::Fast);
        let mut changed = base;
        changed.fps += 0.25;
        let a = fingerprints(&source, "video", base).unwrap();
        let b = fingerprints(&source, "video", changed).unwrap();
        assert_ne!(a.frames, b.frames);
        assert_ne!(a.colmap, b.colmap);
        assert_ne!(a.train, b.train);
    }

    #[test]
    fn colmap_setting_keeps_frames_and_invalidates_following_stages() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let base = PipelineSettings::from_preset(Preset::Fast);
        let mut changed = base;
        changed.match_overlap += 1;
        let a = fingerprints(&source, "video", base).unwrap();
        let b = fingerprints(&source, "video", changed).unwrap();
        assert_eq!(a.frames, b.frames);
        assert_ne!(a.colmap, b.colmap);
        assert_ne!(a.train, b.train);
    }

    #[test]
    fn build_invalidates_only_train_for_train_change() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let base = PipelineSettings::from_preset(Preset::Fast);
        let (layout, manifest) =
            completed_project(&dir.path().join("project"), &source, base, true);
        let mut changed = base;
        changed.train_steps += 1;
        let plan = build(&layout, Some(&manifest), &source, "video", changed).unwrap();
        assert_eq!(plan.invalidate_from.as_deref(), Some("train"));
        assert_eq!(
            plan.stages
                .iter()
                .map(|stage| stage.action.as_str())
                .collect::<Vec<_>>(),
            ["reuse", "reuse", "invalidate"]
        );
    }

    #[test]
    fn legacy_matching_project_reuses_complete_artifacts() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let settings = PipelineSettings::from_preset(Preset::Fast);
        let (layout, manifest) =
            completed_project(&dir.path().join("project"), &source, settings, false);
        let plan = build(&layout, Some(&manifest), &source, "video", settings).unwrap();
        assert!(plan.invalidate_from.is_none());
        assert!(plan.stages.iter().all(|stage| stage.action == "reuse"));
    }

    #[test]
    fn missing_early_artifact_invalidates_later_markers() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"video").unwrap();
        let settings = PipelineSettings::from_preset(Preset::Fast);
        let (layout, manifest) =
            completed_project(&dir.path().join("project"), &source, settings, true);
        fs::remove_file(layout.frames_dir().join("0.jpg")).unwrap();
        let plan = build(&layout, Some(&manifest), &source, "video", settings).unwrap();
        assert_eq!(plan.invalidate_from.as_deref(), Some("frames"));
        assert_eq!(
            plan.stages
                .iter()
                .map(|stage| stage.action.as_str())
                .collect::<Vec<_>>(),
            ["invalidate", "run", "run"]
        );
    }

    #[test]
    fn source_metadata_changes_frames_and_dependents() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("clip.mov");
        fs::write(&source, b"a").unwrap();
        let settings = PipelineSettings::from_preset(Preset::Fast);
        let before = fingerprints(&source, "video", settings).unwrap();
        fs::write(&source, b"longer").unwrap();
        let after = fingerprints(&source, "video", settings).unwrap();
        assert_ne!(before, after);
    }
}
