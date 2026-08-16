import type { CaptureMode } from "./captureMode";
import {
  DEFAULT_FOV,
  LOD_ABOVE,
  MIN_PIXEL_RADIUS,
  MIN_SORT_INTERVAL_MS,
  OBJECT_MAX_STD_DEV,
  OUTDOOR_MAX_STD_DEV,
  ROOM_MIN_ALPHA,
  SPARK_MIN_ALPHA,
  type ViewerKnobs,
  viewerKnobsFor,
} from "./viewerKnobs";

export {
  DEFAULT_FOV,
  LOD_ABOVE,
  MIN_PIXEL_RADIUS,
  MIN_SORT_INTERVAL_MS,
  OBJECT_MAX_STD_DEV,
  OUTDOOR_MAX_STD_DEV,
  ROOM_MIN_ALPHA,
  SPARK_MIN_ALPHA,
};

/** Spark skips tiny_lod when splat count is below this; live checkpoints never reach it. */
export const LIVE_LOD_ABOVE = 1_000_000_000;

/** Spark LoD / alpha knobs, including values that Custom can override. */
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
  minPixelRadius: number;
  minSortIntervalMs: number;
  fov: number;
  moveSpeed: number;
  farMultiplier: number;
};

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

/** SplatMesh decode flags. nonLod keeps packed originals; live skips LoD drive. */
export type SplatLoadFlags = {
  lod: true;
  nonLod: true;
  lodAbove: number;
  enableLod: boolean;
  raycastable: false;
};

/** Builds a Spark profile from stored knobs. Live previews raise lodAbove. */
export function viewerProfileFromKnobs(knobs: ViewerKnobs, live = false): ViewerProfile {
  return {
    lod: true,
    lodAbove: live ? LIVE_LOD_ABOVE : knobs.lodAbove,
    lodSplatScale: knobs.lodSplatScale,
    lodRenderScale: knobs.lodRenderScale,
    behindFoveate: knobs.behindFoveate,
    coneFoveate: knobs.coneFoveate,
    minAlpha: knobs.minAlpha,
    maxStdDev: knobs.maxStdDev,
    clipXY: knobs.clipXY,
    minPixelRadius: knobs.minPixelRadius,
    minSortIntervalMs: knobs.minSortIntervalMs,
    fov: knobs.fov,
    moveSpeed: knobs.moveSpeed,
    farMultiplier: knobs.farMultiplier,
  };
}

/**
 * View-dependent splat budget for Spark. LoD drops off-axis and behind-camera
 * splats; rooms raise minAlpha so faint floaters do not show through walls.
 * Live previews keep lod on and raise lodAbove so Spark skips the WASM tree.
 */
export function viewerProfile(mode: CaptureMode, live = false): ViewerProfile {
  return viewerProfileFromKnobs(viewerKnobsFor(mode), live);
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
    minPixelRadius: profile.minPixelRadius,
    minSortIntervalMs: profile.minSortIntervalMs,
  };
}

/**
 * SplatMesh decode flags matching the HTML export. nonLod keeps packed
 * originals so live previews stay visible when the renderer is not driving LoD.
 */
export function splatLoadFlags(profile: ViewerProfile, live = false): SplatLoadFlags {
  return {
    lod: true,
    nonLod: true,
    lodAbove: profile.lodAbove,
    enableLod: !live,
    raycastable: false,
  };
}
