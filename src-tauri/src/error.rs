//! User-facing pipeline errors. Sidecar failures include a recovery hint.

use std::io;
use std::path::Path;

use crate::settings::CaptureMode;

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("{0}")]
    Message(String),
    #[error("Cancelled")]
    Cancelled,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("{tool} failed (exit {code}). {hint}")]
    Sidecar {
        tool: String,
        code: i32,
        hint: String,
    },
}

impl PipelineError {
    pub fn message(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }

    /// Builds a COLMAP error; `log` is the sidecar tail so flag/dyld failures are not blamed on capture.
    pub fn colmap_failed_with(code: i32, log: &str, mode: CaptureMode) -> Self {
        let last = last_useful_line(log);
        let hint = if last.contains("unrecognised option") || last.contains("Failed to parse") {
            format!("COLMAP rejected a command flag. {last}")
        } else if last.contains("Library not loaded") || last.contains("dyld") {
            format!("COLMAP could not load a library. {last}")
        } else if last.is_empty() {
            pose_hint(mode).into()
        } else {
            format!("{} ({last})", pose_hint(mode))
        };
        Self::Sidecar {
            tool: "COLMAP".into(),
            code,
            hint,
        }
    }

    pub fn ffmpeg_failed(code: i32) -> Self {
        Self::Sidecar {
            tool: "FFmpeg".into(),
            code,
            hint: "Could not read the video. Try MP4/MOV, or replace the bundled ffmpeg sidecar."
                .into(),
        }
    }

    pub fn brush_failed(code: i32) -> Self {
        Self::brush_failed_with(code, "")
    }

    /// Uses `log` so a Burn fusion panic is not reported as a clean Metal/RAM failure.
    pub fn brush_failed_with(code: i32, log: &str) -> Self {
        let hint = if crate::train_log::trainer_panicked(log) {
            "Brush's GPU trainer panicked in Burn fusion. The CLI still reports Done training. Retry the run; if it keeps failing, lower max splats or train resolution.".into()
        } else {
            "Training stopped early. Check GPU/Metal availability and free memory (16 GB RAM minimum).".into()
        };
        Self::Sidecar {
            tool: "Brush".into(),
            code,
            hint,
        }
    }

    /// Attaches `path` when macOS TCC denies Documents/Desktop/Downloads.
    pub fn from_io_path(err: io::Error, path: &Path) -> Self {
        if err.kind() == io::ErrorKind::PermissionDenied {
            Self::Message(format!(
                "macOS blocked access to {}. Allow Simple 3DGS in System Settings → Privacy & Security → Files and Folders, or choose a different folder.",
                path.display()
            ))
        } else {
            Self::Io(err)
        }
    }
}

/// Capture-mode-specific advice when SfM cannot recover poses.
fn pose_hint(mode: CaptureMode) -> &'static str {
    match mode {
        CaptureMode::Object => {
            "Reconstruction could not estimate camera poses. Film a slower orbit with more overlap, less motion blur, and even lighting."
        }
        CaptureMode::Room => {
            "Reconstruction could not estimate camera poses. Walk slowly along the walls with more overlap, even lighting, and textured surfaces. Avoid empty walls, mirrors, and exposure jumps."
        }
        CaptureMode::Outdoor => {
            "Reconstruction could not estimate camera poses. Walk slowly with more overlap, keep the camera slightly tilted down to skip empty sky, and avoid wind-blown vegetation."
        }
    }
}

fn last_useful_line(log: &str) -> String {
    log.lines()
        .map(str::trim)
        .rev()
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::PipelineError;
    use crate::settings::CaptureMode;
    use std::io;

    #[test]
    fn colmap_flag_errors_are_not_blamed_on_capture() {
        let err = PipelineError::colmap_failed_with(
            1,
            "E... Failed to parse options - unrecognised option '--SiftExtraction.use_gpu'.",
            CaptureMode::Object,
        );
        let msg = err.to_string();
        assert!(msg.contains("command flag"));
        assert!(msg.contains("SiftExtraction"));
        assert!(!msg.contains("slower orbit"));
    }

    #[test]
    fn room_pose_hint_is_not_an_orbit() {
        let msg = PipelineError::colmap_failed_with(1, "", CaptureMode::Room).to_string();
        assert!(msg.contains("walls"));
        assert!(!msg.to_lowercase().contains("orbit"));
    }

    #[test]
    fn outdoor_pose_hint_is_not_an_orbit() {
        let msg = PipelineError::colmap_failed_with(1, "", CaptureMode::Outdoor).to_string();
        assert!(msg.contains("sky"));
        assert!(!msg.to_lowercase().contains("orbit"));
    }

    #[test]
    fn permission_denied_names_the_path_and_privacy_settings() {
        let err = PipelineError::from_io_path(
            io::Error::from_raw_os_error(1),
            std::path::Path::new("/Users/frank/Documents/Simple 3DGS/archive"),
        );
        let msg = err.to_string();
        assert!(msg.contains("Documents/Simple 3DGS/archive"));
        assert!(msg.contains("Files and Folders"));
        assert!(!msg.contains("I/O error"));
    }

    #[test]
    fn brush_fusion_panic_is_not_blamed_on_ram() {
        let log = "thread 'cli-trainer' panicked at burn-fusion/src/client.rs:200:14:\nOrdering is bigger than operations";
        let msg = PipelineError::brush_failed_with(0, log).to_string();
        assert!(msg.contains("Burn fusion"));
        assert!(!msg.contains("16 GB"));
        let generic = PipelineError::brush_failed(1).to_string();
        assert!(generic.contains("16 GB"));
    }

    #[test]
    fn other_io_errors_keep_the_generic_prefix() {
        let err = PipelineError::from_io_path(
            io::Error::from_raw_os_error(2),
            std::path::Path::new("/missing"),
        );
        assert!(err.to_string().starts_with("I/O error:"));
    }
}
