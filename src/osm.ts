/** OSM slippy-map tile index for a WGS84 point (same formula as the Rust exporter). */
export function osmTile(lat: number, lon: number, zoom: number): { x: number; y: number } {
  const n = 2 ** zoom;
  const x = Math.floor(((lon + 180) / 360) * n);
  const latRad = (lat * Math.PI) / 180;
  const y = Math.floor(
    ((1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2) * n,
  );
  const max = Math.max(n - 1, 0);
  return {
    x: Math.min(Math.max(x, 0), max),
    y: Math.min(Math.max(y, 0), max),
  };
}

export function osmTileUrl(lat: number, lon: number, zoom: number): string {
  const { x, y } = osmTile(lat, lon, zoom);
  return `https://tile.openstreetmap.org/${zoom}/${x}/${y}.png`;
}

export function osmBrowseUrl(lat: number, lon: number): string {
  return `https://www.openstreetmap.org/?mlat=${lat}&mlon=${lon}#map=16/${lat}/${lon}`;
}
