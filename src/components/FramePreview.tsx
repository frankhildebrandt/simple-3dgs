import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import type { ProjectFrame } from "../types";

type Props = {
  currentPath: string | null;
  frames: ProjectFrame[];
  onSelect?: (path: string) => void;
};

/** Large still plus a filmstrip of extracted frames. */
export function FramePreview({ currentPath, frames, onSelect }: Props) {
  const src = currentPath ? convertFileSrc(currentPath) : null;
  const stripRef = useRef<HTMLDivElement>(null);
  const [windowRange, setWindowRange] = useState({ start: 0, end: 24 });
  const pitch = 80;

  useEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const update = () => {
      const start = Math.max(0, Math.floor(strip.scrollLeft / pitch) - 6);
      const visible = Math.min(96, Math.ceil(strip.clientWidth / pitch) + 12);
      setWindowRange({ start, end: Math.min(frames.length, start + visible) });
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(strip);
    strip.addEventListener("scroll", update, { passive: true });
    return () => {
      observer.disconnect();
      strip.removeEventListener("scroll", update);
    };
  }, [frames.length]);

  const visibleFrames = useMemo(
    () => frames.slice(windowRange.start, windowRange.end),
    [frames, windowRange],
  );

  return (
    <div className="viewer stage-preview">
      <div className="frame-hero">
        {src ? <img src={src} alt="Current frame" /> : <p className="viewer-hint">Waiting for frames</p>}
      </div>
      {frames.length > 0 ? (
        <div ref={stripRef} className="filmstrip" role="list" aria-label="Extracted frames">
          <div className="filmstrip-window" style={{ width: frames.length * pitch }}>
            {visibleFrames.map((frame, offset) => {
              const thumb = convertFileSrc(frame.path);
              const active = frame.path === currentPath;
              return (
                <div key={frame.path} role="listitem" className="filmstrip-item" style={{ left: (windowRange.start + offset) * pitch }}>
                  <button
                    type="button"
                    className={active ? "active" : undefined}
                    onClick={() => onSelect?.(frame.path)}
                    title={frame.name}
                  >
                    <img src={thumb} alt={frame.name} loading="lazy" decoding="async" />
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
