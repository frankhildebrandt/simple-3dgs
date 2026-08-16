//! Video keyframe scoring knobs. Named presets use [`ExtractKnobs::default`].

use serde::{Deserialize, Serialize};

/// Laplacian / MAD / candidate-rate knobs for motion-adaptive extract.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractKnobs {
    pub blur_floor: f32,
    pub moderate_mad: f32,
    pub candidate_min_fps: f32,
    pub candidate_max_fps: f32,
    pub change_candidate_max_fps: f32,
    pub change_mad_sparse: f32,
    pub change_mad_dense: f32,
    pub threshold_relax_steps: u32,
    pub min_frames: u32,
}

impl Default for ExtractKnobs {
    fn default() -> Self {
        Self {
            blur_floor: 15.0,
            moderate_mad: 8.0,
            candidate_min_fps: 8.0,
            candidate_max_fps: 12.0,
            change_candidate_max_fps: 24.0,
            change_mad_sparse: 96.0,
            change_mad_dense: 4.0,
            threshold_relax_steps: 6,
            min_frames: 8,
        }
    }
}

impl ExtractKnobs {
    /// Clamps extract internals to ranges the selector can survive.
    pub fn sanitized(self) -> Self {
        Self {
            blur_floor: self.blur_floor.max(0.0),
            moderate_mad: self.moderate_mad.max(0.0),
            candidate_min_fps: self.candidate_min_fps.clamp(0.25, 60.0),
            candidate_max_fps: self.candidate_max_fps.clamp(0.25, 60.0),
            change_candidate_max_fps: self.change_candidate_max_fps.clamp(0.25, 60.0),
            change_mad_sparse: self.change_mad_sparse.max(0.0),
            change_mad_dense: self.change_mad_dense.max(0.0),
            threshold_relax_steps: self.threshold_relax_steps.min(32),
            min_frames: self.min_frames.clamp(1, 50_000),
        }
    }
}
