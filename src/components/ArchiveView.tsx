import { lazy, Suspense, useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { confirm, open } from "@tauri-apps/plugin-dialog";
import { deleteArchive, import3dgs, listArchive, renameArchive } from "../api";
import { closeSplatWindow, openSplatWindow } from "../splatWindows";
import type { AppConfig, ArchiveEntry, TaskProgress } from "../types";
import { convertArchiveToSpz } from "../spzExport";
import { ArchiveCard } from "./ArchiveCard";
import { ArchiveMenu } from "./ArchiveMenu";

const ArchiveDetail = lazy(() => import("./ArchiveDetail").then((module) => ({ default: module.ArchiveDetail })));
const ArchiveMap = lazy(() => import("./ArchiveMap").then((module) => ({ default: module.ArchiveMap })));

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
  const [task, setTask] = useState<TaskProgress | null>(null);
  const [busy, setBusy] = useState(false);

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

  useEffect(() => {
    let stop: (() => void) | undefined;
    void listen<TaskProgress>("task-progress", (event) => setTask(event.payload)).then((unlisten) => {
      stop = unlisten;
    });
    return () => stop?.();
  }, []);

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
    setBusy(true);
    setTask(null);
    try {
      const entry = await import3dgs(file);
      await reload();
      setSelectedId(entry.id);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
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
    setBusy(true);
    const showPhase = (phase: string) => setTask({
      taskId: "local", kind: "spz", phase, indeterminate: true, cancellable: false, status: "running",
    });
    showPhase("Preparing SPZ");
    try {
      await convertArchiveToSpz(entry, showPhase);
      await reload();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
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
          <button type="button" className="primary" disabled={busy} onClick={() => void onImport()}>
            {busy ? "Importing…" : "Import .3dgs"}
          </button>
        </div>
        {busy && task ? (
          <div className="task-progress" role="status">
            <span>{task.phase}</span>
            {task.indeterminate || !task.totalBytes ? <progress /> : <progress max={task.totalBytes} value={task.currentBytes ?? 0} />}
          </div>
        ) : null}
        {error ? <p className="archive-error">{error}</p> : null}
      </header>
      <Suspense fallback={<section className="archive-map"><p className="archive-map-empty">Loading map…</p></section>}>
        <ArchiveMap entries={entries} selectedId={selectedId} onSelect={setSelectedId} />
      </Suspense>
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
          <Suspense fallback={<p className="archive-empty">Loading viewer…</p>}>
            <ArchiveDetail entry={selected} onClose={() => setSelectedId(null)} onPoster={() => void reload()} />
          </Suspense>
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
