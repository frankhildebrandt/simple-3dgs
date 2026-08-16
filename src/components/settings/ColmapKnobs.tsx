import type {
  ColmapCameraModel,
  ColmapMatcher,
  ColmapMapper,
  PipelineSettings,
  SiftBackend,
} from "../../types";
import { SAFETY } from "../../safety";
import { Knob } from "./Knob";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

const CAMERA_MODELS: ColmapCameraModel[] = [
  "SIMPLE_RADIAL",
  "SIMPLE_PINHOLE",
  "PINHOLE",
  "RADIAL",
  "OPENCV",
];

/** COLMAP SfM knobs that named presets fill from capture mode. */
export function ColmapKnobs({ value, disabled, onChange }: Props) {
  const colmap = value.colmap;

  function patch<K extends keyof typeof colmap>(key: K, next: (typeof colmap)[K]) {
    onChange({ ...value, colmap: { ...colmap, [key]: next } });
  }

  return (
    <fieldset disabled={disabled}>
      <legend>Reconstruction</legend>
      <label
        className="format"
        title="Metal uses the GPU for SIFT extract and matching. CPU is VLFeat extract plus Eigen matching. Metal needs the colmap-metal sidecar."
        data-hint="Metal uses the GPU for SIFT extract and matching. CPU is VLFeat extract plus Eigen matching. Metal needs the colmap-metal sidecar."
      >
        <span>SIFT</span>
        <div className="row">
          <button
            type="button"
            className={colmap.siftBackend === "cpu" ? "selected" : ""}
            onClick={() => patch("siftBackend", "cpu" satisfies SiftBackend)}
          >
            CPU
          </button>
          <button
            type="button"
            className={colmap.siftBackend === "metal" ? "selected" : ""}
            onClick={() => patch("siftBackend", "metal" satisfies SiftBackend)}
          >
            Metal
          </button>
        </div>
        <output>{colmap.siftBackend}</output>
      </label>
      <Knob
        label="Match overlap"
        hint="How many neighboring frames sequential matching considers."
        min={SAFETY.matchOverlapMin}
        max={SAFETY.matchOverlapMax}
        step={1}
        value={value.matchOverlap}
        onChange={(matchOverlap) => onChange({ ...value, matchOverlap })}
      />
      <Knob
        label="Overlap floor"
        hint="Minimum sequential overlap. 0 means use match overlap as-is."
        min={0}
        max={SAFETY.matchOverlapMax}
        step={1}
        value={colmap.minOverlapFloor}
        onChange={(minOverlapFloor) => patch("minOverlapFloor", minOverlapFloor)}
      />
      <label
        className="format"
        title="Exhaustive tries every pair until the frame limit, then sequential."
        data-hint="Exhaustive tries every pair until the frame limit, then sequential."
      >
        <span>Matcher</span>
        <div className="row">
          <button
            type="button"
            className={colmap.matcher === "sequential" ? "selected" : ""}
            onClick={() => patch("matcher", "sequential" satisfies ColmapMatcher)}
          >
            Sequential
          </button>
          <button
            type="button"
            className={colmap.matcher === "exhaustive" ? "selected" : ""}
            onClick={() => patch("matcher", "exhaustive" satisfies ColmapMatcher)}
          >
            Exhaustive
          </button>
        </div>
        <output>{colmap.matcher}</output>
      </label>
      <Knob
        label="Exhaustive limit"
        hint="Above this frame count, exhaustive matching falls back to sequential."
        min={2}
        max={SAFETY.maxFramesMax}
        step={1}
        value={colmap.exhaustiveFrameLimit}
        onChange={(exhaustiveFrameLimit) => patch("exhaustiveFrameLimit", exhaustiveFrameLimit)}
      />
      <label
        className="format"
        title="Global SfM (COLMAP 4) for rooms; incremental mapper for orbits and paths."
        data-hint="Global SfM (COLMAP 4) for rooms; incremental mapper for orbits and paths."
      >
        <span>Mapper</span>
        <div className="row">
          <button
            type="button"
            className={colmap.mapper === "incremental" ? "selected" : ""}
            onClick={() => patch("mapper", "incremental" satisfies ColmapMapper)}
          >
            Incremental
          </button>
          <button
            type="button"
            className={colmap.mapper === "global" ? "selected" : ""}
            onClick={() => patch("mapper", "global" satisfies ColmapMapper)}
          >
            Global
          </button>
        </div>
        <output>{colmap.mapper}</output>
      </label>
      <label title="COLMAP ImageReader.camera_model" data-hint="COLMAP ImageReader.camera_model">
        <span>Camera model</span>
        <select
          value={colmap.cameraModel}
          onChange={(event) => patch("cameraModel", event.currentTarget.value as ColmapCameraModel)}
        >
          {CAMERA_MODELS.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </select>
        <output />
      </label>
      <Knob
        label="Min model size"
        hint="COLMAP Mapper.min_model_size. Smaller models are discarded."
        min={2}
        max={200}
        step={1}
        value={colmap.minModelSize}
        onChange={(minModelSize) => patch("minModelSize", minModelSize)}
      />
      <Knob
        label="Init tri angle"
        hint="Mapper.init_min_tri_angle. 0 omits the flag (COLMAP default)."
        min={0}
        max={90}
        step={1}
        value={colmap.initMinTriAngle}
        onChange={(initMinTriAngle) => patch("initMinTriAngle", initMinTriAngle)}
      />
      <label
        className="check"
        title="Assume one camera and one focal length."
        data-hint="Assume one camera and one focal length."
      >
        <input
          type="checkbox"
          checked={colmap.singleCamera}
          onChange={(event) => patch("singleCamera", event.currentTarget.checked)}
        />
        Single camera
      </label>
      <label
        className="check"
        title="SequentialMatching.quadratic_overlap. Helps room loops close."
        data-hint="SequentialMatching.quadratic_overlap. Helps room loops close."
      >
        <input
          type="checkbox"
          checked={colmap.quadraticOverlap}
          onChange={(event) => patch("quadraticOverlap", event.currentTarget.checked)}
        />
        Quadratic overlap
      </label>
    </fieldset>
  );
}
