import { describe, expect, it } from "vitest";
import { viewerPixelRatio, viewerScaleForSession } from "./viewerPixelRatio";

describe("viewerPixelRatio", () => {
  it("keeps Fast at one CSS pixel so the canvas upsamples", () => {
    expect(viewerPixelRatio("fast", 2)).toBe(1);
    expect(viewerPixelRatio("fast", 1)).toBe(1);
  });

  it("caps Sharp at 2 on Retina", () => {
    expect(viewerPixelRatio("sharp", 2)).toBe(2);
    expect(viewerPixelRatio("sharp", 3)).toBe(2);
  });

  it("does not drop Sharp below 1", () => {
    expect(viewerPixelRatio("sharp", 0.5)).toBe(1);
  });
});

describe("viewerScaleForSession", () => {
  it("forces Fast while training", () => {
    expect(viewerScaleForSession("sharp", true)).toBe("fast");
    expect(viewerScaleForSession("fast", true)).toBe("fast");
  });

  it("keeps the chosen scale when not live", () => {
    expect(viewerScaleForSession("sharp", false)).toBe("sharp");
    expect(viewerScaleForSession("fast", false)).toBe("fast");
  });
});
