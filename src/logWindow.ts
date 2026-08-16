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

export function logWindowUrl(): string {
  return "/?log=1";
}

/** Joins retained log lines for display and clipboard. */
export function logText(lines: string[]): string {
  return lines.join("\n");
}
