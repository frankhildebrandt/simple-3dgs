//! Brush CLI training command. Flags match ArthurBrussee/brush on `main`.
//! Splat cap comes from settings; refine cadence follows frame count unless set.

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
    let brush = settings.brush_knobs();
    let steps = settings.train_steps;
    let export_every = if brush.export_every == 0 {
        preview_export_every(steps)
    } else {
        brush.export_every
    };
    let refine = if brush.refine_every == 0 {
        refine_every(frame_count)
    } else {
        brush.refine_every
    };
    let mut args = vec![
        path_arg(dataset_dir),
        "--total-train-iters".into(),
        steps.to_string(),
        "--max-splats".into(),
        settings.max_splats.to_string(),
        "--refine-every".into(),
        refine.to_string(),
        "--export-path".into(),
        path_arg(export_dir),
        "--export-name".into(),
        EXPORT_NAME.into(),
        "--export-every".into(),
        export_every.to_string(),
        "--lr-mean".into(),
        brush.lr_mean.to_string(),
        "--lr-mean-end".into(),
        brush.lr_mean_end.to_string(),
        "--mean-noise-weight".into(),
        brush.mean_noise_weight.to_string(),
        "--lr-coeffs-dc".into(),
        brush.lr_coeffs_dc.to_string(),
        "--lr-coeffs-sh-scale".into(),
        brush.lr_coeffs_sh_scale.to_string(),
        "--lr-opac".into(),
        brush.lr_opac.to_string(),
        "--lr-scale".into(),
        brush.lr_scale.to_string(),
        "--lr-rotation".into(),
        brush.lr_rotation.to_string(),
        "--ssim-weight".into(),
        brush.ssim_weight.to_string(),
        "--opac-decay".into(),
        brush.opac_decay.to_string(),
        "--lpips-loss-weight".into(),
        brush.lpips_loss_weight.to_string(),
        "--match-alpha-weight".into(),
        brush.match_alpha_weight.to_string(),
        "--background-color".into(),
        brush.background_r.to_string(),
        brush.background_g.to_string(),
        brush.background_b.to_string(),
        "--background-noise-strength".into(),
        brush.background_noise_strength.to_string(),
        "--growth-grad-threshold".into(),
        brush.growth_grad_threshold.to_string(),
        "--growth-select-fraction".into(),
        brush.growth_select_fraction.to_string(),
        "--growth-stop-iter".into(),
        brush.growth_stop_iter.to_string(),
        "--split-at-screen-size".into(),
        brush.split_at_screen_size.to_string(),
        "--sh-degree".into(),
        brush.sh_degree.to_string(),
        "--seed".into(),
        brush.seed.to_string(),
        "--eval-every".into(),
        brush.eval_every.to_string(),
        "--max-scene-batch-cache-size".into(),
        format!("{}GiB", brush.max_scene_batch_cache_gib),
        "--lod-levels".into(),
        brush.lod_levels.to_string(),
        "--lod-refine-steps".into(),
        brush.lod_refine_steps.to_string(),
        "--lod-decimation-keep".into(),
        brush.lod_decimation_keep.to_string(),
        "--lod-image-scale".into(),
        brush.lod_image_scale.to_string(),
    ];
    if let Some(res) = settings.train_resolution() {
        args.push("--max-resolution".into());
        args.push(res.to_string());
    }
    CommandSpec::new("brush", args).watching(export_dir)
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

    #[test]
    fn zero_refine_every_keeps_heuristic() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        let mut brush = settings.brush_knobs();
        brush.refine_every = 0;
        settings.brush = Some(brush);
        let spec = train_spec(Path::new("d"), Path::new("o"), settings, 80);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--refine-every" && w[1] == "80"));
    }

    #[test]
    fn explicit_refine_every_is_passed() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        let mut brush = settings.brush_knobs();
        brush.refine_every = 33;
        brush.sh_degree = 2;
        settings.brush = Some(brush);
        let spec = train_spec(Path::new("d"), Path::new("o"), settings, 80);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--refine-every" && w[1] == "33"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--sh-degree" && w[1] == "2"));
    }

    #[test]
    fn background_color_is_three_argv_values() {
        let spec = train_spec(
            Path::new("d"),
            Path::new("o"),
            PipelineSettings::from_preset(Preset::Fast),
            20,
        );
        let flag = spec
            .args
            .iter()
            .position(|a| a == "--background-color")
            .expect("background-color flag");
        assert_eq!(
            &spec.args[flag..flag + 4],
            ["--background-color", "0", "0", "0"]
        );
    }

    #[test]
    fn native_train_omits_max_resolution_when_uncapped() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.max_image_size = 0;
        let mut brush = settings.brush_knobs();
        brush.train_max_resolution = 0;
        settings.brush = Some(brush);
        let spec = train_spec(Path::new("d"), Path::new("o"), settings, 20);
        assert!(!spec.args.iter().any(|a| a == "--max-resolution"));
    }
}
