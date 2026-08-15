import type { PipelineSettings } from "../types";
import { matchingPreset, PRESET_SETTINGS } from "../types";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

const OPTIONS = [
  {
    id: "fast" as const,
    title: "Fast",
    detail: "~1 fps · max 120 · 800px · 5k steps · 2M splats",
    hint: "Quick preview. Use this first on a new capture.",
  },
  {
    id: "balanced" as const,
    title: "Balanced",
    detail: "~2 fps · max 250 · 1600px · 15k steps · 5M splats",
    hint: "Default trade-off between time and quality.",
  },
  {
    id: "quality" as const,
    title: "Quality",
    detail: "~4 fps · max 500 · 1920px · 30k steps · 10M splats",
    hint: "Hours on Apple Silicon. Needs 16 GB unified memory or more.",
  },
];

export function PresetPicker({ value, disabled, onChange }: Props) {
  const selected = matchingPreset(value);

  return (
    <fieldset className="presets" disabled={disabled}>
      <legend>Preset</legend>
      {OPTIONS.map((option) => (
        <label
          key={option.id}
          className={option.id === selected ? "selected" : ""}
          title={option.hint}
          data-hint={option.hint}
        >
          <input
            type="radio"
            name="preset"
            value={option.id}
            checked={option.id === selected}
            onChange={() =>
              onChange({
                ...PRESET_SETTINGS[option.id],
                startSeconds: value.startSeconds,
                durationSeconds: value.durationSeconds,
                frameFormat: value.frameFormat,
                captureMode: value.captureMode,
              })
            }
          />
          <span>
            <strong>{option.title}</strong>
            <small>{option.detail}</small>
          </span>
        </label>
      ))}
    </fieldset>
  );
}
