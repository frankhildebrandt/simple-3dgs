import { captureTips } from "../captureTips";
import type { CaptureMode } from "../types";
import { useDialogFocus } from "../useDialogFocus";

type Props = {
  mode: CaptureMode;
  open: boolean;
  onClose: () => void;
};

/** Overlay with mode-specific filming advice. */
export function CaptureHints({ mode, open, onClose }: Props) {
  const dialogRef = useDialogFocus(open, onClose);

  if (!open) {
    return null;
  }

  const title = `Capture tips · ${mode}`;

  return (
    <div className="dialog-backdrop" onClick={onClose}>
      <div
        ref={dialogRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="capture-hints-title"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="dialog-head">
          <h2 id="capture-hints-title">{title}</h2>
          <button type="button" className="icon-btn" aria-label="Close" autoFocus onClick={onClose}>
            ×
          </button>
        </header>
        <ul className="hints-list">
          {captureTips(mode).map((tip) => (
            <li key={tip}>{tip}</li>
          ))}
        </ul>
      </div>
    </div>
  );
}
