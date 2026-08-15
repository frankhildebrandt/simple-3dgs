import { openUrl } from "@tauri-apps/plugin-opener";
import { save, open } from "@tauri-apps/plugin-dialog";
import type { ArchiveEntry } from "../types";
import { export3dgs, exportHtml } from "../api";
import { osmBrowseUrl } from "../osm";
import { SplatViewer } from "./SplatViewer";

type Props = {
  entry: ArchiveEntry;
  onClose: () => void;
};

/** Viewer, capture metadata, and export actions for one archive entry. */
export function ArchiveDetail({ entry, onClose }: Props) {
  const geo = entry.geo;
  const captureMode = entry.settings?.captureMode ?? "object";

  async function onExport3dgs() {
    const dest = await save({
      defaultPath: `${entry.title}.3dgs`,
      filters: [{ name: "3DGS archive", extensions: ["3dgs"] }],
    });
    if (typeof dest === "string") {
      await export3dgs(entry.id, dest);
    }
  }

  async function onExportHtml() {
    const dest = await open({ directory: true, multiple: false });
    if (typeof dest === "string") {
      await exportHtml(entry.id, dest);
    }
  }

  return (
    <section className="archive-detail">
      <header className="archive-detail-head">
        <div>
          <h2>{entry.title}</h2>
          <p>
            {entry.sourceName} · {entry.frameCount} frames · {formatBytes(entry.plyBytes)}
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
        </div>
        <div className="row">
          <button type="button" onClick={() => void onExportHtml()}>
            Export HTML
          </button>
          <button type="button" onClick={() => void onExport3dgs()}>
            Export .3dgs
          </button>
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </header>
      <SplatViewer key={entry.id} plyPath={entry.plyPath} captureMode={captureMode} />
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
