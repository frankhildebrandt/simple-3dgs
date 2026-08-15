/** Drawing-buffer scale: Fast upsamples CSS pixels, Sharp uses native Retina. */
export type ViewerScale = "fast" | "sharp";

export const SHARP_PIXEL_RATIO_CAP = 2;

/** Fast is 1 CSS pixel per fragment; Sharp is device pixels, capped at 2. */
export function viewerPixelRatio(scale: ViewerScale, devicePixelRatio: number): number {
  if (scale === "fast") {
    return 1;
  }
  return Math.min(Math.max(devicePixelRatio, 1), SHARP_PIXEL_RATIO_CAP);
}

/** Live training always stays on Fast so the trainer keeps unified memory. */
export function viewerScaleForSession(scale: ViewerScale, live: boolean): ViewerScale {
  return live ? "fast" : scale;
}
