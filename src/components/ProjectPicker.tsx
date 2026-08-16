import type { ProjectEntry } from "../types";

type Props = {
  project: ProjectEntry | null;
  projects: ProjectEntry[];
  disabled: boolean;
  onOpen: (path: string) => void;
};

/** Named project identity and recent-project picker. New/Open live in the File menu. */
export function ProjectPicker({ project, projects, disabled, onOpen }: Props) {
  return (
    <fieldset className="project-picker">
      <legend>Project</legend>
      <div className="inspector-row">
        <span className="inspector-key">Name</span>
        <span className="inspector-value" title={project ? project.title : undefined}>
          {project ? project.title : "None"}
        </span>
      </div>
      {projects.length > 0 ? (
        <label className="inspector-row">
          <span className="inspector-key">Recent</span>
          <select
            disabled={disabled}
            value={project?.dir ?? ""}
            aria-label="Recent projects"
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
    </fieldset>
  );
}
