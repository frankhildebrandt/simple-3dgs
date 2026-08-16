//! End-to-end Video/Images → PLY orchestration with resume and cancel.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::archive::{ArchiveLibrary, IngestRequest};
use crate::brush;
use crate::colmap;
use crate::colmap_log::CameraSnapshot;
use crate::colmap_pose;
use crate::error::PipelineError;
use crate::ffmpeg;
use crate::frame_log::{FramePass, FrameSnapshot};
use crate::geo::{self, GeoFix};
use crate::keyframes::{self, KeyframeConfig};
use crate::manifest::{self, FrameEntry, FrameManifest, ProjectManifest};
use crate::project::{
    assemble_dataset, count_frames, finalize_ply, ProjectLayout, Stage, MIN_FRAMES,
};
use crate::colmap_knobs::SiftBackend;
use crate::settings::{ExtractMode, PipelineSettings};
use crate::sidecar::{self, CancelFlag, CommandSpec, SidecarRunner};
use crate::train_log::TrainSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Video,
    Images,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub project_dir: Option<PathBuf>,
    pub archive_dir: PathBuf,
    pub temp_project: bool,
    pub source: PathBuf,
    pub kind: InputKind,
    pub settings: PipelineSettings,
    pub force: bool,
    pub until: Stage,
}

/// PLY the viewer should load, plus archive ingest outcome.
#[derive(Debug, Clone)]
pub struct PipelineOutcome {
    pub ply: PathBuf,
    pub archive_id: Option<String>,
    pub archive_error: Option<String>,
    pub completed_stage: Stage,
    pub project_dir: PathBuf,
}

pub trait PipelineEvents {
    fn progress(&mut self, stage: Stage, percent: u8, message: &str);
    fn log(&mut self, line: &str);
    fn preview(&mut self, path: &Path);
    fn train_stats(&mut self, _stats: &TrainSnapshot) {}
    fn frame_stats(&mut self, _stats: &FrameSnapshot) {}
    fn camera_stats(&mut self, _stats: &CameraSnapshot) {}
    fn frame_preview(&mut self, _path: &Path) {}
    fn sparse_preview(&mut self, _preview: &colmap_pose::SparsePreview) {}
}

/// Runs extract → COLMAP → Brush, then archives the splat. Returns the PLY to display.
pub fn run_pipeline(
    config: &PipelineConfig,
    runner: &mut dyn SidecarRunner,
    cancel: &CancelFlag,
    events: &mut dyn PipelineEvents,
) -> Result<PipelineOutcome, PipelineError> {
    let library = ArchiveLibrary::open(&config.archive_dir)?;
    let project_dir = resolve_project_dir(config, &library)?;
    let layout = ProjectLayout::new(&project_dir);
    layout.create()?;
    upsert_manifest(config, &layout);

    if config.force {
        layout.clear_from(Stage::Frames)?;
    }

    run_frames(config, &layout, runner, cancel, events)?;
    persist_stage(&layout, Stage::Frames);
    if config.until == Stage::Frames {
        return Ok(stage_outcome(&layout, Stage::Frames));
    }

    run_colmap(&layout, config.settings, runner, cancel, events)?;
    persist_stage(&layout, Stage::Colmap);
    if config.until == Stage::Colmap {
        return Ok(stage_outcome(&layout, Stage::Colmap));
    }

    run_train(&layout, config.settings, runner, cancel, events)?;
    persist_stage(&layout, Stage::Train);

    events.progress(Stage::Train, 100, "Done");
    archive_outcome(config, &library, &layout, runner, events)
}

fn resolve_project_dir(
    config: &PipelineConfig,
    library: &ArchiveLibrary,
) -> Result<PathBuf, PipelineError> {
    if let Some(dir) = &config.project_dir {
        return Ok(dir.clone());
    }
    if config.temp_project {
        Ok(library.scratch_dir(&config.source))
    } else {
        Err(PipelineError::message(
            "Choose a project folder, or enable the temporary project folder.",
        ))
    }
}

fn persist_stage(layout: &ProjectLayout, stage: Stage) {
    let path = manifest::project_file(layout.root());
    if let Ok(mut manifest) = ProjectManifest::load(&path) {
        manifest.touch_stage(stage);
        let _ = manifest.save(&path);
    }
}

fn upsert_manifest(config: &PipelineConfig, layout: &ProjectLayout) {
    let path = manifest::project_file(layout.root());
    let source_kind = match config.kind {
        InputKind::Video => "video",
        InputKind::Images => "images",
    };
    let mut manifest = ProjectManifest::load(&path).unwrap_or_else(|_| {
        ProjectManifest::new(
            manifest::title_from_source(&config.source),
            config.source.to_string_lossy(),
            source_kind,
            config.settings,
            config.temp_project,
        )
    });
    manifest.source_path = config.source.to_string_lossy().into_owned();
    manifest.source_kind = source_kind.into();
    manifest.settings = config.settings;
    manifest.temp = config.temp_project;
    manifest.updated_at = chrono::Utc::now().to_rfc3339();
    let _ = manifest.save(&path);
}

fn stage_outcome(layout: &ProjectLayout, stage: Stage) -> PipelineOutcome {
    PipelineOutcome {
        ply: layout.output_ply(),
        archive_id: layout.archived_id(),
        archive_error: None,
        completed_stage: stage,
        project_dir: layout.root().to_path_buf(),
    }
}

fn archive_outcome(
    config: &PipelineConfig,
    library: &ArchiveLibrary,
    layout: &ProjectLayout,
    runner: &mut dyn SidecarRunner,
    events: &mut dyn PipelineEvents,
) -> Result<PipelineOutcome, PipelineError> {
    if let Some(id) = layout.archived_id() {
        if let Ok(entry) = library.get(&id) {
            return Ok(PipelineOutcome {
                ply: PathBuf::from(entry.ply_path),
                archive_id: Some(id),
                archive_error: None,
                completed_stage: Stage::Train,
                project_dir: layout.root().to_path_buf(),
            });
        }
    }

    let geo = probe_geo(config, runner, events);
    let frame_count = count_frames(&layout.frames_dir()).unwrap_or(0) as u32;
    let source_kind = match config.kind {
        InputKind::Video => "video",
        InputKind::Images => "images",
    };
    match library.ingest(IngestRequest {
        ply: &layout.output_ply(),
        frames_dir: Some(&layout.frames_dir()),
        source: &config.source,
        source_kind,
        settings: Some(config.settings),
        frame_count,
        geo,
        reuse_id: layout.archived_id(),
    }) {
        Ok(entry) => {
            layout.mark_archived(&entry.meta.id)?;
            if config.temp_project {
                let _ = fs::remove_dir_all(layout.root());
            }
            Ok(PipelineOutcome {
                ply: PathBuf::from(entry.ply_path),
                archive_id: Some(entry.meta.id),
                archive_error: None,
                completed_stage: Stage::Train,
                project_dir: layout.root().to_path_buf(),
            })
        }
        Err(err) => {
            let message = err.to_string();
            events.log(&format!("Archive failed: {message}"));
            Ok(PipelineOutcome {
                ply: layout.output_ply(),
                archive_id: None,
                archive_error: Some(message),
                completed_stage: Stage::Train,
                project_dir: layout.root().to_path_buf(),
            })
        }
    }
}

fn probe_geo(
    config: &PipelineConfig,
    runner: &mut dyn SidecarRunner,
    events: &mut dyn PipelineEvents,
) -> Option<GeoFix> {
    match config.kind {
        InputKind::Images => geo::geo_from_image_dir(&config.source),
        InputKind::Video => {
            if !config.source.is_file() {
                return None;
            }
            let spec = ffmpeg::ffmetadata_spec(&config.source);
            let mut dump = String::new();
            let result = runner.run(
                &spec,
                &mut |line| {
                    events.log(line);
                    dump.push_str(line);
                    dump.push('\n');
                },
                &mut |_| {},
            );
            if result.is_err() {
                events.log("Could not read video location tags");
                return None;
            }
            geo::geo_from_ffmetadata(&dump)
        }
    }
}

fn run_frames(
    config: &PipelineConfig,
    layout: &ProjectLayout,
    runner: &mut dyn SidecarRunner,
    cancel: &CancelFlag,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    cancel.check()?;
    if layout.is_complete(Stage::Frames) && count_frames(&layout.frames_dir())? >= MIN_FRAMES {
        events.progress(Stage::Frames, 100, "Frames already extracted");
        return Ok(());
    }

    events.progress(Stage::Frames, 5, "Preparing frames");
    match config.kind {
        InputKind::Video => {
            let video = copy_input_video(&config.source, &layout.input_dir())?;
            extract_video_keyframes(&video, layout, config.settings, runner, events)?;
        }
        InputKind::Images => {
            let mut snap = FrameSnapshot::new(FramePass::Import);
            events.frame_stats(&snap);
            events.progress(Stage::Frames, 20, "Importing stills");
            import_image_folder(&config.source, &layout.frames_dir())?;
            write_still_frames_manifest(layout)?;
            snap.current = Some(count_frames(&layout.frames_dir())? as u32);
            snap.total = snap.current;
            events.frame_stats(&snap);
        }
    }

    let n = count_frames(&layout.frames_dir())?;
    if n < MIN_FRAMES {
        return Err(PipelineError::message(format!(
            "Need at least {MIN_FRAMES} frames, found {n}. Use a longer clip or the Quality preset."
        )));
    }
    layout.mark_complete(Stage::Frames)?;
    events.progress(Stage::Frames, 100, &format!("Extracted {n} frames"));
    Ok(())
}

/// Dense thumbs → score → full-res stills at the selected indices.
fn extract_video_keyframes(
    video: &Path,
    layout: &ProjectLayout,
    settings: PipelineSettings,
    runner: &mut dyn SidecarRunner,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    let candidates = layout.candidates_dir();
    let _ = fs::remove_dir_all(&candidates);
    fs::create_dir_all(&candidates)?;
    let mut snap = FrameSnapshot::new(FramePass::Candidates);
    events.frame_stats(&snap);
    events.progress(Stage::Frames, 15, "Extracting candidate frames");
    run_ffmpeg_extract(
        runner,
        events,
        ffmpeg::candidate_spec(video, &candidates, settings).watching_images(&candidates),
        ffmpeg::candidate_spec_with_hwaccel(video, &candidates, settings, false)
            .watching_images(&candidates),
        &mut snap,
    )?;

    events.progress(Stage::Frames, 55, "Selecting keyframes");
    let scores = keyframes::score_candidates(&candidates)?;
    let config = KeyframeConfig::from_settings(settings);
    let picked = keyframes::select_keyframes(&scores, config);
    if picked.is_empty() {
        let _ = fs::remove_dir_all(&candidates);
        return Err(PipelineError::message(
            "Could not select keyframes from the video. Try a longer clip.",
        ));
    }

    let _ = fs::remove_dir_all(layout.frames_dir());
    fs::create_dir_all(layout.frames_dir())?;
    let extracting = if settings.sanitized().extract_mode == ExtractMode::Change
        && picked.len() >= config.max_keep
    {
        format!(
            "Extracting {} keyframes (capped at {})",
            picked.len(),
            config.max_keep
        )
    } else {
        format!("Extracting {} keyframes", picked.len())
    };
    snap = FrameSnapshot::new(FramePass::Keyframes);
    snap.kept = Some(picked.len() as u32);
    snap.total = Some(picked.len() as u32);
    events.frame_stats(&snap);
    events.progress(Stage::Frames, 70, &extracting);
    let result = run_ffmpeg_extract(
        runner,
        events,
        ffmpeg::select_spec(video, &layout.frames_dir(), settings, &picked)
            .watching_images(layout.frames_dir()),
        ffmpeg::select_spec_with_hwaccel(video, &layout.frames_dir(), settings, &picked, false)
            .watching_images(layout.frames_dir()),
        &mut snap,
    );
    write_video_frames_manifest(layout, settings, &scores, &picked)?;
    let _ = fs::remove_dir_all(&candidates);
    result
}

fn write_video_frames_manifest(
    layout: &ProjectLayout,
    settings: PipelineSettings,
    scores: &[keyframes::CandidateScore],
    picked: &[usize],
) -> Result<(), PipelineError> {
    let ext = settings.sanitized().frame_format.extension();
    let frames = picked
        .iter()
        .enumerate()
        .map(|(out, &index)| {
            let score = scores
                .get(index)
                .copied()
                .unwrap_or(keyframes::CandidateScore {
                    sharpness: 0.0,
                    motion: 0.0,
                });
            FrameEntry {
                name: format!("frame_{:05}.{ext}", out + 1),
                index,
                sharpness: score.sharpness,
                motion: score.motion,
                selected: true,
            }
        })
        .collect();
    FrameManifest { frames }.save(&manifest::frames_file(layout.root()))
}

fn write_still_frames_manifest(layout: &ProjectLayout) -> Result<(), PipelineError> {
    let paths = keyframes::list_stills(&layout.frames_dir())?;
    let scores = keyframes::score_candidates(&layout.frames_dir()).unwrap_or_default();
    let frames = paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let score = scores
                .get(index)
                .copied()
                .unwrap_or(keyframes::CandidateScore {
                    sharpness: 0.0,
                    motion: 0.0,
                });
            FrameEntry {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                index,
                sharpness: score.sharpness,
                motion: score.motion,
                selected: true,
            }
        })
        .collect();
    FrameManifest { frames }.save(&manifest::frames_file(layout.root()))
}

fn run_ffmpeg_extract(
    runner: &mut dyn SidecarRunner,
    events: &mut dyn PipelineEvents,
    primary: CommandSpec,
    fallback: CommandSpec,
    snap: &mut FrameSnapshot,
) -> Result<(), PipelineError> {
    match run_ffmpeg_logged(runner, &primary, events, snap) {
        Ok(()) => Ok(()),
        Err(_) => {
            events.log("VideoToolbox decode failed, retrying without hardware acceleration");
            run_ffmpeg_logged(runner, &fallback, events, snap)
        }
    }
}

fn run_colmap(
    layout: &ProjectLayout,
    settings: PipelineSettings,
    runner: &mut dyn SidecarRunner,
    cancel: &CancelFlag,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    cancel.check()?;
    if layout.is_complete(Stage::Colmap) && layout.sparse_model_dir().join("images.bin").is_file() {
        events.progress(Stage::Colmap, 100, "Camera poses already reconstructed");
        if let Some(preview) = colmap_pose::sparse_preview(&layout.sparse_model_dir()) {
            events.sparse_preview(&preview);
        }
        return Ok(());
    }

    events.progress(Stage::Colmap, 10, "Estimating camera poses");
    if settings.colmap_knobs().sift_backend == SiftBackend::Metal {
        colmap::require_metal_sidecar(
            &sidecar::resolve_binary("colmap")?,
            settings.sanitized().capture_mode,
        )?;
    }
    fs::create_dir_all(layout.sparse_dir())?;
    let selected = selected_frame_count(layout)?;
    let image_list = write_selected_image_list(layout)?;
    let specs = colmap::reconstruction_specs(
        &layout.frames_dir(),
        &layout.database_path(),
        &layout.sparse_dir(),
        settings,
        selected,
        &layout.database_global_path(),
        Some(&image_list),
    );
    for spec in &specs {
        cancel.check()?;
        if is_view_graph_calibrator(spec) {
            copy_database_for_global(layout)?;
        }
        let sub = spec.args.first().map(String::as_str).unwrap_or("");
        let (mut snap, message) = CameraSnapshot::for_spec(sub);
        snap.total = Some(selected as u32);
        events.camera_stats(&snap);
        events.progress(Stage::Colmap, snap.percent(), message);
        run_colmap_logged(runner, spec, events, layout, &mut snap, selected as u32)?;
    }

    if !layout.sparse_model_dir().join("images.bin").is_file() {
        return Err(PipelineError::colmap_failed_with(
            1,
            "",
            settings.sanitized().capture_mode,
        ));
    }
    assemble_dataset(layout)?;
    write_cameras_manifest(layout)?;
    if let Some(preview) = colmap_pose::sparse_preview(&layout.sparse_model_dir()) {
        events.sparse_preview(&preview);
    }
    let mut done = CameraSnapshot::for_spec("mapper").0;
    done.set_counts(
        colmap_pose::registered_count(&layout.sparse_model_dir().join("images.bin")),
        colmap_pose::points3d_count(&layout.sparse_model_dir().join("points3D.bin")),
        Some(selected as u32),
    );
    events.camera_stats(&done);
    layout.mark_complete(Stage::Colmap)?;
    events.progress(Stage::Colmap, 100, "Sparse reconstruction ready");
    Ok(())
}

fn selected_frame_count(layout: &ProjectLayout) -> Result<usize, PipelineError> {
    let manifest = FrameManifest::load(&manifest::frames_file(layout.root()))?;
    let selected = manifest.selected_names().len();
    if selected > 0 {
        Ok(selected)
    } else {
        count_frames(&layout.frames_dir())
    }
}

fn write_selected_image_list(layout: &ProjectLayout) -> Result<PathBuf, PipelineError> {
    let manifest = FrameManifest::load(&manifest::frames_file(layout.root()))?;
    let names = if manifest.frames.is_empty() {
        keyframes::list_stills(&layout.frames_dir())?
            .into_iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect::<Vec<_>>()
    } else {
        manifest
            .selected_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    };
    let path = layout.image_list_path();
    let borrowed: Vec<&str> = names.iter().map(String::as_str).collect();
    manifest::write_image_list(&path, &borrowed)?;
    Ok(path)
}

fn write_cameras_manifest(layout: &ProjectLayout) -> Result<(), PipelineError> {
    let frames = FrameManifest::load(&manifest::frames_file(layout.root()))?;
    let names: Vec<String> = if frames.frames.is_empty() {
        keyframes::list_stills(&layout.frames_dir())?
            .into_iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect()
    } else {
        frames.frames.iter().map(|f| f.name.clone()).collect()
    };
    let cameras = colmap_pose::camera_manifest(&layout.sparse_model_dir(), &names);
    cameras.save(&manifest::cameras_file(layout.root()))
}

fn is_view_graph_calibrator(spec: &CommandSpec) -> bool {
    spec.args.first().map(String::as_str) == Some("view_graph_calibrator")
}

/// Copies `database.db` to `database_global.db` so the calibrator never mutates the original.
fn copy_database_for_global(layout: &ProjectLayout) -> Result<(), PipelineError> {
    let src = layout.database_path();
    let dest = layout.database_global_path();
    if !src.is_file() {
        return Err(PipelineError::message(
            "COLMAP database is missing after matching; cannot calibrate for Room mapping.",
        ));
    }
    fs::copy(src, dest)?;
    Ok(())
}

fn run_train(
    layout: &ProjectLayout,
    settings: PipelineSettings,
    runner: &mut dyn SidecarRunner,
    cancel: &CancelFlag,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    cancel.check()?;
    if layout.is_complete(Stage::Train) && layout.output_ply().is_file() {
        events.progress(Stage::Train, 100, "Splat already trained");
        return Ok(());
    }

    warm_start_from_export(layout, events)?;
    events.progress(
        Stage::Train,
        15,
        &format!(
            "Training Gaussian splat ({} steps)",
            settings.sanitized().train_steps
        ),
    );
    fs::create_dir_all(layout.output_dir())?;
    let frame_count = count_frames(&layout.frames_dir()).unwrap_or(0);
    let spec = brush::train_spec(
        &layout.dataset_dir(),
        &layout.output_dir(),
        settings,
        frame_count,
    );
    run_train_logged(runner, &spec, settings.sanitized().train_steps, events)?;
    finalize_ply(&layout.output_dir(), &layout.output_ply())?;
    if !layout.output_ply().is_file() {
        return Err(PipelineError::message(
            "Brush finished but scene.ply is missing.",
        ));
    }
    layout.mark_complete(Stage::Train)?;
    events.progress(Stage::Train, 100, "Splat exported");
    Ok(())
}

/// Copies the newest `export_{iter}.ply` to `dataset/init.ply` so Brush can warm-start.
fn warm_start_from_export(
    layout: &ProjectLayout,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    let Some(src) = crate::project::newest_export_ply(&layout.output_dir()) else {
        return Ok(());
    };
    fs::create_dir_all(layout.dataset_dir())?;
    fs::copy(&src, layout.init_ply())?;
    events.log(&format!(
        "Continue from {} as dataset/init.ply (warm start, not a full checkpoint)",
        src.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    Ok(())
}

fn run_train_logged(
    runner: &mut dyn SidecarRunner,
    spec: &CommandSpec,
    total_steps: u32,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    let snap = TrainSnapshot::new(total_steps);
    events.train_stats(&snap);
    let started = Instant::now();
    let events = std::cell::RefCell::new(events);
    let snap = std::cell::RefCell::new(snap);
    runner.run(
        spec,
        &mut |line| {
            let mut snap = snap.borrow_mut();
            let changed = snap.ingest(line);
            snap.elapsed_secs = Some(started.elapsed().as_secs());
            snap.refresh_eta();
            let mut ev = events.borrow_mut();
            ev.log(line);
            if changed {
                ev.progress(Stage::Train, snap.percent(), &snap.summary());
                ev.train_stats(&snap);
            }
        },
        &mut |path| {
            let mut snap = snap.borrow_mut();
            snap.ingest_export(path);
            snap.elapsed_secs = Some(started.elapsed().as_secs());
            snap.refresh_eta();
            let mut ev = events.borrow_mut();
            ev.preview(path);
            ev.log(&format!("Preview checkpoint {}", path.display()));
            ev.progress(Stage::Train, snap.percent(), &snap.summary());
            ev.train_stats(&snap);
        },
    )
}

fn run_ffmpeg_logged(
    runner: &mut dyn SidecarRunner,
    spec: &CommandSpec,
    events: &mut dyn PipelineEvents,
    snap: &mut FrameSnapshot,
) -> Result<(), PipelineError> {
    let started = Instant::now();
    let events = std::cell::RefCell::new(events);
    let snap = std::cell::RefCell::new(snap);
    runner.run(
        spec,
        &mut |line| {
            let mut snap = snap.borrow_mut();
            let changed = snap.ingest(line);
            snap.elapsed_secs = Some(started.elapsed().as_secs());
            let mut ev = events.borrow_mut();
            ev.log(line);
            if changed {
                ev.frame_stats(&snap);
                ev.progress(Stage::Frames, snap.percent().max(5), &snap.summary());
            }
        },
        &mut |path| {
            let mut ev = events.borrow_mut();
            if crate::project::is_image(path) {
                ev.frame_preview(path);
            } else {
                ev.preview(path);
            }
        },
    )
}

fn run_colmap_logged(
    runner: &mut dyn SidecarRunner,
    spec: &CommandSpec,
    events: &mut dyn PipelineEvents,
    layout: &ProjectLayout,
    snap: &mut CameraSnapshot,
    frame_total: u32,
) -> Result<(), PipelineError> {
    let started = Instant::now();
    let events = std::cell::RefCell::new(events);
    let snap = std::cell::RefCell::new(snap);
    let last_sparse = std::cell::Cell::new(started);
    runner.run(
        spec,
        &mut |line| {
            let mut snap = snap.borrow_mut();
            let changed = snap.ingest(line);
            snap.elapsed_secs = Some(started.elapsed().as_secs());
            let mut ev = events.borrow_mut();
            for record in crate::colmap_log::records(line) {
                if let Some(msg) = crate::colmap_log::ui_log(record) {
                    ev.log(&msg);
                }
            }
            if changed {
                ev.camera_stats(&snap);
                ev.progress(Stage::Colmap, snap.percent(), &snap.summary());
            }
            if last_sparse.get().elapsed().as_millis() >= 500 {
                last_sparse.set(Instant::now());
                if let Some(preview) = colmap_pose::sparse_preview(&layout.sparse_model_dir()) {
                    snap.set_counts(
                        Some(preview.cameras.len() as u32),
                        Some(preview.points.len() as u32),
                        Some(frame_total),
                    );
                    ev.sparse_preview(&preview);
                }
                ev.camera_stats(&snap);
                ev.progress(Stage::Colmap, snap.percent(), &snap.summary());
            }
        },
        &mut |path| {
            let mut ev = events.borrow_mut();
            if crate::project::is_image(path) {
                ev.frame_preview(path);
            } else {
                ev.preview(path);
            }
        },
    )
}

fn copy_input_video(source: &Path, input_dir: &Path) -> Result<PathBuf, PipelineError> {
    if !source.is_file() {
        return Err(PipelineError::message(format!(
            "Video not found: {}",
            source.display()
        )));
    }
    fs::create_dir_all(input_dir)?;
    let dest = input_dir.join(
        source
            .file_name()
            .ok_or_else(|| PipelineError::message("video path is missing a file name"))?,
    );
    if source.canonicalize().ok() != dest.canonicalize().ok() {
        fs::copy(source, &dest).map_err(|err| PipelineError::from_io_path(err, source))?;
    }
    Ok(dest)
}

fn import_image_folder(source: &Path, frames_dir: &Path) -> Result<(), PipelineError> {
    if !source.is_dir() {
        return Err(PipelineError::message(format!(
            "Image folder not found: {}",
            source.display()
        )));
    }
    fs::create_dir_all(frames_dir)?;
    let mut copied = 0;
    for entry in fs::read_dir(source).map_err(|err| PipelineError::from_io_path(err, source))? {
        let src = entry?.path();
        if src.is_file() && crate::project::is_image(&src) {
            let dest = frames_dir.join(src.file_name().unwrap());
            crate::project::link_or_copy(&src, &dest)?;
            copied += 1;
        }
    }
    if copied == 0 {
        return Err(PipelineError::message(
            "No JPEG or PNG files found in the image folder.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::ArchiveLibrary;
    use crate::preset::Preset;
    use crate::sidecar::FakeRunner;
    use tempfile::tempdir;

    struct CollectingEvents {
        progress: Vec<(String, u8, String)>,
        logs: Vec<String>,
        previews: Vec<String>,
    }

    impl CollectingEvents {
        fn new() -> Self {
            Self {
                progress: Vec::new(),
                logs: Vec::new(),
                previews: Vec::new(),
            }
        }
    }

    impl PipelineEvents for CollectingEvents {
        fn progress(&mut self, stage: Stage, percent: u8, message: &str) {
            self.progress
                .push((stage.as_str().into(), percent, message.into()));
        }

        fn log(&mut self, line: &str) {
            self.logs.push(line.into());
        }

        fn preview(&mut self, path: &Path) {
            self.previews.push(path.display().to_string());
        }
    }

    fn write_video_stub(path: &Path) {
        fs::write(path, b"not-a-real-video").unwrap();
    }

    fn cfg(dir: &Path, source: PathBuf, kind: InputKind) -> PipelineConfig {
        PipelineConfig {
            project_dir: Some(dir.join("project")),
            archive_dir: dir.join("archive"),
            temp_project: false,
            source,
            kind,
            settings: PipelineSettings::from_preset(Preset::Fast),
            force: false,
            until: Stage::Train,
        }
    }

    #[test]
    fn full_pipeline_records_ffmpeg_colmap_brush() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        write_video_stub(&video);
        let config = cfg(dir.path(), video, InputKind::Video);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        let outcome = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        assert!(outcome.ply.is_file());
        assert!(outcome.archive_id.is_some());
        let sidecars: Vec<_> = runner.calls.iter().map(|c| c.sidecar).collect();
        assert_eq!(sidecars[0], "ffmpeg");
        assert_eq!(
            runner
                .calls
                .iter()
                .filter(|c| c.sidecar == "ffmpeg" && !c.args.iter().any(|a| a == "ffmetadata"))
                .count(),
            2,
            "video extract is two FFmpeg passes"
        );
        assert!(sidecars.contains(&"colmap"));
        assert!(sidecars.contains(&"brush"));
        assert_eq!(
            runner
                .calls
                .iter()
                .filter(|c| c.sidecar == "colmap")
                .count(),
            3
        );
        assert!(events.logs.iter().any(|l| l.contains("ffmpeg")));
        assert!(
            !events.previews.is_empty(),
            "brush should emit a live preview ply"
        );
        assert!(
            events
                .progress
                .iter()
                .any(|(_, _, msg)| msg.contains("Gaussians")),
            "train logs should surface splat counts"
        );
        assert!(
            events.logs.iter().any(|l| l.contains("ISO6709")),
            "video geo probe should parse fake QuickTime tags"
        );
    }

    #[test]
    fn resume_skips_completed_stages() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        write_video_stub(&video);
        let config = cfg(dir.path(), video, InputKind::Video);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        let first = runner.calls.len();
        let mut runner2 = FakeRunner::new();
        let mut events2 = CollectingEvents::new();
        run_pipeline(&config, &mut runner2, &CancelFlag::new(), &mut events2).unwrap();
        assert!(runner2.calls.is_empty(), "resume must not re-run sidecars");
        assert_eq!(first, 7, "2 extract + 3 colmap + brush + geo probe");
    }

    #[test]
    fn force_reruns_from_frames() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        write_video_stub(&video);
        let config = cfg(dir.path(), video, InputKind::Video);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        let mut forced = config.clone();
        forced.force = true;
        let mut runner2 = FakeRunner::new();
        let mut events2 = CollectingEvents::new();
        run_pipeline(&forced, &mut runner2, &CancelFlag::new(), &mut events2).unwrap();
        assert_eq!(runner2.calls[0].sidecar, "ffmpeg");
    }

    #[test]
    fn image_folder_skips_ffmpeg() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i:02}.png")), b"img").unwrap();
        }
        let config = cfg(dir.path(), images, InputKind::Images);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        assert!(runner.calls.iter().all(|c| c.sidecar != "ffmpeg"));
        assert_eq!(runner.calls[0].sidecar, "colmap");
    }

    #[test]
    fn room_pipeline_runs_global_mapper() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i:02}.png")), b"img").unwrap();
        }
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = crate::settings::CaptureMode::Room;
        let mut config = cfg(dir.path(), images, InputKind::Images);
        config.settings = settings;
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        let commands: Vec<_> = runner
            .calls
            .iter()
            .filter(|c| c.sidecar == "colmap")
            .map(|c| c.args[0].as_str())
            .collect();
        assert_eq!(
            commands,
            [
                "feature_extractor",
                "exhaustive_matcher",
                "view_graph_calibrator",
                "global_mapper"
            ]
        );
        let layout = ProjectLayout::new(config.project_dir.as_ref().unwrap());
        assert!(layout.sparse_model_dir().join("images.bin").is_file());
        assert!(layout.database_global_path().is_file());
        assert!(events
            .progress
            .iter()
            .any(|(_, _, msg)| msg == "Calibrating cameras"));
        assert!(events
            .progress
            .iter()
            .any(|(_, _, msg)| msg == "Mapping cameras"));
    }

    #[test]
    fn too_few_frames_is_a_clear_error() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        fs::write(images.join("only.jpg"), b"img").unwrap();
        let config = cfg(dir.path(), images, InputKind::Images);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        let err = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("at least"));
    }

    #[test]
    fn colmap_failure_uses_friendly_hint() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i}.jpg")), b"img").unwrap();
        }
        let config = cfg(dir.path(), images, InputKind::Images);
        let mut runner = FakeRunner::new();
        runner.fail_on = Some("colmap");
        let mut events = CollectingEvents::new();
        let err = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("orbit") || msg.contains("COLMAP"));
    }

    #[test]
    fn room_colmap_failure_avoids_orbit_hint() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i}.jpg")), b"img").unwrap();
        }
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = crate::settings::CaptureMode::Room;
        let mut config = cfg(dir.path(), images, InputKind::Images);
        config.settings = settings;
        let mut runner = FakeRunner::new();
        runner.fail_on = Some("colmap");
        let mut events = CollectingEvents::new();
        let err = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("walls"));
        assert!(!msg.to_lowercase().contains("orbit"));
    }

    #[test]
    fn temp_project_is_removed_after_archive() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        write_video_stub(&video);
        let mut config = cfg(dir.path(), video, InputKind::Video);
        config.temp_project = true;
        config.project_dir = None;
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        let outcome = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        assert!(outcome.ply.is_file());
        assert!(outcome.archive_id.is_some());
        let scratch = ArchiveLibrary::open(&config.archive_dir)
            .unwrap()
            .scratch_dir(&config.source);
        assert!(!scratch.exists(), "scratch must be deleted after ingest");
        assert!(
            outcome.ply.starts_with(&config.archive_dir),
            "viewer path must be the archived ply"
        );
    }

    #[test]
    fn temp_project_survives_failed_train() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        write_video_stub(&video);
        let mut config = cfg(dir.path(), video, InputKind::Video);
        config.temp_project = true;
        config.project_dir = None;
        let mut runner = FakeRunner::new();
        runner.fail_on = Some("brush");
        let mut events = CollectingEvents::new();
        let err = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap_err();
        assert!(err.to_string().contains("Brush") || err.to_string().contains("Training"));
        let scratch = ArchiveLibrary::open(&config.archive_dir)
            .unwrap()
            .scratch_dir(&config.source);
        assert!(scratch.is_dir(), "failed runs keep scratch for resume");
    }

    #[test]
    fn until_frames_skips_colmap_and_writes_manifest() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i:02}.png")), b"img").unwrap();
        }
        let mut config = cfg(dir.path(), images, InputKind::Images);
        config.until = Stage::Frames;
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        let outcome = run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        assert!(runner.calls.iter().all(|c| c.sidecar != "colmap"));
        assert_eq!(outcome.completed_stage, Stage::Frames);
        let frames =
            FrameManifest::load(&manifest::frames_file(config.project_dir.as_ref().unwrap()))
                .unwrap();
        assert_eq!(frames.frames.len(), 8);
        assert!(frames.frames.iter().all(|f| f.selected));
    }

    #[test]
    fn colmap_receives_image_list_path() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i:02}.png")), b"img").unwrap();
        }
        let config = cfg(dir.path(), images, InputKind::Images);
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        let extractor = runner
            .calls
            .iter()
            .find(|c| {
                c.sidecar == "colmap"
                    && c.args.first().map(String::as_str) == Some("feature_extractor")
            })
            .unwrap();
        assert!(extractor
            .args
            .windows(2)
            .any(|w| w[0] == "--image_list_path"));
    }

    #[test]
    fn unfinished_train_copies_export_as_init_ply() {
        let dir = tempdir().unwrap();
        let images = dir.path().join("photos");
        fs::create_dir_all(&images).unwrap();
        for i in 0..8 {
            fs::write(images.join(format!("{i:02}.png")), b"img").unwrap();
        }
        let mut config = cfg(dir.path(), images, InputKind::Images);
        config.until = Stage::Colmap;
        let mut runner = FakeRunner::new();
        let mut events = CollectingEvents::new();
        run_pipeline(&config, &mut runner, &CancelFlag::new(), &mut events).unwrap();
        let layout = ProjectLayout::new(config.project_dir.as_ref().unwrap());
        fs::write(layout.output_dir().join("export_1000.ply"), b"ply").unwrap();
        config.until = Stage::Train;
        let mut runner2 = FakeRunner::new();
        let mut events2 = CollectingEvents::new();
        run_pipeline(&config, &mut runner2, &CancelFlag::new(), &mut events2).unwrap();
        assert!(layout.init_ply().is_file());
        assert!(runner2.calls.iter().any(|c| c.sidecar == "brush"));
        assert!(events2
            .logs
            .iter()
            .any(|line| line.contains("init.ply") || line.contains("export_1000")));
    }
}
