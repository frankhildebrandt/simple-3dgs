/** Last path segment, posix or windows. */
export function splatFileName(path: string): string {
  const slash = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slash < 0 ? path : path.slice(slash + 1);
}

/** Spark file kind for an archive splat path. Converted entries are SPZ-only. */
export function splatKindFromPath(path: string): "ply" | "spz" {
  return splatFileName(path).toLowerCase().endsWith(".spz") ? "spz" : "ply";
}
