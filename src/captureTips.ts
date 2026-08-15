import type { CaptureMode } from "./types";

const SHARED = [
  "Keep exposure and focal length fixed. Do not zoom while filming.",
  "Avoid motion blur: stabilize, add light, or slow down.",
  "Quality preset can take hours on Apple Silicon. Start with Fast.",
  "16 GB unified memory is the practical minimum.",
];

const TIPS: Record<CaptureMode, string[]> = {
  object: ["Walk a slow orbit around the subject. Overlap consecutive views.", ...SHARED],
  room: [
    "Walk slowly along the walls. Look slightly at surfaces, not empty floor.",
    "Overlap consecutive views. Empty walls, mirrors, and exposure jumps break matching.",
    "See-through walls come from unobserved angles and floaters, not the viewer. Close the loop and avoid blank surfaces.",
    ...SHARED,
  ],
  outdoor: [
    "Walk slowly along a path. Overlap consecutive views.",
    "Tilt the camera slightly down — empty sky has no features.",
    "Avoid wind-blown vegetation and hard sun/shade exposure jumps.",
    ...SHARED,
  ],
};

/** Capture advice for the selected mode, including shared filming rules. */
export function captureTips(mode: CaptureMode): string[] {
  return TIPS[mode];
}
