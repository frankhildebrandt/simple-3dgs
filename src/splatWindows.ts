import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { splatWindowLabel, splatWindowUrl } from "./splatWindow";

/** Focuses an existing splat viewer window, or opens a new one. */
export async function openSplatWindow(id: string, title: string): Promise<void> {
  const label = splatWindowLabel(id);
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }
  const created = new WebviewWindow(label, {
    url: splatWindowUrl(id),
    title,
    width: 1280,
    height: 800,
    minWidth: 640,
    minHeight: 480,
  });
  await new Promise<void>((resolve, reject) => {
    void created.once("tauri://created", () => resolve());
    void created.once("tauri://error", (event) => {
      reject(new Error(String(event.payload)));
    });
  });
}

/** Closes the splat viewer window for this archive id, if it is open. */
export async function closeSplatWindow(id: string): Promise<void> {
  const existing = await WebviewWindow.getByLabel(splatWindowLabel(id));
  await existing?.close();
}
