import type { ArchiveEntry } from "./types";
import { cacheArchiveSpz, dropArchivePly, getArchive, spzCacheFresh } from "./api";
import { convertToSpzWith, ensureSpzWith } from "./spzCache";
import { loadPlyBytes, plyBytesToSpz } from "./spzTranscode";

function encodeIo() {
  return {
    cacheFresh: spzCacheFresh,
    readPly: loadPlyBytes,
    encode: plyBytesToSpz,
    writeSpz: cacheArchiveSpz,
  };
}

/** Encodes and caches scene.spz when the archive copy is missing or stale. */
export function ensureSpz(entry: Pick<ArchiveEntry, "id" | "plyPath">): Promise<void> {
  return ensureSpzWith(entry, encodeIo());
}

/** Compresses an uncompressed archive entry in place and drops scene.ply. */
export async function convertArchiveToSpz(
  entry: Pick<ArchiveEntry, "id" | "plyPath" | "hasPly">,
): Promise<ArchiveEntry> {
  await convertToSpzWith(entry, {
    ...encodeIo(),
    dropPly: async (id) => {
      await dropArchivePly(id);
    },
  });
  return getArchive(entry.id);
}
