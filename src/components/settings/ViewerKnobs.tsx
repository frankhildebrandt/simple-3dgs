import type { PipelineSettings } from "../../types";
import { MOVE_SPEED_MAX, MOVE_SPEED_MIN } from "../../viewerKnobs";
import { Knob } from "./Knob";

type Props = {
  value: PipelineSettings;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

/** Spark / HTML viewer knobs that named presets fill from capture mode. */
export function ViewerKnobs({ value, disabled, onChange }: Props) {
  const viewer = value.viewer;

  function patch<K extends keyof typeof viewer>(key: K, next: (typeof viewer)[K]) {
    onChange({ ...value, viewer: { ...viewer, [key]: next } });
  }

  return (
    <fieldset disabled={disabled}>
      <legend>Viewer</legend>
      <details>
        <summary>Spark LoD and camera</summary>
        <Knob label="LoD above" hint="Splat count before Spark builds the LoD tree." min={0} max={5_000_000} step={10_000} value={viewer.lodAbove} onChange={(lodAbove) => patch("lodAbove", lodAbove)} />
        <Knob label="Splat scale" hint="lodSplatScale" min={0.05} max={2} step={0.05} value={viewer.lodSplatScale} onChange={(lodSplatScale) => patch("lodSplatScale", lodSplatScale)} />
        <Knob label="Render scale" hint="lodRenderScale" min={0.25} max={6} step={0.05} value={viewer.lodRenderScale} onChange={(lodRenderScale) => patch("lodRenderScale", lodRenderScale)} />
        <Knob label="Behind foveate" hint="behindFoveate" min={0} max={1} step={0.05} value={viewer.behindFoveate} onChange={(behindFoveate) => patch("behindFoveate", behindFoveate)} />
        <Knob label="Cone foveate" hint="coneFoveate" min={0} max={1} step={0.05} value={viewer.coneFoveate} onChange={(coneFoveate) => patch("coneFoveate", coneFoveate)} />
        <Knob label="WebView LoD" hint="HTML export lodSplatCount" min={10_000} max={10_000_000} step={50_000} value={viewer.webviewLodSplatCount} onChange={(webviewLodSplatCount) => patch("webviewLodSplatCount", webviewLodSplatCount)} />
        <Knob label="Min alpha" hint="Faint splats below this are dropped." min={0} max={0.05} step={0.0005} value={viewer.minAlpha} onChange={(minAlpha) => patch("minAlpha", minAlpha)} />
        <Knob label="Max std dev" hint="Gaussian extent" min={0.05} max={5} step={0.05} value={viewer.maxStdDev} onChange={(maxStdDev) => patch("maxStdDev", maxStdDev)} />
        <Knob label="Clip XY" hint="clipXY" min={0.5} max={2} step={0.05} value={viewer.clipXY} onChange={(clipXY) => patch("clipXY", clipXY)} />
        <Knob label="Min pixel r" hint="minPixelRadius" min={0} max={8} step={0.1} value={viewer.minPixelRadius} onChange={(minPixelRadius) => patch("minPixelRadius", minPixelRadius)} />
        <Knob label="Sort interval" hint="minSortIntervalMs" min={0} max={64} step={1} value={viewer.minSortIntervalMs} onChange={(minSortIntervalMs) => patch("minSortIntervalMs", minSortIntervalMs)} />
        <Knob label="FOV" hint="Perspective camera field of view" min={10} max={120} step={1} value={viewer.fov} onChange={(fov) => patch("fov", fov)} />
        <Knob label="Move speed" hint="WASD fly speed in the viewer and HTML export. Shift is 5×, Ctrl is 0.2×." min={MOVE_SPEED_MIN} max={MOVE_SPEED_MAX} step={0.05} value={viewer.moveSpeed} onChange={(moveSpeed) => patch("moveSpeed", moveSpeed)} />
        <Knob label="Far multiplier" hint="camera.far = extent × this" min={1} max={200} step={1} value={viewer.farMultiplier} onChange={(farMultiplier) => patch("farMultiplier", farMultiplier)} />
      </details>
    </fieldset>
  );
}
