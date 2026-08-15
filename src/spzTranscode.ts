import { convertFileSrc } from "@tauri-apps/api/core";
import { SplatFileType, transcodeSpz } from "@sparkjsdev/spark";

export type SpzTranscode = typeof transcodeSpz;

/** Loads a local PLY through Tauri's asset protocol. */
export async function loadPlyBytes(plyPath: string): Promise<Uint8Array> {
  const response = await fetch(convertFileSrc(plyPath));
  if (!response.ok) {
    throw new Error("Cannot encode SPZ: scene.ply is missing.");
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.byteLength === 0) {
    throw new Error("Cannot encode SPZ: scene.ply is empty.");
  }
  return bytes;
}

/** Quantizes a 3DGS PLY into Niantic SPZ v3 via Spark. */
export async function plyBytesToSpz(
  plyBytes: Uint8Array,
  transcode: SpzTranscode = transcodeSpz,
): Promise<Uint8Array> {
  if (plyBytes.byteLength === 0) {
    throw new Error("Cannot encode SPZ: scene.ply is empty.");
  }
  try {
    const { fileBytes } = await transcode({
      inputs: [{ fileBytes: plyBytes, fileType: SplatFileType.PLY, pathOrUrl: "scene.ply" }],
    });
    if (!fileBytes || fileBytes.byteLength === 0) {
      throw new Error("Cannot encode SPZ: encoder returned no data.");
    }
    return fileBytes;
  } catch (err) {
    if (err instanceof Error && err.message.startsWith("Cannot encode SPZ:")) {
      throw err;
    }
    const detail = err instanceof Error ? err.message : String(err);
    throw new Error(`Cannot encode SPZ: ${detail}`);
  }
}
