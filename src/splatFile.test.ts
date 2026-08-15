import { describe, expect, it } from "vitest";
import { splatFileName, splatKindFromPath } from "./splatFile";

describe("splatFileName", () => {
  it("takes the last path segment on posix and windows paths", () => {
    expect(splatFileName("/archive/id/scene.spz")).toBe("scene.spz");
    expect(splatFileName("C:\\archive\\id\\scene.ply")).toBe("scene.ply");
    expect(splatFileName("scene.spz")).toBe("scene.spz");
  });
});

describe("splatKindFromPath", () => {
  it("treats converted archive files as SPZ", () => {
    expect(splatKindFromPath("/archive/id/scene.spz")).toBe("spz");
    expect(splatKindFromPath("/archive/id/SCENE.SPZ")).toBe("spz");
  });

  it("keeps training checkpoints and lossless archives as PLY", () => {
    expect(splatKindFromPath("/tmp/iter_500.ply")).toBe("ply");
    expect(splatKindFromPath("/archive/id/scene.ply")).toBe("ply");
  });
});
