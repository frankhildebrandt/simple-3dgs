import type { PipelineSettings } from "../types";
import { matchingPreset, PRESET_SETTINGS } from "../types";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

const OPTIONS = [
  { id: "fast" as const, title: "Fast", detail: "1 fps · 800px · 5k steps · 2M splats" },
  { id: "balanced" as const, title: "Balanced", detail: "2 fps · 1600px · 15k steps · 5M splats" },
  { id: "quality" as const, title: "Quality", detail: "4 fps · 1920px · 30k steps · 10M splats" },
];

export function PresetPicker({ value, disabled, onChange }: Props) {
  const selected = matchingPreset(value);

  return (
    <fieldset className="presets" disabled={disabled}>
      <legend>Preset</legend>
      {OPTIONS.map((option) => (
        <label key={option.id} className={option.id === selected ? "selected" : ""}>
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
