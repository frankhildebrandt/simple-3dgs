import { describe, expect, it } from "vitest";
import { captureTips } from "./captureTips";

describe("captureTips", () => {
  it("starts object mode with an orbit", () => {
    expect(captureTips("object")[0]).toMatch(/orbit/i);
  });

  it("warns rooms about see-through walls", () => {
    expect(captureTips("room").some((tip) => /see-through/i.test(tip))).toBe(true);
  });

  it("includes the shared memory floor for every mode", () => {
    for (const mode of ["object", "room", "outdoor"] as const) {
      expect(captureTips(mode).some((tip) => tip.includes("16 GB"))).toBe(true);
    }
  });
});
