import type { BrushKnobs } from "./brushKnobs";
import type { CaptureMode } from "./captureMode";
import type { ColmapKnobs } from "./colmapKnobs";
import type { ExtractKnobs } from "./extractKnobs";
import { defaultsFor, knobsEqual } from "./knobDefaults";
import type { ViewerKnobs } from "./viewerKnobs";

export type NamedPreset = "fast" | "balanced" | "quality";

export type Preset = NamedPreset | "custom";

export type { CaptureMode };
export type { ColmapCameraModel, ColmapKnobs, ColmapMatcher, ColmapMapper } from "./colmapKnobs";
export type { BrushKnobs, ExtractKnobs, ViewerKnobs };
export { DEFAULT_BRUSH_KNOBS } from "./brushKnobs";
export { defaultsFor };
export { DEFAULT_EXTRACT_KNOBS } from "./extractKnobs";
export { colmapKnobsFor } from "./colmapKnobs";
export { viewerKnobsFor } from "./viewerKnobs";

export type SourceKind = "video" | "images";

export type Stage = "frames" | "colmap" | "train";

export type ProgressEvent = {
  stage: Stage;
  percent: number;
  message: string;
};

export type TrainStats = {
  total: number;
  iter?: number | null;
  splats?: number | null;
  psnr?: number | null;
  ssim?: number | null;
  trainViews?: number | null;
  evalViews?: number | null;
  elapsedSecs?: number | null;
  etaSecs?: number | null;
};

export type FramePass = "candidates" | "keyframes" | "import";

export type FrameStats = {
  pass: FramePass;
  current?: number | null;
  total?: number | null;
  kept?: number | null;
  elapsedSecs?: number | null;
  etaSecs?: number | null;
  durationSecs?: number | null;
};

export type CameraStep = "features" | "matching" | "calibrating" | "mapping";

export type CameraStats = {
  step: CameraStep;
  processed?: number | null;
  total?: number | null;
  features?: number | null;
  matches?: number | null;
  registered?: number | null;
  points?: number | null;
  trying?: number | null;
  failed?: number | null;
  elapsedSecs?: number | null;
  etaSecs?: number | null;
};

export type SparseCamera = {
  name: string;
  position: [number, number, number];
  quaternion: [number, number, number, number];
};

export type SparsePreview = {
  cameras: SparseCamera[];
  points: [number, number, number][];
  colors: [number, number, number][];
};

export type ProjectFrame = {
  name: string;
  path: string;
  index: number;
  sharpness: number;
  motion: number;
  selected: boolean;
};

export type ProjectEntry = {
  id: string;
  title: string;
  sourcePath: string;
  sourceKind: string;
  settings: PipelineSettings;
  stage: string;
  createdAt: string;
  updatedAt: string;
  temp: boolean;
  dir: string;
  frameCount: number;
  plyPath?: string | null;
  hasFrames: boolean;
  hasCameras: boolean;
};

export type FrameFormat = "jpg" | "png";

export type ExtractMode = "density" | "change";

export type PipelineSettings = {
  fps: number;
  maxImageSize: number;
  startSeconds: number;
  durationSeconds: number;
  frameFormat: FrameFormat;
  jpegQuality: number;
  trainSteps: number;
  matchOverlap: number;
  captureMode: CaptureMode;
  maxSplats: number;
  maxFrames: number;
  extractMode: ExtractMode;
  extractQuality: number;
  extract: ExtractKnobs;
  colmap: ColmapKnobs;
  brush: BrushKnobs;
  viewer: ViewerKnobs;
};

export type PipelineRequest = {
  sourcePath: string;
  sourceKind: SourceKind;
  projectDir?: string | null;
  archiveDir: string;
  tempProject: boolean;
  settings: PipelineSettings;
  force: boolean;
  until?: Stage;
};

export type RunStatus = "idle" | "running" | "paused" | "done" | "error";

export type AppView = "easy" | "expert" | "archive";

export type GeoFix = {
  lat: number;
  lon: number;
  alt?: number | null;
  source: string;
};

export type ArchiveEntry = {
  id: string;
  title: string;
  createdAt: string;
  sourceKind: string;
  sourceName: string;
  settings?: PipelineSettings | null;
  frameCount: number;
  plyBytes: number;
  geo?: GeoFix | null;
  poster?: string | null;
  plyPath: string;
  posterPath?: string | null;
  dir: string;
  hasPly: boolean;
};

export type AppConfig = {
  archiveDir: string;
  uiMode: AppView;
  tempProject: boolean;
  projectDir?: string | null;
  projectsDir?: string | null;
};

export type RunResult = {
  plyPath: string;
  archiveId?: string | null;
  archiveError?: string | null;
  completedStage: Stage;
  projectDir: string;
};

const OBJECT_KNOBS = defaultsFor("object");

export const PRESET_SETTINGS: Record<NamedPreset, PipelineSettings> = {
  fast: {
    fps: 1,
    maxImageSize: 800,
    startSeconds: 0,
    durationSeconds: 0,
    frameFormat: "jpg",
    jpegQuality: 80,
    trainSteps: 5000,
    matchOverlap: 15,
    captureMode: "object",
    maxSplats: 2_000_000,
    maxFrames: 120,
    extractMode: "density",
    extractQuality: 55,
    ...OBJECT_KNOBS,
  },
  balanced: {
    fps: 2,
    maxImageSize: 1600,
    startSeconds: 0,
    durationSeconds: 0,
    frameFormat: "jpg",
    jpegQuality: 95,
    trainSteps: 15000,
    matchOverlap: 15,
    captureMode: "object",
    maxSplats: 5_000_000,
    maxFrames: 250,
    extractMode: "density",
    extractQuality: 55,
    ...OBJECT_KNOBS,
  },
  quality: {
    fps: 4,
    maxImageSize: 1920,
    startSeconds: 0,
    durationSeconds: 0,
    frameFormat: "jpg",
    jpegQuality: 100,
    trainSteps: 30000,
    matchOverlap: 15,
    captureMode: "object",
    maxSplats: 10_000_000,
    maxFrames: 500,
    extractMode: "density",
    extractQuality: 55,
    ...OBJECT_KNOBS,
  },
};

/** Hard video keyframe cap used when applying named presets. */
export function maxFramesCap(mode: CaptureMode): number {
  return mode === "outdoor" ? 10_000 : 800;
}

export type PipelineSettingsInput = Omit<
  PipelineSettings,
  "extract" | "colmap" | "brush" | "viewer"
> & {
  extract?: ExtractKnobs;
  colmap?: ColmapKnobs;
  brush?: BrushKnobs;
  viewer?: ViewerKnobs;
};

/** Fills missing nested groups from capture mode so legacy project JSON still matches. */
export function hydrateSettings(settings: PipelineSettingsInput): PipelineSettings {
  const fallback = defaultsFor(settings.captureMode);
  return {
    ...settings,
    extract: settings.extract ?? fallback.extract,
    colmap: settings.colmap ?? fallback.colmap,
    brush: settings.brush ?? fallback.brush,
    viewer: settings.viewer ?? fallback.viewer,
  };
}

/**
 * Applies a named recipe. Keeps clip trim, still format, and capture mode, then
 * resets nested knobs to that capture type's defaults.
 */
export function applyNamedPreset(id: NamedPreset, current: PipelineSettings): PipelineSettings {
  const recipe = PRESET_SETTINGS[id];
  return {
    ...recipe,
    startSeconds: current.startSeconds,
    durationSeconds: current.durationSeconds,
    frameFormat: current.frameFormat,
    captureMode: current.captureMode,
    ...defaultsFor(current.captureMode),
  };
}

/** Named capture change also reclamps frames and resets nested knobs. Custom only sets the mode. */
export function applyCaptureMode(settings: PipelineSettings, mode: CaptureMode): PipelineSettings {
  if (matchingPreset(settings) === "custom") {
    return { ...settings, captureMode: mode };
  }
  return {
    ...settings,
    captureMode: mode,
    maxFrames: Math.min(settings.maxFrames, maxFramesCap(mode)),
    ...defaultsFor(mode),
  };
}
export function matchingPreset(settings: PipelineSettings): Preset {
  const hydrated = hydrateSettings(settings);
  if (hydrated.extractMode === "change") {
    return "custom";
  }
  const expected = defaultsFor(hydrated.captureMode);
  if (
    !knobsEqual(hydrated.extract, expected.extract) ||
    !knobsEqual(hydrated.colmap, expected.colmap) ||
    !knobsEqual(hydrated.brush, expected.brush) ||
    !knobsEqual(hydrated.viewer, expected.viewer)
  ) {
    return "custom";
  }
  for (const id of ["fast", "balanced", "quality"] as const) {
    const preset = PRESET_SETTINGS[id];
    if (
      preset.fps === hydrated.fps &&
      preset.maxImageSize === hydrated.maxImageSize &&
      preset.trainSteps === hydrated.trainSteps &&
      preset.matchOverlap === hydrated.matchOverlap &&
      preset.maxSplats === hydrated.maxSplats &&
      preset.maxFrames === hydrated.maxFrames
    ) {
      return id;
    }
  }
  return "custom";
}
