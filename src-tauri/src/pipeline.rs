//! End-to-end Video/Images → PLY orchestration with resume and cancel.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::archive::{ArchiveLibrary, IngestRequest};
use crate::brush;
use crate::colmap;
use crate::error::PipelineError;
use crate::ffmpeg;
use crate::geo::{self, GeoFix};
use crate::project::{
    assemble_dataset, count_frames, finalize_ply, ProjectLayout, Stage, MIN_FRAMES,
};
use crate::settings::PipelineSettings;
use crate::sidecar::{CancelFlag, CommandSpec, SidecarRunner};
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
}

/// PLY the viewer should load, plus archive ingest outcome.
#[derive(Debug, Clone)]
pub struct PipelineOutcome {
    pub ply: PathBuf,
    pub archive_id: Option<String>,
    pub archive_error: Option<String>,
}

pub trait PipelineEvents {
    fn progress(&mut self, stage: Stage, percent: u8, message: &str);
    fn log(&mut self, line: &str);
    fn preview(&mut self, path: &Path);
    fn train_stats(&mut self, _stats: &TrainSnapshot) {}
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

    if config.force {
        layout.clear_from(Stage::Frames)?;
    }

    run_frames(config, &layout, runner, cancel, events)?;
    run_colmap(&layout, config.settings, runner, cancel, events)?;
    run_train(&layout, config.settings, runner, cancel, events)?;

    events.progress(Stage::Train, 100, "Done");
    archive_outcome(config, &library, &layout, runner, events)
}

fn resolve_project_dir(
    config: &PipelineConfig,
    library: &ArchiveLibrary,
) -> Result<PathBuf, PipelineError> {
    if config.temp_project {
        Ok(library.scratch_dir(&config.source))
    } else {
        config.project_dir.clone().ok_or_else(|| {
            PipelineError::message(
                "Choose a project folder, or enable the temporary project folder.",
            )
        })
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
            })
        }
        Err(err) => {
            let message = err.to_string();
            events.log(&format!("Archive failed: {message}"));
            Ok(PipelineOutcome {
                ply: layout.output_ply(),
                archive_id: None,
                archive_error: Some(message),
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
            let spec = ffmpeg::extract_spec(&video, &layout.frames_dir(), config.settings);
            match run_logged(runner, &spec, events) {
                Ok(()) => {}
                Err(_) => {
                    events
                        .log("VideoToolbox decode failed, retrying without hardware acceleration");
                    let fallback = ffmpeg::extract_spec_with_hwaccel(
                        &video,
                        &layout.frames_dir(),
                        config.settings,
                        false,
                    );
                    run_logged(runner, &fallback, events)?;
                }
            }
        }
        InputKind::Images => {
            import_image_folder(&config.source, &layout.frames_dir())?;
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
        return Ok(());
    }

    events.progress(Stage::Colmap, 10, "Estimating camera poses");
    fs::create_dir_all(layout.sparse_dir())?;
    let frame_count = count_frames(&layout.frames_dir())?;
    let specs = colmap::reconstruction_specs(
        &layout.frames_dir(),
        &layout.database_path(),
        &layout.sparse_dir(),
        settings,
        frame_count,
        &layout.database_global_path(),
    );
    for spec in &specs {
        cancel.check()?;
        if is_view_graph_calibrator(spec) {
            copy_database_for_global(layout)?;
        }
        let (percent, message) = colmap_progress(spec);
        events.progress(Stage::Colmap, percent, message);
        run_logged(runner, spec, events)?;
    }

    if !layout.sparse_model_dir().join("images.bin").is_file() {
        return Err(PipelineError::colmap_failed_with(
            1,
            "",
            settings.sanitized().capture_mode,
        ));
    }
    assemble_dataset(layout)?;
    layout.mark_complete(Stage::Colmap)?;
    events.progress(Stage::Colmap, 100, "Sparse reconstruction ready");
    Ok(())
}

fn is_view_graph_calibrator(spec: &CommandSpec) -> bool {
    spec.args.first().map(String::as_str) == Some("view_graph_calibrator")
}

/// Progress percent and label from the COLMAP subcommand in `args[0]`.
fn colmap_progress(spec: &CommandSpec) -> (u8, &'static str) {
    match spec.args.first().map(String::as_str) {
        Some("feature_extractor") => (20, "Extracting features"),
        Some("sequential_matcher" | "exhaustive_matcher") => (50, "Matching views"),
        Some("view_graph_calibrator") => (70, "Calibrating cameras"),
        Some("mapper" | "global_mapper") => (85, "Mapping cameras"),
        _ => (10, "Estimating camera poses"),
    }
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
            let mut ev = events.borrow_mut();
            ev.preview(path);
            ev.log(&format!("Preview checkpoint {}", path.display()));
            ev.progress(Stage::Train, snap.percent(), &snap.summary());
            ev.train_stats(&snap);
        },
    )
}

fn run_logged(
    runner: &mut dyn SidecarRunner,
    spec: &CommandSpec,
    events: &mut dyn PipelineEvents,
) -> Result<(), PipelineError> {
    let events = std::cell::RefCell::new(events);
    runner.run(
        spec,
        &mut |line| events.borrow_mut().log(line),
        &mut |path| events.borrow_mut().preview(path),
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
        fs::copy(source, &dest)?;
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
    for entry in fs::read_dir(source)? {
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
        assert_eq!(first, 6, "extract + 3 colmap + brush + geo probe");
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
}
