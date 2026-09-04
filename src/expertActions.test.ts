import { describe, expect, it } from "vitest";
import {
  expertCanRun,
  needsRerunPrompt,
  nextExpertAction,
  rerunDecision,
  rerunDiscardsLater,
} from "./expertActions";

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

describe("needsRerunPrompt", () => {
  it("skips the first run of each next stage", () => {
    expect(needsRerunPrompt("frames", null, "idle")).toBe(false);
    expect(needsRerunPrompt("colmap", "frames", "paused")).toBe(false);
    expect(needsRerunPrompt("train", "colmap", "paused")).toBe(false);
  });

  it("asks when the clicked stage is already done", () => {
    expect(needsRerunPrompt("colmap", "colmap", "paused")).toBe(true);
    expect(needsRerunPrompt("colmap", "train", "done")).toBe(true);
    expect(needsRerunPrompt("frames", "frames", "paused")).toBe(true);
    expect(needsRerunPrompt("train", "train", "done")).toBe(true);
  });

  it("asks when retrying the stage that just failed", () => {
    expect(needsRerunPrompt("colmap", "frames", "error")).toBe(true);
    expect(needsRerunPrompt("frames", null, "error")).toBe(true);
    expect(needsRerunPrompt("train", "colmap", "error")).toBe(true);
  });
});

describe("rerunDecision", () => {
  it("restarts the step from a clean slate", () => {
    expect(rerunDecision("restart", true)).toEqual({ kind: "run", restart: true });
    expect(rerunDecision("restart", false)).toEqual({ kind: "run", restart: true });
  });

  it("keeps finished work when continuing", () => {
    expect(rerunDecision("continue", true)).toEqual({ kind: "keep" });
  });

  it("resumes unfinished files when continuing after a failure", () => {
    expect(rerunDecision("continue", false)).toEqual({ kind: "run", restart: false });
  });
});

describe("rerunDiscardsLater", () => {
  it("warns when an earlier step would wipe later results", () => {
    expect(rerunDiscardsLater("colmap", "train")).toBe(true);
    expect(rerunDiscardsLater("frames", "colmap")).toBe(true);
    expect(rerunDiscardsLater("colmap", "colmap")).toBe(false);
    expect(rerunDiscardsLater("train", "train")).toBe(false);
  });
});
