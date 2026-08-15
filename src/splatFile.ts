/** Last path segment, posix or windows. */
export function splatFileName(path: string): string {
  const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slash < 0 ? path : path.slice(slash + 1);
}

/** Directory of a splat path, or null when the path has no separator. */
export function splatDir(path: string): string | null {
  const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slash < 0 ? null : path.slice(0, slash);
}

/** Sibling file next to a splat, e.g. `view.json`. */
export function splatSidecarPath(path: string, name: string): string | null {
  const dir = splatDir(path);
  return dir == null ? null : `${dir}/${name}`;
}

/** Spark file kind for an archive splat path. Converted entries are SPZ-only. */
export function splatKindFromPath(path: string): "ply" | "spz" {
  return splatFileName(path).toLowerCase().endsWith(".spz") ? "spz" : "ply";
}

/** Viewer hint: keep TCC/ACL/read errors, hide Spark decode internals. */
export function splatLoadHint(err: unknown): string {
  const detail = err instanceof Error ? err.message : String(err);
  if (
    detail.startsWith("Cannot read splat") ||
    detail.startsWith("macOS blocked") ||
    detail.toLowerCase().includes("not allowed")
  ) {
    return detail;
  }
  return "Checkpoint failed to load";
}
