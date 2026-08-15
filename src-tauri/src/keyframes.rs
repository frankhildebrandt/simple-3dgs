//! Motion-adaptive keyframe picks for COLMAP: sharp frames where the camera moved.

use std::path::{Path, PathBuf};

use image::GrayImage;

use crate::error::PipelineError;
use crate::project::{is_image, MIN_FRAMES};
use crate::settings::PipelineSettings;

/// Laplacian variance below this is treated as mush (motion blur / defocus).
pub const BLUR_FLOOR: f32 = 15.0;

/// Mean-abs luma delta (0–255) expected from one candidate step of a typical orbit.
const MODERATE_MAD: f32 = 8.0;

const CANDIDATE_MIN_FPS: f32 = 8.0;
const CANDIDATE_MAX_FPS: f32 = 12.0;
const THRESHOLD_RELAX_STEPS: u32 = 6;

/// Per-candidate scores in extract order (FFmpeg `n` after the fps filter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateScore {
    pub sharpness: f32,
    pub motion: f32,
}

/// Caps and the accumulated-MAD gate for [`select_keyframes`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyframeConfig {
    pub min_keep: usize,
    pub max_keep: usize,
    pub motion_threshold: f32,
    pub blur_floor: f32,
}

impl KeyframeConfig {
    /// Maps pipeline knobs onto selector caps. `fps` is target density while moving.
    pub fn from_settings(settings: PipelineSettings) -> Self {
        let settings = settings.sanitized();
        Self {
            min_keep: MIN_FRAMES,
            max_keep: settings.max_frames as usize,
            motion_threshold: motion_threshold(settings.fps),
            blur_floor: BLUR_FLOOR,
        }
    }
}

/// Dense thumb rate used for scoring. At least 4× the target density, clamped 8–12.
pub fn candidate_fps(target_fps: f32) -> f32 {
    (target_fps * 4.0).clamp(CANDIDATE_MIN_FPS, CANDIDATE_MAX_FPS)
}

/// Accumulated MAD that should yield about `target_fps` keeps under moderate motion.
pub fn motion_threshold(target_fps: f32) -> f32 {
    let target = target_fps.max(0.25);
    (candidate_fps(target) / target) * MODERATE_MAD
}

/// Picks keyframe indices from motion/sharpness scores. Never exceeds `max_keep`.
pub fn select_keyframes(scores: &[CandidateScore], config: KeyframeConfig) -> Vec<usize> {
    if scores.is_empty() {
        return Vec::new();
    }
    let max_keep = config.max_keep.max(1).min(scores.len());
    let min_keep = config.min_keep.min(max_keep).min(scores.len());

    let mut threshold = config.motion_threshold.max(0.0);
    let mut selected = thin_to_max(
        select_by_motion(scores, threshold, config.blur_floor),
        scores,
        max_keep,
    );

    let mut relax = 0;
    while selected.len() < min_keep && relax < THRESHOLD_RELAX_STEPS && threshold > 0.0 {
        threshold *= 0.5;
        selected = thin_to_max(
            select_by_motion(scores, threshold, config.blur_floor),
            scores,
            max_keep,
        );
        relax += 1;
    }

    if selected.len() < min_keep {
        selected = fill_sharpest(selected, scores, min_keep, config.blur_floor);
    }
    thin_to_max(selected, scores, max_keep)
}

/// Scores every still in `dir` (sorted names) as Laplacian variance plus MAD to the previous frame.
pub fn score_candidates(dir: &Path) -> Result<Vec<CandidateScore>, PipelineError> {
    let paths = list_stills(dir)?;
    let mut prev: Option<GrayImage> = None;
    let mut scores = Vec::with_capacity(paths.len());
    for path in paths {
        let gray = match image::open(&path) {
            Ok(img) => img.to_luma8(),
            Err(_) => {
                scores.push(CandidateScore {
                    sharpness: 0.0,
                    motion: 0.0,
                });
                prev = None;
                continue;
            }
        };
        let sharpness = laplacian_variance(&gray);
        let motion = prev
            .as_ref()
            .map(|last| mean_abs_diff(last, &gray))
            .unwrap_or(0.0);
        prev = Some(gray);
        scores.push(CandidateScore { sharpness, motion });
    }
    Ok(scores)
}

/// JPEG/PNG paths in `dir`, sorted so index `i` matches FFmpeg `n` after `fps=`.
pub fn list_stills(dir: &Path) -> Result<Vec<PathBuf>, PipelineError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && is_image(path))
        .collect();
    paths.sort();
    Ok(paths)
}

fn select_by_motion(scores: &[CandidateScore], threshold: f32, blur_floor: f32) -> Vec<usize> {
    let mut keeps = Vec::new();
    let mut window_start = 0;
    let mut acc = 0.0;
    for i in 0..scores.len() {
        if i > 0 {
            acc += scores[i].motion.max(0.0);
        }
        let at_end = i + 1 == scores.len();
        let hit = acc >= threshold;
        let close_end = at_end && (acc > 0.0 || keeps.is_empty());
        if !hit && !close_end {
            continue;
        }
        match pick_sharpest(scores, window_start, i, blur_floor) {
            Some(pick) if keeps.last() != Some(&pick) => keeps.push(pick),
            None if keeps.is_empty() && at_end => {
                if let Some(pick) = pick_least_bad(scores, window_start, i) {
                    keeps.push(pick);
                }
            }
            _ => {}
        }
        window_start = i + 1;
        acc = 0.0;
    }
    keeps
}

fn pick_sharpest(
    scores: &[CandidateScore],
    start: usize,
    end: usize,
    blur_floor: f32,
) -> Option<usize> {
    scores
        .iter()
        .enumerate()
        .take(end + 1)
        .skip(start)
        .filter(|(_, score)| score.sharpness >= blur_floor)
        .max_by(|a, b| a.1.sharpness.total_cmp(&b.1.sharpness))
        .map(|(index, _)| index)
}

fn pick_least_bad(scores: &[CandidateScore], start: usize, end: usize) -> Option<usize> {
    scores
        .iter()
        .enumerate()
        .take(end + 1)
        .skip(start)
        .max_by(|a, b| a.1.sharpness.total_cmp(&b.1.sharpness))
        .map(|(index, _)| index)
}

fn thin_to_max(mut keeps: Vec<usize>, scores: &[CandidateScore], max_keep: usize) -> Vec<usize> {
    keeps.sort_unstable();
    keeps.dedup();
    while keeps.len() > max_keep {
        let mut pair = 0;
        let mut best_gap = usize::MAX;
        for i in 0..keeps.len() - 1 {
            let gap = keeps[i + 1] - keeps[i];
            if gap < best_gap {
                best_gap = gap;
                pair = i;
            }
        }
        let a = keeps[pair];
        let b = keeps[pair + 1];
        let drop = if scores[a].sharpness >= scores[b].sharpness {
            pair + 1
        } else {
            pair
        };
        keeps.remove(drop);
    }
    keeps
}

fn fill_sharpest(
    mut keeps: Vec<usize>,
    scores: &[CandidateScore],
    min_keep: usize,
    blur_floor: f32,
) -> Vec<usize> {
    let mut unused: Vec<usize> = (0..scores.len())
        .filter(|index| !keeps.contains(index))
        .collect();
    unused.sort_by(|a, b| scores[*b].sharpness.total_cmp(&scores[*a].sharpness));
    for index in unused
        .iter()
        .copied()
        .filter(|&i| scores[i].sharpness >= blur_floor)
    {
        if keeps.len() >= min_keep {
            break;
        }
        keeps.push(index);
    }
    if keeps.len() < min_keep {
        for index in unused {
            if keeps.len() >= min_keep {
                break;
            }
            if !keeps.contains(&index) {
                keeps.push(index);
            }
        }
    }
    keeps.sort_unstable();
    keeps.dedup();
    keeps
}

fn laplacian_variance(gray: &GrayImage) -> f32 {
    let width = gray.width();
    let height = gray.height();
    if width < 3 || height < 3 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut sq = 0.0;
    let mut count = 0.0;
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let center = pixel(gray, x, y);
            let lap = pixel(gray, x, y - 1)
                + pixel(gray, x, y + 1)
                + pixel(gray, x - 1, y)
                + pixel(gray, x + 1, y)
                - 4.0 * center;
            sum += lap;
            sq += lap * lap;
            count += 1.0;
        }
    }
    if count == 0.0 {
        return 0.0;
    }
    let mean = sum / count;
    (sq / count) - mean * mean
}

fn mean_abs_diff(left: &GrayImage, right: &GrayImage) -> f32 {
    if left.dimensions() != right.dimensions() {
        return 255.0;
    }
    let mut sum = 0.0;
    let n = left.len() as f32;
    if n == 0.0 {
        return 0.0;
    }
    for (a, b) in left.pixels().zip(right.pixels()) {
        sum += (a[0] as f32 - b[0] as f32).abs();
    }
    sum / n
}

fn pixel(gray: &GrayImage, x: u32, y: u32) -> f32 {
    f32::from(gray.get_pixel(x, y)[0])
}

#[cfg(test)]
fn scores(motion: &[f32], sharpness: &[f32]) -> Vec<CandidateScore> {
    motion
        .iter()
        .zip(sharpness)
        .map(|(&motion, &sharpness)| CandidateScore { sharpness, motion })
        .collect()
}

#[cfg(test)]
fn constant(len: usize, motion: f32, sharpness: f32) -> Vec<CandidateScore> {
    (0..len)
        .map(|i| CandidateScore {
            sharpness,
            motion: if i == 0 { 0.0 } else { motion },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use tempfile::tempdir;

    fn cfg(min_keep: usize, max_keep: usize, motion_threshold: f32) -> KeyframeConfig {
        KeyframeConfig {
            min_keep,
            max_keep,
            motion_threshold,
            blur_floor: 10.0,
        }
    }

    #[test]
    fn pause_keeps_one_frame() {
        let picked = select_keyframes(&constant(80, 0.0, 50.0), cfg(1, 120, 64.0));
        assert_eq!(picked.len(), 1);
    }

    #[test]
    fn fast_pan_keeps_more_than_target_density() {
        let target_fps = 1.0;
        let duration = 10.0;
        let n = 80;
        let picked = select_keyframes(
            &constant(n, 32.0, 50.0),
            cfg(8, 800, motion_threshold(target_fps)),
        );
        assert!(
            picked.len() as f32 > target_fps * duration,
            "fast pan should densify, got {}",
            picked.len()
        );
    }

    #[test]
    fn blur_spike_on_one_hertz_tick_is_skipped() {
        let mut data = constant(24, 8.0, 80.0);
        for tick in [0, 8, 16] {
            data[tick].sharpness = 1.0;
        }
        let picked = select_keyframes(&data, cfg(1, 24, motion_threshold(1.0)));
        for tick in [0usize, 8, 16] {
            assert!(
                !picked.contains(&tick),
                "kept blurry tick {tick} in {picked:?}"
            );
        }
        assert!(!picked.is_empty());
    }

    #[test]
    fn max_keep_is_a_hard_cap_and_stays_spread() {
        let picked = select_keyframes(&constant(200, 40.0, 50.0), cfg(8, 40, 8.0));
        assert_eq!(picked.len(), 40);
        assert!(picked[0] < 20, "first keep should stay near the start");
        assert!(
            *picked.last().unwrap() > 170,
            "last keep should stay near the end"
        );
    }

    #[test]
    fn below_min_keep_fills_instead_of_failing() {
        let picked = select_keyframes(&constant(20, 0.0, 50.0), cfg(8, 20, 64.0));
        assert_eq!(picked.len(), 8);
    }

    #[test]
    fn fewer_candidates_than_min_keep_returns_all() {
        let picked = select_keyframes(&constant(5, 0.0, 50.0), cfg(8, 20, 64.0));
        assert_eq!(picked, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn candidate_fps_is_at_least_four_times_target() {
        assert_eq!(candidate_fps(1.0), 8.0);
        assert_eq!(candidate_fps(2.0), 8.0);
        assert_eq!(candidate_fps(4.0), 12.0);
    }

    #[test]
    fn checkerboard_is_sharper_than_flat_gray() {
        let mut sharp = GrayImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                sharp.put_pixel(x, y, Luma([v]));
            }
        }
        let flat = GrayImage::from_pixel(32, 32, Luma([128]));
        assert!(laplacian_variance(&sharp) > laplacian_variance(&flat) + 100.0);
    }

    #[test]
    fn identical_frames_have_zero_motion() {
        let frame = GrayImage::from_pixel(8, 8, Luma([40]));
        assert_eq!(mean_abs_diff(&frame, &frame), 0.0);
    }

    #[test]
    fn shifted_luma_has_nonzero_motion() {
        let a = GrayImage::from_pixel(8, 8, Luma([10]));
        let b = GrayImage::from_pixel(8, 8, Luma([40]));
        assert!((mean_abs_diff(&a, &b) - 30.0).abs() < 0.001);
    }

    #[test]
    fn score_candidates_follows_sorted_names() {
        let dir = tempdir().unwrap();
        let mut first = RgbImage::new(16, 16);
        let mut second = RgbImage::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                first.put_pixel(x, y, Rgb([v, v, v]));
                second.put_pixel(x, y, Rgb([128, 128, 128]));
            }
        }
        first.save(dir.path().join("frame_00002.jpg")).unwrap();
        second.save(dir.path().join("frame_00001.jpg")).unwrap();
        let scored = score_candidates(dir.path()).unwrap();
        assert_eq!(scored.len(), 2);
        assert!(scored[1].sharpness > scored[0].sharpness);
        assert!(scored[1].motion > 0.0);
    }

    #[test]
    fn scores_helper_preserves_pairs() {
        let data = scores(&[0.0, 4.0], &[10.0, 20.0]);
        assert_eq!(data[1].motion, 4.0);
        assert_eq!(data[1].sharpness, 20.0);
    }
}
