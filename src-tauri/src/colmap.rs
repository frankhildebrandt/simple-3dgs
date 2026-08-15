//! COLMAP SfM command sequence. Sequential matching for orbits and long paths;
//! exhaustive matching for room captures so loops can close (up to 250 frames).
//! Longer rooms fall back to sequential matching with quadratic overlap.
//! Flags match COLMAP 4 (`FeatureExtraction` / `FeatureMatching`, not the old `SiftExtraction.use_gpu`).

use std::path::Path;

use crate::settings::{CaptureMode, PipelineSettings};
use crate::sidecar::{path_arg, CommandSpec};

const CAMERA_MODEL: &str = "SIMPLE_RADIAL";
const EXHAUSTIVE_FRAME_LIMIT: usize = 250;
const SCENE_MIN_OVERLAP: u32 = 20;

/// Feature extraction, matching, then incremental mapping.
pub fn reconstruction_specs(
    image_dir: &Path,
    database: &Path,
    sparse_dir: &Path,
    settings: PipelineSettings,
    frame_count: usize,
) -> Vec<CommandSpec> {
    let settings = settings.sanitized();
    vec![
        feature_extractor_spec(image_dir, database, settings).capture(settings.capture_mode),
        matcher_spec(database, settings, frame_count).capture(settings.capture_mode),
        mapper_spec(image_dir, database, sparse_dir, settings).capture(settings.capture_mode),
    ]
}

pub fn feature_extractor_spec(
    image_dir: &Path,
    database: &Path,
    settings: PipelineSettings,
) -> CommandSpec {
    let mut args = vec![
        "feature_extractor".into(),
        "--database_path".into(),
        path_arg(database),
        "--image_path".into(),
        path_arg(image_dir),
        "--ImageReader.single_camera".into(),
        "1".into(),
        "--ImageReader.camera_model".into(),
        CAMERA_MODEL.into(),
        "--FeatureExtraction.use_gpu".into(),
        "0".into(),
    ];
    if let Some(size) = settings.longest_edge() {
        args.push("--FeatureExtraction.max_image_size".into());
        args.push(size.to_string());
    }
    CommandSpec::new("colmap", args)
}

/// Sequential or exhaustive matching depending on capture mode and frame count.
pub fn matcher_spec(
    database: &Path,
    settings: PipelineSettings,
    frame_count: usize,
) -> CommandSpec {
    if uses_exhaustive(settings.capture_mode, frame_count) {
        exhaustive_matcher_spec(database)
    } else {
        sequential_matcher_spec(database, settings)
    }
}

pub fn sequential_matcher_spec(database: &Path, settings: PipelineSettings) -> CommandSpec {
    let mut args = vec![
        "sequential_matcher".into(),
        "--database_path".into(),
        path_arg(database),
        "--FeatureMatching.use_gpu".into(),
        "0".into(),
        "--SequentialMatching.overlap".into(),
        sequential_overlap(settings).to_string(),
    ];
    if settings.capture_mode == CaptureMode::Room {
        args.push("--SequentialMatching.quadratic_overlap".into());
        args.push("1".into());
    }
    CommandSpec::new("colmap", args)
}

pub fn exhaustive_matcher_spec(database: &Path) -> CommandSpec {
    CommandSpec::new(
        "colmap",
        vec![
            "exhaustive_matcher".into(),
            "--database_path".into(),
            path_arg(database),
            "--FeatureMatching.use_gpu".into(),
            "0".into(),
        ],
    )
}

pub fn mapper_spec(
    image_dir: &Path,
    database: &Path,
    sparse_dir: &Path,
    settings: PipelineSettings,
) -> CommandSpec {
    let mut args = vec![
        "mapper".into(),
        "--database_path".into(),
        path_arg(database),
        "--image_path".into(),
        path_arg(image_dir),
        "--output_path".into(),
        path_arg(sparse_dir),
        "--Mapper.multiple_models".into(),
        "0".into(),
        "--Mapper.min_model_size".into(),
        min_model_size(settings.capture_mode).to_string(),
    ];
    if settings.capture_mode == CaptureMode::Outdoor {
        args.push("--Mapper.init_min_tri_angle".into());
        args.push("8".into());
    }
    CommandSpec::new("colmap", args)
}

fn uses_exhaustive(mode: CaptureMode, frame_count: usize) -> bool {
    mode == CaptureMode::Room && frame_count <= EXHAUSTIVE_FRAME_LIMIT
}

fn sequential_overlap(settings: PipelineSettings) -> u32 {
    let overlap = settings.sanitized().match_overlap;
    match settings.capture_mode {
        CaptureMode::Object => overlap,
        CaptureMode::Room | CaptureMode::Outdoor => overlap.max(SCENE_MIN_OVERLAP),
    }
}

fn min_model_size(mode: CaptureMode) -> u32 {
    match mode {
        CaptureMode::Object => 6,
        CaptureMode::Room | CaptureMode::Outdoor => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;
    use std::path::Path;

    fn object_settings() -> PipelineSettings {
        PipelineSettings::from_preset(Preset::Fast)
    }

    fn room_settings() -> PipelineSettings {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = CaptureMode::Room;
        settings
    }

    fn outdoor_settings() -> PipelineSettings {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = CaptureMode::Outdoor;
        settings
    }

    #[test]
    fn sequence_is_extractor_matcher_mapper() {
        let specs = reconstruction_specs(
            Path::new("frames"),
            Path::new("colmap/database.db"),
            Path::new("colmap/sparse"),
            object_settings(),
            40,
        );
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].args[0], "feature_extractor");
        assert_eq!(specs[1].args[0], "sequential_matcher");
        assert_eq!(specs[2].args[0], "mapper");
        assert!(specs.iter().all(|s| s.sidecar == "colmap"));
    }

    #[test]
    fn object_mode_stays_sequential() {
        let spec = matcher_spec(Path::new("db"), object_settings(), 40);
        assert_eq!(spec.args[0], "sequential_matcher");
    }

    #[test]
    fn room_with_few_frames_uses_exhaustive() {
        let spec = matcher_spec(Path::new("db"), room_settings(), 40);
        assert_eq!(spec.args[0], "exhaustive_matcher");
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--FeatureMatching.use_gpu" && w[1] == "0"));
    }

    #[test]
    fn room_mid_length_still_uses_exhaustive() {
        let spec = matcher_spec(Path::new("db"), room_settings(), 150);
        assert_eq!(spec.args[0], "exhaustive_matcher");
    }

    #[test]
    fn room_with_many_frames_uses_sequential_quadratic_overlap() {
        let mut settings = room_settings();
        settings.match_overlap = 15;
        let spec = matcher_spec(Path::new("db"), settings, 300);
        assert_eq!(spec.args[0], "sequential_matcher");
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--SequentialMatching.overlap" && w[1] == "20"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--SequentialMatching.quadratic_overlap" && w[1] == "1"));
    }

    #[test]
    fn object_sequential_skips_quadratic_overlap() {
        let spec = matcher_spec(Path::new("db"), object_settings(), 40);
        assert_eq!(spec.args[0], "sequential_matcher");
        assert!(!spec.args.iter().any(|a| a.contains("quadratic_overlap")));
    }

    #[test]
    fn outdoor_mapper_lowers_init_tri_angle() {
        let spec = mapper_spec(
            Path::new("frames"),
            Path::new("db"),
            Path::new("sparse"),
            outdoor_settings(),
        );
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--Mapper.min_model_size" && w[1] == "10"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--Mapper.init_min_tri_angle" && w[1] == "8"));
    }

    #[test]
    fn colmap4_uses_feature_extraction_cpu_flags() {
        let spec = feature_extractor_spec(
            Path::new("frames"),
            Path::new("db"),
            PipelineSettings::from_preset(Preset::Balanced),
        );
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--ImageReader.single_camera" && w[1] == "1"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--FeatureExtraction.use_gpu" && w[1] == "0"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--FeatureExtraction.max_image_size" && w[1] == "1600"));
        assert!(!spec
            .args
            .iter()
            .any(|a| a.contains("SiftExtraction.use_gpu")));
        assert!(!spec
            .args
            .iter()
            .any(|a| a.contains("SiftExtraction.max_image_size")));
    }

    #[test]
    fn matcher_uses_configured_overlap_on_cpu() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.match_overlap = 8;
        let spec = sequential_matcher_spec(Path::new("db"), settings);
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--SequentialMatching.overlap" && w[1] == "8"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--FeatureMatching.use_gpu" && w[1] == "0"));
        assert!(!spec.args.iter().any(|a| a.contains("SiftMatching.use_gpu")));
    }

    #[test]
    fn mapper_keeps_a_single_small_model() {
        let spec = mapper_spec(
            Path::new("frames"),
            Path::new("db"),
            Path::new("sparse"),
            object_settings(),
        );
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--Mapper.multiple_models" && w[1] == "0"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "--Mapper.min_model_size" && w[1] == "6"));
        assert!(!spec.args.iter().any(|a| a == "--Mapper.init_min_tri_angle"));
    }
}
