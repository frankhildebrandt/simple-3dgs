import { describe, expect, it } from "vitest";
import { MIN_PIXEL_RADIUS, OBJECT_MAX_STD_DEV, viewerProfile } from "./viewerProfile";
import {
  DISC_FALLOFF,
  DOT_MAX_PIXEL_RADIUS,
  DOT_MAX_STD_DEV,
  DOT_MIN_PIXEL_RADIUS,
  SPLAT_FALLOFF,
  SPLAT_MAX_PIXEL_RADIUS,
  applyViewerMode,
  nextViewerMode,
  viewerModeLabel,
  viewerModeTuning,
} from "./viewerMode";

describe("nextViewerMode", () => {
  it("cycles splats to dots to discs and back", () => {
    expect(nextViewerMode("splats")).toBe("dots");
    expect(nextViewerMode("dots")).toBe("discs");
    expect(nextViewerMode("discs")).toBe("splats");
  });
});

describe("viewerModeLabel", () => {
  it("names the current mode for the cycle button", () => {
    expect(viewerModeLabel("splats")).toBe("Splats");
    expect(viewerModeLabel("dots")).toBe("Dots");
    expect(viewerModeLabel("discs")).toBe("Discs");
  });
});

describe("viewerModeTuning", () => {
  const profile = viewerProfile("object");

  it("keeps profile Gaussian extent and Spark falloff for splats", () => {
    expect(viewerModeTuning("splats", profile)).toEqual({
      maxStdDev: OBJECT_MAX_STD_DEV,
      minPixelRadius: MIN_PIXEL_RADIUS,
      maxPixelRadius: SPLAT_MAX_PIXEL_RADIUS,
      falloff: SPLAT_FALLOFF,
    });
  });

  it("clamps dots to a small solid pixel radius", () => {
    const tuning = viewerModeTuning("dots", profile);
    expect(tuning.maxStdDev).toBe(DOT_MAX_STD_DEV);
    expect(tuning.maxStdDev).toBe(0.15);
    expect(tuning.minPixelRadius).toBe(DOT_MIN_PIXEL_RADIUS);
    expect(tuning.minPixelRadius).toBe(1.5);
    expect(tuning.maxPixelRadius).toBe(DOT_MAX_PIXEL_RADIUS);
    expect(tuning.maxPixelRadius).toBe(2);
    expect(tuning.falloff).toBe(DISC_FALLOFF);
    expect(tuning.maxStdDev).toBeLessThan(profile.maxStdDev);
    expect(tuning.maxPixelRadius).toBeLessThan(SPLAT_MAX_PIXEL_RADIUS);
  });

  it("keeps profile extent for discs and only drops Gaussian falloff", () => {
    const tuning = viewerModeTuning("discs", profile);
    expect(tuning.maxStdDev).toBe(profile.maxStdDev);
    expect(tuning.minPixelRadius).toBe(MIN_PIXEL_RADIUS);
    expect(tuning.maxPixelRadius).toBe(SPLAT_MAX_PIXEL_RADIUS);
    expect(tuning.falloff).toBe(DISC_FALLOFF);
    expect(tuning.falloff).toBe(0);
  });

  it("uses outdoor maxStdDev for splats and discs", () => {
    const outdoor = viewerProfile("outdoor");
    expect(viewerModeTuning("splats", outdoor).maxStdDev).toBe(outdoor.maxStdDev);
    expect(viewerModeTuning("discs", outdoor).maxStdDev).toBe(outdoor.maxStdDev);
    expect(viewerModeTuning("dots", outdoor).maxStdDev).toBe(DOT_MAX_STD_DEV);
  });
});

describe("applyViewerMode", () => {
  it("writes the four Spark knobs onto the renderer", () => {
    const spark = {
      maxStdDev: 99,
      minPixelRadius: 99,
      maxPixelRadius: 99,
      falloff: 99,
    };
    applyViewerMode(spark, "dots", viewerProfile("room"));
    expect(spark).toEqual({
      maxStdDev: DOT_MAX_STD_DEV,
      minPixelRadius: DOT_MIN_PIXEL_RADIUS,
      maxPixelRadius: DOT_MAX_PIXEL_RADIUS,
      falloff: DISC_FALLOFF,
    });
  });
});
