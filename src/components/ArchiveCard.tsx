import { convertFileSrc } from "@tauri-apps/api/core";
import type { ArchiveEntry } from "../types";
import { osmTileUrl } from "../osm";

type Props = {
  entry: ArchiveEntry;
  selected: boolean;
  onSelect: () => void;
};

/** Selectable archive card: OSM tile when geo exists, otherwise poster or placeholder. */
export function ArchiveCard({ entry, selected, onSelect }: Props) {
  const geo = entry.geo;
  const poster = entry.posterPath ? convertFileSrc(entry.posterPath) : null;
  const when = formatWhen(entry.createdAt);

  return (
    <button type="button" className={selected ? "archive-card selected" : "archive-card"} onClick={onSelect}>
      <div className="archive-card-visual">
        {geo ? (
          <>
            <img src={osmTileUrl(geo.lat, geo.lon, 15)} alt="" />
            <span className="archive-card-pin" />
          </>
        ) : poster ? (
          <img src={poster} alt="" />
        ) : (
          <span className="archive-card-placeholder">No location</span>
        )}
      </div>
      <div className="archive-card-body">
        <strong>{entry.title}</strong>
        <small>{when}</small>
        {geo ? (
          <small>
            {geo.lat.toFixed(4)}, {geo.lon.toFixed(4)}
          </small>
        ) : (
          <small>GPS unknown</small>
        )}
      </div>
    </button>
  );
}

function formatWhen(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString();
}
