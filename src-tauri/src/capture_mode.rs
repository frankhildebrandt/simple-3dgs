//! Capture coverage. Named presets still fill COLMAP/viewer knobs from this.

use serde::{Deserialize, Serialize};

/// What the capture covers. Named recipes fill SfM and viewer knobs from this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMode {
    #[default]
    Object,
    Room,
    Outdoor,
}

impl CaptureMode {
    /// Named-preset video keyframe cap. Custom ignores this and uses `max_frames`.
    pub fn max_frames_cap(self) -> u32 {
        match self {
            Self::Object | Self::Room => 800,
            Self::Outdoor => 10_000,
        }
    }
}
