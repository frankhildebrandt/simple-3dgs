import { describe, expect, it } from "vitest";
import { expertCanRun, nextExpertAction } from "./expertActions";

describe("nextExpertAction", () => {
  it("starts at frames", () => {
    expect(nextExpertAction(null)).toBe("frames");
  });

  it("moves to cameras then train", () => {
    expect(nextExpertAction("frames")).toBe("colmap");
    expect(nextExpertAction("colmap")).toBe("train");
  });
});

describe("expertCanRun", () => {
  it("blocks while running or without a source", () => {
    expect(expertCanRun("frames", null, true, true, true)).toBe(false);
    expect(expertCanRun("frames", null, false, false, true)).toBe(false);
    expect(expertCanRun("colmap", null, false, true, true)).toBe(false);
  });

  it("unlocks cameras after frames", () => {
    expect(expertCanRun("colmap", "frames", false, true, true)).toBe(true);
    expect(expertCanRun("train", "frames", false, true, true)).toBe(false);
    expect(expertCanRun("train", "colmap", false, true, true)).toBe(true);
  });
});
