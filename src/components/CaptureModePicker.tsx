import type { CaptureMode, PipelineSettings } from "../types";
import { applyCaptureMode } from "../types";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
  onHelp: () => void;
};

const OPTIONS = [
  {
    id: "object" as const,
    title: "Object",
    detail: "Orbit around a subject",
    hint: "Best for a single item you can walk around.",
  },
  {
    id: "room" as const,
    title: "Room",
    detail: "Indoor walkthrough",
    hint: "Indoor spaces. Uses exhaustive matching on smaller sets.",
  },
  {
    id: "outdoor" as const,
    title: "Outdoor",
    detail: "Path through a space",
    hint: "Outdoor paths. Tilt down so the sky does not dominate.",
  },
];

export function CaptureModePicker({ value, disabled, onChange, onHelp }: Props) {
  const selected: CaptureMode = value.captureMode;
  const current = OPTIONS.find((option) => option.id === selected) ?? OPTIONS[0];

  return (
    <div className="inspector-row">
      <span className="inspector-key">
        Capture
        <button
          type="button"
          className="help-btn"
          aria-label="Capture tips"
          title="Capture tips"
          onClick={onHelp}
        >
          ?
        </button>
      </span>
      <select
        value={selected}
        disabled={disabled}
        title={current.hint}
        aria-label="Capture"
        onChange={(event) =>
          onChange(applyCaptureMode(value, event.currentTarget.value as CaptureMode))
        }
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
