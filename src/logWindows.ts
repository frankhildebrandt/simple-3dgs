import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { logWindowLabel, logWindowUrl } from "./logWindow";

/** Focuses the pipeline log window, or opens a new one. */
export async function openLogWindow(): Promise<void> {
  const label = logWindowLabel();
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }
  const created = new WebviewWindow(label, {
    url: logWindowUrl(),
    title: "Pipeline log",
    width: 720,
    height: 560,
    minWidth: 420,
    minHeight: 280,
  });
  await new Promise<void>((resolve, reject) => {
    void created.once("tauri://created", () => resolve());
    void created.once("tauri://error", (event) => {
      reject(new Error(String(event.payload)));
    });
  });
}
