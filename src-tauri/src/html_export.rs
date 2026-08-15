//! Standalone Spark HTML viewer next to `scene.ply`.

use std::fs;
use std::path::Path;

use crate::archive::ArchiveEntry;
use crate::colmap_pose::ViewPose;
use crate::error::PipelineError;
use crate::project::{OUTPUT_PLY, VIEW_JSON};
use crate::settings::CaptureMode;

/// Writes `index.html` + copies `scene.ply` and `meta.json` into `dest_dir`.
pub fn export_html(entry: &ArchiveEntry, dest_dir: &Path) -> Result<(), PipelineError> {
    fs::create_dir_all(dest_dir)?;
    let ply_src = Path::new(&entry.ply_path);
    if !ply_src.is_file() {
        return Err(PipelineError::message(
            "Cannot export HTML: scene.ply is missing.",
        ));
    }
    fs::copy(ply_src, dest_dir.join(OUTPUT_PLY))?;
    let view_src = Path::new(&entry.dir).join(VIEW_JSON);
    let view = crate::colmap_pose::read_view_json(&view_src);
    if view.is_some() && view_src.is_file() {
        fs::copy(&view_src, dest_dir.join(VIEW_JSON))?;
    }
    let meta_src = Path::new(&entry.dir).join("meta.json");
    if meta_src.is_file() {
        fs::copy(&meta_src, dest_dir.join("meta.json"))?;
    }
    fs::write(
        dest_dir.join("index.html"),
        viewer_html(entry, view.as_ref()),
    )?;
    Ok(())
}

/// Spark 2.1 + Three 0.180 viewer with optional OSM link when geo is present.
pub fn viewer_html(entry: &ArchiveEntry, view: Option<&ViewPose>) -> String {
    let title = escape_html(&entry.meta.title);
    let created = escape_html(&entry.meta.created_at);
    let source = escape_html(&entry.meta.source_name);
    let geo_block = match &entry.meta.geo {
        Some(geo) => {
            let lat = geo.lat;
            let lon = geo.lon;
            let osm =
                format!("https://www.openstreetmap.org/?mlat={lat}&mlon={lon}#map=16/{lat}/{lon}");
            let (x, y) = osm_tile(lat, lon, 15);
            format!(
                r#"<aside class="geo">
  <a href="{osm}" target="_blank" rel="noreferrer">OpenStreetMap {lat:.5}, {lon:.5}</a>
  <div class="mini-map">
    <img src="https://tile.openstreetmap.org/15/{x}/{y}.png" alt="Map of capture location" width="256" height="256"/>
    <span class="pin"></span>
  </div>
</aside>"#
            )
        }
        None => String::new(),
    };
    let (camera_js, orbit_target) = camera_setup_js(view);
    let mode = entry
        .meta
        .settings
        .map(|s| s.capture_mode)
        .unwrap_or_default();
    let profile = spark_viewer_profile(mode);
    let spark_ctor = format!(
        "const spark = new SparkRenderer({{ renderer, minAlpha: {:.8}, lodRenderScale: {}, behindFoveate: {} }});",
        profile.min_alpha, profile.lod_render_scale, profile.behind_foveate
    );
    let splat_ctor = format!(
        "const splat = new SplatMesh({{ url: \"./{OUTPUT_PLY}\", lod: true, lodAbove: {} }});",
        profile.lod_above
    );
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{title} — Simple 3DGS</title>
  <style>
    html, body {{ margin: 0; height: 100%; background: #101114; color: #e8e6e1; font-family: system-ui, sans-serif; }}
    canvas {{ display: block; width: 100%; height: 100%; }}
    .overlay {{ position: absolute; left: 1rem; top: 1rem; z-index: 2; max-width: 20rem; }}
    .overlay h1 {{ margin: 0; font-size: 1.1rem; }}
    .overlay p {{ margin: 0.25rem 0 0; color: #9a9aa3; font-size: 0.85rem; }}
    .geo {{ margin-top: 0.75rem; }}
    .geo a {{ color: #d98a32; }}
    .mini-map {{ position: relative; width: 160px; height: 160px; overflow: hidden; border-radius: 8px; margin-top: 0.4rem; }}
    .mini-map img {{ width: 100%; height: 100%; object-fit: cover; }}
    .pin {{ position: absolute; left: 50%; top: 50%; width: 10px; height: 10px; margin: -5px 0 0 -5px; background: #d98a32; border-radius: 50%; border: 2px solid #fff; }}
  </style>
  <script type="importmap">
  {{
    "imports": {{
      "three": "https://cdn.jsdelivr.net/npm/three@0.180.0/build/three.module.js",
      "three/addons/": "https://cdn.jsdelivr.net/npm/three@0.180.0/examples/jsm/",
      "@sparkjsdev/spark": "https://sparkjs.dev/releases/spark/2.1.0/spark.module.js"
    }}
  }}
  </script>
</head>
<body>
  <div class="overlay">
    <h1>{title}</h1>
    <p>{source}</p>
    <p>{created}</p>
    {geo_block}
  </div>
  <script type="module">
    import * as THREE from "three";
    import {{ OrbitControls }} from "three/addons/controls/OrbitControls.js";
    import {{ SparkRenderer, SplatMesh }} from "@sparkjsdev/spark";

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x101114);
    const camera = new THREE.PerspectiveCamera(60, window.innerWidth / Math.max(window.innerHeight, 1), 0.01, 1000);
    {camera_js}
    const renderer = new THREE.WebGLRenderer({{ antialias: true }});
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(window.innerWidth, window.innerHeight);
    document.body.appendChild(renderer.domElement);
    {spark_ctor}
    scene.add(spark);
    {splat_ctor}
    splat.quaternion.set(1, 0, 0, 0);
    scene.add(splat);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    {orbit_target}
    const homePos = camera.position.clone();
    const homeQuat = camera.quaternion.clone();
    const homeTarget = controls.target.clone();
    window.addEventListener("keydown", (event) => {{
      if (event.code !== "Space" || event.repeat) return;
      event.preventDefault();
      camera.position.copy(homePos);
      camera.quaternion.copy(homeQuat);
      controls.target.copy(homeTarget);
    }});
    window.addEventListener("resize", () => {{
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(window.innerWidth, window.innerHeight);
    }});
    renderer.setAnimationLoop(() => {{
      controls.update();
      renderer.render(scene, camera);
    }});
  </script>
</body>
</html>
"#
    )
}

const LOD_ABOVE: u32 = 250_000;
const SPARK_MIN_ALPHA: f64 = 0.5 / 255.0;
const ROOM_MIN_ALPHA: f64 = 2.0 / 255.0;

struct SparkViewerProfile {
    lod_above: u32,
    behind_foveate: f64,
    lod_render_scale: f64,
    min_alpha: f64,
}

/// Matches `src/viewerProfile.ts`: LoD for the view cone, higher minAlpha in rooms.
fn spark_viewer_profile(mode: CaptureMode) -> SparkViewerProfile {
    match mode {
        CaptureMode::Room => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            behind_foveate: 0.1,
            lod_render_scale: 1.5,
            min_alpha: ROOM_MIN_ALPHA,
        },
        CaptureMode::Outdoor => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            behind_foveate: 0.1,
            lod_render_scale: 2.0,
            min_alpha: SPARK_MIN_ALPHA,
        },
        CaptureMode::Object => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            behind_foveate: 0.2,
            lod_render_scale: 1.0,
            min_alpha: SPARK_MIN_ALPHA,
        },
    }
}

fn camera_setup_js(view: Option<&ViewPose>) -> (String, String) {
    match view {
        Some(pose) => {
            let [px, py, pz] = pose.position;
            let [qx, qy, qz, qw] = pose.quaternion;
            let [tx, ty, tz] = pose.look_target(2.0);
            (
                format!(
                    "camera.position.set({px:.8}, {py:.8}, {pz:.8});\n    camera.quaternion.set({qx:.8}, {qy:.8}, {qz:.8}, {qw:.8});"
                ),
                format!("controls.target.set({tx:.8}, {ty:.8}, {tz:.8});"),
            )
        }
        None => ("camera.position.set(0, 0.4, 2.4);".into(), String::new()),
    }
}

/// OSM slippy-map tile indices for a WGS84 point.
pub fn osm_tile(lat: f64, lon: f64, zoom: u32) -> (u32, u32) {
    let n = 2f64.powi(zoom as i32);
    let x = ((lon + 180.0) / 360.0 * n).floor();
    let lat_rad = lat.to_radians();
    let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0 * n)
        .floor();
    let max = (n - 1.0).max(0.0);
    (x.clamp(0.0, max) as u32, y.clamp(0.0, max) as u32)
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::{ArchiveLibrary, IngestRequest};
    use crate::geo::{GeoFix, GeoSource};
    use crate::preset::Preset;
    use crate::settings::PipelineSettings;
    use tempfile::tempdir;

    #[test]
    fn berlin_tile_is_in_expected_cell() {
        let (x, y) = osm_tile(52.52, 13.405, 15);
        assert_eq!(x, 17604);
        assert_eq!(y, 10746);
    }

    #[test]
    fn html_embeds_ply_name_and_osm_link() {
        let html = sample_html(true);
        assert!(html.contains("./scene.ply"));
        assert!(html.contains("openstreetmap.org"));
        assert!(html.contains("52.52000"));
        assert!(html.contains("@sparkjsdev/spark"));
        assert!(html.contains("quaternion.set(1, 0, 0, 0)"));
        assert!(html.contains("event.code !== \"Space\""));
        assert!(html.contains("lod: true"));
        assert!(html.contains("lodAbove: 250000"));
        assert!(html.contains("lodRenderScale: 1"));
        assert!(html.contains("behindFoveate: 0.2"));
    }

    #[test]
    fn html_room_profile_raises_min_alpha() {
        let mut settings = PipelineSettings::from_preset(Preset::Balanced);
        settings.capture_mode = CaptureMode::Room;
        let html = viewer_html(
            &ArchiveEntry {
                meta: crate::archive::ArchiveMeta {
                    id: "x".into(),
                    title: "Hall".into(),
                    created_at: "2026-08-15T12:00:00Z".into(),
                    source_kind: "video".into(),
                    source_name: "clip.mp4".into(),
                    settings: Some(settings),
                    frame_count: 80,
                    ply_bytes: 12,
                    geo: None,
                    poster: None,
                },
                ply_path: "/tmp/scene.ply".into(),
                poster_path: None,
                dir: "/tmp".into(),
            },
            None,
        );
        assert!(html.contains("lodRenderScale: 1.5"));
        assert!(html.contains("behindFoveate: 0.1"));
        assert!(html.contains(&format!("minAlpha: {:.8}", ROOM_MIN_ALPHA)));
    }

    #[test]
    fn html_omits_map_without_geo() {
        let html = sample_html(false);
        assert!(!html.contains("openstreetmap.org"));
        assert!(html.contains("./scene.ply"));
    }

    #[test]
    fn export_writes_folder() {
        let dir = tempdir().unwrap();
        let lib = ArchiveLibrary::open(dir.path().join("archive")).unwrap();
        let ply = dir.path().join("scene.ply");
        fs::write(&ply, b"ply\n").unwrap();
        let entry = lib
            .ingest(IngestRequest {
                ply: &ply,
                frames_dir: None,
                source: Path::new("clip.mp4"),
                source_kind: "video",
                settings: None,
                frame_count: 8,
                geo: Some(GeoFix {
                    lat: 52.52,
                    lon: 13.405,
                    alt: None,
                    source: GeoSource::Quicktime,
                }),
                reuse_id: None,
            })
            .unwrap();
        let dest = dir.path().join("export");
        export_html(&entry, &dest).unwrap();
        assert!(dest.join("index.html").is_file());
        assert!(dest.join("scene.ply").is_file());
        assert!(dest.join("meta.json").is_file());
        let html = fs::read_to_string(dest.join("index.html")).unwrap();
        assert!(html.contains("openstreetmap.org"));
    }

    fn sample_html(with_geo: bool) -> String {
        let entry = ArchiveEntry {
            meta: crate::archive::ArchiveMeta {
                id: "x".into(),
                title: "Gate <1>".into(),
                created_at: "2026-08-15T12:00:00Z".into(),
                source_kind: "video".into(),
                source_name: "clip.mp4".into(),
                settings: None,
                frame_count: 8,
                ply_bytes: 12,
                geo: with_geo.then_some(GeoFix {
                    lat: 52.52,
                    lon: 13.405,
                    alt: None,
                    source: GeoSource::Quicktime,
                }),
                poster: None,
            },
            ply_path: "/tmp/scene.ply".into(),
            poster_path: None,
            dir: "/tmp".into(),
        };
        let html = viewer_html(&entry, None);
        assert!(html.contains("Gate &lt;1&gt;"));
        html
    }

    #[test]
    fn html_inlines_first_camera_pose() {
        let html = viewer_html(
            &ArchiveEntry {
                meta: crate::archive::ArchiveMeta {
                    id: "x".into(),
                    title: "Gate".into(),
                    created_at: "2026-08-15T12:00:00Z".into(),
                    source_kind: "video".into(),
                    source_name: "clip.mp4".into(),
                    settings: None,
                    frame_count: 8,
                    ply_bytes: 12,
                    geo: None,
                    poster: None,
                },
                ply_path: "/tmp/scene.ply".into(),
                poster_path: None,
                dir: "/tmp".into(),
            },
            Some(&ViewPose {
                position: [1.0, 2.0, 3.0],
                quaternion: [0.0, 0.0, 0.0, 1.0],
            }),
        );
        assert!(html.contains("camera.position.set(1.00000000, 2.00000000, 3.00000000)"));
        assert!(html.contains("controls.target.set(1.00000000, 2.00000000, 1.00000000)"));
    }
}
