import { describe, expect, it } from "vitest";
import { clampMenuPosition } from "./archiveMenu";

describe("clampMenuPosition", () => {
  it("keeps an in-bounds click unchanged", () => {
    expect(
      clampMenuPosition({ x: 40, y: 50 }, { width: 180, height: 120 }, { width: 800, height: 600 }),
    ).toEqual({ x: 40, y: 50 });
  });

  it("clamps to the bottom-right inset", () => {
    expect(
      clampMenuPosition({ x: 790, y: 590 }, { width: 180, height: 120 }, { width: 800, height: 600 }),
    ).toEqual({ x: 612, y: 472 });
  });

  it("clamps to the top-left inset", () => {
    expect(
      clampMenuPosition({ x: -20, y: -4 }, { width: 180, height: 120 }, { width: 800, height: 600 }),
    ).toEqual({ x: 8, y: 8 });
  });
});
