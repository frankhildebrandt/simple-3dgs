import type { CaptureMode } from "./types";

/** Spark LoD / alpha knobs derived from capture type. No UI sliders. */
export type ViewerProfile = {
  lod: true;
  lodAbove: number;
  behindFoveate: number;
  lodRenderScale: number;
  minAlpha: number;
};

export const LOD_ABOVE = 250_000;
export const SPARK_MIN_ALPHA = 0.5 / 255;
export const ROOM_MIN_ALPHA = 2 / 255;

/**
 * View-dependent splat budget for Spark. LoD drops off-axis and behind-camera
 * splats; rooms raise minAlpha so faint floaters do not show through walls.
 */
export function viewerProfile(mode: CaptureMode): ViewerProfile {
  switch (mode) {
    case "room":
      return {
        lod: true,
        lodAbove: LOD_ABOVE,
        behindFoveate: 0.1,
        lodRenderScale: 1.5,
        minAlpha: ROOM_MIN_ALPHA,
      };
    case "outdoor":
      return {
        lod: true,
        lodAbove: LOD_ABOVE,
        behindFoveate: 0.1,
        lodRenderScale: 2,
        minAlpha: SPARK_MIN_ALPHA,
      };
    case "object":
      return {
        lod: true,
        lodAbove: LOD_ABOVE,
        behindFoveate: 0.2,
        lodRenderScale: 1,
        minAlpha: SPARK_MIN_ALPHA,
      };
  }
}
