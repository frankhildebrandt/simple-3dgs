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

  async function pickProjectDir() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && config) {
      await persist({ ...config, tempProject: false, projectDir: selected });
    }
  }

  async function setTempProject(tempProject: boolean) {
    if (!config) {
      return;
    }
    if (tempProject) {
      await persist({ ...config, tempProject: true });
      return;
    }
    if (config.projectDir) {
      await persist({ ...config, tempProject: false });
      return;
    }
    await pickProjectDir();
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
            title="Delete intermediate COLMAP and Brush files after a run"
            data-hint="Delete intermediate COLMAP and Brush files after a run"
          >
            <input
              type="checkbox"
              checked={config?.tempProject ?? true}
              disabled={!config}
              onChange={(event) => void setTempProject(event.currentTarget.checked)}
            />
            Temporary project folder
          </label>
          {config && !config.tempProject ? (
            <>
              <p className="dropzone-label">
                {config.projectDir ? config.projectDir : "Choose a project folder"}
              </p>
              <button
                type="button"
                title="Keep frames, COLMAP, and checkpoints for debugging"
                data-hint="Keep frames, COLMAP, and checkpoints for debugging"
                onClick={() => void pickProjectDir()}
              >
                Choose project folder
              </button>
            </>
          ) : null}
        </fieldset>
      </section>
    </div>
  );
}
