import { useEffect, useRef } from "react";
import L from "leaflet";
import "leaflet/dist/leaflet.css";
import type { ArchiveEntry } from "../types";

type Props = {
  entries: ArchiveEntry[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

/** Full-width OSM map with a circle marker per archived capture that has GPS. */
export function ArchiveMap({ entries, selectedId, onSelect }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<L.Map | null>(null);
  const layerRef = useRef<L.LayerGroup | null>(null);
  const fittedRef = useRef("");
  const onSelectRef = useRef(onSelect);
  onSelectRef.current = onSelect;

  useEffect(() => {
    const host = hostRef.current;
    if (!host || mapRef.current) {
      return;
    }
    const map = L.map(host, { zoomControl: true, attributionControl: true }).setView([20, 0], 2);
    L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "&copy; OpenStreetMap",
      maxZoom: 19,
    }).addTo(map);
    const layer = L.layerGroup().addTo(map);
    mapRef.current = map;
    layerRef.current = layer;
    return () => {
      map.remove();
      mapRef.current = null;
      layerRef.current = null;
      fittedRef.current = "";
    };
  }, []);

  useEffect(() => {
    const map = mapRef.current;
    const layer = layerRef.current;
    if (!map || !layer) {
      return;
    }
    layer.clearLayers();
    const points: L.LatLngExpression[] = [];
    for (const entry of entries) {
      if (!entry.geo) {
        continue;
      }
      const selected = entry.id === selectedId;
      const marker = L.circleMarker([entry.geo.lat, entry.geo.lon], {
        radius: selected ? 10 : 7,
        color: selected ? "#f0c070" : "#d98a32",
        fillColor: "#d98a32",
        fillOpacity: 0.92,
        weight: selected ? 3 : 2,
      });
      const id = entry.id;
      marker.on("click", () => onSelectRef.current(id));
      marker.bindTooltip(entry.title);
      marker.addTo(layer);
      points.push([entry.geo.lat, entry.geo.lon]);
    }
    const boundsKey = points.map((p) => (Array.isArray(p) ? p.join(",") : "")).join("|");
    if (boundsKey && boundsKey !== fittedRef.current) {
      fittedRef.current = boundsKey;
      if (points.length === 1) {
        map.setView(points[0], 14);
      } else {
        map.fitBounds(L.latLngBounds(points), { padding: [28, 28], maxZoom: 16 });
      }
    }
  }, [entries, selectedId]);

  const located = entries.filter((entry) => entry.geo).length;

  return (
    <section className="archive-map">
      <div className="archive-map-host" ref={hostRef} />
      {located === 0 ? (
        <p className="archive-map-empty">No GPS in this archive yet. Cards below still open the viewer.</p>
      ) : null}
    </section>
  );
}
