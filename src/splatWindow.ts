/** Reads the dedicated viewer id from a location search string. */
export function splatIdFromSearch(search: string): string | null {
  const query = search.startsWith("?") ? search.slice(1) : search;
  const id = new URLSearchParams(query).get("splat");
  if (!id) {
    return null;
  }
  const trimmed = id.trim();
  return trimmed.length > 0 ? trimmed : null;
}

/** Tauri window label for an archive entry. */
export function splatWindowLabel(id: string): string {
  return `splat-${id}`;
}

export function splatWindowUrl(id: string): string {
  return `/?splat=${encodeURIComponent(id)}`;
}
