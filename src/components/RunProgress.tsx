import { useEffect, useState } from "react";
import { openLogWindow } from "../logWindows";
import type { CameraStats, FrameStats, PipelinePlan, ProgressEvent, RunStatus, TrainStats } from "../types";
import { formatPercent, stagePercents } from "../stageProgress";

type Props = {
  status: RunStatus;
  progress: ProgressEvent | null;
  frames: FrameStats | null;
  cameras: CameraStats | null;
  train: TrainStats | null;
  error: string | null;
  projectDir?: string | null;
  plan?: PipelinePlan | null;
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
  if (pass === "scoring") {
    return "Scoring";
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

export function RunProgress({ status, progress, frames, cameras, train, error, projectDir, plan }: Props) {
  const percents = stagePercents(progress, status);
  const liveText = error ?? progress?.message ?? status;
  const [announcement, setAnnouncement] = useState(liveText);

  useEffect(() => {
    const timer = window.setTimeout(() => setAnnouncement(liveText), 750);
    return () => window.clearTimeout(timer);
  }, [liveText]);

  return (
    <section className="progress">
      <div className="progress-head">
        <strong>Progress</strong>
        <button type="button" className="log-open" onClick={() => void openLogWindow(projectDir)}>
          Log
        </button>
      </div>
      <p className="progress-message">
        {error ?? progress?.message ?? (status === "idle" ? "Idle" : status === "paused" ? "Paused" : status)}
      </p>
      <span className="sr-only" aria-live="polite" aria-atomic="true">{announcement}</span>
      <ol className="stages stage-cards">
          {STAGES.map((stage) => {
            const active = progress?.stage === stage.id && status === "running";
            const done =
              status === "done" ||
              percents[stage.id] === 100 ||
              (progress !== null &&
                STAGES.findIndex((s) => s.id === progress.stage) >
                  STAGES.findIndex((s) => s.id === stage.id));
            const pct = percents[stage.id];
            const planned = plan?.stages.find((item) => item.stage === stage.id);
            return (
              <li key={stage.id} className={done ? "done" : active ? "active" : ""} title={planned?.reason}>
                <div className="stage-card-head">
                  <span>{stage.label}</span>
                  <span className="stage-pct">
                    {pct == null ? (planned?.action === "reuse" ? "Reusable" : "Waiting") : formatPercent(pct)}
                  </span>
                </div>
                <progress max={100} value={pct ?? 0} />
                {stage.id === "frames" && frames ? (
                  <dl className="train-stats">
                    <div><dt>Phase</dt><dd>{passLabel(frames.pass)}</dd></div>
                    <div><dt>Frame</dt><dd>{formatCount(frames.current)}{frames.total != null ? ` / ${formatCount(frames.total)}` : ""}</dd></div>
                    <div><dt>Kept</dt><dd>{formatCount(frames.kept)}</dd></div>
                    <div><dt>Time</dt><dd>{formatElapsed(frames.elapsedSecs)}</dd></div>
                    <div><dt>ETA</dt><dd>{formatElapsed(frames.etaSecs)}</dd></div>
                  </dl>
                ) : null}
                {stage.id === "colmap" && cameras ? (
                  <dl className="train-stats">
                    <div><dt>Phase</dt><dd>{cameraStepLabel(cameras.step)}</dd></div>
                    <div><dt>Images</dt><dd>{formatCount(cameras.registered ?? cameras.processed)}{cameras.total != null ? ` / ${formatCount(cameras.total)}` : ""}</dd></div>
                    <div><dt>Features</dt><dd>{formatCount(cameras.features)}</dd></div>
                    <div><dt>Matches</dt><dd>{formatCount(cameras.matches)}</dd></div>
                    <div><dt>Points</dt><dd>{formatCount(cameras.points)}</dd></div>
                    {cameras.strategy ? <div><dt>Strategy</dt><dd>{cameras.strategy}</dd></div> : null}
                    {cameras.attempt != null || cameras.trying != null ? <div><dt>Attempt</dt><dd>#{cameras.attempt ?? cameras.trying}</dd></div> : null}
                    {cameras.graphCoverage != null ? <div><dt>Coverage</dt><dd>{formatPercent(cameras.graphCoverage * 100)}</dd></div> : null}
                    {cameras.graphRigidity != null ? <div><dt>Rigidity</dt><dd>{formatPercent(cameras.graphRigidity * 100)}</dd></div> : null}
                    {cameras.graphMedianDegree != null ? <div><dt>Median degree</dt><dd>{formatFixed(cameras.graphMedianDegree, 1)}</dd></div> : null}
                    {cameras.failed ? <div><dt>Failed</dt><dd>{formatCount(cameras.failed)}</dd></div> : null}
                    <div><dt>Time</dt><dd>{formatElapsed(cameras.elapsedSecs)}</dd></div>
                    <div><dt>ETA</dt><dd>{formatElapsed(cameras.etaSecs)}</dd></div>
                  </dl>
                ) : null}
                {stage.id === "train" && train ? (
                  <dl className="train-stats">
                    <div><dt>Step</dt><dd>{formatCount(train.iter)} / {formatCount(train.total)}</dd></div>
                    <div><dt>Gaussians</dt><dd>{formatCount(train.splats)}</dd></div>
                    <div><dt>PSNR</dt><dd>{formatFixed(train.psnr, 1)}</dd></div>
                    <div><dt>SSIM</dt><dd>{formatFixed(train.ssim, 3)}</dd></div>
                    <div><dt>Views</dt><dd>{formatCount(train.trainViews)}</dd></div>
                    <div><dt>Time</dt><dd>{formatElapsed(train.elapsedSecs)}</dd></div>
                    <div><dt>ETA</dt><dd>{formatElapsed(train.etaSecs)}</dd></div>
                  </dl>
                ) : null}
              </li>
            );
          })}
      </ol>
    </section>
  );
}
