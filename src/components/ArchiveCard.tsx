import type { MouseEvent } from "react";
import { useEffect, useRef } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { ArchiveEntry } from "../types";

type Props = {
  entry: ArchiveEntry;
  selected: boolean;
  renaming: boolean;
  onSelect: () => void;
  onOpen: () => void;
  onContextMenu: (event: MouseEvent) => void;
  onRename: (title: string) => void;
  onCancelRename: () => void;
};

/** Selectable archive card: poster first, otherwise a placeholder. */
export function ArchiveCard({
  entry,
  selected,
  renaming,
  onSelect,
  onOpen,
  onContextMenu,
  onRename,
  onCancelRename,
}: Props) {
  const poster = entry.posterPath ? convertFileSrc(entry.posterPath) : null;
  const geo = entry.geo;
  const when = formatWhen(entry.createdAt);

  return (
    <article
      className={selected ? "archive-card selected" : "archive-card"}
      onClick={onSelect}
      onDoubleClick={onOpen}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event);
      }}
    >
      <div className="archive-card-visual">
        {poster ? (
          <img src={poster} alt="" />
        ) : (
          <span className="archive-card-placeholder">No preview</span>
        )}
      </div>
      <div className="archive-card-body">
        {renaming ? (
          <RenameField title={entry.title} onRename={onRename} onCancel={onCancelRename} />
        ) : (
          <strong>{entry.title}</strong>
        )}
        <small>{when}</small>
        {geo ? (
          <small>
            {geo.lat.toFixed(4)}, {geo.lon.toFixed(4)}
          </small>
        ) : (
          <small>GPS unknown</small>
        )}
      </div>
    </article>
  );
}

function RenameField({
  title,
  onRename,
  onCancel,
}: {
  title: string;
  onRename: (title: string) => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);

  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);

  function commit() {
    if (done.current) {
      return;
    }
    done.current = true;
    const next = ref.current?.value.trim() ?? "";
    if (!next || next === title) {
      onCancel();
      return;
    }
    onRename(next);
  }

  function cancel() {
    if (done.current) {
      return;
    }
    done.current = true;
    onCancel();
  }

  return (
    <input
      ref={ref}
      className="archive-card-title"
      defaultValue={title}
      aria-label="Splat name"
      onClick={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.code === "Enter") {
          event.preventDefault();
          commit();
        }
        if (event.code === "Escape") {
          event.preventDefault();
          cancel();
        }
      }}
    />
  );
}

function formatWhen(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString();
}
