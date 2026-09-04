import { lazy, Suspense, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { save, open, confirm } from "@tauri-apps/plugin-dialog";
import type { ArchiveEntry, TaskProgress } from "../types";
import { export3dgs, exportHtml, exportSpz, setArchivePoster } from "../api";
import { osmBrowseUrl } from "../osm";
import { convertArchiveToSpz, ensureSpz } from "../spzExport";

const SplatViewer = lazy(() => import("./SplatViewer").then((module) => ({ default: module.SplatViewer })));

type Props = {
  entry: ArchiveEntry;
  onClose: () => void;
  onPoster: () => void;
};

type ExportKind = "html" | "spz" | "archive" | "convert";

/** Viewer, capture metadata, and export actions for one archive entry. */
export function ArchiveDetail({ entry, onClose, onPoster }: Props) {
  const geo = entry.geo;
  const captureMode = entry.settings?.captureMode ?? "object";
  const [exporting, setExporting] = useState<ExportKind | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [task, setTask] = useState<TaskProgress | null>(null);
  const busy = exporting != null;
  const showLocalPhase = (phase: string) => setTask({
    taskId: "local", kind: "spz", phase, indeterminate: true, cancellable: false, status: "running",
  });

  useEffect(() => {
    let stop: (() => void) | undefined;
    void listen<TaskProgress>("task-progress", (event) => setTask(event.payload)).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, []);

  async function onExport3dgs() {
    const dest = await save({
      defaultPath: `${entry.title}.3dgs`,
      filters: [{ name: "3DGS archive", extensions: ["3dgs"] }],
    });
    if (typeof dest !== "string") {
      return;
    }
    setExporting("archive");
    setTask(null);
    try {
      await export3dgs(entry.id, dest);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(null);
    }
  }

  async function onExportHtml() {
    const dest = await open({ directory: true, multiple: false });
    if (typeof dest !== "string") {
      return;
    }
    setExporting("html");
    showLocalPhase("Preparing SPZ");
    try {
      await ensureSpz(entry, showLocalPhase);
      await exportHtml(entry.id, dest);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(null);
    }
  }

  async function onExportSpz() {
    const dest = await save({
      defaultPath: `${entry.title}.spz`,
      filters: [{ name: "SPZ", extensions: ["spz"] }],
    });
    if (typeof dest !== "string") {
      return;
    }
    setExporting("spz");
    showLocalPhase("Preparing SPZ");
    try {
      await ensureSpz(entry, showLocalPhase);
      await exportSpz(entry.id, dest);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(null);
    }
  }

  async function onConvertSpz() {
    const ok = await confirm(
      `Replace the uncompressed PLY of “${entry.title}” with SPZ? This saves disk space and cannot be undone.`,
      { title: "Convert to SPZ", kind: "warning" },
    );
    if (!ok) {
      return;
    }
    setExporting("convert");
    showLocalPhase("Preparing SPZ");
    try {
      await convertArchiveToSpz(entry, showLocalPhase);
      setError(null);
      onPoster();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(null);
    }
  }

  return (
    <section className="archive-detail">
      <header className="archive-detail-head">
        <div>
          <h2>{entry.title}</h2>
          <p>
            {entry.sourceName} · {entry.frameCount} frames · {formatBytes(entry.plyBytes)}
            {entry.hasPly ? "" : " · SPZ"}
          </p>
          {geo ? (
            <p>
              <button
                type="button"
                className="link"
                onClick={() => void openUrl(osmBrowseUrl(geo.lat, geo.lon))}
              >
                {geo.lat.toFixed(5)}, {geo.lon.toFixed(5)}
              </button>
              {geo.alt != null ? ` · ${geo.alt.toFixed(1)} m` : null}
            </p>
          ) : (
            <p>No GPS on this capture.</p>
          )}
          {error ? <p className="archive-error">{error}</p> : null}
          {busy && task ? (
            <div className="task-progress" role="status">
              <span>{task.phase}</span>
              {task.indeterminate || !task.totalBytes ? (
                <progress />
              ) : (
                <progress max={task.totalBytes} value={task.currentBytes ?? 0} />
              )}
            </div>
          ) : null}
        </div>
        <div className="row">
          <button type="button" disabled={busy} onClick={() => void onExportHtml()}>
            {exporting === "html" ? "Encoding SPZ…" : "Export HTML"}
          </button>
          <button type="button" disabled={busy} onClick={() => void onExportSpz()}>
            {exporting === "spz" ? "Encoding SPZ…" : "Export .spz"}
          </button>
          {entry.hasPly ? (
            <button type="button" disabled={busy} onClick={() => void onConvertSpz()}>
              {exporting === "convert" ? "Encoding SPZ…" : "Convert to SPZ"}
            </button>
          ) : null}
          <button type="button" disabled={busy} onClick={() => void onExport3dgs()}>
            Export .3dgs
          </button>
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </header>
      <Suspense fallback={<div className="viewer"><p className="viewer-hint">Loading 3D viewer…</p></div>}>
        <SplatViewer
          key={`${entry.id}:${entry.plyPath}`}
          plyPath={entry.plyPath}
          captureMode={captureMode}
          viewer={entry.settings?.viewer}
          onSetPreview={async (jpegBase64) => {
            await setArchivePoster(entry.id, jpegBase64);
            onPoster();
          }}
        />
      </Suspense>
    </section>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
