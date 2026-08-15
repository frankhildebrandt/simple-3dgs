import { describe, expect, it, vi } from "vitest";

vi.mock("@sparkjsdev/spark", () => ({
  SplatFileType: { PLY: "ply" },
  transcodeSpz: vi.fn(),
}));

import { plyBytesToSpz } from "./spzTranscode";

describe("plyBytesToSpz", () => {
  it("rejects an empty PLY before calling Spark", async () => {
    const transcode = vi.fn();
    await expect(plyBytesToSpz(new Uint8Array(), transcode)).rejects.toThrow(
      "Cannot encode SPZ: scene.ply is empty.",
    );
    expect(transcode).not.toHaveBeenCalled();
  });

  it("passes PLY bytes to Spark and returns the SPZ payload", async () => {
    const ply = new Uint8Array([1, 2, 3]);
    const spz = new Uint8Array([9, 8, 7]);
    const transcode = vi.fn().mockResolvedValue({ fileBytes: spz, clippedCount: 0 });
    await expect(plyBytesToSpz(ply, transcode)).resolves.toEqual(spz);
    expect(transcode).toHaveBeenCalledWith({
      inputs: [{ fileBytes: ply, fileType: "ply", pathOrUrl: "scene.ply" }],
    });
  });

  it("rejects an empty encoder result", async () => {
    const transcode = vi.fn().mockResolvedValue({ fileBytes: new Uint8Array(), clippedCount: 0 });
    await expect(plyBytesToSpz(new Uint8Array([1]), transcode)).rejects.toThrow(
      "Cannot encode SPZ: encoder returned no data.",
    );
  });

  it("wraps Spark failures", async () => {
    const transcode = vi.fn().mockRejectedValue(new Error("worker crashed"));
    await expect(plyBytesToSpz(new Uint8Array([1]), transcode)).rejects.toThrow(
      "Cannot encode SPZ: worker crashed",
    );
  });
});
