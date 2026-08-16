//! Brush CLI knobs. Zero refine/export cadence keeps the old frame/step heuristic.

use serde::{Deserialize, Serialize};

/// Training hyperparameters forwarded to `brush-cli`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrushKnobs {
    pub lr_mean: f32,
    pub lr_mean_end: f32,
    pub mean_noise_weight: f32,
    pub lr_coeffs_dc: f32,
    pub lr_coeffs_sh_scale: f32,
    pub lr_opac: f32,
    pub lr_scale: f32,
    pub lr_rotation: f32,
    pub ssim_weight: f32,
    pub opac_decay: f32,
    pub lpips_loss_weight: f32,
    pub match_alpha_weight: f32,
    pub background_r: f32,
    pub background_g: f32,
    pub background_b: f32,
    pub background_noise_strength: f32,
    pub refine_every: u32,
    pub growth_grad_threshold: f32,
    pub growth_select_fraction: f32,
    pub growth_stop_iter: u32,
    pub split_at_screen_size: f32,
    pub sh_degree: u32,
    pub seed: u32,
    pub eval_every: u32,
    pub export_every: u32,
    pub max_scene_batch_cache_gib: f32,
    pub lod_levels: u32,
    pub lod_refine_steps: u32,
    pub lod_decimation_keep: u32,
    pub lod_image_scale: u32,
    /// Cap for `--max-resolution`. `0` means native (omit the flag when extract is native).
    pub train_max_resolution: u32,
}

impl Default for BrushKnobs {
    fn default() -> Self {
        Self {
            lr_mean: 2e-5,
            lr_mean_end: 2e-7,
            mean_noise_weight: 50.0,
            lr_coeffs_dc: 2e-3,
            lr_coeffs_sh_scale: 10.0,
            lr_opac: 0.012,
            lr_scale: 5e-3,
            lr_rotation: 2e-3,
            ssim_weight: 0.2,
            opac_decay: 0.004,
            lpips_loss_weight: 0.0,
            match_alpha_weight: 0.1,
            background_r: 0.0,
            background_g: 0.0,
            background_b: 0.0,
            background_noise_strength: 0.1,
            refine_every: 0,
            growth_grad_threshold: 0.0025,
            growth_select_fraction: 0.25,
            growth_stop_iter: 15_000,
            split_at_screen_size: 0.5,
            sh_degree: 3,
            seed: 42,
            eval_every: 1000,
            export_every: 0,
            max_scene_batch_cache_gib: 6.0,
            lod_levels: 0,
            lod_refine_steps: 5000,
            lod_decimation_keep: 50,
            lod_image_scale: 50,
            train_max_resolution: 1920,
        }
    }
}

impl BrushKnobs {
    /// Clamps Brush knobs to ranges the CLI can survive.
    pub fn sanitized(self) -> Self {
        Self {
            lr_mean: self.lr_mean.max(0.0),
            lr_mean_end: self.lr_mean_end.max(0.0),
            mean_noise_weight: self.mean_noise_weight.max(0.0),
            lr_coeffs_dc: self.lr_coeffs_dc.max(0.0),
            lr_coeffs_sh_scale: self.lr_coeffs_sh_scale.max(0.0),
            lr_opac: self.lr_opac.max(0.0),
            lr_scale: self.lr_scale.max(0.0),
            lr_rotation: self.lr_rotation.max(0.0),
            ssim_weight: self.ssim_weight.clamp(0.0, 1.0),
            opac_decay: self.opac_decay.max(0.0),
            lpips_loss_weight: self.lpips_loss_weight.max(0.0),
            match_alpha_weight: self.match_alpha_weight.max(0.0),
            background_r: self.background_r.clamp(0.0, 1.0),
            background_g: self.background_g.clamp(0.0, 1.0),
            background_b: self.background_b.clamp(0.0, 1.0),
            background_noise_strength: self.background_noise_strength.max(0.0),
            refine_every: self.refine_every.min(100_000),
            growth_grad_threshold: self.growth_grad_threshold.max(0.0),
            growth_select_fraction: self.growth_select_fraction.clamp(0.0, 1.0),
            growth_stop_iter: self.growth_stop_iter.min(1_000_000),
            split_at_screen_size: self.split_at_screen_size.max(0.0),
            sh_degree: self.sh_degree.min(5),
            seed: self.seed,
            eval_every: self.eval_every.max(1),
            export_every: self.export_every.min(1_000_000),
            max_scene_batch_cache_gib: self.max_scene_batch_cache_gib.clamp(0.5, 128.0),
            lod_levels: self.lod_levels.min(16),
            lod_refine_steps: self.lod_refine_steps.min(1_000_000),
            lod_decimation_keep: self.lod_decimation_keep.clamp(1, 100),
            lod_image_scale: self.lod_image_scale.clamp(1, 100),
            train_max_resolution: match self.train_max_resolution {
                0 => 0,
                n => n.clamp(64, 16_384),
            },
        }
    }
}
