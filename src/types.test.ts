import { describe, expect, it } from "vitest";
import { matchingPreset, maxFramesCap, PRESET_SETTINGS } from "./types";

describe("matchingPreset", () => {
  it("matches balanced including maxFrames", () => {
    expect(matchingPreset(PRESET_SETTINGS.balanced)).toBe("balanced");
  });

  it("drops off the preset when maxFrames changes", () => {
    expect(
      matchingPreset({ ...PRESET_SETTINGS.balanced, maxFrames: 400 }),
    ).toBeNull();
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
