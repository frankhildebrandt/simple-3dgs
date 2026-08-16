import type { AppView } from "./types";
import type { ViewerMode } from "./viewerMode";
import { VIEWER_MODES } from "./viewerMode";

const APP_VIEWS: AppView[] = ["easy", "expert", "archive"];

export type MenuProjectAction = "new" | "open";

const PROJECT_ACTIONS: MenuProjectAction[] = ["new", "open"];

/** Reads Easy / Expert / Archive from a native menu event. */
export function parseMenuView(payload: string): AppView | null {
  return APP_VIEWS.find((view) => view === payload) ?? null;
}

/** Reads Splats / Dots / Discs from a native menu event. */
export function parseMenuMode(payload: string): ViewerMode | null {
  return VIEWER_MODES.find((mode) => mode === payload) ?? null;
}

/** Reads New Project / Open from a native File menu event. */
export function parseMenuProject(payload: string): MenuProjectAction | null {
  return PROJECT_ACTIONS.find((action) => action === payload) ?? null;
}
