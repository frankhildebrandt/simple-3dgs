import type { PipelineSettings, SourceKind } from "../../types";
import { SAFETY } from "../../safety";
import { Knob } from "./Knob";

type Props = {
  value: PipelineSettings;
  sourceKind: SourceKind;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

function patchNumber(
  value: PipelineSettings,
  onChange: (settings: PipelineSettings) => void,
  key: "startSeconds" | "durationSeconds",
  raw: string,
) {
  const parsed = Number(raw);
  if (!Number.isFinite(parsed)) {
    return;
  }
  onChange({ ...value, [key]: parsed });
}

/** Frame extract knobs: density/change, clip trim, format, and resolution. */
export function FrameKnobs({ value, sourceKind, disabled, onChange }: Props) {
  const video = sourceKind === "video";
  const nativeSize = value.maxImageSize === 0;
  const change = value.extractMode === "change";
  const extract = value.extract;

  return (
    <fieldset disabled={disabled}>
      <legend>Frames</legend>
      <label
        className={`format ${video ? "" : "dim"}`.trim()}
        title="Density uses a target fps and cap. Change takes a new frame when the picture moves."
        data-hint="Density uses a target fps and cap. Change takes a new frame when the picture moves."
      >
        <span>Extract</span>
        <div className="row">
          <button
            type="button"
            className={!change ? "selected" : ""}
            disabled={!video}
            onClick={() => onChange({ ...value, extractMode: "density" })}
          >
            Density
          </button>
          <button
            type="button"
            className={change ? "selected" : ""}
            disabled={!video}
            onClick={() => onChange({ ...value, extractMode: "change" })}
          >
            Change
          </button>
        </div>
        <output>{change ? "on change" : "by density"}</output>
      </label>
      {change ? (
        <Knob
          label="Extract quality"
          hint="How soon a new frame is taken when the picture changes. 100 keeps more overlap."
          min={1}
          max={100}
          step={1}
          value={value.extractQuality}
          disabled={!video}
          dim={!video}
          onChange={(extractQuality) => onChange({ ...value, extractQuality })}
        />
      ) : (
        <Knob
          label="Target density"
          hint="Frame density while the camera moves. Ignored for image folders."
          min={SAFETY.fpsMin}
          max={SAFETY.fpsMax}
          step={0.25}
          value={value.fps}
          disabled={!video}
          dim={!video}
          onChange={(fps) => onChange({ ...value, fps })}
        />
      )}
      <Knob
        label="Max frames"
        hint="Hard cap on extracted video frames."
        min={SAFETY.maxFramesMin}
        max={SAFETY.maxFramesMax}
        step={1}
        value={value.maxFrames}
        disabled={!video}
        dim={!video}
        onChange={(maxFrames) => onChange({ ...value, maxFrames })}
      />
      {video && value.colmap.matcher === "exhaustive" && value.maxFrames > value.colmap.exhaustiveFrameLimit ? (
        <p className="note">
          Exhaustive matching only up to {value.colmap.exhaustiveFrameLimit} frames; extra frames use sequential matching.
        </p>
      ) : null}
      <label className={video ? undefined : "dim"} title="Skip this many seconds at the start of the clip" data-hint="Skip this many seconds at the start of the clip">
        <span>Start</span>
        <input
          type="number"
          min={0}
          step={0.5}
          value={value.startSeconds}
          disabled={!video}
          onChange={(event) => patchNumber(value, onChange, "startSeconds", event.currentTarget.value)}
        />
        <output>s</output>
      </label>
      <label
        className={video ? undefined : "dim"}
        title="0 means the rest of the clip"
        data-hint="0 means the rest of the clip"
      >
        <span>Duration</span>
        <input
          type="number"
          min={0}
          step={0.5}
          value={value.durationSeconds}
          disabled={!video}
          onChange={(event) =>
            patchNumber(value, onChange, "durationSeconds", event.currentTarget.value)
          }
        />
        <output>{value.durationSeconds === 0 ? "full" : "s"}</output>
      </label>
      <label
        className={`format ${video ? "" : "dim"}`.trim()}
        title="PNG is lossless and slower; JPEG is smaller"
        data-hint="PNG is lossless and slower; JPEG is smaller"
      >
        <span>Format</span>
        <div className="row">
          <button
            type="button"
            className={value.frameFormat === "jpg" ? "selected" : ""}
            disabled={!video}
            onClick={() => onChange({ ...value, frameFormat: "jpg" })}
          >
            JPG
          </button>
          <button
            type="button"
            className={value.frameFormat === "png" ? "selected" : ""}
            disabled={!video}
            onClick={() => onChange({ ...value, frameFormat: "png" })}
          >
            PNG
          </button>
        </div>
        <output>{value.frameFormat === "png" ? "lossless" : "lossy"}</output>
      </label>
      <Knob
        label="JPEG quality"
        hint="Higher keeps more detail and uses more disk."
        min={1}
        max={100}
        step={1}
        value={value.jpegQuality}
        disabled={!video || value.frameFormat !== "jpg"}
        dim={!video || value.frameFormat !== "jpg"}
        onChange={(jpegQuality) => onChange({ ...value, jpegQuality })}
      />
      {nativeSize ? (
        <label
          className="dim"
          title="Skip downscale. Training uses train max resolution, or native if that is 0."
          data-hint="Skip downscale. Training uses train max resolution, or native if that is 0."
        >
          <span>Longest edge</span>
          <input type="range" min={SAFETY.maxImageSizeMin} max={SAFETY.maxImageSizeMax} value={SAFETY.maxImageSizeMax} disabled />
          <output>native</output>
        </label>
      ) : (
        <Knob
          label="Longest edge"
          hint="Downscale longest edge after extract. Smaller is faster."
          min={SAFETY.maxImageSizeMin}
          max={SAFETY.maxImageSizeMax}
          step={64}
          value={value.maxImageSize}
          onChange={(maxImageSize) => onChange({ ...value, maxImageSize })}
        />
      )}
      <label
        className="check"
        title="Skip downscale. Training uses the train max resolution knob, or native if 0."
        data-hint="Skip downscale. Training uses the train max resolution knob, or native if 0."
      >
        <input
          type="checkbox"
          checked={nativeSize}
          onChange={(event) =>
            onChange({
              ...value,
              maxImageSize: event.currentTarget.checked ? 0 : 1600,
            })
          }
        />
        Keep original resolution
      </label>
      <details>
        <summary>Keyframe scoring</summary>
        <Knob
          label="Blur floor"
          hint="Laplacian variance below this is treated as motion blur."
          min={0}
          max={80}
          step={0.5}
          value={extract.blurFloor}
          disabled={!video}
          dim={!video}
          onChange={(blurFloor) => onChange({ ...value, extract: { ...extract, blurFloor } })}
        />
        <Knob
          label="Moderate MAD"
          hint="Expected luma change per candidate step of a typical orbit."
          min={0}
          max={64}
          step={0.5}
          value={extract.moderateMad}
          disabled={!video}
          dim={!video}
          onChange={(moderateMad) => onChange({ ...value, extract: { ...extract, moderateMad } })}
        />
        <Knob
          label="Candidate min fps"
          hint="Lower clamp on the dense thumb rate used for scoring."
          min={0.25}
          max={60}
          step={0.25}
          value={extract.candidateMinFps}
          disabled={!video}
          dim={!video}
          onChange={(candidateMinFps) =>
            onChange({ ...value, extract: { ...extract, candidateMinFps } })
          }
        />
        <Knob
          label="Candidate max fps"
          hint="Upper clamp on the dense thumb rate used for scoring."
          min={0.25}
          max={60}
          step={0.25}
          value={extract.candidateMaxFps}
          disabled={!video}
          dim={!video}
          onChange={(candidateMaxFps) =>
            onChange({ ...value, extract: { ...extract, candidateMaxFps } })
          }
        />
        <Knob
          label="Change max fps"
          hint="Candidate thumb rate at extract quality 100."
          min={0.25}
          max={60}
          step={0.25}
          value={extract.changeCandidateMaxFps}
          disabled={!video}
          dim={!video}
          onChange={(changeCandidateMaxFps) =>
            onChange({ ...value, extract: { ...extract, changeCandidateMaxFps } })
          }
        />
        <Knob
          label="Change MAD sparse"
          hint="Accumulated luma gate at extract quality 1."
          min={0}
          max={200}
          step={1}
          value={extract.changeMadSparse}
          disabled={!video}
          dim={!video}
          onChange={(changeMadSparse) =>
            onChange({ ...value, extract: { ...extract, changeMadSparse } })
          }
        />
        <Knob
          label="Change MAD dense"
          hint="Accumulated luma gate at extract quality 100."
          min={0}
          max={200}
          step={1}
          value={extract.changeMadDense}
          disabled={!video}
          dim={!video}
          onChange={(changeMadDense) =>
            onChange({ ...value, extract: { ...extract, changeMadDense } })
          }
        />
        <Knob
          label="Relax steps"
          hint="How many times the motion gate is halved to reach min frames."
          min={0}
          max={32}
          step={1}
          value={extract.thresholdRelaxSteps}
          disabled={!video}
          dim={!video}
          onChange={(thresholdRelaxSteps) =>
            onChange({ ...value, extract: { ...extract, thresholdRelaxSteps } })
          }
        />
        <Knob
          label="Min frames"
          hint="Selector fills sharp frames up to this count when motion is low."
          min={1}
          max={500}
          step={1}
          value={extract.minFrames}
          disabled={!video}
          dim={!video}
          onChange={(minFrames) => onChange({ ...value, extract: { ...extract, minFrames } })}
        />
      </details>
    </fieldset>
  );
}
