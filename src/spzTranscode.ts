import { SplatFileType, transcodeSpz } from "@sparkjsdev/spark";
import { readSplatFile } from "./api";

export type SpzTranscode = typeof transcodeSpz;

/** Turns a Tauri IPC payload into splat bytes. */
export function splatBytesFromInvoke(data: unknown): Uint8Array {
  if (data instanceof Uint8Array) {
    return data;
  }
  if (data instanceof ArrayBuffer) {
    return new Uint8Array(data);
  }
  if (ArrayBuffer.isView(data)) {
    return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  }
  if (Array.isArray(data)) {
    return Uint8Array.from(data);
  }
  throw new Error("Cannot read splat: unexpected payload.");
}

/** Loads a local PLY or SPZ through Tauri IPC, not the asset protocol. */
export async function loadPlyBytes(plyPath: string): Promise<Uint8Array> {
  const bytes = splatBytesFromInvoke(await readSplatFile(plyPath));
  if (bytes.byteLength === 0) {
    throw new Error("Cannot read splat: file is empty.");
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
