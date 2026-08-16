//! Spark viewer knobs. Named presets fill these from capture mode.

use serde::{Deserialize, Serialize};

use crate::capture_mode::CaptureMode;

pub const LOD_ABOVE: u32 = 100_000;
pub const WEBVIEW_LOD_SPLAT_COUNT: u32 = 1_500_000;
pub const SPARK_MIN_ALPHA: f64 = 0.5 / 255.0;
pub const ROOM_MIN_ALPHA: f64 = 2.0 / 255.0;
pub const MIN_PIXEL_RADIUS: f64 = 1.0;
pub const MIN_SORT_INTERVAL_MS: u32 = 8;
pub const DEFAULT_FOV: f64 = 60.0;

/// Runtime Spark / HTML-export knobs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerKnobs {
    pub lod_above: u32,
    pub lod_splat_scale: f64,
    pub lod_render_scale: f64,
    pub behind_foveate: f64,
    pub cone_foveate: f64,
    pub webview_lod_splat_count: u32,
    pub min_alpha: f64,
    pub max_std_dev: f64,
    /// Spark and the UI use `clipXY`; serde camelCase would otherwise emit `clipXy`.
    #[serde(rename = "clipXY", alias = "clipXy")]
    pub clip_xy: f64,
    pub min_pixel_radius: f64,
    pub min_sort_interval_ms: u32,
    pub fov: f64,
    pub move_speed: f64,
    pub far_multiplier: f64,
}

impl ViewerKnobs {
    /// Today's capture-mode Spark profile, used when the nested group is absent.
    pub fn for_capture(mode: CaptureMode) -> Self {
        match mode {
            CaptureMode::Room => Self {
                lod_above: LOD_ABOVE,
                lod_splat_scale: 0.7,
                lod_render_scale: 2.0,
                behind_foveate: 0.1,
                cone_foveate: 0.4,
                webview_lod_splat_count: WEBVIEW_LOD_SPLAT_COUNT,
                min_alpha: ROOM_MIN_ALPHA,
                max_std_dev: 5.0_f64.sqrt(),
                clip_xy: 1.2,
                min_pixel_radius: MIN_PIXEL_RADIUS,
                min_sort_interval_ms: MIN_SORT_INTERVAL_MS,
                fov: DEFAULT_FOV,
                move_speed: 0.5,
                far_multiplier: 40.0,
            },
            CaptureMode::Outdoor => Self {
                lod_above: LOD_ABOVE,
                lod_splat_scale: 0.5,
                lod_render_scale: 3.0,
                behind_foveate: 0.1,
                cone_foveate: 0.3,
                webview_lod_splat_count: WEBVIEW_LOD_SPLAT_COUNT,
                min_alpha: SPARK_MIN_ALPHA,
                max_std_dev: 4.0_f64.sqrt(),
                clip_xy: 1.1,
                min_pixel_radius: MIN_PIXEL_RADIUS,
                min_sort_interval_ms: MIN_SORT_INTERVAL_MS,
                fov: DEFAULT_FOV,
                move_speed: 2.0,
                far_multiplier: 80.0,
            },
            CaptureMode::Object => Self {
                lod_above: LOD_ABOVE,
                lod_splat_scale: 1.0,
                lod_render_scale: 1.5,
                behind_foveate: 0.2,
                cone_foveate: 0.5,
                webview_lod_splat_count: WEBVIEW_LOD_SPLAT_COUNT,
                min_alpha: SPARK_MIN_ALPHA,
                max_std_dev: 5.0_f64.sqrt(),
                clip_xy: 1.2,
                min_pixel_radius: MIN_PIXEL_RADIUS,
                min_sort_interval_ms: MIN_SORT_INTERVAL_MS,
                fov: DEFAULT_FOV,
                move_speed: 0.8,
                far_multiplier: 40.0,
            },
        }
    }

    /// Clamps viewer knobs to ranges Spark can survive.
    pub fn sanitized(self) -> Self {
        Self {
            lod_above: self.lod_above.min(1_000_000_000),
            lod_splat_scale: self.lod_splat_scale.max(0.01),
            lod_render_scale: self.lod_render_scale.max(0.01),
            behind_foveate: self.behind_foveate.clamp(0.0, 1.0),
            cone_foveate: self.cone_foveate.clamp(0.0, 1.0),
            webview_lod_splat_count: self.webview_lod_splat_count.clamp(10_000, 20_000_000),
            min_alpha: self.min_alpha.clamp(0.0, 1.0),
            max_std_dev: self.max_std_dev.max(0.01),
            clip_xy: self.clip_xy.max(0.1),
            min_pixel_radius: self.min_pixel_radius.max(0.0),
            min_sort_interval_ms: self.min_sort_interval_ms.min(1000),
            fov: self.fov.clamp(10.0, 120.0),
            move_speed: self.move_speed.max(0.01),
            far_multiplier: self.far_multiplier.max(1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frontend and Spark spell the acronym `clipXY`; serde camelCase would emit `clipXy`.
    const TS_VIEWER: &str = r#"{
        "lodAbove": 100000,
        "lodSplatScale": 1.0,
        "lodRenderScale": 1.5,
        "behindFoveate": 0.2,
        "coneFoveate": 0.5,
        "webviewLodSplatCount": 1500000,
        "minAlpha": 0.002,
        "maxStdDev": 2.236,
        "clipXY": 1.2,
        "minPixelRadius": 1.0,
        "minSortIntervalMs": 8,
        "fov": 60.0,
        "moveSpeed": 0.8,
        "farMultiplier": 40.0
    }"#;

    #[test]
    fn deserializes_spark_clip_xy_acronym() {
        let knobs: ViewerKnobs = serde_json::from_str(TS_VIEWER).unwrap();
        assert_eq!(knobs.clip_xy, 1.2);
    }

    #[test]
    fn serializes_spark_clip_xy_acronym() {
        let json = serde_json::to_string(&ViewerKnobs::for_capture(CaptureMode::Object)).unwrap();
        assert!(json.contains("\"clipXY\""), "{json}");
        assert!(!json.contains("\"clipXy\""), "{json}");
    }

    #[test]
    fn accepts_legacy_serde_camel_case_clip_xy() {
        let json = TS_VIEWER.replace("clipXY", "clipXy");
        let knobs: ViewerKnobs = serde_json::from_str(&json).unwrap();
        assert_eq!(knobs.clip_xy, 1.2);
    }
}
