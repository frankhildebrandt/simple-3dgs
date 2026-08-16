import { describe, expect, it } from "vitest";
import { DEFAULT_BRUSH_KNOBS } from "./brushKnobs";
import { colmapKnobsFor } from "./colmapKnobs";
import { defaultsFor } from "./knobDefaults";
import {
  applyCaptureMode,
  applyNamedPreset,
  hydrateSettings,
  matchingPreset,
  maxFramesCap,
  PRESET_SETTINGS,
} from "./types";
import { viewerKnobsFor } from "./viewerKnobs";

describe("matchingPreset", () => {
  it("matches balanced including maxFrames", () => {
    expect(matchingPreset(PRESET_SETTINGS.balanced)).toBe("balanced");
  });

  it("becomes custom when maxFrames changes", () => {
    expect(
      matchingPreset({ ...PRESET_SETTINGS.balanced, maxFrames: 400 }),
    ).toBe("custom");
  });

  it("is custom in change extract mode", () => {
    expect(
      matchingPreset({ ...PRESET_SETTINGS.balanced, extractMode: "change" }),
    ).toBe("custom");
  });

  it("stays balanced when clip trim differs", () => {
    expect(
      matchingPreset({ ...PRESET_SETTINGS.balanced, startSeconds: 3, durationSeconds: 12 }),
    ).toBe("balanced");
  });

  it("stays balanced on room capture with room defaults", () => {
    expect(
      matchingPreset({
        ...PRESET_SETTINGS.balanced,
        captureMode: "room",
        ...defaultsFor("room"),
      }),
    ).toBe("balanced");
  });

  it("is custom when a brush knob diverges", () => {
    expect(
      matchingPreset({
        ...PRESET_SETTINGS.balanced,
        brush: { ...DEFAULT_BRUSH_KNOBS, shDegree: 2 },
      }),
    ).toBe("custom");
  });
});

describe("defaultsFor", () => {
  it("gives room a different COLMAP and viewer profile than object", () => {
    const object = defaultsFor("object");
    const room = defaultsFor("room");
    expect(room.colmap).not.toEqual(object.colmap);
    expect(room.viewer).not.toEqual(object.viewer);
    expect(room.colmap.mapper).toBe("global");
    expect(object.colmap.mapper).toBe("incremental");
    expect(room.colmap).toEqual(colmapKnobsFor("room"));
    expect(room.viewer).toEqual(viewerKnobsFor("room"));
    expect(room.extract).toEqual(object.extract);
    expect(room.brush).toEqual(object.brush);
  });
});

describe("applyNamedPreset", () => {
  it("keeps current values when selecting custom would, and named apply preserves capture", () => {
    const current = {
      ...PRESET_SETTINGS.fast,
      startSeconds: 2,
      frameFormat: "png" as const,
      captureMode: "outdoor" as const,
    };
    const applied = applyNamedPreset("quality", current);
    expect(applied.trainSteps).toBe(30_000);
    expect(applied.startSeconds).toBe(2);
    expect(applied.frameFormat).toBe("png");
    expect(applied.captureMode).toBe("outdoor");
    expect(applied.colmap).toEqual(colmapKnobsFor("outdoor"));
    expect(applied.viewer).toEqual(viewerKnobsFor("outdoor"));
  });
});

describe("hydrateSettings", () => {
  it("fills missing nested knobs from capture mode", () => {
    const legacy = {
      ...PRESET_SETTINGS.fast,
      captureMode: "room" as const,
      extract: undefined,
      colmap: undefined,
      brush: undefined,
      viewer: undefined,
    };
    const hydrated = hydrateSettings(legacy);
    expect(hydrated.colmap).toEqual(colmapKnobsFor("room"));
    expect(hydrated.viewer).toEqual(viewerKnobsFor("room"));
  });
});

describe("applyCaptureMode", () => {
  it("resets nested knobs on a named preset", () => {
    const next = applyCaptureMode(PRESET_SETTINGS.balanced, "room");
    expect(next.captureMode).toBe("room");
    expect(next.colmap).toEqual(colmapKnobsFor("room"));
    expect(matchingPreset(next)).toBe("balanced");
  });

  it("does not overwrite nested knobs in custom", () => {
    const custom = {
      ...PRESET_SETTINGS.balanced,
      brush: { ...DEFAULT_BRUSH_KNOBS, shDegree: 1 },
    };
    const next = applyCaptureMode(custom, "outdoor");
    expect(next.captureMode).toBe("outdoor");
    expect(next.brush.shDegree).toBe(1);
    expect(next.maxFrames).toBe(PRESET_SETTINGS.balanced.maxFrames);
  });
});

describe("maxFramesCap", () => {
  it("keeps object and room at 800", () => {
    expect(maxFramesCap("object")).toBe(800);
    expect(maxFramesCap("room")).toBe(800);
  });

  it("allows outdoor paths up to 10000", () => {
    expect(maxFramesCap("outdoor")).toBe(10_000);
  });
});
