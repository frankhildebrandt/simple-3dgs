import { open } from "@tauri-apps/plugin-dialog";
import { saveConfig } from "../api";
import type { AppConfig } from "../types";

type Props = {
  config: AppConfig | null;
  onConfig: (config: AppConfig) => void;
};

/** Global app preferences that are not reconstruction knobs. */
export function SettingsView({ config, onConfig }: Props) {
  async function persist(next: AppConfig) {
    onConfig(await saveConfig(next));
  }

  async function pickArchiveDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && config) {
      await persist({ ...config, archiveDir: selected });
    }
  }

  async function pickProjectsDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && config) {
      await persist({ ...config, tempProject: false, projectsDir: selected });
    }
  }

  async function setTempProject(tempProject: boolean) {
    if (!config) {
      return;
    }
    await persist({ ...config, tempProject });
  }

  return (
    <div className="settings-page">
      <header className="settings-page-head">
        <h2>Settings</h2>
        <p>App-wide paths. Training quality lives in Expert mode.</p>
      </header>
      <section className="settings">
        <fieldset>
          <legend>Archive</legend>
          <p className="dropzone-label">
            {config ? config.archiveDir : "Choose an archive folder"}
          </p>
          <button
            type="button"
            title="Finished reconstructions are stored here"
            data-hint="Finished reconstructions are stored here"
            onClick={() => void pickArchiveDir()}
          >
            Choose archive folder
          </button>
        </fieldset>
        <fieldset>
          <legend>Project</legend>
          <label
            className="check"
            title="Delete intermediate COLMAP and Brush files after a successful archive"
            data-hint="Delete intermediate COLMAP and Brush files after a successful archive"
          >
            <input
              type="checkbox"
              checked={config?.tempProject ?? true}
              disabled={!config}
              onChange={(event) => void setTempProject(event.currentTarget.checked)}
            />
            Temporary project folder
          </label>
          <p className="dropzone-label">
            {config?.projectsDir ? config.projectsDir : "Named projects live next to the archive"}
          </p>
          <button
            type="button"
            title="Named projects are stored here so you can continue later"
            data-hint="Named projects are stored here so you can continue later"
            onClick={() => void pickProjectsDir()}
          >
            Choose projects folder
          </button>
        </fieldset>
      </section>
    </div>
  );
}
