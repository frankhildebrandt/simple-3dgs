import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  ArchiveEntry,
  PipelineRequest,
  ProjectEntry,
  ProjectFrame,
  RunResult,
  SparsePreview,
} from "./types";

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

/** Reads a local PLY or SPZ as a raw IPC body. */
export function readSplatFile(path: string): Promise<unknown> {
  return invoke("read_splat_file", { path });
}

export function createProject(request: {
  title?: string | null;
  sourcePath: string;
  sourceKind: string;
  settings: PipelineRequest["settings"];
  temp: boolean;
  projectsDir?: string | null;
  projectDir?: string | null;
}): Promise<ProjectEntry> {
  return invoke<ProjectEntry>("create_project", { request });
}

export function openProject(path: string): Promise<ProjectEntry> {
  return invoke<ProjectEntry>("open_project", { path });
}

export function listProjects(projectsDir?: string | null): Promise<ProjectEntry[]> {
  return invoke<ProjectEntry[]>("list_projects", { projectsDir: projectsDir ?? null });
}

export function listProjectFrames(projectDir: string): Promise<ProjectFrame[]> {
  return invoke<ProjectFrame[]>("list_project_frames", { projectDir });
}

export function getSparsePreview(projectDir: string): Promise<SparsePreview> {
  return invoke<SparsePreview>("get_sparse_preview", { projectDir });
}

export function getPipelineLogs(): Promise<string[]> {
  return invoke<string[]>("pipeline_logs");
}
