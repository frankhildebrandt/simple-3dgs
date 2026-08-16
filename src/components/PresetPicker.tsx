import type { NamedPreset, PipelineSettings, Preset } from "../types";
import { applyNamedPreset, matchingPreset } from "../types";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

const OPTIONS: Array<{
  id: NamedPreset | "custom";
  title: string;
  detail: string;
  hint: string;
}> = [
  {
    id: "fast",
    title: "Fast",
    detail: "~1 fps · max 120 · 800px · 5k steps · 2M splats",
    hint: "Quick preview. Use this first on a new capture.",
  },
  {
    id: "balanced",
    title: "Balanced",
    detail: "~2 fps · max 250 · 1600px · 15k steps · 5M splats",
    hint: "Default trade-off between time and quality.",
  },
  {
    id: "quality",
    title: "Quality",
    detail: "~4 fps · max 500 · 1920px · 30k steps · 10M splats",
    hint: "Hours on Apple Silicon. Needs 16 GB unified memory or more.",
  },
  {
    id: "custom",
    title: "Custom",
    detail: "No recipe — every extract, SfM, train, and viewer knob is yours",
    hint: "Keeps the current values. No silent floors or capture-mode overrides.",
  },
];

export function PresetPicker({ value, disabled, onChange }: Props) {
  const selected = matchingPreset(value);
  const current = OPTIONS.find((option) => option.id === selected) ?? OPTIONS[OPTIONS.length - 1];

  return (
    <div className="inspector-row">
      <span className="inspector-key">Preset</span>
      <select
        value={selected}
        disabled={disabled}
        title={current.hint}
        aria-label="Preset"
        onChange={(event) => {
          const id = event.currentTarget.value as Preset;
          if (id === "custom") {
            onChange(value);
            return;
          }
          onChange(applyNamedPreset(id, value));
        }}
      >
        {OPTIONS.map((option) => (
          <option key={option.id} value={option.id} title={option.hint}>
            {option.title} — {option.detail}
          </option>
        ))}
      </select>
    </div>
  );
}
