//! Live FFmpeg extract snapshot: pass, frame counts, and mapped percent.

use serde::Serialize;

use crate::duration::{eta_secs, format_duration, parse_ffmpeg_duration, parse_hms};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FramePass {
    Candidates,
    Keyframes,
    Import,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSnapshot {
    pub pass: FramePass,
    pub current: Option<u32>,
    pub total: Option<u32>,
    pub kept: Option<u32>,
    pub elapsed_secs: Option<u64>,
    pub eta_secs: Option<u64>,
    pub duration_secs: Option<f32>,
    #[serde(skip)]
    pub out_secs: Option<f32>,
}

impl FrameSnapshot {
    pub fn new(pass: FramePass) -> Self {
        Self {
            pass,
            current: None,
            total: None,
            kept: None,
            elapsed_secs: None,
            eta_secs: None,
            duration_secs: None,
            out_secs: None,
        }
    }

    /// Updates from one FFmpeg log or `-progress` line. Returns whether anything changed.
    pub fn ingest(&mut self, line: &str) -> bool {
        let line = line.trim();
        if let Some(secs) = parse_ffmpeg_duration(line) {
            self.duration_secs = Some(effective_duration(secs, self.duration_secs));
            self.refresh_eta();
            return true;
        }
        if let Some((key, value)) = parse_progress_field(line) {
            if self.ingest_field(key, value) {
                return true;
            }
        }
        if let Some(frame) = parse_stderr_frame(line) {
            self.current = Some(frame);
            self.refresh_eta();
            return true;
        }
        false
    }

    fn ingest_field(&mut self, key: &str, value: &str) -> bool {
        match key {
            "frame" => {
                if let Ok(frame) = value.parse::<u32>() {
                    self.current = Some(frame);
                    self.refresh_eta();
                    return true;
                }
            }
            "out_time_us" => {
                if let Ok(us) = value.parse::<u64>() {
                    self.out_secs = Some((us as f32) / 1_000_000.0);
                    self.refresh_eta();
                    return true;
                }
            }
            "out_time" => {
                if let Some(secs) = parse_hms(value) {
                    self.out_secs = Some(secs);
                    self.refresh_eta();
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn refresh_eta(&mut self) {
        let Some(elapsed) = self.elapsed_secs else {
            self.eta_secs = None;
            return;
        };
        if let (Some(out), Some(dur)) = (self.out_secs, self.duration_secs) {
            if dur > 0.0 && out > 0.0 && out < dur {
                let remaining = ((dur - out) * elapsed as f32 / out) as u64;
                self.eta_secs = Some(remaining);
                return;
            }
        }
        let (Some(done), Some(total)) = (self.current, self.total) else {
            self.eta_secs = None;
            return;
        };
        self.eta_secs = eta_secs(elapsed, done, total);
    }

    /// Maps this pass onto 0–100. Candidates occupy 0–50, keyframes 50–100.
    pub fn percent(&self) -> u8 {
        let fraction = if let (Some(out), Some(dur)) =
            (self.out_secs, self.duration_secs.filter(|d| *d > 0.0))
        {
            ((out / dur) * 100.0).clamp(0.0, 100.0) as u8
        } else {
            match (self.current, self.total.filter(|n| *n > 0)) {
                (Some(cur), Some(total)) => {
                    (u64::from(cur) * 100 / u64::from(total)).min(100) as u8
                }
                _ => 0,
            }
        };
        match self.pass {
            FramePass::Candidates => (u16::from(fraction) * 50 / 100) as u8,
            FramePass::Keyframes => 50 + (u16::from(fraction) * 50 / 100) as u8,
            FramePass::Import => fraction.max(5),
        }
        .min(99)
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(match self.pass {
            FramePass::Candidates => "Candidate frames".into(),
            FramePass::Keyframes => "Extracting keyframes".into(),
            FramePass::Import => "Importing stills".into(),
        });
        if let (Some(cur), Some(total)) = (self.current, self.total) {
            parts.push(format!("{cur} / {total}"));
        } else if let Some(cur) = self.current {
            parts.push(format!("frame {cur}"));
        }
        if let Some(kept) = self.kept {
            parts.push(format!("{kept} kept"));
        }
        if let Some(secs) = self.elapsed_secs {
            parts.push(format_duration(secs));
        }
        if let Some(secs) = self.eta_secs {
            parts.push(format!("ETA {}", format_duration(secs)));
        }
        parts.join(" · ")
    }
}

fn effective_duration(parsed: f32, previous: Option<f32>) -> f32 {
    previous
        .filter(|d| *d > 0.0 && *d <= parsed)
        .unwrap_or(parsed)
}

fn parse_progress_field(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    if key.contains(' ') {
        return None;
    }
    Some((key.trim(), value.trim()))
}

/// Parses `frame=  123 fps=` from classic FFmpeg stderr.
fn parse_stderr_frame(line: &str) -> Option<u32> {
    let rest = line.split("frame=").nth(1)?;
    rest.split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_pipe_maps_candidates_to_first_half() {
        let mut snap = FrameSnapshot::new(FramePass::Candidates);
        assert!(snap.ingest("  Duration: 00:00:10.00, start: 0.000000, bitrate: 1 kb/s"));
        assert_eq!(snap.duration_secs, Some(10.0));
        assert!(snap.ingest("out_time_us=5000000"));
        assert_eq!(snap.percent(), 25);
        assert!(snap.summary().contains("Candidate"));
    }

    #[test]
    fn keyframe_pass_starts_at_fifty() {
        let mut snap = FrameSnapshot::new(FramePass::Keyframes);
        snap.duration_secs = Some(10.0);
        snap.ingest("out_time=00:00:10.00");
        assert!(snap.percent() >= 99 || snap.percent() >= 50);
        assert!(snap.percent() >= 50);
    }

    #[test]
    fn stderr_frame_counter() {
        let mut snap = FrameSnapshot::new(FramePass::Candidates);
        assert!(snap.ingest("frame=  42 fps= 12 q=28.0 size=   0kB time=00:00:01.00"));
        assert_eq!(snap.current, Some(42));
    }

    #[test]
    fn ignores_unrelated_lines() {
        let mut snap = FrameSnapshot::new(FramePass::Import);
        assert!(!snap.ingest("configuration: --enable-videotoolbox"));
    }
}
