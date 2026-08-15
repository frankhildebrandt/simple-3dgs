import { describe, expect, it, vi } from "vitest";
import { archiveSpzPath, convertToSpzWith, ensureSpzWith, isSpzCacheFresh, OUTPUT_SPZ } from "./spzCache";

describe("archiveSpzPath", () => {
  it("places scene.spz beside the archive entry", () => {
    expect(archiveSpzPath("/archive/2026-08-15_gate_ab12")).toBe(
      "/archive/2026-08-15_gate_ab12/scene.spz",
    );
    expect(archiveSpzPath("/archive/id/")).toBe("/archive/id/scene.spz");
    expect(OUTPUT_SPZ).toBe("scene.spz");
  });
});

describe("isSpzCacheFresh", () => {
  it("is stale when the cache is missing or older than the PLY", () => {
    expect(isSpzCacheFresh(100, null)).toBe(false);
    expect(isSpzCacheFresh(100, 99)).toBe(false);
  });

  it("is fresh when the SPZ is at least as new as the PLY", () => {
    expect(isSpzCacheFresh(100, 100)).toBe(true);
    expect(isSpzCacheFresh(100, 101)).toBe(true);
  });
});

describe("ensureSpzWith", () => {
  const entry = { id: "scene-1", plyPath: "/archive/scene-1/scene.ply" };

  it("skips transcode when the archive cache is fresh", async () => {
    const io = {
      cacheFresh: vi.fn().mockResolvedValue(true),
      readPly: vi.fn(),
      encode: vi.fn(),
      writeSpz: vi.fn(),
    };
    await ensureSpzWith(entry, io);
    expect(io.cacheFresh).toHaveBeenCalledWith("scene-1");
    expect(io.readPly).not.toHaveBeenCalled();
    expect(io.encode).not.toHaveBeenCalled();
    expect(io.writeSpz).not.toHaveBeenCalled();
  });

  it("encodes and writes when the cache is stale", async () => {
    const ply = new Uint8Array([1, 2]);
    const spz = new Uint8Array([3, 4]);
    const io = {
      cacheFresh: vi.fn().mockResolvedValue(false),
      readPly: vi.fn().mockResolvedValue(ply),
      encode: vi.fn().mockResolvedValue(spz),
      writeSpz: vi.fn().mockResolvedValue(undefined),
    };
    await ensureSpzWith(entry, io);
    expect(io.readPly).toHaveBeenCalledWith(entry.plyPath);
    expect(io.encode).toHaveBeenCalledWith(ply);
    expect(io.writeSpz).toHaveBeenCalledWith("scene-1", spz);
  });
});

describe("convertToSpzWith", () => {
  const entry = { id: "scene-1", plyPath: "/archive/scene-1/scene.ply", hasPly: true };

  it("skips work when the archive is already SPZ", async () => {
    const io = {
      cacheFresh: vi.fn(),
      readPly: vi.fn(),
      encode: vi.fn(),
      writeSpz: vi.fn(),
      dropPly: vi.fn(),
    };
    await convertToSpzWith({ ...entry, hasPly: false }, io);
    expect(io.encode).not.toHaveBeenCalled();
    expect(io.dropPly).not.toHaveBeenCalled();
  });

  it("encodes then drops the uncompressed PLY", async () => {
    const ply = new Uint8Array([1, 2]);
    const spz = new Uint8Array([3, 4]);
    const io = {
      cacheFresh: vi.fn().mockResolvedValue(false),
      readPly: vi.fn().mockResolvedValue(ply),
      encode: vi.fn().mockResolvedValue(spz),
      writeSpz: vi.fn().mockResolvedValue(undefined),
      dropPly: vi.fn().mockResolvedValue(undefined),
    };
    await convertToSpzWith(entry, io);
    expect(io.writeSpz).toHaveBeenCalledWith("scene-1", spz);
    expect(io.dropPly).toHaveBeenCalledWith("scene-1");
  });
});
