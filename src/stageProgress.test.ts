import { describe, expect, it } from "vitest";
import { formatPercent, stagePercents } from "./stageProgress";

describe("formatPercent", () => {
  it("rounds and clamps", () => {
    expect(formatPercent(42.4)).toBe("42%");
    expect(formatPercent(150)).toBe("100%");
    expect(formatPercent(-2)).toBe("0%");
  });
});

describe("stagePercents", () => {
  it("fills earlier stages when cameras are running", () => {
    const percents = stagePercents({ stage: "colmap", percent: 51, message: "Matching" }, "running");
    expect(percents.frames).toBe(100);
    expect(percents.colmap).toBe(51);
    expect(percents.train).toBeNull();
  });

  it("marks everything complete when done", () => {
    const percents = stagePercents({ stage: "train", percent: 100, message: "Done" }, "done");
    expect(percents.frames).toBe(100);
    expect(percents.colmap).toBe(100);
    expect(percents.train).toBe(100);
  });

  it("treats the paused stage as finished", () => {
    const percents = stagePercents({ stage: "frames", percent: 100, message: "Extracted" }, "paused");
    expect(percents.frames).toBe(100);
    expect(percents.colmap).toBeNull();
  });
});
