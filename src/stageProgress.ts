import type { ProgressEvent, RunStatus, Stage } from "./types";

const STAGES: Stage[] = ["frames", "colmap", "train"];

/** Percent label for the live bar, always a whole number plus a percent sign. */
export function formatPercent(value: number): string {
  return `${Math.round(Math.min(100, Math.max(0, value)))}%`;
}

/** Per-stage percent for the checklist. Completed stages are 100; later ones are null. */
export function stagePercents(
  progress: ProgressEvent | null,
  status: RunStatus,
): Record<Stage, number | null> {
  const out: Record<Stage, number | null> = {
    frames: null,
    colmap: null,
    train: null,
  };
  if (status === "done") {
    out.frames = 100;
    out.colmap = 100;
    out.train = 100;
    return out;
  }
  if (!progress) {
    return out;
  }
  const current = STAGES.indexOf(progress.stage);
  for (const [index, stage] of STAGES.entries()) {
    if (index < current) {
      out[stage] = 100;
    } else if (index === current) {
      out[stage] = progress.percent;
    }
  }
  if (status === "paused") {
    const doneUntil = STAGES.indexOf(progress.stage);
    for (const [index, stage] of STAGES.entries()) {
      if (index <= doneUntil) {
        out[stage] = 100;
      }
    }
  }
  return out;
}
