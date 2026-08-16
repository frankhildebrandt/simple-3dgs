import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelPipeline,
  createProject,
  getSparsePreview,
  listProjectFrames,
  listProjects,
  openProject,
  startPipeline,
} from "../api";
import { parseMenuProject } from "../appMenu";
import { expertCanRun } from "../expertActions";
import { pickProjectDir } from "../projectDialog";
import type {
  AppConfig,
  CameraStats,
  FrameStats,
  PipelineSettings,
  ProgressEvent,
  ProjectEntry,
  ProjectFrame,
  RunStatus,
  SourceKind,
  SparsePreview,
  Stage,
  TrainStats,
} from "../types";
import { hydrateSettings, matchingPreset, PRESET_SETTINGS } from "../types";
import { CaptureHints } from "./CaptureHints";
import { CaptureModePicker } from "./CaptureModePicker";
import { CameraPreview } from "./CameraPreview";
import { DropZone } from "./DropZone";
import { FramePreview } from "./FramePreview";
import { PresetPicker } from "./PresetPicker";
import { ProjectPicker } from "./ProjectPicker";
import { RunProgress } from "./RunProgress";
import { SettingsPanel } from "./SettingsPanel";
import { SplatViewer } from "./SplatViewer";

type Props = {
  config: AppConfig | null;
  density: "easy" | "expert";
  fullscreen: boolean;
  onToggleFullscreen: () => void;
};

function stageFromProject(entry: ProjectEntry): Stage | null {
  if (entry.plyPath) {
    return "train";
  }
  if (entry.hasCameras) {
    return "colmap";
  }
  if (entry.hasFrames) {
    return "frames";
  }
  return null;
}

/** Source, settings, and live viewer for a reconstruction run. */
export function ReconstructView({ config, density, fullscreen, onToggleFullscreen }: Props) {
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [sourceKind, setSourceKind] = useState<SourceKind>("video");
  const [settings, setSettings] = useState<PipelineSettings>(PRESET_SETTINGS.balanced);
  const [force, setForce] = useState(false);
  const [status, setStatus] = useState<RunStatus>("idle");
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [train, setTrain] = useState<TrainStats | null>(null);
  const [frameStats, setFrameStats] = useState<FrameStats | null>(null);
  const [cameraStats, setCameraStats] = useState<CameraStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [plyPath, setPlyPath] = useState<string | null>(null);
  const [hintsOpen, setHintsOpen] = useState(false);
  const [project, setProject] = useState<ProjectEntry | null>(null);
  const [projects, setProjects] = useState<ProjectEntry[]>([]);
  const [completed, setCompleted] = useState<Stage | null>(null);
  const [framePath, setFramePath] = useState<string | null>(null);
  const [frames, setFrames] = useState<ProjectFrame[]>([]);
  const [sparse, setSparse] = useState<SparsePreview | null>(null);

  const onSource = useCallback((path: string, kind: SourceKind) => {
    setSourcePath(path);
    setSourceKind(kind);
  }, []);

  const refreshProjects = useCallback(async () => {
    if (!config) {
      return;
    }
    try {
      setProjects(await listProjects(config.projectsDir));
    } catch {
      setProjects([]);
    }
  }, [config]);

  useEffect(() => {
    void refreshProjects();
  }, [refreshProjects]);

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
      await bind<TrainStats>("pipeline-train-stats", setTrain);
      await bind<FrameStats>("pipeline-frame-stats", setFrameStats);
      await bind<CameraStats>("pipeline-camera-stats", setCameraStats);
      await bind<string>("pipeline-preview", setPlyPath);
      await bind<string>("pipeline-frame-preview", (path) => {
        setFramePath(path);
        setFrames((prev) => (prev.some((frame) => frame.path === path) ? prev : [...prev, {
          name: path.split("/").pop() ?? path,
          path,
          index: prev.length,
          sharpness: 0,
          motion: 0,
          selected: true,
        }]));
      });
      await bind<SparsePreview>("pipeline-sparse", setSparse);
      await bind<string>("pipeline-complete", (path) => {
        setPlyPath(path);
        setStatus("done");
        setCompleted("train");
      });
      await bind<string>("pipeline-stage", (stage) => {
        setCompleted(stage as Stage);
        setStatus("paused");
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

  async function applyProject(entry: ProjectEntry) {
    setProject(entry);
    setSourcePath(entry.sourcePath);
    setSourceKind(entry.sourceKind === "images" ? "images" : "video");
    if (entry.settings) {
      setSettings(hydrateSettings(entry.settings));
    }
    const stage = stageFromProject(entry);
    setCompleted(stage);
    setStatus(stage ? "paused" : "idle");
    setError(null);
    setPlyPath(entry.plyPath ?? null);
    try {
      const listed = await listProjectFrames(entry.dir);
      setFrames(listed);
      setFramePath(listed[0]?.path ?? null);
    } catch {
      setFrames([]);
      setFramePath(null);
    }
    try {
      const preview = await getSparsePreview(entry.dir);
      setSparse(preview.cameras.length || preview.points.length ? preview : null);
    } catch {
      setSparse(null);
    }
  }

  function resetProject() {
    setProject(null);
    setCompleted(null);
    setStatus("idle");
    setPlyPath(null);
    setFrames([]);
    setFramePath(null);
    setSparse(null);
    setProgress(null);
    setError(null);
  }

  const runningRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void listen<string>("menu-project", (event) => {
      const action = parseMenuProject(event.payload);
      if (!action || runningRef.current) {
        return;
      }
      if (action === "new") {
        resetProject();
        return;
      }
      void pickProjectDir().then((path) => {
        if (!path) {
          return;
        }
        return openProject(path).then(applyProject);
      }).catch((err: unknown) => {
        setError(err instanceof Error ? err.message : String(err));
      });
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
  }, []);

  async function ensureProject(): Promise<ProjectEntry | null> {
    if (project) {
      return project;
    }
    if (!sourcePath || !config) {
      return null;
    }
    const created = await createProject({
      sourcePath,
      sourceKind,
      settings,
      temp: config.tempProject,
      projectsDir: config.projectsDir,
    });
    setProject(created);
    await refreshProjects();
    return created;
  }

  async function runUntil(until: Stage) {
    if (!sourcePath || !config?.archiveDir) {
      return;
    }
    let active = project;
    try {
      active = await ensureProject();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setStatus("error");
      return;
    }
    if (!active) {
      return;
    }
    setStatus("running");
    setError(null);
    setTrain(null);
    setFrameStats(null);
    setCameraStats(null);
    setProgress(null);
    if (until === "frames") {
      setPlyPath(null);
      setSparse(null);
      setFrames([]);
      setFramePath(null);
    }
    try {
      const result = await startPipeline({
        sourcePath,
        sourceKind,
        projectDir: active.dir,
        archiveDir: config.archiveDir,
        tempProject: active.temp,
        settings,
        force: density === "expert" && force,
        until,
      });
      setCompleted(result.completedStage);
      if (result.completedStage === "train") {
        setPlyPath(result.plyPath);
        setStatus("done");
      } else {
        setStatus("paused");
        setProgress({
          stage: result.completedStage,
          percent: 100,
          message: result.completedStage === "frames" ? "Frames ready" : "Cameras ready",
        });
      }
      if (result.archiveError) {
        setError(result.archiveError);
      }
      try {
        const listed = await listProjectFrames(result.projectDir);
        setFrames(listed);
        setFramePath((current) => current ?? listed[0]?.path ?? null);
      } catch {
        /* filmstrip is optional after a run */
      }
      if (result.completedStage === "colmap" || result.completedStage === "train") {
        try {
          const preview = await getSparsePreview(result.projectDir);
          setSparse(preview.cameras.length || preview.points.length ? preview : null);
        } catch {
          /* sparse preview is optional */
        }
      }
      await refreshProjects();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setStatus(message === "Cancelled" ? "idle" : "error");
    }
  }

  const running = status === "running";
  runningRef.current = running;
  const hasProject = Boolean(project || (sourcePath && config));
  const canEasy =
    Boolean(sourcePath && config?.archiveDir) && !running;

  let main: "splat" | "frames" | "cameras" = "splat";
  if (plyPath) {
    main = "splat";
  } else if (sparse && (progress?.stage === "colmap" || completed === "colmap" || completed === "train")) {
    main = "cameras";
  } else if (framePath || frames.length > 0) {
    main = "frames";
  }

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="inspector">
          {density === "expert" ? (
            <ProjectPicker
              project={project}
              projects={projects}
              disabled={running}
              onOpen={(path) => {
                void openProject(path).then(applyProject).catch((err: unknown) => {
                  setError(err instanceof Error ? err.message : String(err));
                });
              }}
            />
          ) : null}
          <DropZone
            sourcePath={sourcePath}
            sourceKind={sourceKind}
            disabled={running}
            onSource={onSource}
          />
          <section className="inspector-block">
            <h2>Setup</h2>
            <CaptureModePicker
              value={settings}
              disabled={running}
              onChange={setSettings}
              onHelp={() => setHintsOpen(true)}
            />
            <PresetPicker value={settings} disabled={running} onChange={setSettings} />
          </section>
          {density === "expert" || matchingPreset(settings) === "custom" ? (
            <SettingsPanel
              value={settings}
              sourceKind={sourceKind}
              disabled={running}
              onChange={setSettings}
            />
          ) : null}
          {density === "expert" ? (
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
          ) : null}
        </div>
        {density === "expert" ? (
          <div className="inspector-actions">
            <button
              type="button"
              className="primary"
              disabled={!expertCanRun("frames", completed, running, Boolean(sourcePath), hasProject)}
              onClick={() => void runUntil("frames")}
            >
              Extract frames
            </button>
            <button
              type="button"
              disabled={!expertCanRun("colmap", completed, running, Boolean(sourcePath), hasProject)}
              onClick={() => void runUntil("colmap")}
            >
              Estimate cameras
            </button>
            <button
              type="button"
              disabled={!expertCanRun("train", completed, running, Boolean(sourcePath), hasProject)}
              onClick={() => void runUntil("train")}
            >
              Train splat
            </button>
            <button type="button" disabled={!running} onClick={() => void cancelPipeline()}>
              Cancel
            </button>
          </div>
        ) : (
          <div className="inspector-actions">
            <button type="button" className="primary" disabled={!canEasy} onClick={() => void runUntil("train")}>
              Reconstruct
            </button>
            <button type="button" disabled={!running} onClick={() => void cancelPipeline()}>
              Cancel
            </button>
          </div>
        )}
      </aside>
      <main className="main">
        {main === "cameras" && sparse ? (
          <CameraPreview preview={sparse} />
        ) : main === "frames" ? (
          <FramePreview currentPath={framePath} frames={frames} onSelect={setFramePath} />
        ) : (
          <SplatViewer
            plyPath={plyPath}
            captureMode={settings.captureMode}
            viewer={settings.viewer}
            live={running}
            fullscreen={fullscreen}
            onToggleFullscreen={onToggleFullscreen}
          />
        )}
        <RunProgress
          status={status}
          progress={progress}
          frames={frameStats}
          cameras={cameraStats}
          train={train}
          error={error}
        />
      </main>
      <CaptureHints mode={settings.captureMode} open={hintsOpen} onClose={() => setHintsOpen(false)} />
    </div>
  );
}
