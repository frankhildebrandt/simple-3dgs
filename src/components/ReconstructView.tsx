import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { cancelPipeline, startPipeline } from "../api";
import type { AppConfig, PipelineSettings, ProgressEvent, RunStatus, SourceKind, TrainStats } from "../types";
import { PRESET_SETTINGS } from "../types";
import { CaptureHints } from "./CaptureHints";
import { CaptureModePicker } from "./CaptureModePicker";
import { DropZone } from "./DropZone";
import { PresetPicker } from "./PresetPicker";
import { RunProgress } from "./RunProgress";
import { SettingsPanel } from "./SettingsPanel";
import { SplatViewer } from "./SplatViewer";

type Props = {
  config: AppConfig | null;
  density: "easy" | "expert";
  fullscreen: boolean;
  onToggleFullscreen: () => void;
};

/** Source, settings, and live viewer for a reconstruction run. */
export function ReconstructView({ config, density, fullscreen, onToggleFullscreen }: Props) {
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [sourceKind, setSourceKind] = useState<SourceKind>("video");
  const [settings, setSettings] = useState<PipelineSettings>(PRESET_SETTINGS.balanced);
  const [force, setForce] = useState(false);
  const [status, setStatus] = useState<RunStatus>("idle");
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [train, setTrain] = useState<TrainStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [plyPath, setPlyPath] = useState<string | null>(null);
  const [hintsOpen, setHintsOpen] = useState(false);

  const onSource = useCallback((path: string, kind: SourceKind) => {
    setSourcePath(path);
    setSourceKind(kind);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const unsubs: Array<() => void> = [];
    const bind = async <T,>(event: string, handler: (payload: T) => void) => {
      const off = await listen<T>(event, (e) => handler(e.payload));
      if (cancelled) {
        off();
        return;
      }
      unsubs.push(off);
    };
    void (async () => {
      await bind<ProgressEvent>("pipeline-progress", setProgress);
      await bind<string>("pipeline-log", (line) => {
        setLogs((prev) => [...prev, line]);
      });
      await bind<TrainStats>("pipeline-train-stats", setTrain);
      await bind<string>("pipeline-preview", setPlyPath);
      await bind<string>("pipeline-complete", (path) => {
        setPlyPath(path);
        setStatus("done");
      });
      await bind<string>("pipeline-error", (message) => {
        setError(message);
        setStatus(message === "Cancelled" ? "idle" : "error");
      });
    })();
    return () => {
      cancelled = true;
      for (const off of unsubs) {
        off();
      }
    };
  }, []);

  async function reconstruct() {
    if (!sourcePath || !config?.archiveDir) {
      return;
    }
    if (!config.tempProject && !config.projectDir) {
      return;
    }
    setStatus("running");
    setError(null);
    setLogs([]);
    setTrain(null);
    setProgress(null);
    setPlyPath(null);
    try {
      const result = await startPipeline({
        sourcePath,
        sourceKind,
        projectDir: config.tempProject ? null : config.projectDir,
        archiveDir: config.archiveDir,
        tempProject: config.tempProject,
        settings,
        force: density === "expert" && force,
      });
      setPlyPath(result.plyPath);
      setStatus("done");
      if (result.archiveError) {
        setError(result.archiveError);
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setStatus(message === "Cancelled" ? "idle" : "error");
    }
  }

  const running = status === "running";
  const canRun =
    Boolean(sourcePath && config?.archiveDir) &&
    Boolean(config?.tempProject || config?.projectDir) &&
    !running;

  return (
    <div className="app">
      <aside className="sidebar">
        <DropZone
          sourcePath={sourcePath}
          sourceKind={sourceKind}
          disabled={running}
          onSource={onSource}
        />
        <CaptureModePicker
          value={settings}
          disabled={running}
          onChange={setSettings}
          onHelp={() => setHintsOpen(true)}
        />
        <PresetPicker value={settings} disabled={running} onChange={setSettings} />
        {density === "expert" ? (
          <>
            <SettingsPanel
              value={settings}
              sourceKind={sourceKind}
              disabled={running}
              onChange={setSettings}
            />
            <label
              className="force"
              title="Ignore cached stages and run the pipeline from scratch"
              data-hint="Ignore cached stages and run the pipeline from scratch"
            >
              <input
                type="checkbox"
                checked={force}
                disabled={running}
                onChange={(event) => setForce(event.currentTarget.checked)}
              />
              Rebuild from scratch
            </label>
          </>
        ) : null}
        <div className="row">
          <button type="button" className="primary" disabled={!canRun} onClick={() => void reconstruct()}>
            Reconstruct
          </button>
          <button type="button" disabled={!running} onClick={() => void cancelPipeline()}>
            Cancel
          </button>
        </div>
      </aside>
      <main className="main">
        <SplatViewer
          plyPath={plyPath}
          captureMode={settings.captureMode}
          live={running}
          fullscreen={fullscreen}
          onToggleFullscreen={onToggleFullscreen}
        />
        <RunProgress status={status} progress={progress} train={train} logs={logs} error={error} />
      </main>
      <CaptureHints mode={settings.captureMode} open={hintsOpen} onClose={() => setHintsOpen(false)} />
    </div>
  );
}
