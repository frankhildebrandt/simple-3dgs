import type { PipelineSettings, SourceKind } from "../types";
import { BrushKnobs } from "./settings/BrushKnobs";
import { ColmapKnobs } from "./settings/ColmapKnobs";
import { FrameKnobs } from "./settings/FrameKnobs";
import { ViewerKnobs } from "./settings/ViewerKnobs";

type Props = {
  value: PipelineSettings;
  sourceKind: SourceKind;
  disabled: boolean;
  onChange: (settings: PipelineSettings) => void;
};

/** Expert / Custom reconstruction knobs. Named presets still fill the same fields. */
export function SettingsPanel({ value, sourceKind, disabled, onChange }: Props) {
  return (
    <div className="settings">
      <FrameKnobs value={value} sourceKind={sourceKind} disabled={disabled} onChange={onChange} />
      <ColmapKnobs value={value} disabled={disabled} onChange={onChange} />
      <BrushKnobs value={value} disabled={disabled} onChange={onChange} />
      <ViewerKnobs value={value} disabled={disabled} onChange={onChange} />
    </div>
  );
}
