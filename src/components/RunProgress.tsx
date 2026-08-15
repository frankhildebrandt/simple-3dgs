import type { CameraStats, FrameStats, ProgressEvent, RunStatus, TrainStats } from "../types";
import { formatPercent, stagePercents } from "../stageProgress";

type Props = {
  status: RunStatus;
  progress: ProgressEvent | null;
  frames: FrameStats | null;
  cameras: CameraStats | null;
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
  if (secs >= 90 * 60) {
    const hours = Math.floor(secs / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  }
  const minutes = Math.floor(secs / 60);
  const seconds = Math.floor(secs % 60);
  if (minutes === 0) {
    return `${seconds}s`;
  }
  return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
}

function passLabel(pass: FrameStats["pass"]): string {
  if (pass === "candidates") {
    return "Candidates";
  }
  if (pass === "keyframes") {
    return "Keyframes";
  }
  return "Import";
}

function cameraStepLabel(step: CameraStats["step"]): string {
  if (step === "features") {
    return "Features";
  }
  if (step === "matching") {
    return "Matching";
  }
  if (step === "calibrating") {
    return "Calibrate";
  }
  return "Mapping";
}

export function RunProgress({ status, progress, frames, cameras, train, logs, error }: Props) {
  const percents = stagePercents(progress, status);
  const stats = frames && progress?.stage === "frames"
    ? "frames"
    : cameras && progress?.stage === "colmap"
      ? "cameras"
      : train
        ? "train"
        : null;

  return (
    <section className="progress">
      <ol className="stages">
        {STAGES.map((stage) => {
          const active = progress?.stage === stage.id && status === "running";
          const done =
            status === "done" ||
            percents[stage.id] === 100 ||
            (progress !== null &&
              STAGES.findIndex((s) => s.id === progress.stage) >
                STAGES.findIndex((s) => s.id === stage.id));
          const pct = percents[stage.id];
          return (
            <li key={stage.id} className={done ? "done" : active ? "active" : ""}>
              <span>{stage.label}</span>
              {pct != null ? <span className="stage-pct">{formatPercent(pct)}</span> : null}
            </li>
          );
        })}
      </ol>
      <p className="progress-message">
        {error ?? progress?.message ?? (status === "idle" ? "Idle" : status === "paused" ? "Paused" : status)}
      </p>
      {progress && (status === "running" || status === "paused") ? (
        <div className="progress-bar-row">
          <progress max={100} value={progress.percent} />
          <span className="progress-pct">{formatPercent(progress.percent)}</span>
        </div>
      ) : null}
      {stats === "frames" && frames ? (
        <dl className="train-stats">
          <div>
            <dt>Pass</dt>
            <dd>{passLabel(frames.pass)}</dd>
          </div>
          <div>
            <dt>Frame</dt>
            <dd>
              {formatCount(frames.current)}
              {frames.total != null ? ` / ${formatCount(frames.total)}` : ""}
            </dd>
          </div>
          <div>
            <dt>Kept</dt>
            <dd>{formatCount(frames.kept)}</dd>
          </div>
          <div>
            <dt>Time</dt>
            <dd>{formatElapsed(frames.elapsedSecs)}</dd>
          </div>
          <div>
            <dt>ETA</dt>
            <dd>{formatElapsed(frames.etaSecs)}</dd>
          </div>
        </dl>
      ) : null}
      {stats === "cameras" && cameras ? (
        <dl className="train-stats">
          <div>
            <dt>Step</dt>
            <dd>{cameraStepLabel(cameras.step)}</dd>
          </div>
          <div>
            <dt>Images</dt>
            <dd>
              {formatCount(cameras.processed ?? cameras.registered)}
              {cameras.total != null ? ` / ${formatCount(cameras.total)}` : ""}
            </dd>
          </div>
          <div>
            <dt>Features</dt>
            <dd>{formatCount(cameras.features)}</dd>
          </div>
          <div>
            <dt>Points</dt>
            <dd>{formatCount(cameras.points)}</dd>
          </div>
          <div>
            <dt>Time</dt>
            <dd>{formatElapsed(cameras.elapsedSecs)}</dd>
          </div>
          <div>
            <dt>ETA</dt>
            <dd>{formatElapsed(cameras.etaSecs)}</dd>
          </div>
        </dl>
      ) : null}
      {stats === "train" && train ? (
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
          <div>
            <dt>ETA</dt>
            <dd>{formatElapsed(train.etaSecs)}</dd>
          </div>
        </dl>
      ) : null}
      <pre className="logs" aria-label="Pipeline log">
        {logs.slice(-80).join("\n")}
      </pre>
    </section>
  );
}
