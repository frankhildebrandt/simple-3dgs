import { useEffect, useState } from "react";
import { getConfig } from "./api";
import { ArchiveView } from "./components/ArchiveView";
import { ReconstructView } from "./components/ReconstructView";
import { toggleNativeFullscreen, watchNativeFullscreen } from "./fullscreen";
import type { AppConfig, AppView } from "./types";
import "./App.css";

export default function App() {
  const [view, setView] = useState<AppView>("reconstruct");
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    void getConfig()
      .then(setConfig)
      .catch(() => setConfig(null));
  }, []);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void watchNativeFullscreen(setFullscreen).then((unlisten) => {
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
  }, []);

  return (
    <div className={fullscreen ? "shell viewer-fullscreen" : "shell"}>
      <nav className="topnav">
        <button
          type="button"
          className={view === "reconstruct" ? "active" : undefined}
          onClick={() => setView("reconstruct")}
        >
          Reconstruct
        </button>
        <button
          type="button"
          className={view === "archive" ? "active" : undefined}
          onClick={() => setView("archive")}
        >
          Archive
        </button>
      </nav>
      {view === "reconstruct" ? (
        <ReconstructView
          config={config}
          onConfig={setConfig}
          fullscreen={fullscreen}
          onToggleFullscreen={() => {
            void toggleNativeFullscreen().then(setFullscreen);
          }}
        />
      ) : (
        <ArchiveView config={config} onConfig={setConfig} />
      )}
    </div>
  );
}
