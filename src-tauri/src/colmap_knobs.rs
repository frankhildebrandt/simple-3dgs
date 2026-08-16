//! COLMAP SfM knobs. Named presets fill these from [`crate::capture_mode::CaptureMode`].

use serde::{Deserialize, Serialize};

use crate::capture_mode::CaptureMode;

/// COLMAP camera model string passed to `ImageReader.camera_model`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CameraModel {
    #[default]
    #[serde(rename = "SIMPLE_RADIAL")]
    SimpleRadial,
    #[serde(rename = "SIMPLE_PINHOLE")]
    SimplePinhole,
    #[serde(rename = "PINHOLE")]
    Pinhole,
    #[serde(rename = "RADIAL")]
    Radial,
    #[serde(rename = "OPENCV")]
    OpenCv,
}

impl CameraModel {
    pub fn as_colmap(self) -> &'static str {
        match self {
            Self::SimpleRadial => "SIMPLE_RADIAL",
            Self::SimplePinhole => "SIMPLE_PINHOLE",
            Self::Pinhole => "PINHOLE",
            Self::Radial => "RADIAL",
            Self::OpenCv => "OPENCV",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColmapMatcher {
    #[default]
    Sequential,
    Exhaustive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColmapMapper {
    #[default]
    Incremental,
    Global,
}

/// Explicit SfM knobs. Missing archive JSON hydrates via [`ColmapKnobs::for_capture`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColmapKnobs {
    pub camera_model: CameraModel,
    pub single_camera: bool,
    pub matcher: ColmapMatcher,
    pub exhaustive_frame_limit: u32,
    pub quadratic_overlap: bool,
    pub min_overlap_floor: u32,
    pub mapper: ColmapMapper,
    pub min_model_size: u32,
    pub init_min_tri_angle: u32,
}

impl ColmapKnobs {
    /// Today's capture-mode heuristics, used when the nested group is absent.
    pub fn for_capture(mode: CaptureMode) -> Self {
        match mode {
            CaptureMode::Object => Self {
                camera_model: CameraModel::SimpleRadial,
                single_camera: true,
                matcher: ColmapMatcher::Sequential,
                exhaustive_frame_limit: 250,
                quadratic_overlap: false,
                min_overlap_floor: 0,
                mapper: ColmapMapper::Incremental,
                min_model_size: 6,
                init_min_tri_angle: 0,
            },
            CaptureMode::Room => Self {
                camera_model: CameraModel::SimpleRadial,
                single_camera: true,
                matcher: ColmapMatcher::Exhaustive,
                exhaustive_frame_limit: 250,
                quadratic_overlap: true,
                min_overlap_floor: 20,
                mapper: ColmapMapper::Global,
                min_model_size: 10,
                init_min_tri_angle: 0,
            },
            CaptureMode::Outdoor => Self {
                camera_model: CameraModel::SimpleRadial,
                single_camera: true,
                matcher: ColmapMatcher::Sequential,
                exhaustive_frame_limit: 250,
                quadratic_overlap: false,
                min_overlap_floor: 20,
                mapper: ColmapMapper::Incremental,
                min_model_size: 10,
                init_min_tri_angle: 8,
            },
        }
    }

    /// Clamps SfM knobs to ranges COLMAP can survive.
    pub fn sanitized(self) -> Self {
        Self {
            camera_model: self.camera_model,
            single_camera: self.single_camera,
            matcher: self.matcher,
            exhaustive_frame_limit: self.exhaustive_frame_limit.clamp(2, 50_000),
            quadratic_overlap: self.quadratic_overlap,
            min_overlap_floor: self.min_overlap_floor.min(200),
            mapper: self.mapper,
            min_model_size: self.min_model_size.clamp(2, 10_000),
            init_min_tri_angle: self.init_min_tri_angle.min(90),
        }
    }
}
