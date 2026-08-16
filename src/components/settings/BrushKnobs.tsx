import type { PipelineSettings } from "../../types";
import { SAFETY } from "../../safety";
import { Knob } from "./Knob";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

/** Brush training knobs. Zero refine/export cadence keeps the old heuristic. */
export function BrushKnobs({ value, disabled, onChange }: Props) {
  const brush = value.brush;

  function patch<K extends keyof typeof brush>(key: K, next: (typeof brush)[K]) {
    onChange({ ...value, brush: { ...brush, [key]: next } });
  }

  return (
    <fieldset disabled={disabled}>
      <legend>Training</legend>
      <Knob
        label="Train steps"
        hint="Brush optimization steps. More is slower and usually cleaner."
        min={SAFETY.trainStepsMin}
        max={SAFETY.trainStepsMax}
        step={100}
        value={value.trainSteps}
        onChange={(trainSteps) => onChange({ ...value, trainSteps })}
      />
      <Knob
        label="Max splats"
        hint="Upper bound on Gaussians, in millions. Needs RAM."
        min={SAFETY.maxSplatsMin / 1_000_000}
        max={SAFETY.maxSplatsMax / 1_000_000}
        step={0.1}
        value={value.maxSplats / 1_000_000}
        display={(value.maxSplats / 1_000_000).toFixed(1)}
        onChange={(millions) =>
          onChange({ ...value, maxSplats: Math.round(millions * 1_000_000) })
        }
      />
      <Knob
        label="Train max res"
        hint="Brush --max-resolution. 0 means native when extract is native."
        min={0}
        max={SAFETY.maxImageSizeMax}
        step={64}
        value={brush.trainMaxResolution}
        onChange={(trainMaxResolution) => patch("trainMaxResolution", trainMaxResolution)}
      />
      <Knob
        label="SH degree"
        hint="Spherical harmonics degree. 3 is the Brush default."
        min={0}
        max={5}
        step={1}
        value={brush.shDegree}
        onChange={(shDegree) => patch("shDegree", shDegree)}
      />
      <Knob
        label="Refine every"
        hint="Densify cadence. 0 keeps the old frame-count heuristic (50–200)."
        min={0}
        max={2000}
        step={1}
        value={brush.refineEvery}
        onChange={(refineEvery) => patch("refineEvery", refineEvery)}
      />
      <Knob
        label="Export every"
        hint="PLY preview cadence. 0 keeps the old steps/10 clamp (250–1000)."
        min={0}
        max={20_000}
        step={10}
        value={brush.exportEvery}
        onChange={(exportEvery) => patch("exportEvery", exportEvery)}
      />
      <details>
        <summary>Learning rates</summary>
        <Knob label="LR mean" hint="--lr-mean" min={0} max={0.001} step={1e-6} value={brush.lrMean} onChange={(lrMean) => patch("lrMean", lrMean)} />
        <Knob label="LR mean end" hint="--lr-mean-end" min={0} max={0.0001} step={1e-8} value={brush.lrMeanEnd} onChange={(lrMeanEnd) => patch("lrMeanEnd", lrMeanEnd)} />
        <Knob label="Mean noise" hint="--mean-noise-weight" min={0} max={200} step={1} value={brush.meanNoiseWeight} onChange={(meanNoiseWeight) => patch("meanNoiseWeight", meanNoiseWeight)} />
        <Knob label="LR DC" hint="--lr-coeffs-dc" min={0} max={0.05} step={1e-4} value={brush.lrCoeffsDc} onChange={(lrCoeffsDc) => patch("lrCoeffsDc", lrCoeffsDc)} />
        <Knob label="LR SH scale" hint="--lr-coeffs-sh-scale" min={0} max={50} step={0.1} value={brush.lrCoeffsShScale} onChange={(lrCoeffsShScale) => patch("lrCoeffsShScale", lrCoeffsShScale)} />
        <Knob label="LR opacity" hint="--lr-opac" min={0} max={0.1} step={0.001} value={brush.lrOpac} onChange={(lrOpac) => patch("lrOpac", lrOpac)} />
        <Knob label="LR scale" hint="--lr-scale" min={0} max={0.05} step={1e-4} value={brush.lrScale} onChange={(lrScale) => patch("lrScale", lrScale)} />
        <Knob label="LR rotation" hint="--lr-rotation" min={0} max={0.05} step={1e-4} value={brush.lrRotation} onChange={(lrRotation) => patch("lrRotation", lrRotation)} />
      </details>
      <details>
        <summary>Densify and loss</summary>
        <Knob label="Growth grad" hint="--growth-grad-threshold" min={0} max={0.05} step={0.0001} value={brush.growthGradThreshold} onChange={(growthGradThreshold) => patch("growthGradThreshold", growthGradThreshold)} />
        <Knob label="Growth fraction" hint="--growth-select-fraction" min={0} max={1} step={0.01} value={brush.growthSelectFraction} onChange={(growthSelectFraction) => patch("growthSelectFraction", growthSelectFraction)} />
        <Knob label="Growth stop" hint="--growth-stop-iter" min={0} max={SAFETY.trainStepsMax} step={100} value={brush.growthStopIter} onChange={(growthStopIter) => patch("growthStopIter", growthStopIter)} />
        <Knob label="Split screen" hint="--split-at-screen-size. 0 disables." min={0} max={4} step={0.05} value={brush.splitAtScreenSize} onChange={(splitAtScreenSize) => patch("splitAtScreenSize", splitAtScreenSize)} />
        <Knob label="SSIM weight" hint="--ssim-weight" min={0} max={1} step={0.01} value={brush.ssimWeight} onChange={(ssimWeight) => patch("ssimWeight", ssimWeight)} />
        <Knob label="Opacity decay" hint="--opac-decay" min={0} max={0.05} step={0.0005} value={brush.opacDecay} onChange={(opacDecay) => patch("opacDecay", opacDecay)} />
        <Knob label="LPIPS weight" hint="--lpips-loss-weight" min={0} max={1} step={0.01} value={brush.lpipsLossWeight} onChange={(lpipsLossWeight) => patch("lpipsLossWeight", lpipsLossWeight)} />
        <Knob label="Match alpha" hint="--match-alpha-weight" min={0} max={1} step={0.01} value={brush.matchAlphaWeight} onChange={(matchAlphaWeight) => patch("matchAlphaWeight", matchAlphaWeight)} />
        <Knob label="BG noise" hint="--background-noise-strength" min={0} max={1} step={0.01} value={brush.backgroundNoiseStrength} onChange={(backgroundNoiseStrength) => patch("backgroundNoiseStrength", backgroundNoiseStrength)} />
        <Knob label="Background R" hint="Background red 0–1" min={0} max={1} step={0.01} value={brush.backgroundR} onChange={(backgroundR) => patch("backgroundR", backgroundR)} />
        <Knob label="Background G" hint="Background green 0–1" min={0} max={1} step={0.01} value={brush.backgroundG} onChange={(backgroundG) => patch("backgroundG", backgroundG)} />
        <Knob label="Background B" hint="Background blue 0–1" min={0} max={1} step={0.01} value={brush.backgroundB} onChange={(backgroundB) => patch("backgroundB", backgroundB)} />
      </details>
      <details>
        <summary>Process</summary>
        <Knob label="Seed" hint="--seed" min={0} max={10_000} step={1} value={brush.seed} onChange={(seed) => patch("seed", seed)} />
        <Knob label="Eval every" hint="--eval-every" min={1} max={20_000} step={10} value={brush.evalEvery} onChange={(evalEvery) => patch("evalEvery", evalEvery)} />
        <Knob label="Cache GiB" hint="--max-scene-batch-cache-size" min={0.5} max={128} step={0.5} value={brush.maxSceneBatchCacheGib} onChange={(maxSceneBatchCacheGib) => patch("maxSceneBatchCacheGib", maxSceneBatchCacheGib)} />
        <Knob label="LOD levels" hint="Brush post-train LoD. 0 disables." min={0} max={16} step={1} value={brush.lodLevels} onChange={(lodLevels) => patch("lodLevels", lodLevels)} />
        <Knob label="LOD refine" hint="--lod-refine-steps" min={0} max={50_000} step={100} value={brush.lodRefineSteps} onChange={(lodRefineSteps) => patch("lodRefineSteps", lodRefineSteps)} />
        <Knob label="LOD keep %" hint="--lod-decimation-keep" min={1} max={100} step={1} value={brush.lodDecimationKeep} onChange={(lodDecimationKeep) => patch("lodDecimationKeep", lodDecimationKeep)} />
        <Knob label="LOD image %" hint="--lod-image-scale" min={1} max={100} step={1} value={brush.lodImageScale} onChange={(lodImageScale) => patch("lodImageScale", lodImageScale)} />
      </details>
    </fieldset>
  );
}
