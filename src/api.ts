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

export function setArchiveDir(path: string): Promise<AppConfig> {
  return invoke<AppConfig>("set_archive_dir", { path });
}

export function listArchive(): Promise<ArchiveEntry[]> {
  return invoke<ArchiveEntry[]>("list_archive");
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
