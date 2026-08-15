import type { Stage } from "./types";

export type ExpertAction = "frames" | "colmap" | "train";

/** Which pipeline stage Expert should run next, based on the last completed marker. */
export function nextExpertAction(completed: Stage | null): ExpertAction {
  if (completed === "frames") {
    return "colmap";
  }
  if (completed === "colmap" || completed === "train") {
    return "train";
  }
  return "frames";
}

/** Whether an Expert stage button can start. Source and project must already be set. */
export function expertCanRun(
  action: ExpertAction,
  completed: Stage | null,
  running: boolean,
  hasSource: boolean,
  hasProject: boolean,
): boolean {
  if (running || !hasSource || !hasProject) {
    return false;
  }
  if (action === "frames") {
    return true;
  }
  if (action === "colmap") {
    return completed === "frames" || completed === "colmap" || completed === "train";
  }
  return completed === "colmap" || completed === "train";
}
