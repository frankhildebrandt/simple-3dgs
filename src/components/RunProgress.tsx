import type { ProgressEvent, RunStatus, TrainStats } from "../types";

type Props = {
  status: RunStatus;
  progress: ProgressEvent | null;
  train: TrainStats | null;
  logs: string[];
  error: string | null;
};

const STAGES = [
  { id: "frames", label: "Frames" },
  { id: "colmap", label: "Cameras" },
  { id: "train", label: "Splat" },
] as const;

function formatCount(value: number | null | undefined): string {
  if (value == null) {
    return "—";
  }
  return value.toLocaleString("en");
}

function formatFixed(value: number | null | undefined, digits: number): string {
  if (value == null) {
    return "—";
  }
  return value.toFixed(digits);
}

function formatElapsed(secs: number | null | undefined): string {
  if (secs == null) {
    return "—";
  }
  const minutes = Math.floor(secs / 60);
  const seconds = Math.floor(secs % 60);
  if (minutes === 0) {
    return `${seconds}s`;
  }
  return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
}

export function RunProgress({ status, progress, train, logs, error }: Props) {
  return (
    <section className="progress">
      <ol className="stages">
        {STAGES.map((stage) => {
          const active = progress?.stage === stage.id;
          const done =
            status === "done" ||
            (progress !== null &&
              STAGES.findIndex((s) => s.id === progress.stage) >
                STAGES.findIndex((s) => s.id === stage.id));
          return (
            <li key={stage.id} className={done ? "done" : active ? "active" : ""}>
              {stage.label}
            </li>
          );
        })}
      </ol>
      <p className="progress-message">
        {error ?? progress?.message ?? (status === "idle" ? "Idle" : status)}
      </p>
      {progress && status === "running" ? (
        <progress max={100} value={progress.percent} />
      ) : null}
      {train ? (
        <dl className="train-stats">
          <div>
            <dt>Step</dt>
            <dd>
              {formatCount(train.iter)} / {formatCount(train.total)}
            </dd>
          </div>
          <div>
            <dt>Gaussians</dt>
            <dd>{formatCount(train.splats)}</dd>
          </div>
          <div>
            <dt>PSNR</dt>
            <dd>{formatFixed(train.psnr, 1)}</dd>
          </div>
          <div>
            <dt>SSIM</dt>
            <dd>{formatFixed(train.ssim, 3)}</dd>
          </div>
          <div>
            <dt>Views</dt>
            <dd>{formatCount(train.trainViews)}</dd>
          </div>
          <div>
            <dt>Time</dt>
            <dd>{formatElapsed(train.elapsedSecs)}</dd>
          </div>
        </dl>
      ) : null}
      <pre className="logs" aria-label="Pipeline log">
        {logs.slice(-80).join("\n")}
      </pre>
    </section>
  );
}
