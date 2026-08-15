//! FFmpeg frame extraction command construction.

use std::path::Path;

use crate::keyframes;
use crate::settings::{FrameFormat, PipelineSettings};
use crate::sidecar::{path_arg, CommandSpec};

pub const THUMB_EDGE: u32 = 320;

/// Builds the FFmpeg `-vf` filter for extract rate and optional longest-edge scale.
#[cfg(test)]
pub fn video_filter(fps: f32, max_width: Option<u32>) -> String {
    filter_graph(fps, None, max_width)
}

/// Dense low-res thumbs used to score sharpness and motion.
pub fn candidate_spec(video: &Path, thumbs_dir: &Path, settings: PipelineSettings) -> CommandSpec {
    candidate_spec_with_hwaccel(video, thumbs_dir, settings, true)
}

pub fn candidate_spec_with_hwaccel(
    video: &Path,
    thumbs_dir: &Path,
    settings: PipelineSettings,
    hwaccel: bool,
) -> CommandSpec {
    let settings = settings.sanitized();
    extract_args(ExtractArgs {
        video,
        out_dir: thumbs_dir,
        settings,
        hwaccel,
        fps: keyframes::candidate_fps(settings.fps),
        max_width: Some(THUMB_EDGE),
        indices: None,
        format: FrameFormat::Jpg,
        variable_rate: false,
    })
}

/// Full-resolution stills at the selected candidate indices (`n` after `fps=`).
pub fn select_spec(
    video: &Path,
    frames_dir: &Path,
    settings: PipelineSettings,
    indices: &[usize],
) -> CommandSpec {
    select_spec_with_hwaccel(video, frames_dir, settings, indices, true)
}

pub fn select_spec_with_hwaccel(
    video: &Path,
    frames_dir: &Path,
    settings: PipelineSettings,
    indices: &[usize],
    hwaccel: bool,
) -> CommandSpec {
    let settings = settings.sanitized();
    extract_args(ExtractArgs {
        video,
        out_dir: frames_dir,
        settings,
        hwaccel,
        fps: keyframes::candidate_fps(settings.fps),
        max_width: settings.longest_edge(),
        indices: Some(indices),
        format: settings.frame_format,
        variable_rate: true,
    })
}

/// Maps UI JPEG quality 1–100 (best) onto FFmpeg `-q:v` 2–31 (lower is better).
pub fn jpeg_q_scale(quality: u8) -> u8 {
    let quality = u16::from(quality.clamp(1, 100));
    (2 + ((100 - quality) * 29) / 99) as u8
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

struct ExtractArgs<'a> {
    video: &'a Path,
    out_dir: &'a Path,
    settings: PipelineSettings,
    hwaccel: bool,
    fps: f32,
    max_width: Option<u32>,
    indices: Option<&'a [usize]>,
    format: FrameFormat,
    variable_rate: bool,
}

fn extract_args(req: ExtractArgs<'_>) -> CommandSpec {
    let mut args = Vec::new();
    args.push("-y".into());
    if req.hwaccel {
        args.push("-hwaccel".into());
        args.push("videotoolbox".into());
    }
    if req.settings.start_seconds > 0.0 {
        args.push("-ss".into());
        args.push(format_seconds(req.settings.start_seconds));
    }
    args.push("-i".into());
    args.push(path_arg(req.video));
    if req.settings.duration_seconds > 0.0 {
        args.push("-t".into());
        args.push(format_seconds(req.settings.duration_seconds));
    }
    args.push("-vf".into());
    args.push(filter_graph(req.fps, req.indices, req.max_width));
    if req.variable_rate {
        args.push("-fps_mode".into());
        args.push("vfr".into());
    }
    match req.format {
        FrameFormat::Jpg => {
            args.push("-q:v".into());
            args.push(jpeg_q_scale(req.settings.jpeg_quality).to_string());
        }
        FrameFormat::Png => {
            args.push("-pix_fmt".into());
            args.push("rgb24".into());
        }
    }
    let name = format!("frame_%05d.{}", req.format.extension());
    args.push(path_arg(&req.out_dir.join(name)));
    CommandSpec::new("ffmpeg", args)
}

fn filter_graph(fps: f32, indices: Option<&[usize]>, max_width: Option<u32>) -> String {
    let mut parts = vec![format!("fps={fps}")];
    if let Some(indices) = indices {
        let expr = indices
            .iter()
            .map(|index| format!("eq(n\\,{index})"))
            .collect::<Vec<_>>()
            .join("+");
        parts.push(format!("select='{expr}'"));
    }
    if let Some(width) = max_width {
        parts.push(format!("scale='min(iw,{width})':-2"));
    }
    parts.join(",")
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
    fn candidate_extract_uses_dense_thumbs() {
        let spec = candidate_spec(
            Path::new("/tmp/in.mp4"),
            Path::new("/tmp/_candidates"),
            balanced(),
        );
        assert_eq!(spec.sidecar, "ffmpeg");
        assert!(spec.args.contains(&"-hwaccel".into()));
        assert!(spec.args.contains(&"videotoolbox".into()));
        let vf = spec
            .args
            .windows(2)
            .find(|w| w[0] == "-vf")
            .map(|w| w[1].as_str())
            .unwrap();
        assert!(vf.contains("fps=8"));
        assert!(vf.contains("320"));
        assert_eq!(
            spec.args[spec.args.len() - 1],
            "/tmp/_candidates/frame_%05d.jpg"
        );
    }

    #[test]
    fn select_keeps_candidate_indices_and_vfr() {
        let spec = select_spec(
            Path::new("clip.mp4"),
            Path::new("frames"),
            balanced(),
            &[0, 5, 12],
        );
        let vf = spec
            .args
            .windows(2)
            .find(|w| w[0] == "-vf")
            .map(|w| w[1].as_str())
            .unwrap();
        assert!(vf.contains("eq(n\\,0)"));
        assert!(vf.contains("eq(n\\,5)"));
        assert!(vf.contains("eq(n\\,12)"));
        assert!(vf.contains("1600"));
        assert!(spec
            .args
            .windows(2)
            .any(|w| w[0] == "-fps_mode" && w[1] == "vfr"));
        assert_eq!(
            spec.args.last().map(String::as_str),
            Some("frames/frame_%05d.jpg")
        );
    }

    #[test]
    fn jpeg_quality_100_is_qscale_2() {
        assert_eq!(jpeg_q_scale(100), 2);
        assert_eq!(jpeg_q_scale(1), 31);
    }

    #[test]
    fn png_select_skips_jpeg_qscale_and_uses_rgb() {
        let mut settings = balanced();
        settings.frame_format = FrameFormat::Png;
        let spec = select_spec(Path::new("clip.mp4"), Path::new("frames"), settings, &[1]);
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
        let spec = candidate_spec_with_hwaccel(
            Path::new("clip.mov"),
            Path::new("thumbs"),
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
        let spec = candidate_spec(Path::new("clip.mp4"), Path::new("thumbs"), settings);
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
