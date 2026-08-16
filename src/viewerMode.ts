import type { ViewerProfile } from "./viewerProfile";

export type ViewerMode = "splats" | "dots" | "discs";

export const SPLAT_FALLOFF = 1;
export const DISC_FALLOFF = 0;
/** Spark default; full Gaussians may span hundreds of pixels up close. */
export const SPLAT_MAX_PIXEL_RADIUS = 512;
export const DOT_MAX_STD_DEV = 0.15;
export const DOT_MIN_PIXEL_RADIUS = 1.5;
export const DOT_MAX_PIXEL_RADIUS = 2;

export type ViewerModeTuning = {
  maxStdDev: number;
  minPixelRadius: number;
  maxPixelRadius: number;
  falloff: number;
};

export type SparkModeTarget = {
  maxStdDev: number;
  minPixelRadius: number;
  maxPixelRadius: number;
  falloff: number;
};

export const VIEWER_MODES: ViewerMode[] = ["splats", "dots", "discs"];

const MODE_LABEL: Record<ViewerMode, string> = {
  splats: "Splats",
  dots: "Dots",
  discs: "Discs",
};

/** Returns the next display mode in Splats → Dots → Discs. */
export function nextViewerMode(mode: ViewerMode): ViewerMode {
  return VIEWER_MODES[(VIEWER_MODES.indexOf(mode) + 1) % VIEWER_MODES.length];
}

/** Button label for the current display mode. */
export function viewerModeLabel(mode: ViewerMode): string {
  return MODE_LABEL[mode];
}

/**
 * Spark extent / falloff for a display mode. Splats keep the capture profile;
 * dots clamp to a solid pixel; discs keep extent and drop the Gaussian kernel.
 */
export function viewerModeTuning(mode: ViewerMode, profile: ViewerProfile): ViewerModeTuning {
  switch (mode) {
    case "dots":
      return {
        maxStdDev: DOT_MAX_STD_DEV,
        minPixelRadius: DOT_MIN_PIXEL_RADIUS,
        maxPixelRadius: DOT_MAX_PIXEL_RADIUS,
        falloff: DISC_FALLOFF,
      };
    case "discs":
      return {
        maxStdDev: profile.maxStdDev,
        minPixelRadius: profile.minPixelRadius,
        maxPixelRadius: SPLAT_MAX_PIXEL_RADIUS,
        falloff: DISC_FALLOFF,
      };
    case "splats":
      return {
        maxStdDev: profile.maxStdDev,
        minPixelRadius: profile.minPixelRadius,
        maxPixelRadius: SPLAT_MAX_PIXEL_RADIUS,
        falloff: SPLAT_FALLOFF,
      };
  }
}

/** Copies display-mode knobs onto a SparkRenderer (or a test double). */
export function applyViewerMode(
  spark: SparkModeTarget,
  mode: ViewerMode,
  profile: ViewerProfile,
): void {
  Object.assign(spark, viewerModeTuning(mode, profile));
}
