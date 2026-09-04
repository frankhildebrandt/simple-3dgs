import type { RunStatus, Stage } from "./types";

export type ExpertAction = "frames" | "colmap" | "train";

export type RerunChoice = "continue" | "restart";

export type RerunDecision = { kind: "keep" } | { kind: "run"; restart: boolean };

const STAGE_ORDER: Record<ExpertAction, number> = {
  frames: 0,
  colmap: 1,
  train: 2,
};

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

/** Whether this stage (or a later one) already has a completion marker. */
export function stageAlreadyComplete(action: ExpertAction, completed: Stage | null): boolean {
  return completed !== null && STAGE_ORDER[completed] >= STAGE_ORDER[action];
}

/** Re-running a finished stage, or retrying the one that just failed, needs a keep-or-redo prompt. */
export function needsRerunPrompt(
  action: ExpertAction,
  completed: Stage | null,
  status: RunStatus,
): boolean {
  if (stageAlreadyComplete(action, completed)) {
    return true;
  }
  return status === "error" && nextExpertAction(completed) === action;
}

/** Continue keeps finished outputs; restart wipes this step. Unfinished work can resume. */
export function rerunDecision(choice: RerunChoice, alreadyComplete: boolean): RerunDecision {
  if (choice === "restart") {
    return { kind: "run", restart: true };
  }
  if (alreadyComplete) {
    return { kind: "keep" };
  }
  return { kind: "run", restart: false };
}

/** Restarting an earlier finished stage also deletes later markers and outputs. */
export function rerunDiscardsLater(action: ExpertAction, completed: Stage | null): boolean {
  return stageAlreadyComplete(action, completed) && completed !== action;
}
