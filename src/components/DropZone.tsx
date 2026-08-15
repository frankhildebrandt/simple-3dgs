import { useEffect, useRef } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-dialog";
import type { SourceKind } from "../types";

type Props = {
  sourcePath: string | null;
  sourceKind: SourceKind;
  disabled: boolean;
  onSource: (path: string, kind: SourceKind) => void;
};

const VIDEO_EXT = [".mp4", ".mov", ".m4v", ".mkv", ".webm", ".avi"];

function kindFromPath(path: string): SourceKind {
  const lower = path.toLowerCase();
  return VIDEO_EXT.some((ext) => lower.endsWith(ext)) ? "video" : "images";
}

export function DropZone({ sourcePath, sourceKind, disabled, onSource }: Props) {
  const unlisten = useRef<(() => void) | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCurrentWebviewWindow()
      .onDragDropEvent((event) => {
        if (disabled || event.payload.type !== "drop") {
          return;
        }
        const [path] = event.payload.paths;
        if (path) {
          onSource(path, kindFromPath(path));
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten.current = fn;
      })
      .catch(() => {
        // Browser preview has no Tauri webview drag-drop.
      });
    return () => {
      cancelled = true;
      unlisten.current?.();
    };
  }, [disabled, onSource]);

  async function pickVideo() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Video", extensions: ["mp4", "mov", "m4v", "mkv", "webm", "avi"] }],
    });
    if (typeof selected === "string") {
      onSource(selected, "video");
    }
  }

  async function pickImages() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      onSource(selected, "images");
    }
  }

  const label = sourcePath
    ? `${sourceKind === "video" ? "Video" : "Images"}: ${sourcePath}`
    : "Drop a video, or choose a file / image folder";

  return (
    <section className="dropzone">
      <p className="dropzone-label">{label}</p>
      <div className="row">
        <button type="button" disabled={disabled} onClick={() => void pickVideo()}>
          Choose video
        </button>
        <button type="button" disabled={disabled} onClick={() => void pickImages()}>
          Choose image folder
        </button>
      </div>
    </section>
  );
}
