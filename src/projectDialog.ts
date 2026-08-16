import { open } from "@tauri-apps/plugin-dialog";

/** Directory picker for an existing reconstruction project. */
export async function pickProjectDir(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}
