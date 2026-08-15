import { getCurrentWindow } from "@tauri-apps/api/window";

/** Toggles native macOS fullscreen (own Space). Returns the resulting state. */
export async function toggleNativeFullscreen(): Promise<boolean> {
  const win = getCurrentWindow();
  await win.setFullscreen(!(await win.isFullscreen()));
  return win.isFullscreen();
}

/** Keeps React in sync when fullscreen is entered or left via any macOS gesture. */
export async function watchNativeFullscreen(onChange: (on: boolean) => void): Promise<() => void> {
  const win = getCurrentWindow();
  onChange(await win.isFullscreen());
  const sync = async () => {
    onChange(await win.isFullscreen());
  };
  const offResize = await win.onResized(sync);
  const offMoved = await win.onMoved(sync);
  return () => {
    offResize();
    offMoved();
  };
}
