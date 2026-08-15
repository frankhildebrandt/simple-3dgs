import { describe, expect, it } from "vitest";
import {
  LIVE_LOD_ABOVE,
  LOD_ABOVE,
  MIN_PIXEL_RADIUS,
  MIN_SORT_INTERVAL_MS,
  OBJECT_MAX_STD_DEV,
  OUTDOOR_MAX_STD_DEV,
  ROOM_MIN_ALPHA,
  SPARK_MIN_ALPHA,
  sparkTuning,
  splatLoadFlags,
  viewerProfile,
} from "./viewerProfile";

describe("viewerProfile", () => {
  it("builds LoD for objects only above a splat threshold", () => {
    const profile = viewerProfile("object");
    expect(profile.lod).toBe(true);
    expect(profile.lodAbove).toBe(LOD_ABOVE);
    expect(profile.lodAbove).toBe(100_000);
    expect(profile.lodSplatScale).toBe(1);
    expect(profile.lodRenderScale).toBe(1.5);
    expect(profile.behindFoveate).toBe(0.2);
    expect(profile.coneFoveate).toBe(0.5);
    expect(profile.minAlpha).toBe(SPARK_MIN_ALPHA);
    expect(profile.maxStdDev).toBe(OBJECT_MAX_STD_DEV);
    expect(profile.clipXY).toBe(1.2);
  });

  it("drops faint room floaters and coarsens splats behind the camera", () => {
    const profile = viewerProfile("room");
    expect(profile.lodSplatScale).toBe(0.7);
    expect(profile.behindFoveate).toBe(0.1);
    expect(profile.lodRenderScale).toBe(2);
    expect(profile.coneFoveate).toBe(0.4);
    expect(profile.minAlpha).toBe(ROOM_MIN_ALPHA);
    expect(profile.minAlpha).toBeGreaterThan(SPARK_MIN_ALPHA);
    expect(profile.maxStdDev).toBe(OBJECT_MAX_STD_DEV);
    expect(profile.clipXY).toBe(1.2);
  });

  it("uses a coarser LoD floor outdoors", () => {
    const profile = viewerProfile("outdoor");
    expect(profile.lodSplatScale).toBe(0.5);
    expect(profile.behindFoveate).toBe(0.1);
    expect(profile.lodRenderScale).toBe(3);
    expect(profile.coneFoveate).toBe(0.3);
    expect(profile.minAlpha).toBe(SPARK_MIN_ALPHA);
    expect(profile.maxStdDev).toBe(OUTDOOR_MAX_STD_DEV);
    expect(profile.clipXY).toBe(1.1);
  });

  it("keeps lod on and raises lodAbove while a live training preview is showing", () => {
    expect(viewerProfile("object", true).lod).toBe(true);
    expect(viewerProfile("room", true).lod).toBe(true);
    expect(viewerProfile("object", true).lodAbove).toBe(LIVE_LOD_ABOVE);
    expect(viewerProfile("object", true).lodAbove).toBe(1_000_000_000);
    expect(viewerProfile("object", true).lodAbove).toBeGreaterThan(LOD_ABOVE);
    expect(viewerProfile("object", true).lodAbove).toBeLessThan(2 ** 31);
  });
});

describe("sparkTuning", () => {
  it("copies LoD knobs and the shared pixel/sort floors", () => {
    const tuning = sparkTuning(viewerProfile("outdoor"));
    expect(tuning.lodSplatScale).toBe(0.5);
    expect(tuning.lodRenderScale).toBe(3);
    expect(tuning.coneFoveate).toBe(0.3);
    expect(tuning.minPixelRadius).toBe(MIN_PIXEL_RADIUS);
    expect(tuning.minSortIntervalMs).toBe(MIN_SORT_INTERVAL_MS);
    expect(tuning.maxStdDev).toBe(OUTDOOR_MAX_STD_DEV);
    expect(tuning.clipXY).toBe(1.1);
  });
});

describe("splatLoadFlags", () => {
  it("keeps packed originals and drives LoD after training", () => {
    const flags = splatLoadFlags(viewerProfile("object"));
    expect(flags).toEqual({
      lod: true,
      nonLod: true,
      lodAbove: LOD_ABOVE,
      enableLod: true,
      raycastable: false,
    });
  });

  it("keeps packed originals and skips LoD drive during a live preview", () => {
    const flags = splatLoadFlags(viewerProfile("object", true), true);
    expect(flags.lod).toBe(true);
    expect(flags.nonLod).toBe(true);
    expect(flags.lodAbove).toBe(LIVE_LOD_ABOVE);
    expect(flags.enableLod).toBe(false);
    expect(flags.raycastable).toBe(false);
  });
});
