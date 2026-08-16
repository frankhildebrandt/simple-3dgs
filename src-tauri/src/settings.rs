//! Explicit pipeline knobs. Presets only fill these; the pipeline never reads a preset name.

use serde::{Deserialize, Serialize};

use crate::brush_knobs::BrushKnobs;
use crate::colmap_knobs::ColmapKnobs;
use crate::extract_knobs::ExtractKnobs;
use crate::preset::Preset;
use crate::viewer_knobs::ViewerKnobs;

pub use crate::capture_mode::CaptureMode;

pub const FPS_MIN: f32 = 0.05;
pub const FPS_MAX: f32 = 60.0;
pub const MAX_IMAGE_SIZE_MIN: u32 = 64;
pub const MAX_IMAGE_SIZE_MAX: u32 = 16_384;
pub const TRAIN_STEPS_MIN: u32 = 1;
pub const TRAIN_STEPS_MAX: u32 = 1_000_000;
pub const MATCH_OVERLAP_MIN: u32 = 1;
pub const MATCH_OVERLAP_MAX: u32 = 200;
pub const MAX_SPLATS_MIN: u32 = 1_000;
pub const MAX_SPLATS_MAX: u32 = 100_000_000;
pub const MAX_FRAMES_MIN: u32 = 1;
pub const MAX_FRAMES_MAX: u32 = 50_000;

/// Still format written by FFmpeg. PNG is lossless; JPEG uses [`PipelineSettings::jpeg_quality`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FrameFormat {
    #[default]
    Jpg,
    Png,
}

impl FrameFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
        }
    }
}

/// How video stills are chosen. Density uses fps plus a frame cap; Change uses picture motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExtractMode {
    #[default]
    Density,
    Change,
}

/// User-facing reconstruction settings. Zeroed time fields mean "whole clip".
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineSettings {
    pub fps: f32,
    pub max_image_size: u32,
    pub start_seconds: f32,
    pub duration_seconds: f32,
    #[serde(default)]
    pub frame_format: FrameFormat,
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,
    pub train_steps: u32,
    pub match_overlap: u32,
    #[serde(default)]
    pub capture_mode: CaptureMode,
    /// Brush `--max-splats`. Missing in old archive JSON; Balanced default applies.
    #[serde(default = "default_max_splats")]
    pub max_splats: u32,
    /// Hard cap on motion-adaptive video keyframes. Missing in old archive JSON.
    #[serde(default = "default_max_frames")]
    pub max_frames: u32,
    /// Density (fps + max frames) or Change (quality 1–100). Missing in old archive JSON.
    #[serde(default)]
    pub extract_mode: ExtractMode,
    /// Change-mode overlap quality. 100 extracts sooner when the picture moves.
    #[serde(default = "default_extract_quality")]
    pub extract_quality: u8,
    /// Missing in old archive JSON; hydrated from capture mode in [`Self::sanitized`].
    #[serde(default)]
    pub extract: Option<ExtractKnobs>,
    #[serde(default)]
    pub colmap: Option<ColmapKnobs>,
    #[serde(default)]
    pub brush: Option<BrushKnobs>,
    #[serde(default)]
    pub viewer: Option<ViewerKnobs>,
}

fn default_jpeg_quality() -> u8 {
    95
}

fn default_max_splats() -> u32 {
    5_000_000
}

fn default_max_frames() -> u32 {
    250
}

fn default_extract_quality() -> u8 {
    55
}

impl PipelineSettings {
    pub fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::Fast => Self::named_recipe(1.0, 800, 80, 5_000, 2_000_000, 120),
            Preset::Balanced => Self::named_recipe(2.0, 1600, 95, 15_000, 5_000_000, 250),
            Preset::Quality => Self::named_recipe(4.0, 1920, 100, 30_000, 10_000_000, 500),
        }
    }

    /// Named recipes leave nested groups unset so capture mode still hydrates them.
    fn named_recipe(
        fps: f32,
        max_image_size: u32,
        jpeg_quality: u8,
        train_steps: u32,
        max_splats: u32,
        max_frames: u32,
    ) -> Self {
        Self {
            fps,
            max_image_size,
            start_seconds: 0.0,
            duration_seconds: 0.0,
            frame_format: FrameFormat::Jpg,
            jpeg_quality,
            train_steps,
            match_overlap: 15,
            capture_mode: CaptureMode::Object,
            max_splats,
            max_frames,
            extract_mode: ExtractMode::Density,
            extract_quality: default_extract_quality(),
            extract: None,
            colmap: None,
            brush: None,
            viewer: None,
        }
    }

    /// Clamps values to ranges the sidecars can survive. No capture-mode policy floors.
    pub fn sanitized(self) -> Self {
        Self {
            fps: self.fps.clamp(FPS_MIN, FPS_MAX),
            max_image_size: match self.max_image_size {
                0 => 0,
                n => n.clamp(MAX_IMAGE_SIZE_MIN, MAX_IMAGE_SIZE_MAX),
            },
            start_seconds: self.start_seconds.max(0.0),
            duration_seconds: self.duration_seconds.max(0.0),
            frame_format: self.frame_format,
            jpeg_quality: self.jpeg_quality.clamp(1, 100),
            train_steps: self.train_steps.clamp(TRAIN_STEPS_MIN, TRAIN_STEPS_MAX),
            match_overlap: self.match_overlap.clamp(MATCH_OVERLAP_MIN, MATCH_OVERLAP_MAX),
            capture_mode: self.capture_mode,
            max_splats: self.max_splats.clamp(MAX_SPLATS_MIN, MAX_SPLATS_MAX),
            max_frames: self.max_frames.clamp(MAX_FRAMES_MIN, MAX_FRAMES_MAX),
            extract_mode: self.extract_mode,
            extract_quality: self.extract_quality.clamp(1, 100),
            extract: Some(self.extract_knobs()),
            colmap: Some(self.colmap_knobs()),
            brush: Some(self.brush_knobs()),
            viewer: Some(self.viewer_knobs()),
        }
    }

    /// Keyframe scoring knobs; defaults when the nested group is missing.
    pub fn extract_knobs(self) -> ExtractKnobs {
        self.extract.unwrap_or_default().sanitized()
    }

    /// SfM knobs; capture-mode profile when the nested group is missing.
    pub fn colmap_knobs(self) -> ColmapKnobs {
        self.colmap
            .unwrap_or_else(|| ColmapKnobs::for_capture(self.capture_mode))
            .sanitized()
    }

    /// Brush knobs; CLI defaults when the nested group is missing.
    pub fn brush_knobs(self) -> BrushKnobs {
        self.brush.unwrap_or_default().sanitized()
    }

    /// Spark knobs; capture-mode profile when the nested group is missing.
    pub fn viewer_knobs(self) -> ViewerKnobs {
        self.viewer
            .unwrap_or_else(|| ViewerKnobs::for_capture(self.capture_mode))
            .sanitized()
    }

    /// Longest edge for FFmpeg scale / COLMAP SIFT / Brush. `None` keeps the source size.
    pub fn longest_edge(self) -> Option<u32> {
        (self.max_image_size > 0).then_some(self.max_image_size)
    }

    /// Brush `--max-resolution`. `None` means native (omit the flag).
    pub fn train_resolution(self) -> Option<u32> {
        let cap = self.brush_knobs().train_max_resolution;
        match self.longest_edge() {
            Some(edge) => Some(if cap == 0 { edge } else { edge.min(cap) }),
            None => (cap > 0).then_some(cap),
        }
    }
}

impl Default for PipelineSettings {
    fn default() -> Self {
        Self::from_preset(Preset::Balanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_is_heavier_than_fast() {
        let fast = PipelineSettings::from_preset(Preset::Fast);
        let quality = PipelineSettings::from_preset(Preset::Quality);
        assert!(fast.fps < quality.fps);
        assert!(fast.train_steps < quality.train_steps);
        assert!(fast.max_image_size < quality.max_image_size);
        assert!(fast.max_splats < quality.max_splats);
    }

    #[test]
    fn sanitized_clamps_out_of_range_knobs() {
        let raw = PipelineSettings {
            fps: 99.0,
            max_image_size: 12,
            start_seconds: -3.0,
            duration_seconds: -1.0,
            jpeg_quality: 0,
            train_steps: 1,
            match_overlap: 80,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        let clean = raw.sanitized();
        assert_eq!(clean.fps, FPS_MAX);
        assert_eq!(clean.max_image_size, MAX_IMAGE_SIZE_MIN);
        assert_eq!(clean.start_seconds, 0.0);
        assert_eq!(clean.duration_seconds, 0.0);
        assert_eq!(clean.jpeg_quality, 1);
        assert_eq!(clean.train_steps, 1);
        assert_eq!(clean.match_overlap, 80);
    }

    #[test]
    fn sanitized_clamps_splat_cap() {
        let low = PipelineSettings {
            max_splats: 1,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(low.sanitized().max_splats, MAX_SPLATS_MIN);
        let high = PipelineSettings {
            max_splats: 200_000_000,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(high.sanitized().max_splats, MAX_SPLATS_MAX);
    }

    #[test]
    fn quality_preset_uses_max_jpeg() {
        let quality = PipelineSettings::from_preset(Preset::Quality);
        assert_eq!(quality.frame_format, FrameFormat::Jpg);
        assert_eq!(quality.jpeg_quality, 100);
    }

    #[test]
    fn zero_max_image_size_means_native() {
        let settings = PipelineSettings {
            max_image_size: 0,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(settings.longest_edge(), None);
        assert_eq!(settings.train_resolution(), Some(1920));
    }

    #[test]
    fn from_preset_uses_object_mode() {
        assert_eq!(
            PipelineSettings::from_preset(Preset::Fast).capture_mode,
            CaptureMode::Object
        );
    }

    #[test]
    fn sanitized_preserves_capture_mode() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = CaptureMode::Room;
        assert_eq!(settings.sanitized().capture_mode, CaptureMode::Room);
    }

    #[test]
    fn missing_capture_mode_defaults_to_object() {
        let json = r#"{
            "fps": 1.0,
            "maxImageSize": 800,
            "startSeconds": 0.0,
            "durationSeconds": 0.0,
            "trainSteps": 5000,
            "matchOverlap": 15
        }"#;
        let settings: PipelineSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.capture_mode, CaptureMode::Object);
        assert_eq!(settings.max_splats, 5_000_000);
        assert_eq!(settings.max_frames, 250);
        assert_eq!(settings.extract_mode, ExtractMode::Density);
        assert_eq!(settings.extract_quality, 55);
    }

    #[test]
    fn sanitized_clamps_extract_quality() {
        let low = PipelineSettings {
            extract_quality: 0,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(low.sanitized().extract_quality, 1);
        let high = PipelineSettings {
            extract_quality: 200,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(high.sanitized().extract_quality, 100);
    }

    #[test]
    fn presets_stay_on_density_extract() {
        assert_eq!(
            PipelineSettings::from_preset(Preset::Fast).extract_mode,
            ExtractMode::Density
        );
        assert_eq!(
            PipelineSettings::from_preset(Preset::Quality).extract_mode,
            ExtractMode::Density
        );
    }

    #[test]
    fn presets_raise_frame_budget_with_quality() {
        assert_eq!(PipelineSettings::from_preset(Preset::Fast).max_frames, 120);
        assert_eq!(
            PipelineSettings::from_preset(Preset::Balanced).max_frames,
            250
        );
        assert_eq!(
            PipelineSettings::from_preset(Preset::Quality).max_frames,
            500
        );
    }

    #[test]
    fn max_frames_cap_is_higher_outdoors() {
        assert_eq!(CaptureMode::Object.max_frames_cap(), 800);
        assert_eq!(CaptureMode::Room.max_frames_cap(), 800);
        assert_eq!(CaptureMode::Outdoor.max_frames_cap(), 10_000);
    }

    #[test]
    fn sanitized_does_not_apply_capture_frame_cap() {
        let low = PipelineSettings {
            max_frames: 1,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(low.sanitized().max_frames, 1);
        let high = PipelineSettings {
            max_frames: 9_000,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(high.sanitized().max_frames, 9_000);
        let mut outdoor = PipelineSettings::from_preset(Preset::Fast);
        outdoor.capture_mode = CaptureMode::Outdoor;
        outdoor.max_frames = 20_000;
        assert_eq!(outdoor.sanitized().max_frames, 20_000);
        outdoor.max_frames = 80_000;
        assert_eq!(outdoor.sanitized().max_frames, MAX_FRAMES_MAX);
    }

    #[test]
    fn legacy_room_json_hydrates_room_colmap() {
        let json = r#"{
            "fps": 1.0,
            "maxImageSize": 800,
            "startSeconds": 0.0,
            "durationSeconds": 0.0,
            "trainSteps": 5000,
            "matchOverlap": 15,
            "captureMode": "room"
        }"#;
        let settings: PipelineSettings = serde_json::from_str(json).unwrap();
        assert!(settings.colmap.is_none());
        let colmap = settings.sanitized().colmap.expect("hydrated");
        assert_eq!(colmap.mapper, crate::colmap_knobs::ColmapMapper::Global);
        assert_eq!(colmap.min_overlap_floor, 20);
        assert_eq!(colmap.matcher, crate::colmap_knobs::ColmapMatcher::Exhaustive);
        assert_eq!(colmap.sift_backend, crate::colmap_knobs::SiftBackend::Cpu);
    }

    #[test]
    fn explicit_zero_overlap_floor_is_kept() {
        let mut settings = PipelineSettings::from_preset(Preset::Fast);
        settings.capture_mode = CaptureMode::Room;
        let mut colmap = ColmapKnobs::for_capture(CaptureMode::Room);
        colmap.min_overlap_floor = 0;
        settings.colmap = Some(colmap);
        assert_eq!(settings.sanitized().colmap_knobs().min_overlap_floor, 0);
    }

    #[test]
    fn presets_raise_splat_budget_with_quality() {
        assert_eq!(
            PipelineSettings::from_preset(Preset::Fast).max_splats,
            2_000_000
        );
        assert_eq!(
            PipelineSettings::from_preset(Preset::Balanced).max_splats,
            5_000_000
        );
        assert_eq!(
            PipelineSettings::from_preset(Preset::Quality).max_splats,
            10_000_000
        );
    }
}
