import type { CaptureMode, PipelineSettings } from "../types";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

const OPTIONS = [
  { id: "object" as const, title: "Object", detail: "Orbit around a subject" },
  { id: "room" as const, title: "Room", detail: "Indoor walkthrough" },
  { id: "outdoor" as const, title: "Outdoor", detail: "Path through a space" },
];

export function CaptureModePicker({ value, disabled, onChange }: Props) {
  const selected: CaptureMode = value.captureMode;

  return (
    <fieldset className="presets capture-modes" disabled={disabled}>
      <legend>Capture</legend>
      {OPTIONS.map((option) => (
        <label key={option.id} className={option.id === selected ? "selected" : ""}>
          <input
            type="radio"
            name="capture-mode"
            value={option.id}
            checked={option.id === selected}
            onChange={() => onChange({ ...value, captureMode: option.id })}
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
