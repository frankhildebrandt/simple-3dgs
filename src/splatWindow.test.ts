import { describe, expect, it } from "vitest";
import { splatIdFromSearch, splatWindowLabel, splatWindowUrl } from "./splatWindow";

describe("splatIdFromSearch", () => {
  it("reads a splat query param", () => {
    expect(splatIdFromSearch("?splat=2026-08-15_gate_ab12")).toBe("2026-08-15_gate_ab12");
    expect(splatIdFromSearch("splat=2026-08-15_gate_ab12")).toBe("2026-08-15_gate_ab12");
  });

  it("returns null when missing or blank", () => {
    expect(splatIdFromSearch("")).toBeNull();
    expect(splatIdFromSearch("?view=archive")).toBeNull();
    expect(splatIdFromSearch("?splat=")).toBeNull();
    expect(splatIdFromSearch("?splat=%20")).toBeNull();
  });
});

describe("splatWindowLabel", () => {
  it("prefixes the archive id", () => {
    expect(splatWindowLabel("2026-08-15_gate_ab12")).toBe("splat-2026-08-15_gate_ab12");
  });
});

describe("splatWindowUrl", () => {
  it("encodes the id in a relative viewer URL", () => {
    expect(splatWindowUrl("a b")).toBe("/?splat=a%20b");
  });
});
