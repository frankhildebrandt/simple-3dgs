//! Brush CLI training command. Flags match ArthurBrussee/brush on `main`.
//! Splat cap comes from settings; refine cadence follows frame count.

use std::path::Path;

use crate::settings::PipelineSettings;
use crate::sidecar::{path_arg, CommandSpec};

const EXPORT_NAME: &str = "export_{iter}.ply";

/// How often Brush writes a PLY so the viewer can show a live preview.
pub fn preview_export_every(train_steps: u32) -> u32 {
    (train_steps / 10).clamp(250, 1000)
}

/// Refine about once per scene-covering set of views. Rooms with many frames stay at 200.
pub fn refine_every(frame_count: usize) -> u32 {
    (frame_count as u32).clamp(50, 200)
}

/// Headless train: COLMAP dataset in, numbered PLY checkpoints out.
pub fn train_spec(
    dataset_dir: &Path,
    export_dir: &Path,
    settings: PipelineSettings,
    frame_count: usize,
) -> CommandSpec {
    let settings = settings.sanitized();
    let steps = settings.train_steps;
    let every = preview_export_every(steps);
    CommandSpec::new(
        "brush",
        vec![
            path_arg(dataset_dir),
            "--total-train-iters".into(),
            steps.to_string(),
            "--max-resolution".into(),
            settings.train_resolution().to_string(),
            "--max-splats".into(),
            settings.max_splats.to_string(),
            "--refine-every".into(),
            refine_every(frame_count).to_string(),
            "--export-path".into(),
            path_arg(export_dir),
            "--export-name".into(),
            EXPORT_NAME.into(),
            "--export-every".into(),
            every.to_string(),
        ],
    )
    .watching(export_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;
    use crate::settings::CaptureMode;
    use std::path::Path;

    fn room_quality() -> PipelineSettings {
        let mut settings = PipelineSettings::from_preset(Preset::Quality);
        settings.capture_mode = CaptureMode::Room;
        settings
    }

    #[test]
    fn train_spec_exports_iter_checkpoints() {
        let spec = train_spec(
            Path::new("/tmp/dataset"),
            Path::new("/tmp/output"),
            PipelineSettings::from_preset(Preset::Quality),
            80,
        );
        assert_eq!(spec.sidecar, "brush");
        assert_eq!(spec.args[0], "/tmp/dataset");
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--total-train-iters" && w[1] == "30000"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--export-name" && w[1] == "export_{iter}.ply"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--export-every" && w[1] == "1000"));
        assert_eq!(spec.watch_dir.as_deref(), Some(Path::new("/tmp/output")));
    }

    #[test]
    fn custom_steps_override_preset() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.train_steps = 2500;
        let spec = train_spec(Path::new("d"), Path::new("o"), settings, 8);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--total-train-iters" && w[1] == "2500"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--export-every" && w[1] == "250"));
    }

    #[test]
    fn preset_caps_are_passed_through() {
        let spec = train_spec(
            Path::new("d"),
            Path::new("o"),
            PipelineSettings::from_preset(Preset::Fast),
            20,
        );
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--max-splats" && w[1] == "2000000"));
        let quality = train_spec(
            Path::new("d"),
            Path::new("o"),
            PipelineSettings::from_preset(Preset::Quality),
            80,
        );
        assert!(quality
            .args
            .windows(2)
            .any(|w| w[0] == "--max-splats" && w[1] == "10000000"));
    }

    #[test]
    fn custom_splat_cap_is_not_overridden_by_capture_mode() {
        let mut settings = room_quality();
        settings.max_splats = 8_000_000;
        let spec = train_spec(Path::new("d"), Path::new("o"), settings, 120);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--max-splats" && w[1] == "8000000"));
    }

    #[test]
    fn refine_every_tracks_coverage_then_caps() {
        assert_eq!(refine_every(20), 50);
        assert_eq!(refine_every(80), 80);
        assert_eq!(refine_every(400), 200);
        let spec = train_spec(Path::new("d"), Path::new("o"), room_quality(), 20);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--refine-every" && w[1] == "50"));
    }
}
