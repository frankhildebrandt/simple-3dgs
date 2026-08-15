import type { ArchiveEntry } from "./types";

export const OUTPUT_SPZ = "scene.spz";

export type SpzIo = {
  cacheFresh: (id: string) => Promise<boolean>;
  readPly: (plyPath: string) => Promise<Uint8Array>;
  encode: (plyBytes: Uint8Array) => Promise<Uint8Array>;
  writeSpz: (id: string, bytes: Uint8Array) => Promise<void>;
};

export type ConvertSpzIo = SpzIo & {
  dropPly: (id: string) => Promise<void>;
};

/** Archive cache path for the derived SPZ next to scene.ply. */
export function archiveSpzPath(entryDir: string): string {
  const base = entryDir.endsWith("/") || entryDir.endsWith("\\") ? entryDir.slice(0, -1) : entryDir;
  return `${base}/${OUTPUT_SPZ}`;
}

/** True when the cached SPZ is at least as new as the PLY. */
export function isSpzCacheFresh(plyMtimeMs: number, spzMtimeMs: number | null): boolean {
  return spzMtimeMs != null && spzMtimeMs >= plyMtimeMs;
}

/** Encodes scene.ply to scene.spz unless a fresh cache already exists. */
export async function ensureSpzWith(
  entry: Pick<ArchiveEntry, "id" | "plyPath">,
  io: SpzIo,
): Promise<void> {
  if (await io.cacheFresh(entry.id)) {
    return;
  }
  const plyBytes = await io.readPly(entry.plyPath);
  const spzBytes = await io.encode(plyBytes);
  await io.writeSpz(entry.id, spzBytes);
}

/** Encodes SPZ, then deletes the lossless PLY so the archive actually shrinks. */
export async function convertToSpzWith(
  entry: Pick<ArchiveEntry, "id" | "plyPath" | "hasPly">,
  io: ConvertSpzIo,
): Promise<void> {
  if (!entry.hasPly) {
    return;
  }
  await ensureSpzWith(entry, io);
  await io.dropPly(entry.id);
}
