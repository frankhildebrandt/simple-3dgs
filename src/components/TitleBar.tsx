import type { AppView } from "../types";

type Props = {
  view: AppView;
  settingsOpen: boolean;
  onView: (view: AppView) => void;
  onSettings: () => void;
};

const MODES: { id: AppView; label: string }[] = [
  { id: "easy", label: "Easy" },
  { id: "expert", label: "Expert" },
  { id: "archive", label: "Archive" },
];

/** Overlay chrome: brand left, view modes and settings on the right. */
export function TitleBar({ view, settingsOpen, onView, onSettings }: Props) {
  return (
    <header className="titlebar">
      <div className="titlebar-brand" data-tauri-drag-region>
        <img src="/logo.svg" width={24} height={24} alt="" />
        <h1>Simple 3DGS</h1>
      </div>
      <div className="titlebar-drag" data-tauri-drag-region />
      <div className="titlebar-actions">
        <div className="seg" role="tablist" aria-label="View mode">
          {MODES.map((mode) => (
            <button
              key={mode.id}
              type="button"
              role="tab"
              aria-selected={!settingsOpen && view === mode.id}
              className={!settingsOpen && view === mode.id ? "active" : undefined}
              onClick={() => onView(mode.id)}
            >
              {mode.label}
            </button>
          ))}
        </div>
        <button
          type="button"
          className={settingsOpen ? "icon-btn active" : "icon-btn"}
          aria-label="Settings"
          title="Settings"
          aria-pressed={settingsOpen}
          onClick={onSettings}
        >
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path
              fill="currentColor"
              d="M19.14 12.94c.04-.31.06-.63.06-.94s-.02-.63-.06-.94l2.03-1.58a.5.5 0 0 0 .12-.64l-1.92-3.32a.5.5 0 0 0-.6-.22l-2.39.96a7.1 7.1 0 0 0-1.63-.94l-.36-2.54a.5.5 0 0 0-.5-.42h-3.84a.5.5 0 0 0-.5.42l-.36 2.54c-.59.24-1.13.55-1.63.94l-2.39-.96a.5.5 0 0 0-.6.22L2.71 8.84a.5.5 0 0 0 .12.64l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94L2.83 14.52a.5.5 0 0 0-.12.64l1.92 3.32c.13.23.4.32.64.22l2.39-.96c.5.39 1.04.7 1.63.94l.36 2.54c.05.24.26.42.5.42h3.84c.24 0 .45-.18.5-.42l.36-2.54c.59-.24 1.13-.55 1.63-.94l2.39.96c.24.1.51 0 .64-.22l1.92-3.32a.5.5 0 0 0-.12-.64zM12 15.6A3.6 3.6 0 1 1 12 8.4a3.6 3.6 0 0 1 0 7.2"
            />
          </svg>
        </button>
      </div>
    </header>
  );
}
