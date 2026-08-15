import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { deleteArchive, import3dgs, listArchive, renameArchive } from "../api";
import { closeSplatWindow, openSplatWindow } from "../splatWindows";
import type { AppConfig, ArchiveEntry } from "../types";
import { convertArchiveToSpz } from "../spzExport";
import { ArchiveCard } from "./ArchiveCard";
import { ArchiveDetail } from "./ArchiveDetail";
import { ArchiveMap } from "./ArchiveMap";
import { ArchiveMenu } from "./ArchiveMenu";

type Props = {
  config: AppConfig | null;
};

type MenuState = {
  id: string;
  x: number;
  y: number;
};

/** Map + card grid of finished trainings; selecting one opens the splat viewer. */
export function ArchiveView({ config }: Props) {
  const [entries, setEntries] = useState<ArchiveEntry[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
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

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void listen<string>("archive-changed", () => {
      void reload();
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [reload]);

  const selected = entries.find((entry) => entry.id === selectedId) ?? null;
  const menuEntry = menu ? entries.find((entry) => entry.id === menu.id) : null;

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

  async function onOpen(entry: ArchiveEntry) {
    setMenu(null);
    try {
      await openSplatWindow(entry.id, entry.title);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function onRename(id: string, title: string) {
    setRenamingId(null);
    try {
      await renameArchive(id, title);
      await reload();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function onConvertSpz(entry: ArchiveEntry) {
    setMenu(null);
    if (!entry.hasPly) {
      return;
    }
    const ok = await confirm(
      `Replace the uncompressed PLY of “${entry.title}” with SPZ? This saves disk space and cannot be undone.`,
      { title: "Convert to SPZ", kind: "warning" },
    );
    if (!ok) {
      return;
    }
    try {
      await convertArchiveToSpz(entry);
      await reload();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function onDelete(entry: ArchiveEntry) {
    setMenu(null);
    const ok = await confirm(`Delete “${entry.title}”? This cannot be undone.`, {
      title: "Delete splat",
      kind: "warning",
    });
    if (!ok) {
      return;
    }
    try {
      await closeSplatWindow(entry.id);
      await deleteArchive(entry.id);
      if (selectedId === entry.id) {
        setSelectedId(null);
      }
      await reload();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="archive">
      <header className="archive-toolbar">
        <p className="dropzone-label">
          {config ? config.archiveDir : "No archive folder"}
        </p>
        <div className="row">
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
                renaming={entry.id === renamingId}
                onSelect={() => setSelectedId(entry.id)}
                onOpen={() => void onOpen(entry)}
                onContextMenu={(event) => {
                  setSelectedId(entry.id);
                  setMenu({ id: entry.id, x: event.clientX, y: event.clientY });
                }}
                onRename={(title) => void onRename(entry.id, title)}
                onCancelRename={() => setRenamingId(null)}
              />
            ))
          )}
        </div>
        {selected ? (
          <ArchiveDetail entry={selected} onClose={() => setSelectedId(null)} onPoster={() => void reload()} />
        ) : (
          <p className="archive-empty">Select a card or map marker to open the viewer.</p>
        )}
      </div>
      {menu && menuEntry ? (
        <ArchiveMenu
          x={menu.x}
          y={menu.y}
          onOpen={() => void onOpen(menuEntry)}
          onRename={() => {
            setRenamingId(menuEntry.id);
            setMenu(null);
          }}
          canConvertSpz={menuEntry.hasPly}
          onConvertSpz={() => void onConvertSpz(menuEntry)}
          onDelete={() => void onDelete(menuEntry)}
          onClose={() => setMenu(null)}
        />
      ) : null}
    </div>
  );
}
