import { convertFileSrc } from "@tauri-apps/api/core";
import type { ProjectFrame } from "../types";

type Props = {
  currentPath: string | null;
  frames: ProjectFrame[];
  onSelect?: (path: string) => void;
};

/** Large still plus a filmstrip of extracted frames. */
export function FramePreview({ currentPath, frames, onSelect }: Props) {
  const src = currentPath ? convertFileSrc(currentPath) : null;
  return (
    <div className="viewer stage-preview">
      <div className="frame-hero">
        {src ? <img src={src} alt="Current frame" /> : <p className="viewer-hint">Waiting for frames</p>}
      </div>
      {frames.length > 0 ? (
        <ol className="filmstrip" aria-label="Extracted frames">
          {frames.map((frame) => {
            const thumb = convertFileSrc(frame.path);
            const active = frame.path === currentPath;
            return (
              <li key={frame.path}>
                <button
                  type="button"
                  className={active ? "active" : undefined}
                  onClick={() => onSelect?.(frame.path)}
                  title={frame.name}
                >
                  <img src={thumb} alt={frame.name} />
                </button>
              </li>
            );
          })}
        </ol>
      ) : null}
    </div>
  );
}
