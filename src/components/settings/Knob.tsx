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

/** Slider plus typed value; ranges match backend `sanitized()`. */
export function Knob({
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
