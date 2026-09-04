/** True when this webview is the dedicated pipeline log window. */
export function isLogWindowSearch(search: string): boolean {
  const query = search.startsWith("?") ? search.slice(1) : search;
  const value = new URLSearchParams(query).get("log");
  if (value === null) {
    return false;
  }
  const trimmed = value.trim();
  return trimmed !== "0" && trimmed !== "false";
}

export function logWindowLabel(): string {
  return "pipeline-log";
}

export function logWindowUrl(projectDir?: string | null): string {
  const params = new URLSearchParams({ log: "1" });
  if (projectDir) params.set("project", projectDir);
  return `/?${params.toString()}`;
}

export function logProjectFromSearch(search: string): string | null {
  const query = search.startsWith("?") ? search.slice(1) : search;
  return new URLSearchParams(query).get("project");
}

/** Joins retained log lines for display and clipboard. */
export function logText(lines: string[]): string {
  return lines.join("\n");
}
