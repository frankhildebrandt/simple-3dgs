import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, ArchiveEntry, PipelineRequest, RunResult } from "./types";

export function startPipeline(request: PipelineRequest): Promise<RunResult> {
  return invoke<RunResult>("start_pipeline", { request });
}

export function cancelPipeline(): Promise<void> {
  return invoke<void>("cancel_pipeline");
}

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function saveConfig(config: AppConfig): Promise<AppConfig> {
  return invoke<AppConfig>("save_config", { config });
}

export function listArchive(): Promise<ArchiveEntry[]> {
  return invoke<ArchiveEntry[]>("list_archive");
}

export function getArchive(id: string): Promise<ArchiveEntry> {
  return invoke<ArchiveEntry>("get_archive", { id });
}

export function renameArchive(id: string, title: string): Promise<ArchiveEntry> {
  return invoke<ArchiveEntry>("rename_archive", { id, title });
}

export function deleteArchive(id: string): Promise<void> {
  return invoke<void>("delete_archive", { id });
}

export function setArchivePoster(id: string, jpegBase64: string): Promise<ArchiveEntry> {
  return invoke<ArchiveEntry>("set_archive_poster", { id, jpegBase64 });
}

export function import3dgs(path: string): Promise<ArchiveEntry> {
  return invoke<ArchiveEntry>("import_3dgs", { path });
}

export function export3dgs(id: string, destPath: string): Promise<void> {
  return invoke<void>("export_3dgs", { id, destPath });
}

export function exportHtml(id: string, destDir: string): Promise<void> {
  return invoke<void>("export_html", { id, destDir });
}

export function spzCacheFresh(id: string): Promise<boolean> {
  return invoke<boolean>("spz_cache_fresh", { id });
}

/** Writes SPZ bytes into `{archive}/{id}/scene.spz` via a raw IPC body. */
export function cacheArchiveSpz(id: string, bytes: Uint8Array): Promise<void> {
  return invoke<void>("cache_archive_spz", bytes, { headers: { id } });
}

export function exportSpz(id: string, destPath: string): Promise<void> {
  return invoke<void>("export_spz", { id, destPath });
}

export function dropArchivePly(id: string): Promise<ArchiveEntry> {
  return invoke<ArchiveEntry>("drop_archive_ply", { id });
}
