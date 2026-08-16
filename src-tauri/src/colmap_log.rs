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
    pub trying: Option<u32>,
    pub failed: Option<u32>,
    pub elapsed_secs: Option<u64>,
    pub eta_secs: Option<u64>,
    #[serde(skip)]
    visible: Option<u32>,
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
            trying: None,
            failed: None,
            elapsed_secs: None,
            eta_secs: None,
            visible: None,
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

    /// Updates from a COLMAP log chunk (one or more glog records).
    /// Returns whether progress-facing fields changed — not per-image try/fail chatter.
    pub fn ingest(&mut self, chunk: &str) -> bool {
        let mut changed = false;
        for record in records(chunk) {
            changed |= self.ingest_record(record);
        }
        if changed {
            self.refresh_eta();
        }
        changed
    }

    fn ingest_record(&mut self, line: &str) -> bool {
        let line = strip_glog(line);
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
        if line.contains("Registering initial image pair") {
            changed |= self.set_registered(2);
        }
        if let Some(n) = parse_num_reg_frames(line) {
            changed |= self.set_registered(n);
        }
        if let Some(id) = parse_trying_image(line) {
            self.trying = Some(id);
        }
        if let Some(n) = parse_visible_points(line) {
            self.visible = Some(n);
        }
        if line.contains("Could not register") {
            self.failed = Some(self.failed.unwrap_or(0).saturating_add(1));
        }
        changed
    }

    fn set_registered(&mut self, n: u32) -> bool {
        let next = if let Some(total) = self.total {
            n.min(total)
        } else {
            n
        };
        let changed = self.registered != Some(next) || self.processed != Some(next);
        self.registered = Some(next);
        self.processed = Some(next);
        changed
    }

    pub fn set_counts(&mut self, registered: Option<u32>, points: Option<u32>, total: Option<u32>) {
        if let Some(n) = registered {
            self.set_registered(n);
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
        if let Some(id) = self.trying {
            parts.push(format!("trying #{id}"));
        }
        if let Some(n) = self.visible {
            parts.push(format!("sees {n} pts"));
        }
        if let Some(n) = self.failed.filter(|n| *n > 0) {
            parts.push(format!("{n} failed"));
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

/// Splits a sidecar chunk into glog records when COLMAP concatenates them.
pub fn records(chunk: &str) -> Vec<&str> {
    let chunk = chunk.trim();
    if chunk.is_empty() {
        return Vec::new();
    }
    let starts = glog_starts(chunk);
    if starts.len() <= 1 {
        return vec![chunk];
    }
    let mut out = Vec::with_capacity(starts.len());
    for pair in starts.windows(2) {
        let piece = chunk[pair[0]..pair[1]].trim();
        if !piece.is_empty() {
            out.push(piece);
        }
    }
    if let Some(&last) = starts.last() {
        let piece = chunk[last..].trim();
        if !piece.is_empty() {
            out.push(piece);
        }
    }
    out
}

/// Mapper chatter is parsed for stats but omitted from the UI log.
pub fn ui_log(line: &str) -> Option<String> {
    let message = strip_glog(line);
    if message.is_empty() || is_mapper_chatter(message) {
        return None;
    }
    Some(message.to_string())
}

fn is_mapper_chatter(line: &str) -> bool {
    let line = line.trim_start_matches("=> ").trim_start_matches("=&gt; ");
    line.starts_with("Registering image #")
        || line.starts_with("Registering image with structure-less")
        || line.starts_with("Image sees ")
        || line.starts_with("Could not register")
}

fn strip_glog(line: &str) -> &str {
    let line = line.trim();
    let bytes = line.as_bytes();
    if bytes
        .first()
        .is_some_and(|b| matches!(b, b'I' | b'W' | b'E' | b'F'))
    {
        if let Some(idx) = line.find("] ") {
            return &line[idx + 2..];
        }
        if let Some(idx) = line.find(']') {
            let rest = line[idx + 1..].trim_start();
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    line
}

fn glog_starts(chunk: &str) -> Vec<usize> {
    let bytes = chunk.as_bytes();
    let mut starts = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if is_glog_at(chunk, i) && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            starts.push(i);
            i += 5;
        } else {
            i += 1;
        }
    }
    starts
}

fn is_glog_at(chunk: &str, i: usize) -> bool {
    let rest = &chunk[i..];
    let bytes = rest.as_bytes();
    if bytes
        .first()
        .is_none_or(|b| !matches!(b, b'I' | b'W' | b'E' | b'F'))
    {
        return false;
    }
    let after = &rest[1..];
    let digits = after.bytes().take_while(u8::is_ascii_digit).count();
    if digits != 4 && digits != 8 {
        return false;
    }
    let time = after.get(digits + 1..).unwrap_or("");
    after[digits..].starts_with(' ')
        && time.len() >= 8
        && time.as_bytes()[2] == b':'
        && time.as_bytes()[5] == b':'
}

fn parse_bracket_progress(line: &str, needle: &str) -> Option<(u32, u32)> {
    let rest = line.split(needle).nth(1)?;
    let (cur, rest) = rest.split_once('/')?;
    let total = rest.split(']').next()?;
    Some((cur.trim().parse().ok()?, total.trim().parse().ok()?))
}

fn parse_features(line: &str) -> Option<u32> {
    if line.contains("Image sees") {
        return None;
    }
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

fn parse_num_reg_frames(line: &str) -> Option<u32> {
    let rest = line.split("num_reg_frames=").nth(1)?;
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn parse_trying_image(line: &str) -> Option<u32> {
    let rest = line.split("Registering image #").nth(1)?;
    rest.split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

fn parse_visible_points(line: &str) -> Option<u32> {
    if !line.contains("Image sees") || !line.contains("points") {
        return None;
    }
    let rest = line.split("Image sees").nth(1)?;
    rest.split('/').next()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAPPER_TAIL: &str = r#"I20260816 17:08:10.424074 0x1f629de80 incremental_pipeline.cc:546] Registering image with structure-less fallback
I20260816 17:08:10.424075 0x1f629de80 incremental_pipeline.cc:548] => Image sees 2752 / 53464 correspondences
I20260816 17:08:10.424287 0x1f629de80 incremental_pipeline.cc:563] => Could not register, trying another image.
I20260816 17:08:10.424289 0x1f629de80 incremental_pipeline.cc:537] Registering image #407 (num_reg_frames=2)
I20260816 17:08:10.424301 0x1f629de80 incremental_pipeline.cc:540] => Image sees 2171 / 12989 points
I20260816 17:08:10.424303 0x1f629de80 incremental_pipeline.cc:546] Registering image with structure-less fallback
I20260816 17:08:10.424304 0x1f629de80 incremental_pipeline.cc:548] => Image sees 2715 / 51975 correspondences
I20260816 17:08:10.424522 0x1f629de80 incremental_pipeline.cc:563] => Could not register, trying another image.
I20260816 17:08:10.427004 0x1f629de80 incremental_pipeline.cc:537] Registering image #462 (num_reg_frames=2)
I20260816 17:08:10.427005 0x1f629de80 incremental_pipeline.cc:540] => Image sees 58 / 12103 points
I20260816 17:08:10.427197 0x1f629de80 incremental_pipeline.cc:563] => Could not register, trying another image.
I20260816 17:08:10.427391 0x1f629de80 incremental_pipeline.cc:723] Keeping successful reconstruction
I20260816 17:08:10.430540 0x1f629de80 timer.cc:90] Elapsed time: 0.009 [minutes]
"#;

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
        assert!(map.ingest("Registering image #12 (num_reg_frames=3)"));
        assert_eq!(map.registered, Some(3));
        assert_eq!(map.processed, Some(3));
        assert_eq!(map.trying, Some(12));
        assert!(map.summary().contains("trying #12"));
    }

    #[test]
    fn image_id_is_not_registered_count() {
        let mut map = CameraSnapshot::new(CameraStep::Mapping, 85, 14);
        assert!(!map.ingest("Registering image #407 (frame_00407.jpg)"));
        assert_eq!(map.registered, None);
        assert_eq!(map.trying, Some(407));
    }

    #[test]
    fn mapper_tail_uses_reg_frames_and_counts_failures() {
        let mut map = CameraSnapshot::new(CameraStep::Mapping, 85, 14);
        map.total = Some(480);
        assert!(map.ingest(MAPPER_TAIL));
        assert_eq!(map.registered, Some(2));
        assert_eq!(map.processed, Some(2));
        assert_eq!(map.trying, Some(462));
        assert_eq!(map.failed, Some(3));
        assert_eq!(map.visible, Some(58));
        assert!(map.summary().contains("2 / 480"));
        assert!(map.summary().contains("trying #462"));
        assert!(map.summary().contains("3 failed"));
        assert_eq!(map.percent(), 85);
    }

    #[test]
    fn concatenated_glog_splits_into_records() {
        let chunk = MAPPER_TAIL.lines().collect::<Vec<_>>().join(" ");
        let parts = records(&chunk);
        assert!(parts.len() >= 8, "got {}", parts.len());
        assert!(strip_glog(parts[0]).contains("structure-less fallback"));
        assert!(parts
            .iter()
            .any(|p| strip_glog(p).contains("Keeping successful")));
    }

    #[test]
    fn ui_log_drops_mapper_chatter() {
        assert_eq!(
            ui_log("I20260816 17:08:10.424289 0x1 incremental_pipeline.cc:537] Registering image #407 (num_reg_frames=2)"),
            None
        );
        assert_eq!(
            ui_log("I20260816 17:08:10.424522 0x1 incremental_pipeline.cc:563] => Could not register, trying another image."),
            None
        );
        assert_eq!(
            ui_log("I20260816 17:08:10.427391 0x1 incremental_pipeline.cc:723] Keeping successful reconstruction"),
            Some("Keeping successful reconstruction".into())
        );
        assert_eq!(
            ui_log("$ colmap mapper --database_path db"),
            Some("$ colmap mapper --database_path db".into())
        );
    }

    #[test]
    fn initial_pair_counts_as_two_cameras() {
        let mut map = CameraSnapshot::new(CameraStep::Mapping, 85, 14);
        assert!(map.ingest("Registering initial image pair #11 and #20"));
        assert_eq!(map.registered, Some(2));
    }

    #[test]
    fn try_fail_chatter_does_not_report_progress_change() {
        let mut map = CameraSnapshot::new(CameraStep::Mapping, 85, 14);
        assert!(map.ingest("Registering image #12 (num_reg_frames=2)"));
        assert!(!map.ingest("=> Could not register, trying another image."));
        assert!(!map.ingest("=> Image sees 2171 / 12989 points"));
        assert_eq!(map.failed, Some(1));
        assert_eq!(map.visible, Some(2171));
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
