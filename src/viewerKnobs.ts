import type { CaptureMode } from "./captureMode";

export const LOD_ABOVE = 100_000;
export const WEBVIEW_LOD_SPLAT_COUNT = 1_500_000;
export const SPARK_MIN_ALPHA = 0.5 / 255;
export const ROOM_MIN_ALPHA = 2 / 255;
export const MIN_PIXEL_RADIUS = 1;
export const MIN_SORT_INTERVAL_MS = 8;
export const OBJECT_MAX_STD_DEV = Math.sqrt(5);
export const OUTDOOR_MAX_STD_DEV = Math.sqrt(4);
export const DEFAULT_FOV = 60;
export const MOVE_SPEED_MIN = 0.01;
export const MOVE_SPEED_MAX = 20;

/** Spark / HTML viewer knobs that used to follow capture mode only. */
export type ViewerKnobs = {
  lodAbove: number;
  lodSplatScale: number;
  lodRenderScale: number;
  behindFoveate: number;
  coneFoveate: number;
  webviewLodSplatCount: number;
  minAlpha: number;
  maxStdDev: number;
  clipXY: number;
  minPixelRadius: number;
  minSortIntervalMs: number;
  fov: number;
  moveSpeed: number;
  farMultiplier: number;
};

const SHARED: Pick<
  ViewerKnobs,
  "lodAbove" | "webviewLodSplatCount" | "minPixelRadius" | "minSortIntervalMs" | "fov"
> = {
  lodAbove: LOD_ABOVE,
  webviewLodSplatCount: WEBVIEW_LOD_SPLAT_COUNT,
  minPixelRadius: MIN_PIXEL_RADIUS,
  minSortIntervalMs: MIN_SORT_INTERVAL_MS,
  fov: DEFAULT_FOV,
};

/** Viewer profile that Fast/Balanced/Quality still apply for a capture type. */
export function viewerKnobsFor(mode: CaptureMode): ViewerKnobs {
  switch (mode) {
    case "room":
      return {
        ...SHARED,
        lodSplatScale: 0.7,
        lodRenderScale: 2,
        behindFoveate: 0.1,
        coneFoveate: 0.4,
        minAlpha: ROOM_MIN_ALPHA,
        maxStdDev: OBJECT_MAX_STD_DEV,
        clipXY: 1.2,
        moveSpeed: 0.5,
        farMultiplier: 40,
      };
    case "outdoor":
      return {
        ...SHARED,
        lodSplatScale: 0.5,
        lodRenderScale: 3,
        behindFoveate: 0.1,
        coneFoveate: 0.3,
        minAlpha: SPARK_MIN_ALPHA,
        maxStdDev: OUTDOOR_MAX_STD_DEV,
        clipXY: 1.1,
        moveSpeed: 2,
        farMultiplier: 80,
      };
    case "object":
      return {
        ...SHARED,
        lodSplatScale: 1,
        lodRenderScale: 1.5,
        behindFoveate: 0.2,
        coneFoveate: 0.5,
        minAlpha: SPARK_MIN_ALPHA,
        maxStdDev: OBJECT_MAX_STD_DEV,
        clipXY: 1.2,
        moveSpeed: 0.8,
        farMultiplier: 40,
      };
  }
}
