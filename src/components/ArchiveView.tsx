import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { import3dgs, listArchive, setArchiveDir } from "../api";
import type { AppConfig, ArchiveEntry } from "../types";
import { ArchiveCard } from "./ArchiveCard";
import { ArchiveDetail } from "./ArchiveDetail";
import { ArchiveMap } from "./ArchiveMap";

type Props = {
  config: AppConfig | null;
  onConfig: (config: AppConfig) => void;
};

/** Map + card grid of finished trainings; selecting one opens the splat viewer. */
export function ArchiveView({ config, onConfig }: Props) {
  const [entries, setEntries] = useState<ArchiveEntry[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const list = await listArchive();
      setEntries(list);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload, config?.archiveDir]);

  const selected = entries.find((entry) => entry.id === selectedId) ?? null;

  async function pickArchiveDir() {
    const selectedDir = await open({ directory: true, multiple: false });
    if (typeof selectedDir === "string") {
      onConfig(await setArchiveDir(selectedDir));
    }
  }

  async function onImport() {
    const file = await open({
      multiple: false,
      filters: [{ name: "3DGS archive", extensions: ["3dgs"] }],
    });
    if (typeof file !== "string") {
      return;
    }
    try {
      const entry = await import3dgs(file);
      await reload();
      setSelectedId(entry.id);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="archive">
      <header className="archive-toolbar">
        <p className="dropzone-label">
          {config ? `Archive: ${config.archiveDir}` : "Choose an archive folder"}
        </p>
        <div className="row">
          <button type="button" onClick={() => void pickArchiveDir()}>
            Choose archive folder
          </button>
          <button type="button" className="primary" onClick={() => void onImport()}>
            Import .3dgs
          </button>
        </div>
        {error ? <p className="archive-error">{error}</p> : null}
      </header>
      <ArchiveMap entries={entries} selectedId={selectedId} onSelect={setSelectedId} />
      <div className="archive-split">
        <div className="archive-cards">
          {entries.length === 0 ? (
            <p className="archive-empty">Finished trainings land here. Import a .3dgs file or reconstruct a capture.</p>
          ) : (
            entries.map((entry) => (
              <ArchiveCard
                key={entry.id}
                entry={entry}
                selected={entry.id === selectedId}
                onSelect={() => setSelectedId(entry.id)}
              />
            ))
          )}
        </div>
        {selected ? (
          <ArchiveDetail entry={selected} onClose={() => setSelectedId(null)} />
        ) : (
          <p className="archive-empty">Select a card or map marker to open the viewer.</p>
        )}
      </div>
    </div>
  );
}
