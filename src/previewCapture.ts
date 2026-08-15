const JPEG_DATA_URL = "data:image/jpeg;base64,";
export const PREVIEW_MAX_WIDTH = 1280;
export const PREVIEW_JPEG_QUALITY = 0.85;

/** Strips a JPEG data URL to raw base64, or null when the payload is missing. */
export function jpegBase64FromDataUrl(dataUrl: string): string | null {
  if (!dataUrl.startsWith(JPEG_DATA_URL)) {
    return null;
  }
  const payload = dataUrl.slice(JPEG_DATA_URL.length).replace(/\s/g, "");
  return payload.length > 0 ? payload : null;
}

/** Caps preview width while keeping aspect ratio. */
export function previewSize(
  width: number,
  height: number,
  maxWidth = PREVIEW_MAX_WIDTH,
): { width: number; height: number } {
  if (width <= 0 || height <= 0) {
    return { width: 1, height: 1 };
  }
  if (width <= maxWidth) {
    return { width, height };
  }
  return {
    width: maxWidth,
    height: Math.max(1, Math.round((height * maxWidth) / width)),
  };
}

type JpegCanvas = {
  width: number;
  height: number;
  toDataURL: (type?: string, quality?: number) => string;
};

/** Encodes a rendered WebGL canvas as JPEG base64, scaling down when wider than maxWidth. */
export function jpegBase64FromCanvas(
  canvas: JpegCanvas,
  drawScaled: (source: JpegCanvas, width: number, height: number) => string | null,
  maxWidth = PREVIEW_MAX_WIDTH,
  quality = PREVIEW_JPEG_QUALITY,
): string | null {
  const size = previewSize(canvas.width, canvas.height, maxWidth);
  if (size.width === canvas.width) {
    return jpegBase64FromDataUrl(canvas.toDataURL("image/jpeg", quality));
  }
  const scaled = drawScaled(canvas, size.width, size.height);
  if (scaled) {
    return jpegBase64FromDataUrl(scaled);
  }
  return jpegBase64FromDataUrl(canvas.toDataURL("image/jpeg", quality));
}

/** Draws `source` into a 2D canvas of `width`×`height` and returns a JPEG data URL. */
export function scaledJpegDataUrl(
  source: CanvasImageSource,
  width: number,
  height: number,
  quality = PREVIEW_JPEG_QUALITY,
): string | null {
  const offscreen = document.createElement("canvas");
  offscreen.width = width;
  offscreen.height = height;
  const ctx = offscreen.getContext("2d");
  if (!ctx) {
    return null;
  }
  ctx.drawImage(source, 0, 0, width, height);
  return offscreen.toDataURL("image/jpeg", quality);
}
