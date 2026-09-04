import type { Stage } from "../types";
import { useDialogFocus } from "../useDialogFocus";

type Props = {
  stage: Stage;
  alreadyComplete: boolean;
  discardsLater: boolean;
  onChoose: (choice: "continue" | "restart") => void;
  onCancel: () => void;
};

/** Keep existing stage outputs, or wipe this step and run it again. */
export function RerunDialog({
  stage,
  alreadyComplete,
  discardsLater,
  onChoose,
  onCancel,
}: Props) {
  const dialogRef = useDialogFocus(true, onCancel);

  return (
    <div className="dialog-backdrop" onClick={onCancel}>
      <div
        ref={dialogRef}
        className="dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="rerun-title"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="dialog-head">
          <h2 id="rerun-title">{titleFor(stage)}</h2>
          <button type="button" className="icon-btn" aria-label="Close" onClick={onCancel}>
            ×
          </button>
        </header>
        <p>{bodyFor(stage, alreadyComplete, discardsLater)}</p>
        <div className="dialog-actions">
          {!alreadyComplete ? (
            <button type="button" onClick={() => onChoose("continue")}>Continue</button>
          ) : null}
          <button type="button" className="primary" onClick={() => onChoose("restart")}>
            Start this step over
          </button>
        </div>
      </div>
    </div>
  );
}

function titleFor(stage: Stage): string {
  switch (stage) {
    case "frames":
      return "Extract frames";
    case "colmap":
      return "Estimate cameras";
    case "train":
      return "Train splat";
  }
}

function bodyFor(stage: Stage, alreadyComplete: boolean, discardsLater: boolean): string {
  if (!alreadyComplete) {
    return "This step did not finish. Continue from the current files, or start this step from scratch?";
  }
  let body: string;
  switch (stage) {
    case "frames":
      body = "Frames are already complete. Start frame extraction over from scratch?";
      break;
    case "colmap":
      body = "Cameras are already complete. Start camera estimation over from scratch?";
      break;
    case "train":
      body = "The splat is already complete. Start training over from scratch?";
      break;
  }
  if (discardsLater) {
    body += " Starting this step over will also discard later results.";
  }
  return body;
}
