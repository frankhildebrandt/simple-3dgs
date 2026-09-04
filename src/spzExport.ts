import type { ArchiveEntry } from "./types";
import { cacheArchiveSpz, dropArchivePly, getArchive, spzCacheFresh } from "./api";
import { convertToSpzWith, ensureSpzWith } from "./spzCache";
import { loadPlyBytes, plyBytesToSpz } from "./spzTranscode";

function encodeIo(onPhase?: (phase: string) => void) {
  return {
    cacheFresh: spzCacheFresh,
    readPly: async (path: string) => {
      onPhase?.("Read PLY");
      return loadPlyBytes(path);
    },
    encode: async (bytes: Uint8Array) => {
      onPhase?.("Encode SPZ");
      return plyBytesToSpz(bytes);
    },
    writeSpz: async (id: string, bytes: Uint8Array) => {
      onPhase?.("Write SPZ");
      return cacheArchiveSpz(id, bytes);
    },
  };
}

/** Encodes and caches scene.spz when the archive copy is missing or stale. */
export function ensureSpz(
  entry: Pick<ArchiveEntry, "id" | "plyPath">,
  onPhase?: (phase: string) => void,
): Promise<void> {
  return ensureSpzWith(entry, encodeIo(onPhase));
}

/** Compresses an uncompressed archive entry in place and drops scene.ply. */
export async function convertArchiveToSpz(
  entry: Pick<ArchiveEntry, "id" | "plyPath" | "hasPly">,
  onPhase?: (phase: string) => void,
): Promise<ArchiveEntry> {
  await convertToSpzWith(entry, {
    ...encodeIo(onPhase),
    dropPly: async (id) => {
      onPhase?.("Remove PLY");
      await dropArchivePly(id);
    },
  });
  return getArchive(entry.id);
}
