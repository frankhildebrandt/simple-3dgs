import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { cancelPipeline, setArchiveDir, startPipeline } from "../api";
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
  onConfig: (config: AppConfig) => void;
  fullscreen: boolean;
  onToggleFullscreen: () => void;
};

/** Source, settings, and live viewer for a reconstruction run. */
export function ReconstructView({ config, onConfig, fullscreen, onToggleFullscreen }: Props) {
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [sourceKind, setSourceKind] = useState<SourceKind>("video");
  const [projectDir, setProjectDir] = useState<string | null>(null);
  const [tempProject, setTempProject] = useState(true);
  const [settings, setSettings] = useState<PipelineSettings>(PRESET_SETTINGS.balanced);
  const [force, setForce] = useState(false);
  const [status, setStatus] = useState<RunStatus>("idle");
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [train, setTrain] = useState<TrainStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [plyPath, setPlyPath] = useState<string | null>(null);

  const onSource = useCallback((path: string, kind: SourceKind) => {
    setSourcePath(path);
    setSourceKind(kind);
  }, []);

  useEffect(() => {
    const unsubs: Array<() => void> = [];
    const bind = async () => {
      unsubs.push(
        await listen<ProgressEvent>("pipeline-progress", (event) => {
          setProgress(event.payload);
        }),
      );
      unsubs.push(
        await listen<string>("pipeline-log", (event) => {
          setLogs((prev) => [...prev, event.payload]);
        }),
      );
      unsubs.push(
        await listen<TrainStats>("pipeline-train-stats", (event) => {
          setTrain(event.payload);
        }),
      );
      unsubs.push(
        await listen<string>("pipeline-preview", (event) => {
          setPlyPath(event.payload);
        }),
      );
      unsubs.push(
        await listen<string>("pipeline-complete", (event) => {
          setPlyPath(event.payload);
          setStatus("done");
        }),
      );
      unsubs.push(
        await listen<string>("pipeline-error", (event) => {
          setError(event.payload);
          setStatus(event.payload === "Cancelled" ? "idle" : "error");
        }),
      );
    };
    void bind();
    return () => {
      for (const off of unsubs) {
        off();
      }
    };
  }, []);

  async function pickProjectDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      setProjectDir(selected);
    }
  }

  async function pickArchiveDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      onConfig(await setArchiveDir(selected));
    }
  }

  async function reconstruct() {
    if (!sourcePath || !config?.archiveDir) {
      return;
    }
    if (!tempProject && !projectDir) {
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
        projectDir: tempProject ? null : projectDir,
        archiveDir: config.archiveDir,
        tempProject,
        settings,
        force,
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
    Boolean(sourcePath && config?.archiveDir) && (tempProject || Boolean(projectDir)) && !running;

  return (
    <div className="app">
      <aside className="sidebar">
        <header>
          <h1>Simple 3DGS</h1>
          <p className="tagline">Video in, Gaussian splat out.</p>
        </header>
        <DropZone
          sourcePath={sourcePath}
          sourceKind={sourceKind}
          disabled={running}
          onSource={onSource}
        />
        <section>
          <p className="dropzone-label">
            {config ? `Archive: ${config.archiveDir}` : "Choose an archive folder"}
          </p>
          <button type="button" disabled={running} onClick={() => void pickArchiveDir()}>
            Choose archive folder
          </button>
        </section>
        <label className="force">
          <input
            type="checkbox"
            checked={tempProject}
            disabled={running}
            onChange={(event) => setTempProject(event.currentTarget.checked)}
          />
          Temporary project folder
        </label>
        {tempProject ? null : (
          <section>
            <p className="dropzone-label">
              {projectDir ? `Project: ${projectDir}` : "Choose a project folder"}
            </p>
            <button type="button" disabled={running} onClick={() => void pickProjectDir()}>
              Choose project folder
            </button>
          </section>
        )}
        <CaptureModePicker value={settings} disabled={running} onChange={setSettings} />
        <PresetPicker value={settings} disabled={running} onChange={setSettings} />
        <SettingsPanel
          value={settings}
          sourceKind={sourceKind}
          disabled={running}
          onChange={setSettings}
        />
        <label className="force">
          <input
            type="checkbox"
            checked={force}
            disabled={running}
            onChange={(event) => setForce(event.currentTarget.checked)}
          />
          Rebuild from scratch
        </label>
        <div className="row">
          <button type="button" className="primary" disabled={!canRun} onClick={() => void reconstruct()}>
            Reconstruct
          </button>
          <button type="button" disabled={!running} onClick={() => void cancelPipeline()}>
            Cancel
          </button>
        </div>
        <CaptureHints mode={settings.captureMode} />
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
    </div>
  );
}
