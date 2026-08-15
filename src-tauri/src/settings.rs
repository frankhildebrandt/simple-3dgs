//! Explicit pipeline knobs. Presets only fill these; the pipeline never reads a preset name.

use serde::{Deserialize, Serialize};

use crate::preset::Preset;

/// What the capture covers. Drives COLMAP matching, mapper flags, and capture hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Object,
    Room,
    Outdoor,
}

impl CaptureMode {
    /// Hard video keyframe cap. Outdoor paths can run much longer than orbits or rooms.
    pub fn max_frames_cap(self) -> u32 {
        match self {
            Self::Object | Self::Room => 800,
            Self::Outdoor => 10_000,
        }
    }
}

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
            Preset::Fast => Self {
                fps: 1.0,
                max_image_size: 800,
                start_seconds: 0.0,
                duration_seconds: 0.0,
                frame_format: FrameFormat::Jpg,
                jpeg_quality: 80,
                train_steps: 5_000,
                match_overlap: 15,
                capture_mode: CaptureMode::Object,
                max_splats: 2_000_000,
                max_frames: 120,
                extract_mode: ExtractMode::Density,
                extract_quality: default_extract_quality(),
            },
            Preset::Balanced => Self {
                fps: 2.0,
                max_image_size: 1600,
                start_seconds: 0.0,
                duration_seconds: 0.0,
                frame_format: FrameFormat::Jpg,
                jpeg_quality: 95,
                train_steps: 15_000,
                match_overlap: 15,
                capture_mode: CaptureMode::Object,
                max_splats: 5_000_000,
                max_frames: 250,
                extract_mode: ExtractMode::Density,
                extract_quality: default_extract_quality(),
            },
            Preset::Quality => Self {
                fps: 4.0,
                max_image_size: 1920,
                start_seconds: 0.0,
                duration_seconds: 0.0,
                frame_format: FrameFormat::Jpg,
                jpeg_quality: 100,
                train_steps: 30_000,
                match_overlap: 15,
                capture_mode: CaptureMode::Object,
                max_splats: 10_000_000,
                max_frames: 500,
                extract_mode: ExtractMode::Density,
                extract_quality: default_extract_quality(),
            },
        }
    }

    /// Clamps values to ranges the sidecars can survive.
    pub fn sanitized(self) -> Self {
        Self {
            fps: self.fps.clamp(0.25, 12.0),
            max_image_size: match self.max_image_size {
                0 => 0,
                n => n.clamp(320, 8192),
            },
            start_seconds: self.start_seconds.max(0.0),
            duration_seconds: self.duration_seconds.max(0.0),
            frame_format: self.frame_format,
            jpeg_quality: self.jpeg_quality.clamp(1, 100),
            train_steps: self.train_steps.clamp(100, 100_000),
            match_overlap: self.match_overlap.clamp(2, 50),
            capture_mode: self.capture_mode,
            max_splats: self.max_splats.clamp(100_000, 20_000_000),
            max_frames: self.max_frames.clamp(8, self.capture_mode.max_frames_cap()),
            extract_mode: self.extract_mode,
            extract_quality: self.extract_quality.clamp(1, 100),
        }
    }

    /// Longest edge for FFmpeg scale / COLMAP SIFT / Brush. `None` keeps the source size.
    pub fn longest_edge(self) -> Option<u32> {
        (self.max_image_size >= 320).then_some(self.max_image_size)
    }

    /// Brush `--max-resolution`. Native extract still needs a cap.
    pub fn train_resolution(self) -> u32 {
        self.longest_edge().unwrap_or(1920)
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
        assert_eq!(clean.fps, 12.0);
        assert_eq!(clean.max_image_size, 320);
        assert_eq!(clean.start_seconds, 0.0);
        assert_eq!(clean.duration_seconds, 0.0);
        assert_eq!(clean.jpeg_quality, 1);
        assert_eq!(clean.train_steps, 100);
        assert_eq!(clean.match_overlap, 50);
    }

    #[test]
    fn sanitized_clamps_splat_cap() {
        let low = PipelineSettings {
            max_splats: 1,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(low.sanitized().max_splats, 100_000);
        let high = PipelineSettings {
            max_splats: 99_000_000,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(high.sanitized().max_splats, 20_000_000);
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
        assert_eq!(settings.train_resolution(), 1920);
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
    fn sanitized_clamps_max_frames() {
        let low = PipelineSettings {
            max_frames: 1,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(low.sanitized().max_frames, 8);
        let high = PipelineSettings {
            max_frames: 9_000,
            ..PipelineSettings::from_preset(Preset::Fast)
        };
        assert_eq!(high.sanitized().max_frames, 800);
        let mut room = PipelineSettings::from_preset(Preset::Fast);
        room.capture_mode = CaptureMode::Room;
        room.max_frames = 9_000;
        assert_eq!(room.sanitized().max_frames, 800);
        let mut outdoor = PipelineSettings::from_preset(Preset::Fast);
        outdoor.capture_mode = CaptureMode::Outdoor;
        outdoor.max_frames = 10_000;
        assert_eq!(outdoor.sanitized().max_frames, 10_000);
        outdoor.max_frames = 20_000;
        assert_eq!(outdoor.sanitized().max_frames, 10_000);
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
