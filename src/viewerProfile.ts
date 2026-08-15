import type { CaptureMode } from "./types";

/** Spark LoD / alpha knobs derived from capture type. No UI sliders. */
export type ViewerProfile = {
  lod: true;
  lodAbove: number;
  lodSplatScale: number;
  lodRenderScale: number;
  behindFoveate: number;
  coneFoveate: number;
  minAlpha: number;
  maxStdDev: number;
  clipXY: number;
};

export const LOD_ABOVE = 100_000;
/** Spark skips tiny_lod when splat count is below this; live checkpoints never reach it. */
export const LIVE_LOD_ABOVE = 1_000_000_000;
export const SPARK_MIN_ALPHA = 0.5 / 255;
export const ROOM_MIN_ALPHA = 2 / 255;
export const MIN_PIXEL_RADIUS = 1;
export const MIN_SORT_INTERVAL_MS = 8;
export const OBJECT_MAX_STD_DEV = Math.sqrt(5);
export const OUTDOOR_MAX_STD_DEV = Math.sqrt(4);

export type SparkTuning = {
  minAlpha: number;
  lodSplatScale: number;
  lodRenderScale: number;
  behindFoveate: number;
  coneFoveate: number;
  maxStdDev: number;
  clipXY: number;
  minPixelRadius: number;
  minSortIntervalMs: number;
};

/**
 * View-dependent splat budget for Spark. LoD drops off-axis and behind-camera
 * splats; rooms raise minAlpha so faint floaters do not show through walls.
 * Live previews keep lod on and raise lodAbove so Spark skips the WASM tree.
 */
export function viewerProfile(mode: CaptureMode, live = false): ViewerProfile {
  const lodAbove = live ? LIVE_LOD_ABOVE : LOD_ABOVE;
  switch (mode) {
    case "room":
      return {
        lod: true,
        lodAbove,
        lodSplatScale: 0.7,
        lodRenderScale: 2,
        behindFoveate: 0.1,
        coneFoveate: 0.4,
        minAlpha: ROOM_MIN_ALPHA,
        maxStdDev: OBJECT_MAX_STD_DEV,
        clipXY: 1.2,
      };
    case "outdoor":
      return {
        lod: true,
        lodAbove,
        lodSplatScale: 0.5,
        lodRenderScale: 3,
        behindFoveate: 0.1,
        coneFoveate: 0.3,
        minAlpha: SPARK_MIN_ALPHA,
        maxStdDev: OUTDOOR_MAX_STD_DEV,
        clipXY: 1.1,
      };
    case "object":
      return {
        lod: true,
        lodAbove,
        lodSplatScale: 1,
        lodRenderScale: 1.5,
        behindFoveate: 0.2,
        coneFoveate: 0.5,
        minAlpha: SPARK_MIN_ALPHA,
        maxStdDev: OBJECT_MAX_STD_DEV,
        clipXY: 1.2,
      };
  }
}

/** SparkRenderer fields that stay in sync with `viewerProfile`. */
export function sparkTuning(profile: ViewerProfile): SparkTuning {
  return {
    minAlpha: profile.minAlpha,
    lodSplatScale: profile.lodSplatScale,
    lodRenderScale: profile.lodRenderScale,
    behindFoveate: profile.behindFoveate,
    coneFoveate: profile.coneFoveate,
    maxStdDev: profile.maxStdDev,
    clipXY: profile.clipXY,
    minPixelRadius: MIN_PIXEL_RADIUS,
    minSortIntervalMs: MIN_SORT_INTERVAL_MS,
  };
}
