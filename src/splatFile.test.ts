import { describe, expect, it } from "vitest";
import { splatFileName, splatKindFromPath, splatLoadHint, splatSidecarPath } from "./splatFile";

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

describe("splatSidecarPath", () => {
  it("joins a sibling name onto the splat directory", () => {
    expect(splatSidecarPath("/archive/id/scene.ply", "view.json")).toBe("/archive/id/view.json");
    expect(splatSidecarPath("C:\\archive\\id\\scene.spz", "view.json")).toBe("C:\\archive\\id/view.json");
    expect(splatSidecarPath("scene.ply", "view.json")).toBeNull();
  });
});

describe("splatLoadHint", () => {
  it("keeps TCC and read failures visible", () => {
    expect(splatLoadHint(new Error("Cannot read splat: file is empty."))).toBe(
      "Cannot read splat: file is empty.",
    );
    expect(
      splatLoadHint("macOS blocked access to /Users/frank/Documents/Simple 3DGS/archive"),
    ).toContain("macOS blocked access");
    expect(splatLoadHint(new Error("createWebviewWindow not allowed"))).toContain("not allowed");
  });

  it("hides Spark internals behind the checkpoint hint", () => {
    expect(splatLoadHint(new Error("Failed to parse PLY header"))).toBe("Checkpoint failed to load");
  });
});
