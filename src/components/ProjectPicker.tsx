import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectEntry } from "../types";

type Props = {
  project: ProjectEntry | null;
  projects: ProjectEntry[];
  disabled: boolean;
  onNew: () => void;
  onOpen: (path: string) => void;
};

/** Named project identity: create, open, or pick from the projects folder. */
export function ProjectPicker({ project, projects, disabled, onNew, onOpen }: Props) {
  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      onOpen(selected);
    }
  }

  return (
    <section className="project-picker">
      <p className="dropzone-label">
        {project ? `Project: ${project.title}` : "No project yet"}
      </p>
      <div className="row">
        <button type="button" disabled={disabled} onClick={onNew}>
          New project
        </button>
        <button type="button" disabled={disabled} onClick={() => void pickFolder()}>
          Open…
        </button>
      </div>
      {projects.length > 0 ? (
        <label className="project-select">
          Recent
          <select
            disabled={disabled}
            value={project?.dir ?? ""}
            onChange={(event) => {
              if (event.currentTarget.value) {
                onOpen(event.currentTarget.value);
              }
            }}
          >
            <option value="">Choose…</option>
            {projects.map((entry) => (
              <option key={entry.dir} value={entry.dir}>
                {entry.title} ({entry.stage})
              </option>
            ))}
          </select>
        </label>
      ) : null}
    </section>
  );
}
