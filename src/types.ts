export type Preset = "fast" | "balanced" | "quality";

export type CaptureMode = "object" | "room" | "outdoor";

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
};

export type PipelineRequest = {
  sourcePath: string;
  sourceKind: SourceKind;
  projectDir?: string | null;
  archiveDir: string;
  tempProject: boolean;
  settings: PipelineSettings;
  force: boolean;
};

export type RunStatus = "idle" | "running" | "done" | "error";

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
};

export type RunResult = {
  plyPath: string;
  archiveId?: string | null;
  archiveError?: string | null;
};

export const PRESET_SETTINGS: Record<Preset, PipelineSettings> = {
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
  },
};

/** Hard video keyframe cap. Outdoor paths can run much longer than orbits or rooms. */
export function maxFramesCap(mode: CaptureMode): number {
  return mode === "outdoor" ? 10_000 : 800;
}

/** Returns the named preset when core knobs still match, ignoring clip trim. */
export function matchingPreset(settings: PipelineSettings): Preset | null {
  if (settings.extractMode === "change") {
    return null;
  }
  for (const id of ["fast", "balanced", "quality"] as const) {
    const preset = PRESET_SETTINGS[id];
    if (
      preset.fps === settings.fps &&
      preset.maxImageSize === settings.maxImageSize &&
      preset.trainSteps === settings.trainSteps &&
      preset.matchOverlap === settings.matchOverlap &&
      preset.maxSplats === settings.maxSplats &&
      preset.maxFrames === settings.maxFrames
    ) {
      return id;
    }
  }
  return null;
}
