/** Video keyframe scoring knobs. Named presets use these constants. */

export type ExtractKnobs = {
  blurFloor: number;
  moderateMad: number;
  candidateMinFps: number;
  candidateMaxFps: number;
  changeCandidateMaxFps: number;
  changeMadSparse: number;
  changeMadDense: number;
  thresholdRelaxSteps: number;
  minFrames: number;
};

export const DEFAULT_EXTRACT_KNOBS: ExtractKnobs = {
  blurFloor: 15,
  moderateMad: 8,
  candidateMinFps: 8,
  candidateMaxFps: 12,
  changeCandidateMaxFps: 24,
  changeMadSparse: 96,
  changeMadDense: 4,
  thresholdRelaxSteps: 6,
  minFrames: 8,
};
