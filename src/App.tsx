import { useEffect, useState } from "react";
import { getConfig, saveConfig } from "./api";
import { ArchiveView } from "./components/ArchiveView";
import { ReconstructView } from "./components/ReconstructView";
import { SettingsView } from "./components/SettingsView";
import { SplatWindow } from "./components/SplatWindow";
import { TitleBar } from "./components/TitleBar";
import { toggleNativeFullscreen, watchNativeFullscreen } from "./fullscreen";
import { splatIdFromSearch } from "./splatWindow";
import type { AppConfig, AppView } from "./types";
import "./App.css";

export default function App() {
  const splatId = splatIdFromSearch(window.location.search);
  if (splatId) {
    return <SplatWindow splatId={splatId} />;
  }
  return <Shell />;
}

function Shell() {
  const [view, setView] = useState<AppView>("easy");
  const [density, setDensity] = useState<"easy" | "expert">("easy");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [configError, setConfigError] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    void getConfig()
      .then((loaded) => {
        setConfig(loaded);
        setConfigError(null);
        setView(loaded.uiMode);
        if (loaded.uiMode === "easy" || loaded.uiMode === "expert") {
          setDensity(loaded.uiMode);
        }
      })
      .catch((err: unknown) => {
        setConfig(null);
        setConfigError(err instanceof Error ? err.message : String(err));
      });
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

  async function persist(next: AppConfig) {
    setConfig(await saveConfig(next));
  }

  function onView(next: AppView) {
    setView(next);
    setSettingsOpen(false);
    if (next === "easy" || next === "expert") {
      setDensity(next);
    }
    if (config && config.uiMode !== next) {
      void persist({ ...config, uiMode: next });
    }
  }

  const reconstruct = view === "easy" || view === "expert";

  return (
    <div className={fullscreen ? "shell viewer-fullscreen" : "shell"}>
      <TitleBar
        view={view}
        settingsOpen={settingsOpen}
        onView={onView}
        onSettings={() => setSettingsOpen((open) => !open)}
      />
      <div className="shell-body">
        {configError ? <p className="archive-error config-error">{configError}</p> : null}
        <div className="shell-page" hidden={settingsOpen || !reconstruct}>
          <ReconstructView
            config={config}
            density={density}
            fullscreen={fullscreen}
            onToggleFullscreen={() => {
              void toggleNativeFullscreen().then(setFullscreen);
            }}
          />
        </div>
        {!settingsOpen && view === "archive" ? <ArchiveView config={config} /> : null}
        {settingsOpen ? <SettingsView config={config} onConfig={setConfig} /> : null}
      </div>
    </div>
  );
}
