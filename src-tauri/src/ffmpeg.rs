//! FFmpeg frame extraction command construction.

use std::path::Path;

use crate::settings::{FrameFormat, PipelineSettings};
use crate::sidecar::{path_arg, CommandSpec};

/// Builds the FFmpeg `-vf` filter for extract rate and optional longest-edge scale.
pub fn video_filter(fps: f32, max_width: Option<u32>) -> String {
    match max_width {
        Some(width) => format!("fps={fps},scale='min(iw,{width})':-2"),
        None => format!("fps={fps}"),
    }
}

/// Maps UI JPEG quality 1–100 (best) onto FFmpeg `-q:v` 2–31 (lower is better).
pub fn jpeg_q_scale(quality: u8) -> u8 {
    let quality = u16::from(quality.clamp(1, 100));
    (2 + ((100 - quality) * 29) / 99) as u8
}

/// FFmpeg command that writes numbered stills into `frames_dir`.
pub fn extract_spec(video: &Path, frames_dir: &Path, settings: PipelineSettings) -> CommandSpec {
    extract_spec_with_hwaccel(video, frames_dir, settings, true)
}

pub fn extract_spec_with_hwaccel(
    video: &Path,
    frames_dir: &Path,
    settings: PipelineSettings,
    hwaccel: bool,
) -> CommandSpec {
    let settings = settings.sanitized();
    let mut args = Vec::new();
    args.push("-y".into());
    if hwaccel {
        args.push("-hwaccel".into());
        args.push("videotoolbox".into());
    }
    if settings.start_seconds > 0.0 {
        args.push("-ss".into());
        args.push(format_seconds(settings.start_seconds));
    }
    args.push("-i".into());
    args.push(path_arg(video));
    if settings.duration_seconds > 0.0 {
        args.push("-t".into());
        args.push(format_seconds(settings.duration_seconds));
    }
    args.push("-vf".into());
    args.push(video_filter(settings.fps, settings.longest_edge()));
    match settings.frame_format {
        FrameFormat::Jpg => {
            args.push("-q:v".into());
            args.push(jpeg_q_scale(settings.jpeg_quality).to_string());
        }
        FrameFormat::Png => {
            args.push("-pix_fmt".into());
            args.push("rgb24".into());
        }
    }
    let name = format!("frame_%05d.{}", settings.frame_format.extension());
    args.push(path_arg(&frames_dir.join(name)));
    CommandSpec::new("ffmpeg", args)
}

/// Dumps container tags to stdout as ffmetadata. Stderr still carries QuickTime keys.
pub fn ffmetadata_spec(video: &Path) -> CommandSpec {
    CommandSpec::new(
        "ffmpeg",
        vec![
            "-hide_banner".into(),
            "-i".into(),
            path_arg(video),
            "-f".into(),
            "ffmetadata".into(),
            "-".into(),
        ],
    )
}

fn format_seconds(value: f32) -> String {
    format!("{value:.3}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;
    use crate::settings::FrameFormat;
    use std::path::Path;

    fn balanced() -> PipelineSettings {
        PipelineSettings::from_preset(Preset::Balanced)
    }

    #[test]
    fn filter_includes_fps_and_optional_scale() {
        assert_eq!(video_filter(2.0, None), "fps=2");
        let scaled = video_filter(1.0, Some(800));
        assert!(scaled.contains("fps=1"));
        assert!(scaled.contains("800"));
        assert!(scaled.contains("scale="));
    }

    #[test]
    fn extract_uses_videotoolbox_and_jpg_pattern() {
        let spec = extract_spec(
            Path::new("/tmp/in.mp4"),
            Path::new("/tmp/frames"),
            balanced(),
        );
        assert_eq!(spec.sidecar, "ffmpeg");
        assert!(spec.args.contains(&"-hwaccel".into()));
        assert!(spec.args.contains(&"videotoolbox".into()));
        assert_eq!(spec.args[spec.args.len() - 1], "/tmp/frames/frame_%05d.jpg");
        assert!(spec.args.windows(2).any(|w| w[0] == "-q:v" && w[1] == "3"));
        assert!(!spec.args.contains(&"-ss".into()));
        assert!(!spec.args.contains(&"-t".into()));
    }

    #[test]
    fn jpeg_quality_100_is_qscale_2() {
        assert_eq!(jpeg_q_scale(100), 2);
        assert_eq!(jpeg_q_scale(1), 31);
    }

    #[test]
    fn png_export_skips_jpeg_qscale_and_uses_rgb() {
        let mut settings = balanced();
        settings.frame_format = FrameFormat::Png;
        let spec = extract_spec(Path::new("clip.mp4"), Path::new("frames"), settings);
        assert_eq!(
            spec.args.last().map(String::as_str),
            Some("frames/frame_%05d.png")
        );
        assert!(!spec.args.contains(&"-q:v".into()));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "-pix_fmt" && w[1] == "rgb24"));
    }

    #[test]
    fn software_decode_omits_hwaccel() {
        let spec = extract_spec_with_hwaccel(
            Path::new("clip.mov"),
            Path::new("frames"),
            PipelineSettings::from_preset(Preset::Fast),
            false,
        );
        assert!(!spec.args.iter().any(|a| a == "-hwaccel"));
    }

    #[test]
    fn trim_adds_ss_before_input_and_duration_after() {
        let mut settings = balanced();
        settings.start_seconds = 12.5;
        settings.duration_seconds = 40.0;
        let spec = extract_spec(Path::new("clip.mp4"), Path::new("frames"), settings);
        let ss = spec.args.iter().position(|a| a == "-ss").unwrap();
        let input = spec.args.iter().position(|a| a == "-i").unwrap();
        let duration = spec.args.iter().position(|a| a == "-t").unwrap();
        assert!(ss < input);
        assert!(input < duration);
        assert_eq!(spec.args[ss + 1], "12.500");
        assert_eq!(spec.args[duration + 1], "40.000");
    }

    #[test]
    fn ffmetadata_dumps_to_stdout() {
        let spec = ffmetadata_spec(Path::new("/tmp/clip.mov"));
        assert_eq!(spec.sidecar, "ffmpeg");
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "-f" && w[1] == "ffmetadata"));
        assert_eq!(spec.args.last().map(String::as_str), Some("-"));
    }
}
