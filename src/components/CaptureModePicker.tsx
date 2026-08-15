import type { CaptureMode, PipelineSettings } from "../types";
import { maxFramesCap } from "../types";

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

  return (
    <fieldset className="presets capture-modes">
      <legend>
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
      </legend>
      {OPTIONS.map((option) => (
        <label
          key={option.id}
          className={option.id === selected ? "selected" : ""}
          title={option.hint}
          data-hint={option.hint}
        >
          <input
            type="radio"
            name="capture-mode"
            value={option.id}
            checked={option.id === selected}
            disabled={disabled}
            onChange={() =>
              onChange({
                ...value,
                captureMode: option.id,
                maxFrames: Math.min(value.maxFrames, maxFramesCap(option.id)),
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
