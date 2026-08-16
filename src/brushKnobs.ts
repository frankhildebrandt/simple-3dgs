/** Brush CLI knobs. Zero refine/export cadence means keep the old frame/step heuristic. */

export type BrushKnobs = {
  lrMean: number;
  lrMeanEnd: number;
  meanNoiseWeight: number;
  lrCoeffsDc: number;
  lrCoeffsShScale: number;
  lrOpac: number;
  lrScale: number;
  lrRotation: number;
  ssimWeight: number;
  opacDecay: number;
  lpipsLossWeight: number;
  matchAlphaWeight: number;
  backgroundR: number;
  backgroundG: number;
  backgroundB: number;
  backgroundNoiseStrength: number;
  refineEvery: number;
  growthGradThreshold: number;
  growthSelectFraction: number;
  growthStopIter: number;
  splitAtScreenSize: number;
  shDegree: number;
  seed: number;
  evalEvery: number;
  exportEvery: number;
  maxSceneBatchCacheGib: number;
  lodLevels: number;
  lodRefineSteps: number;
  lodDecimationKeep: number;
  lodImageScale: number;
  trainMaxResolution: number;
};

export const DEFAULT_BRUSH_KNOBS: BrushKnobs = {
  lrMean: 2e-5,
  lrMeanEnd: 2e-7,
  meanNoiseWeight: 50,
  lrCoeffsDc: 2e-3,
  lrCoeffsShScale: 10,
  lrOpac: 0.012,
  lrScale: 5e-3,
  lrRotation: 2e-3,
  ssimWeight: 0.2,
  opacDecay: 0.004,
  lpipsLossWeight: 0,
  matchAlphaWeight: 0.1,
  backgroundR: 0,
  backgroundG: 0,
  backgroundB: 0,
  backgroundNoiseStrength: 0.1,
  refineEvery: 0,
  growthGradThreshold: 0.0025,
  growthSelectFraction: 0.25,
  growthStopIter: 15000,
  splitAtScreenSize: 0.5,
  shDegree: 3,
  seed: 42,
  evalEvery: 1000,
  exportEvery: 0,
  maxSceneBatchCacheGib: 6,
  lodLevels: 0,
  lodRefineSteps: 5000,
  lodDecimationKeep: 50,
  lodImageScale: 50,
  trainMaxResolution: 1920,
};
