//! Parse Brush CLI log lines into a live training snapshot.

use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrainSnapshot {
    pub total: u32,
    pub iter: Option<u32>,
    pub splats: Option<u32>,
    pub psnr: Option<f32>,
    pub ssim: Option<f32>,
    pub train_views: Option<u32>,
    pub eval_views: Option<u32>,
    pub elapsed_secs: Option<u64>,
}

impl TrainSnapshot {
    pub fn new(total: u32) -> Self {
        Self {
            total,
            iter: None,
            splats: None,
            psnr: None,
            ssim: None,
            train_views: None,
            eval_views: None,
            elapsed_secs: None,
        }
    }

    /// Updates the snapshot from one Brush log line. Returns whether anything changed.
    pub fn ingest(&mut self, line: &str) -> bool {
        let line = strip_log_prefix(line);
        if let Some((train, eval)) = parse_dataset(line) {
            self.train_views = Some(train);
            self.eval_views = Some(eval);
            return true;
        }
        if let Some((iter, splats)) = parse_refine(line) {
            self.iter = Some(iter);
            self.splats = Some(splats);
            return true;
        }
        if let Some((iter, psnr, ssim)) = parse_eval(line) {
            self.iter = Some(iter);
            self.psnr = Some(psnr);
            self.ssim = Some(ssim);
            return true;
        }
        false
    }

    /// Reads the iteration from a Brush `export_{iter}.ply` checkpoint path.
    pub fn ingest_export(&mut self, path: &Path) -> bool {
        let Some(iter) = parse_export_iter(path) else {
            return false;
        };
        if self.iter.is_some_and(|current| iter <= current) {
            return false;
        }
        self.iter = Some(iter);
        true
    }

    pub fn percent(&self) -> u8 {
        let Some(iter) = self.iter else {
            return 15;
        };
        let total = self.total.max(1);
        let mapped = 15 + (u64::from(iter) * 80 / u64::from(total));
        mapped.min(99) as u8
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(iter) = self.iter {
            parts.push(format!("Step {iter} / {}", self.total));
        } else if let Some(views) = self.train_views {
            parts.push(format!("{views} views loaded"));
            parts.push(format!("{} steps", self.total));
        } else {
            parts.push(format!("Training ({} steps)", self.total));
        }
        if let Some(splats) = self.splats {
            parts.push(format!("{} Gaussians", splats));
        }
        if let Some(psnr) = self.psnr {
            parts.push(format!("PSNR {psnr:.1}"));
        }
        if let Some(ssim) = self.ssim {
            parts.push(format!("SSIM {ssim:.3}"));
        }
        if let Some(secs) = self.elapsed_secs {
            parts.push(format_elapsed(secs));
        }
        parts.join(" · ")
    }
}

fn format_elapsed(secs: u64) -> String {
    let minutes = secs / 60;
    let seconds = secs % 60;
    if minutes == 0 {
        format!("{seconds}s")
    } else {
        format!("{minutes}m {seconds:02}s")
    }
}

fn parse_export_iter(path: &Path) -> Option<u32> {
    path.file_stem()?
        .to_str()?
        .strip_prefix("export_")?
        .parse()
        .ok()
}

/// Brush detaches `cli-trainer`, so a Burn panic still prints "Done training" and may exit 0.
pub fn trainer_panicked(text: &str) -> bool {
    text.contains("Ordering is bigger than operations")
        || text.contains("panicked at")
        || text.contains("CallError(task panicked")
}

fn strip_log_prefix(line: &str) -> &str {
    let line = line.trim();
    if let Some(idx) = line.find("Loaded dataset") {
        return &line[idx..];
    }
    if let Some(idx) = line.find("Refine iter") {
        return &line[idx..];
    }
    if let Some(idx) = line.find("Eval iter") {
        return &line[idx..];
    }
    line
}

fn parse_dataset(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("Loaded dataset with ")?;
    let (train, rest) = rest.split_once(" training")?;
    let eval = rest
        .trim_start_matches(',')
        .trim_start()
        .split_whitespace()
        .next()?;
    Some((train.trim().parse().ok()?, eval.parse().ok()?))
}

fn parse_refine(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("Refine iter ")?;
    let (iter, rest) = rest.split_once(',')?;
    let splats = rest.trim().split_whitespace().next()?;
    Some((iter.trim().parse().ok()?, splats.parse().ok()?))
}

fn parse_eval(line: &str) -> Option<(u32, f32, f32)> {
    let rest = line.strip_prefix("Eval iter ")?;
    let (iter, rest) = rest.split_once(':')?;
    let rest = rest.trim();
    let psnr = rest.strip_prefix("PSNR ")?.split(',').next()?.trim();
    let ssim = rest.split("ssim ").nth(1)?.trim();
    Some((
        iter.trim().parse().ok()?,
        psnr.parse().ok()?,
        ssim.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dataset_refine_and_eval() {
        let mut snap = TrainSnapshot::new(15_000);
        assert!(snap.ingest("INFO brush_cli: Loaded dataset with 42 training, 0 eval views"));
        assert_eq!(snap.train_views, Some(42));
        assert!(snap.ingest("Refine iter 500, 123456 splats."));
        assert_eq!(snap.iter, Some(500));
        assert_eq!(snap.splats, Some(123456));
        assert!(snap.ingest("Eval iter 1000: PSNR 24.5, ssim 0.812"));
        assert_eq!(snap.psnr, Some(24.5));
        assert_eq!(snap.ssim, Some(0.812));
        assert!(snap.summary().contains("500") || snap.summary().contains("1000"));
        assert!(snap.percent() > 15);
    }

    #[test]
    fn ignores_unrelated_lines() {
        let mut snap = TrainSnapshot::new(1000);
        assert!(!snap.ingest("Starting up"));
        assert_eq!(snap.iter, None);
    }

    #[test]
    fn trainer_panicked_catches_burn_fusion_and_ignores_done() {
        assert!(trainer_panicked(
            "Ordering is bigger than operations: ordering len 49, operations len 0"
        ));
        assert!(trainer_panicked(
            "thread 'cli-trainer' panicked at /Users/frank/.cargo/git/checkouts/burn-6c277d792b0d5d7a/b6e27bd/crates/burn-fusion/src/client.rs:200:14:"
        ));
        assert!(trainer_panicked(
            "called `Result::unwrap()` on an `Err` value: CallError(task panicked on device runner thread: Ordering is bigger than operations)"
        ));
        assert!(!trainer_panicked(
            "INFO brush_cli: Done training! Took FormattedDuration(1s)."
        ));
        assert!(!trainer_panicked("Refine iter 50, 12000 splats."));
    }

    #[test]
    fn ingest_export_reads_padded_iter() {
        let mut snap = TrainSnapshot::new(15_000);
        assert!(snap.ingest_export(Path::new("/tmp/export_01000.ply")));
        assert_eq!(snap.iter, Some(1000));
        assert!(!snap.ingest_export(Path::new("/tmp/export_0500.ply")));
        assert!(!snap.ingest_export(Path::new("/tmp/scene.ply")));
    }
}
