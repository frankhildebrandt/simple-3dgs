//! Elapsed / ETA formatting shared by Frames, Cameras, and Train stats.

/// Formats a duration for live stats. Uses hours once the span is over 90 minutes.
pub fn format_duration(secs: u64) -> String {
    if secs >= 90 * 60 {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        format!("{hours}h {minutes:02}m")
    } else if secs >= 60 {
        let minutes = secs / 60;
        let seconds = secs % 60;
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{secs}s")
    }
}

/// Linear ETA from work already done. `None` until something has finished.
pub fn eta_secs(elapsed: u64, done: u32, total: u32) -> Option<u64> {
    if done == 0 || total == 0 || done >= total {
        return None;
    }
    Some(elapsed.saturating_mul(u64::from(total.saturating_sub(done))) / u64::from(done))
}

/// Parses `Duration: HH:MM:SS.xx` from an FFmpeg banner line.
pub fn parse_ffmpeg_duration(line: &str) -> Option<f32> {
    let rest = line.split("Duration:").nth(1)?;
    let token = rest.split(',').next()?.trim();
    parse_hms(token)
}

/// Parses `HH:MM:SS` / `HH:MM:SS.xx` into seconds.
pub fn parse_hms(token: &str) -> Option<f32> {
    let mut parts = token.split(':');
    let hours: f32 = parts.next()?.trim().parse().ok()?;
    let minutes: f32 = parts.next()?.trim().parse().ok()?;
    let seconds: f32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_uses_hours_after_ninety_minutes() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(65), "1m 05s");
        assert_eq!(format_duration(90 * 60), "1h 30m");
        assert_eq!(format_duration(2 * 3600 + 5 * 60), "2h 05m");
    }

    #[test]
    fn eta_needs_progress() {
        assert_eq!(eta_secs(10, 0, 100), None);
        assert_eq!(eta_secs(10, 100, 100), None);
        assert_eq!(eta_secs(10, 25, 100), Some(30));
    }

    #[test]
    fn ffmpeg_banner_duration() {
        let line = "  Duration: 00:01:23.45, start: 0.000000, bitrate: 8000 kb/s";
        assert!((parse_ffmpeg_duration(line).unwrap() - 83.45).abs() < 0.01);
        assert_eq!(parse_ffmpeg_duration("frame=12"), None);
    }
}
