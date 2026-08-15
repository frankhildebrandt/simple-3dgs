import { describe, expect, it } from "vitest";
import { LOD_ABOVE, ROOM_MIN_ALPHA, SPARK_MIN_ALPHA, viewerProfile } from "./viewerProfile";

describe("viewerProfile", () => {
  it("builds LoD for objects only above a splat threshold", () => {
    const profile = viewerProfile("object");
    expect(profile.lod).toBe(true);
    expect(profile.lodAbove).toBe(LOD_ABOVE);
    expect(profile.lodRenderScale).toBe(1);
    expect(profile.behindFoveate).toBe(0.2);
    expect(profile.minAlpha).toBe(SPARK_MIN_ALPHA);
  });

  it("drops faint room floaters and coarsens splats behind the camera", () => {
    const profile = viewerProfile("room");
    expect(profile.behindFoveate).toBe(0.1);
    expect(profile.lodRenderScale).toBe(1.5);
    expect(profile.minAlpha).toBe(ROOM_MIN_ALPHA);
    expect(profile.minAlpha).toBeGreaterThan(SPARK_MIN_ALPHA);
  });

  it("uses a coarser LoD floor outdoors", () => {
    const profile = viewerProfile("outdoor");
    expect(profile.behindFoveate).toBe(0.1);
    expect(profile.lodRenderScale).toBe(2);
    expect(profile.minAlpha).toBe(SPARK_MIN_ALPHA);
  });
});
