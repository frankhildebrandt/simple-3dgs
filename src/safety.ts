/** Crash/OOM bounds. Named presets stay well inside; Custom may use the full range. */

export const SAFETY = {
  fpsMin: 0.05,
  fpsMax: 60,
  maxImageSizeMin: 64,
  maxImageSizeMax: 16384,
  trainStepsMin: 1,
  trainStepsMax: 1_000_000,
  matchOverlapMin: 1,
  matchOverlapMax: 200,
  maxSplatsMin: 1_000,
  maxSplatsMax: 100_000_000,
  maxFramesMin: 1,
  maxFramesMax: 50_000,
} as const;
