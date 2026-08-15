import { useEffect, useRef, useState } from "react";
import { clampMenuPosition } from "../archiveMenu";

type Props = {
  x: number;
  y: number;
  canConvertSpz?: boolean;
  onOpen: () => void;
  onRename: () => void;
  onConvertSpz?: () => void;
  onDelete: () => void;
  onClose: () => void;
};

/** Cursor-anchored actions for one archive card. */
export function ArchiveMenu({
  x,
  y,
  canConvertSpz = false,
  onOpen,
  onRename,
  onConvertSpz,
  onDelete,
  onClose,
}: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useEffect(() => {
    const el = ref.current;
    if (!el) {
      return;
    }
    setPos(
      clampMenuPosition(
        { x, y },
        { width: el.offsetWidth, height: el.offsetHeight },
        { width: window.innerWidth, height: window.innerHeight },
      ),
    );
  }, [x, y]);

  useEffect(() => {
    const onDocDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) {
        onClose();
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.code === "Escape") {
        onClose();
      }
    };
    document.addEventListener("mousedown", onDocDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div ref={ref} className="archive-menu" style={{ left: pos.x, top: pos.y }} role="menu">
      <button type="button" role="menuitem" onClick={onOpen}>
        Open in new window
      </button>
      <button type="button" role="menuitem" onClick={onRename}>
        Rename
      </button>
      {canConvertSpz && onConvertSpz ? (
        <button type="button" role="menuitem" onClick={onConvertSpz}>
          Convert to SPZ
        </button>
      ) : null}
      <button type="button" role="menuitem" onClick={onDelete}>
        Delete
      </button>
    </div>
  );
}
