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
        } else if metal_sift_unavailable(log) {
            format!(
                "COLMAP GPU SIFT is unavailable. Homebrew COLMAP crashes on Qt/OpenGL; Metal needs sift.metallib from scripts/fetch-sidecars.sh --colmap-metal. Or switch SIFT to CPU in Expert settings. {last}"
            )
        } else if faiss_matcher_aborted(log) {
            format!(
                "COLMAP CPU matching crashed in FAISS. Retry; matching now uses Eigen brute-force instead of FAISS. Switching SIFT to CPU is not enough. {last}"
            )
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
        Self::ffmpeg_failed_with(code, "")
    }

    /// Uses `log` so a select-filter parse abort is not reported as a bad video file.
    pub fn ffmpeg_failed_with(code: i32, log: &str) -> Self {
        let hint = if ffmpeg_select_expr_failed(log) {
            "FFmpeg could not parse the keyframe select filter. Retry the run; if it keeps failing, lower max frames.".into()
        } else {
            "Could not read the video. Try MP4/MOV, or replace the bundled ffmpeg sidecar.".into()
        };
        Self::Sidecar {
            tool: "FFmpeg".into(),
            code,
            hint,
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
        .find(|line| !line.is_empty() && !is_crash_noise(line))
        .unwrap_or("")
        .to_string()
}

fn is_crash_noise(line: &str) -> bool {
    line.starts_with('@') || line.starts_with("***") || line.starts_with("PC:")
}

/// True when FFmpeg's select expr parser rejected a too-deep `eq+eq+…` graph.
fn ffmpeg_select_expr_failed(log: &str) -> bool {
    log.contains("Error while parsing expression")
        || (log.contains("Parsed_select") && log.contains("Cannot allocate memory"))
}

/// True when CPU matching aborted (FAISS IVF k>0, or OpenMP/OpenBLAS SIGSEGV).
fn faiss_matcher_aborted(log: &str) -> bool {
    let lower = log.to_lowercase();
    let matching = lower.contains("feature matching")
        || lower.contains("feature matcher")
        || lower.contains("processing block");
    if !matching {
        return false;
    }
    lower.contains("faissexception")
        || lower.contains("faiss::")
        || lower.contains("sigsegv")
        || lower.contains("sigabrt")
}

/// True when GPU SIFT could not start: missing Metal sidecar or Homebrew Qt/OpenGL abort.
fn metal_sift_unavailable(log: &str) -> bool {
    let lower = log.to_lowercase();
    lower.contains("failed to initialize metal")
        || lower.contains("failed to load metal library")
        || lower.contains("siftmetal")
        || lower.contains("sift.metallib")
        || lower.contains("qt.qpa.plugin")
        || lower.contains("qt platform plugin")
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
    fn metal_sift_failure_is_not_blamed_on_capture() {
        let err = PipelineError::colmap_failed_with(
            1,
            "E... Failed to initialize Metal SIFT extractor",
            CaptureMode::Object,
        );
        let msg = err.to_string();
        assert!(msg.contains("GPU SIFT"));
        assert!(msg.contains("--colmap-metal"));
        assert!(!msg.contains("slower orbit"));
    }

    #[test]
    fn homebrew_qt_cocoa_abort_is_not_blamed_on_capture() {
        let log = r#"qt.qpa.plugin: Could not find the Qt platform plugin "cocoa" in ""
This application failed to start because no Qt platform plugin could be initialized. Reinstalling the application may fix this problem.
*** SIGABRT (@0x18a0e25e8) received by PID 99998
@        0x189d5fe00 (unknown)"#;
        let err = PipelineError::colmap_failed_with(1, log, CaptureMode::Object);
        let msg = err.to_string();
        assert!(msg.contains("GPU SIFT"));
        assert!(msg.contains("Qt/OpenGL") || msg.contains("cocoa") || msg.contains("platform plugin"));
        assert!(msg.contains("--colmap-metal"));
        assert!(!msg.contains("slower orbit"));
        assert!(!msg.contains("0x189d5fe00"));
    }

    #[test]
    fn ffmpeg_select_parse_failure_is_not_a_bad_file() {
        let log = r#"[Parsed_select_1 @ 0x7a8c08d80] Error while parsing expression 'eq(n,1)+eq(n,6)+eq(n,11)'
[AVFilterGraph @ 0x7a9008680] Error initializing filters
Error opening output files: Cannot allocate memory"#;
        let err = PipelineError::ffmpeg_failed_with(1, log);
        let msg = err.to_string();
        assert!(msg.contains("select filter"));
        assert!(!msg.contains("Try MP4/MOV"));
    }

    #[test]
    fn faiss_ivf_matcher_abort_is_not_blamed_on_capture() {
        let log = r#"Creating SIFT CPU feature matcher
Generating exhaustive image pairs...
Processing block [1/5, 1/5]
libc++abi: terminating due to uncaught exception of type faiss::FaissException: Error in virtual void faiss::IndexIVF::search(...) at IndexIVF.cpp:303: Error: 'k > 0' failed
*** SIGABRT (@0x18a0e25e8) received by PID 14236
@        0x18a02f3b8 (unknown)"#;
        let err = PipelineError::colmap_failed_with(1, log, CaptureMode::Room);
        let msg = err.to_string();
        assert!(msg.contains("FAISS"));
        assert!(msg.contains("brute-force") || msg.contains("Eigen"));
        assert!(!msg.contains("Walk slowly"));
        assert!(!msg.contains("0x18a02f3b8"));
    }

    #[test]
    fn matcher_sigsegv_is_not_blamed_on_capture() {
        let log = r#"Creating SIFT CPU feature matcher
Generating exhaustive image pairs...
Processing block [1/5, 1/5]
*** SIGSEGV (@0xffffffffbfc00807) received by PID 22936
@        0x18a02f3b8 (unknown)"#;
        let err = PipelineError::colmap_failed_with(1, log, CaptureMode::Room);
        let msg = err.to_string();
        assert!(msg.contains("FAISS") || msg.contains("OpenMP"));
        assert!(msg.contains("brute-force") || msg.contains("Eigen"));
        assert!(!msg.contains("Walk slowly"));
        assert!(!msg.contains("0x18a02f3b8"));
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
