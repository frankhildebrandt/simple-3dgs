import { describe, expect, it, vi } from "vitest";
import { jpegBase64FromCanvas, jpegBase64FromDataUrl, previewSize } from "./previewCapture";

describe("jpegBase64FromDataUrl", () => {
  it("strips a JPEG data URL", () => {
    expect(jpegBase64FromDataUrl("data:image/jpeg;base64,abc123")).toBe("abc123");
  });

  it("rejects empty or non-JPEG payloads", () => {
    expect(jpegBase64FromDataUrl("data:image/jpeg;base64,")).toBeNull();
    expect(jpegBase64FromDataUrl("data:image/png;base64,abc")).toBeNull();
    expect(jpegBase64FromDataUrl("abc123")).toBeNull();
  });
});

describe("previewSize", () => {
  it("keeps sizes at or below the cap", () => {
    expect(previewSize(800, 600)).toEqual({ width: 800, height: 600 });
    expect(previewSize(1280, 720)).toEqual({ width: 1280, height: 720 });
  });

  it("scales down wide frames", () => {
    expect(previewSize(2560, 1440)).toEqual({ width: 1280, height: 720 });
  });
});

describe("jpegBase64FromCanvas", () => {
  it("uses the source canvas when already small enough", () => {
    const canvas = {
      width: 640,
      height: 360,
      toDataURL: vi.fn(() => "data:image/jpeg;base64,small"),
    };
    const drawScaled = vi.fn();
    expect(jpegBase64FromCanvas(canvas, drawScaled)).toBe("small");
    expect(drawScaled).not.toHaveBeenCalled();
  });

  it("uses a scaled data URL when wider than the cap", () => {
    const canvas = {
      width: 2560,
      height: 1440,
      toDataURL: vi.fn(() => "data:image/jpeg;base64,full"),
    };
    const drawScaled = vi.fn(() => "data:image/jpeg;base64,scaled");
    expect(jpegBase64FromCanvas(canvas, drawScaled)).toBe("scaled");
    expect(drawScaled).toHaveBeenCalledWith(canvas, 1280, 720);
  });

  it("falls back to the full canvas when scaling fails", () => {
    const canvas = {
      width: 2560,
      height: 1440,
      toDataURL: vi.fn(() => "data:image/jpeg;base64,full"),
    };
    expect(jpegBase64FromCanvas(canvas, () => null)).toBe("full");
  });
});
