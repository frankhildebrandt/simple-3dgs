import type { BrushKnobs } from "./brushKnobs";
import { DEFAULT_BRUSH_KNOBS } from "./brushKnobs";
import type { CaptureMode } from "./captureMode";
import type { ColmapKnobs } from "./colmapKnobs";
import { colmapKnobsFor } from "./colmapKnobs";
import type { ExtractKnobs } from "./extractKnobs";
import { DEFAULT_EXTRACT_KNOBS } from "./extractKnobs";
import type { ViewerKnobs } from "./viewerKnobs";
import { viewerKnobsFor } from "./viewerKnobs";

export type KnobGroups = {
  extract: ExtractKnobs;
  colmap: ColmapKnobs;
  brush: BrushKnobs;
  viewer: ViewerKnobs;
};

/** Nested knobs Fast/Balanced/Quality still fill from capture type. */
export function defaultsFor(mode: CaptureMode): KnobGroups {
  return {
    extract: { ...DEFAULT_EXTRACT_KNOBS },
    colmap: colmapKnobsFor(mode),
    brush: { ...DEFAULT_BRUSH_KNOBS },
    viewer: viewerKnobsFor(mode),
  };
}

/** True when two knob snapshots match within float noise from JSON. */
export function knobsEqual<T extends Record<string, unknown>>(left: T, right: T): boolean {
  for (const key of Object.keys(left) as Array<keyof T>) {
    const a = left[key];
    const b = right[key];
    if (typeof a === "number" && typeof b === "number") {
      if (!numbersClose(a, b)) {
        return false;
      }
      continue;
    }
    if (a !== b) {
      return false;
    }
  }
  return true;
}

function numbersClose(a: number, b: number): boolean {
  return Math.abs(a - b) <= 1e-6 * Math.max(1, Math.abs(a), Math.abs(b));
}
