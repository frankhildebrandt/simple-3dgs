import type { PipelineSettings, SourceKind } from "../types";

type Props = {
  value: PipelineSettings;
  sourceKind: SourceKind;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

function patch(
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

export function SettingsPanel({ value, sourceKind, disabled, onChange }: Props) {
  const video = sourceKind === "video";
  const nativeSize = value.maxImageSize === 0;

  return (
    <div className="settings">
      <fieldset disabled={disabled}>
        <legend>Frame control</legend>
        <label className={video ? undefined : "dim"}>
          <span>Extract rate</span>
          <input
            type="range"
            min={0.5}
            max={8}
            step={0.5}
            value={value.fps}
            disabled={!video}
            onChange={(event) => patch(value, onChange, "fps", event.currentTarget.value)}
          />
          <output>{value.fps} fps</output>
        </label>
        <label className={video ? undefined : "dim"}>
          <span>Start</span>
          <input
            type="number"
            min={0}
            step={0.5}
            value={value.startSeconds}
            disabled={!video}
            onChange={(event) =>
              patch(value, onChange, "startSeconds", event.currentTarget.value)
            }
          />
          <output>s</output>
        </label>
        <label className={video ? undefined : "dim"}>
          <span>Duration</span>
          <input
            type="number"
            min={0}
            step={0.5}
            value={value.durationSeconds}
            disabled={!video}
            onChange={(event) =>
              patch(value, onChange, "durationSeconds", event.currentTarget.value)
            }
          />
          <output>{value.durationSeconds === 0 ? "full clip" : "s"}</output>
        </label>
        <label className={`format ${video ? "" : "dim"}`.trim()}>
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
        <label className={video && value.frameFormat === "jpg" ? undefined : "dim"}>
          <span>JPEG quality</span>
          <input
            type="range"
            min={1}
            max={100}
            step={1}
            value={value.jpegQuality}
            disabled={!video || value.frameFormat !== "jpg"}
            onChange={(event) => patch(value, onChange, "jpegQuality", event.currentTarget.value)}
          />
          <output>{value.frameFormat === "png" ? "n/a" : `${value.jpegQuality}`}</output>
        </label>
        <label>
          <span>Longest edge</span>
          <input
            type="range"
            min={640}
            max={4096}
            step={64}
            value={nativeSize ? 4096 : value.maxImageSize}
            disabled={nativeSize}
            onChange={(event) =>
              patch(value, onChange, "maxImageSize", event.currentTarget.value)
            }
          />
          <output>{nativeSize ? "native" : `${value.maxImageSize} px`}</output>
        </label>
        <label className="check">
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
        <label>
          <span>Match overlap</span>
          <input
            type="range"
            min={5}
            max={30}
            step={1}
            value={value.matchOverlap}
            onChange={(event) =>
              patch(value, onChange, "matchOverlap", event.currentTarget.value)
            }
          />
          <output>{value.matchOverlap} frames</output>
        </label>
        <label>
          <span>Train steps</span>
          <input
            type="range"
            min={1000}
            max={50000}
            step={500}
            value={value.trainSteps}
            onChange={(event) =>
              patch(value, onChange, "trainSteps", event.currentTarget.value)
            }
          />
          <output>{value.trainSteps.toLocaleString("en")}</output>
        </label>
        <label>
          <span>Max splats</span>
          <input
            type="range"
            min={500_000}
            max={20_000_000}
            step={500_000}
            value={value.maxSplats}
            onChange={(event) => patch(value, onChange, "maxSplats", event.currentTarget.value)}
          />
          <output>{(value.maxSplats / 1_000_000).toFixed(1)}M</output>
        </label>
      </fieldset>
    </div>
  );
}
