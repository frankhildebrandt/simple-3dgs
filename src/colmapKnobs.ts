import type { CaptureMode } from "./captureMode";

export type ColmapMatcher = "sequential" | "exhaustive";

export type ColmapMapper = "incremental" | "global";

export type SiftBackend = "cpu" | "metal";

export type ColmapCameraModel =
  | "SIMPLE_RADIAL"
  | "SIMPLE_PINHOLE"
  | "PINHOLE"
  | "RADIAL"
  | "OPENCV";

/** SfM knobs that used to follow capture mode only. */
export type ColmapKnobs = {
  cameraModel: ColmapCameraModel;
  singleCamera: boolean;
  matcher: ColmapMatcher;
  exhaustiveFrameLimit: number;
  quadraticOverlap: boolean;
  minOverlapFloor: number;
  mapper: ColmapMapper;
  minModelSize: number;
  initMinTriAngle: number;
  siftBackend: SiftBackend;
};

const SHARED: Pick<
  ColmapKnobs,
  "cameraModel" | "singleCamera" | "exhaustiveFrameLimit" | "siftBackend"
> = {
  cameraModel: "SIMPLE_RADIAL",
  singleCamera: true,
  exhaustiveFrameLimit: 250,
  siftBackend: "cpu",
};

/** COLMAP profile that Fast/Balanced/Quality still apply for a capture type. */
export function colmapKnobsFor(mode: CaptureMode): ColmapKnobs {
  switch (mode) {
    case "room":
      return {
        ...SHARED,
        matcher: "exhaustive",
        quadraticOverlap: true,
        minOverlapFloor: 20,
        mapper: "global",
        minModelSize: 10,
        initMinTriAngle: 0,
      };
    case "outdoor":
      return {
        ...SHARED,
        matcher: "sequential",
        quadraticOverlap: false,
        minOverlapFloor: 20,
        mapper: "incremental",
        minModelSize: 10,
        initMinTriAngle: 8,
      };
    case "object":
      return {
        ...SHARED,
        matcher: "sequential",
        quadraticOverlap: false,
        minOverlapFloor: 0,
        mapper: "incremental",
        minModelSize: 6,
        initMinTriAngle: 0,
      };
  }
}
