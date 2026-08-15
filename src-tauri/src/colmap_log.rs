//! Live COLMAP snapshot: features, matches, registered cameras, mapped percent.

use serde::Serialize;

use crate::duration::{eta_secs, format_duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CameraStep {
    Features,
    Matching,
    Calibrating,
    Mapping,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraSnapshot {
    pub step: CameraStep,
    pub processed: Option<u32>,
    pub total: Option<u32>,
    pub features: Option<u32>,
    pub matches: Option<u32>,
    pub registered: Option<u32>,
    pub points: Option<u32>,
    pub elapsed_secs: Option<u64>,
    pub eta_secs: Option<u64>,
    #[serde(skip)]
    base: u8,
    #[serde(skip)]
    span: u8,
}

impl CameraSnapshot {
    pub fn new(step: CameraStep, base: u8, span: u8) -> Self {
        Self {
            step,
            processed: None,
            total: None,
            features: None,
            matches: None,
            registered: None,
            points: None,
            elapsed_secs: None,
            eta_secs: None,
            base,
            span: span.max(1),
        }
    }

    /// Progress band and label for a COLMAP subcommand in `args[0]`.
    pub fn for_spec(subcommand: &str) -> (Self, &'static str) {
        match subcommand {
            "feature_extractor" => (
                Self::new(CameraStep::Features, 20, 30),
                "Extracting features",
            ),
            "sequential_matcher" | "exhaustive_matcher" => {
                (Self::new(CameraStep::Matching, 50, 20), "Matching views")
            }
            "view_graph_calibrator" => (
                Self::new(CameraStep::Calibrating, 70, 15),
                "Calibrating cameras",
            ),
            "mapper" | "global_mapper" => {
                (Self::new(CameraStep::Mapping, 85, 14), "Mapping cameras")
            }
            _ => (
                Self::new(CameraStep::Features, 10, 10),
                "Estimating camera poses",
            ),
        }
    }

    /// Updates from one COLMAP log line. Returns whether anything changed.
    pub fn ingest(&mut self, line: &str) -> bool {
        let line = strip_log_prefix(line);
        let mut changed = false;
        if let Some((cur, total)) = parse_bracket_progress(line, "Processed file [") {
            self.processed = Some(cur);
            self.total = Some(total);
            changed = true;
        }
        if let Some((cur, total)) = parse_bracket_progress(line, "Matching file [")
            .or_else(|| parse_bracket_progress(line, "Matching image pair ["))
        {
            self.processed = Some(cur);
            self.total = Some(total);
            changed = true;
        }
        if let Some(n) = parse_features(line) {
            self.features = Some(self.features.unwrap_or(0).saturating_add(n));
            changed = true;
        }
        if let Some(n) = parse_matches(line) {
            self.matches = Some(n);
            changed = true;
        }
        if let Some(n) = parse_registering(line) {
            self.registered = Some(n);
            if let Some(total) = self.total {
                self.processed = Some(n.min(total));
            } else {
                self.processed = Some(n);
            }
            changed = true;
        }
        if changed {
            self.refresh_eta();
        }
        changed
    }

    pub fn set_counts(&mut self, registered: Option<u32>, points: Option<u32>, total: Option<u32>) {
        if let Some(n) = registered {
            self.registered = Some(n);
            self.processed = Some(n);
        }
        if let Some(n) = points {
            self.points = Some(n);
        }
        if let Some(n) = total {
            self.total = Some(n);
        }
        self.refresh_eta();
    }

    fn refresh_eta(&mut self) {
        let (Some(elapsed), Some(done), Some(total)) =
            (self.elapsed_secs, self.processed, self.total)
        else {
            self.eta_secs = None;
            return;
        };
        self.eta_secs = eta_secs(elapsed, done, total);
    }

    pub fn percent(&self) -> u8 {
        let fraction = match (self.processed, self.total.filter(|n| *n > 0)) {
            (Some(cur), Some(total)) => (u64::from(cur.min(total)) * 100 / u64::from(total)) as u8,
            _ => 0,
        };
        let mapped = u16::from(self.base) + u16::from(self.span) * u16::from(fraction) / 100;
        mapped.min(99) as u8
    }

    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(match self.step {
            CameraStep::Features => "Extracting features".into(),
            CameraStep::Matching => "Matching views".into(),
            CameraStep::Calibrating => "Calibrating cameras".into(),
            CameraStep::Mapping => "Mapping cameras".into(),
        });
        if let (Some(cur), Some(total)) = (self.processed, self.total) {
            parts.push(format!("{cur} / {total}"));
        } else if let Some(reg) = self.registered {
            parts.push(format!("{reg} cameras"));
        }
        if let Some(n) = self.features {
            parts.push(format!("{n} features"));
        }
        if let Some(n) = self.matches {
            parts.push(format!("{n} matches"));
        }
        if let Some(n) = self.points {
            parts.push(format!("{n} points"));
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

fn strip_log_prefix(line: &str) -> &str {
    let line = line.trim();
    if let Some(idx) = line.find("Processed file [") {
        return &line[idx..];
    }
    if let Some(idx) = line.find("Matching file [") {
        return &line[idx..];
    }
    if let Some(idx) = line.find("Matching image pair [") {
        return &line[idx..];
    }
    if let Some(idx) = line.find("Registering image") {
        return &line[idx..];
    }
    line
}

fn parse_bracket_progress(line: &str, needle: &str) -> Option<(u32, u32)> {
    let rest = line.split(needle).nth(1)?;
    let (cur, rest) = rest.split_once('/')?;
    let total = rest.split(']').next()?;
    Some((cur.trim().parse().ok()?, total.trim().parse().ok()?))
}

fn parse_features(line: &str) -> Option<u32> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find(" features")?;
    let before = line[..idx].split_whitespace().last()?;
    before.parse().ok()
}

fn parse_matches(line: &str) -> Option<u32> {
    let rest = line.split("in total").next()?;
    let token = rest.split_whitespace().last()?;
    if line.to_ascii_lowercase().contains("match") {
        token.parse().ok()
    } else {
        None
    }
}

fn parse_registering(line: &str) -> Option<u32> {
    let rest = line.split("Registering image #").nth(1)?;
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processed_file_maps_inside_feature_band() {
        let mut snap = CameraSnapshot::new(CameraStep::Features, 20, 30);
        assert!(snap.ingest("I0815 12:00:00.1 1 feature.cc:1] Processed file [25/100]"));
        assert_eq!(snap.processed, Some(25));
        assert_eq!(snap.total, Some(100));
        assert_eq!(snap.percent(), 27);
        assert!(snap.summary().contains("25 / 100"));
    }

    #[test]
    fn matching_pairs_and_registering() {
        let mut snap = CameraSnapshot::new(CameraStep::Matching, 50, 20);
        assert!(snap.ingest("Matching image pair [10/200]"));
        assert_eq!(snap.percent(), 51);
        let mut map = CameraSnapshot::new(CameraStep::Mapping, 85, 14);
        assert!(map.ingest("Registering image #12 (frame_00012.jpg)"));
        assert_eq!(map.registered, Some(12));
    }

    #[test]
    fn for_spec_covers_room_and_object() {
        let (_, msg) = CameraSnapshot::for_spec("feature_extractor");
        assert_eq!(msg, "Extracting features");
        let (snap, msg) = CameraSnapshot::for_spec("view_graph_calibrator");
        assert_eq!(msg, "Calibrating cameras");
        assert_eq!(snap.step, CameraStep::Calibrating);
        let (_, msg) = CameraSnapshot::for_spec("global_mapper");
        assert_eq!(msg, "Mapping cameras");
    }

    #[test]
    fn ignores_unrelated() {
        let mut snap = CameraSnapshot::new(CameraStep::Features, 20, 30);
        assert!(!snap.ingest("COLMAP 4.1.0"));
    }
}
