import type { PipelineSettings, SourceKind } from "../types";
import { maxFramesCap } from "../types";

type Props = {
  value: PipelineSettings;
  sourceKind: SourceKind;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

type KnobProps = {
  label: string;
  hint: string;
  min: number;
  max: number;
  step: number;
  value: number;
  disabled?: boolean;
  dim?: boolean;
  display?: string;
  onChange: (value: number) => void;
};

function patchNumber(
  value: PipelineSettings,
  onChange: (settings: PipelineSettings) => void,
  key: keyof PipelineSettings,
  raw: string,
) {
  const parsed = Number(raw);
  if (!Number.isFinite(parsed)) {
    return;
  }
  onChange({ ...value, [key]: parsed });
}

/** Slider plus typed value; ranges match backend `sanitized()`. */
function Knob({
  label,
  hint,
  min,
  max,
  step,
  value,
  disabled,
  dim,
  display,
  onChange,
}: KnobProps) {
  return (
    <label className={dim ? "dim" : undefined} title={hint} data-hint={hint}>
      <span>{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
      />
      <input
        type="number"
        min={min}
        max={max}
        step={step}
        value={display ?? value}
        disabled={disabled}
        aria-label={label}
        onChange={(event) => onChange(Number(event.currentTarget.value))}
      />
    </label>
  );
}

export function SettingsPanel({ value, sourceKind, disabled, onChange }: Props) {
  const video = sourceKind === "video";
  const nativeSize = value.maxImageSize === 0;

  return (
    <div className="settings">
      <fieldset disabled={disabled}>
        <legend>Frames</legend>
        <Knob
          label="Target density"
          hint="Frame density while the camera moves. Ignored for image folders."
          min={0.25}
          max={12}
          step={0.25}
          value={value.fps}
          disabled={!video}
          dim={!video}
          onChange={(fps) => onChange({ ...value, fps })}
        />
        <Knob
          label="Max frames"
          hint={
            value.captureMode === "outdoor"
              ? "Hard cap on extracted video frames. Outdoor allows up to 10,000."
              : "Hard cap on extracted video frames."
          }
          min={8}
          max={maxFramesCap(value.captureMode)}
          step={1}
          value={value.maxFrames}
          disabled={!video}
          dim={!video}
          onChange={(maxFrames) => onChange({ ...value, maxFrames })}
        />
        {video && value.captureMode === "room" && value.maxFrames > 250 ? (
          <p className="note">
            Exhaustive matching only up to 250 frames; extra frames use sequential matching.
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
            title="Skip downscale. Training still caps native stills at 1920 internally."
            data-hint="Skip downscale. Training still caps native stills at 1920 internally."
          >
            <span>Longest edge</span>
            <input type="range" min={320} max={8192} value={8192} disabled />
            <output>native</output>
          </label>
        ) : (
          <Knob
            label="Longest edge"
            hint="Downscale longest edge after extract. Smaller is faster."
            min={320}
            max={8192}
            step={64}
            value={value.maxImageSize}
            onChange={(maxImageSize) => onChange({ ...value, maxImageSize })}
          />
        )}
        <label
          className="check"
          title="Skip downscale. Training still caps native stills at 1920 internally."
          data-hint="Skip downscale. Training still caps native stills at 1920 internally."
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
      </fieldset>

      <fieldset disabled={disabled}>
        <legend>Reconstruction</legend>
        <Knob
          label="Match overlap"
          hint="How many neighboring frames COLMAP matches."
          min={2}
          max={50}
          step={1}
          value={value.matchOverlap}
          onChange={(matchOverlap) => onChange({ ...value, matchOverlap })}
        />
      </fieldset>

      <fieldset disabled={disabled}>
        <legend>Training</legend>
        <Knob
          label="Train steps"
          hint="Brush optimization steps. More is slower and usually cleaner."
          min={100}
          max={100000}
          step={100}
          value={value.trainSteps}
          onChange={(trainSteps) => onChange({ ...value, trainSteps })}
        />
        <Knob
          label="Max splats"
          hint="Upper bound on Gaussians, in millions. Needs RAM."
          min={0.1}
          max={20}
          step={0.1}
          value={value.maxSplats / 1_000_000}
          display={(value.maxSplats / 1_000_000).toFixed(1)}
          onChange={(millions) =>
            onChange({ ...value, maxSplats: Math.round(millions * 1_000_000) })
          }
        />
      </fieldset>
    </div>
  );
}
