//! Standalone Spark HTML viewer that opens from disk (`file://`).

use std::fs;
use std::path::Path;

use base64::Engine;

use crate::archive::ArchiveEntry;
use crate::colmap_pose::ViewPose;
use crate::error::PipelineError;
use crate::project::{OUTPUT_SPZ, VIEW_JSON};
use crate::settings::CaptureMode;

const SCENE_JS: &str = "scene.js";

/// Writes `index.html` + `scene.js` (SPZ as classic-script payload) into `dest_dir`.
pub fn export_html(entry: &ArchiveEntry, dest_dir: &Path) -> Result<(), PipelineError> {
    fs::create_dir_all(dest_dir)?;
    let spz_src = Path::new(&entry.dir).join(OUTPUT_SPZ);
    if !spz_src.is_file() {
        return Err(PipelineError::message(
            "Cannot export HTML: scene.spz is missing.",
        ));
    }
    let spz = fs::read(&spz_src)?;
    if spz.is_empty() {
        return Err(PipelineError::message(
            "Cannot export HTML: scene.spz is empty.",
        ));
    }
    fs::copy(&spz_src, dest_dir.join(OUTPUT_SPZ))?;
    fs::write(dest_dir.join(SCENE_JS), scene_js_payload(&spz))?;
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
    let camera_js = camera_setup_js(view);
    let mode = entry
        .meta
        .settings
        .map(|s| s.capture_mode)
        .unwrap_or_default();
    let profile = spark_viewer_profile(mode);
    let move_speed = profile.move_speed;
    let spark_ctor = format!(
        "const spark = new SparkRenderer({{ renderer, enableLod: true, lodSplatCount: {WEBVIEW_LOD_SPLAT_COUNT}, minAlpha: {:.8}, lodSplatScale: {}, lodRenderScale: {}, behindFoveate: {}, coneFoveate: {}, maxStdDev: {}, clipXY: {}, minPixelRadius: 1, minSortIntervalMs: 8 }});",
        profile.min_alpha,
        profile.lod_splat_scale,
        profile.lod_render_scale,
        profile.behind_foveate,
        profile.cone_foveate,
        profile.max_std_dev,
        profile.clip_xy
    );
    let splat_ctor = format!(
        "const splat = new SplatMesh({{ fileBytes: sceneBytes, fileType: \"spz\", fileName: \"{OUTPUT_SPZ}\", lod: true, nonLod: true, lodAbove: {}, raycastable: false }});",
        profile.lod_above
    );
    let mode_js = view_mode_js(profile.max_std_dev);
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
    .actions {{ position: absolute; top: 1rem; right: 1rem; z-index: 2; display: flex; gap: 0.4rem; }}
    .actions button {{ background: #1c1e24; color: #e8e6e1; border: 1px solid #3a3d46; border-radius: 8px; padding: 0.35rem 0.7rem; font: inherit; cursor: pointer; }}
  </style>
  <script type="importmap">
  {{
    "imports": {{
      "three": "https://cdn.jsdelivr.net/npm/three@0.180.0/build/three.module.min.js",
      "three/addons/": "https://cdn.jsdelivr.net/npm/three@0.180.0/examples/jsm/",
      "@sparkjsdev/spark": "https://cdn.jsdelivr.net/npm/@sparkjsdev/spark@2.1.0/dist/spark.module.min.js"
    }}
  }}
  </script>
</head>
<body>
  <div class="overlay">
    <h1>{title}</h1>
    <p>{source}</p>
    <p>{created}</p>
    <p>WASD fly · Q up · E down · Space start · Shift faster</p>
    {geo_block}
  </div>
  <div class="actions">
    <button type="button" id="mode">Splats</button>
    <button type="button" id="scale">Sharp</button>
  </div>
  <script src="./scene.js"></script>
  <script type="module">
    import * as THREE from "three";
    import {{ SparkRenderer, SplatMesh, SparkControls }} from "@sparkjsdev/spark";

    function sceneBytesFromPage() {{
      const b64 = globalThis.SCENE_SPZ_B64;
      if (typeof b64 !== "string" || b64.length === 0) {{
        throw new Error("Missing scene.js splat payload.");
      }}
      const bin = atob(b64);
      const bytes = new Uint8Array(bin.length);
      const chunk = 0x8000;
      for (let i = 0; i < bin.length; i += chunk) {{
        const slice = bin.slice(i, i + chunk);
        bytes.set(Uint8Array.from(slice, (c) => c.charCodeAt(0)), i);
      }}
      return bytes;
    }}
    const sceneBytes = sceneBytesFromPage();

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x101114);
    const camera = new THREE.PerspectiveCamera(60, window.innerWidth / Math.max(window.innerHeight, 1), 0.01, 1000);
    {camera_js}
    const renderer = new THREE.WebGLRenderer({{ antialias: false, alpha: false, powerPreference: "high-performance" }});
    let sharp = false;
    const scaleBtn = document.getElementById("scale");
    function applyScale() {{
      const w = window.innerWidth;
      const h = Math.max(window.innerHeight, 1);
      const cap = sharp ? 1920 : 1280;
      const dpr = sharp ? Math.min(window.devicePixelRatio || 1, 2) : 1;
      const targetW = w * dpr;
      const targetH = h * dpr;
      const long = Math.max(targetW, targetH);
      const scale = long > cap ? cap / long : 1;
      renderer.setPixelRatio(1);
      renderer.setSize(Math.max(1, Math.round(targetW * scale)), Math.max(1, Math.round(targetH * scale)), false);
      renderer.domElement.style.width = "100%";
      renderer.domElement.style.height = "100%";
    }}
    applyScale();
    scaleBtn.addEventListener("click", () => {{
      sharp = !sharp;
      scaleBtn.textContent = sharp ? "Fast" : "Sharp";
      applyScale();
    }});
    renderer.domElement.tabIndex = 0;
    renderer.domElement.style.outline = "none";
    renderer.domElement.style.touchAction = "none";
    document.body.appendChild(renderer.domElement);
    {spark_ctor}
    scene.add(spark);
    {mode_js}
    {splat_ctor}
    splat.quaternion.set(1, 0, 0, 0);
    scene.add(splat);
    const controls = new SparkControls({{ canvas: renderer.domElement }});
    controls.fpsMovement.keycodeMoveMapping = {{
      KeyW: new THREE.Vector3(0, 0, -1),
      KeyS: new THREE.Vector3(0, 0, 1),
      KeyA: new THREE.Vector3(-1, 0, 0),
      KeyD: new THREE.Vector3(1, 0, 0),
      KeyQ: new THREE.Vector3(0, 1, 0),
      KeyE: new THREE.Vector3(0, -1, 0),
    }};
    controls.fpsMovement.moveSpeed = {move_speed};
    const homePos = camera.position.clone();
    const homeQuat = camera.quaternion.clone();
    window.addEventListener("keydown", (event) => {{
      if (["KeyW", "KeyA", "KeyS", "KeyD", "KeyQ", "KeyE"].includes(event.code)) {{
        event.preventDefault();
      }}
      if (event.code !== "Space" || event.repeat || event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.target && event.target.tagName === "BUTTON") return;
      event.preventDefault();
      camera.position.copy(homePos);
      camera.quaternion.copy(homeQuat);
    }});
    window.addEventListener("resize", () => {{
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
      applyScale();
    }});
    renderer.setAnimationLoop(() => {{
      controls.update(camera);
      renderer.render(scene, camera);
    }});
  </script>
</body>
</html>
"#
    )
}

const LOD_ABOVE: u32 = 100_000;
/// Safari/WebView fill-rate budget; Spark's desktop default is 2.5M.
const WEBVIEW_LOD_SPLAT_COUNT: u32 = 1_500_000;
const SPARK_MIN_ALPHA: f64 = 0.5 / 255.0;
const ROOM_MIN_ALPHA: f64 = 2.0 / 255.0;

struct SparkViewerProfile {
    lod_above: u32,
    lod_splat_scale: f64,
    lod_render_scale: f64,
    behind_foveate: f64,
    cone_foveate: f64,
    min_alpha: f64,
    max_std_dev: &'static str,
    clip_xy: f64,
    move_speed: f64,
}

/// Matches `src/viewerProfile.ts`: LoD for the view cone, higher minAlpha in rooms.
fn spark_viewer_profile(mode: CaptureMode) -> SparkViewerProfile {
    match mode {
        CaptureMode::Room => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            lod_splat_scale: 0.7,
            lod_render_scale: 2.0,
            behind_foveate: 0.1,
            cone_foveate: 0.4,
            min_alpha: ROOM_MIN_ALPHA,
            max_std_dev: "Math.sqrt(5)",
            clip_xy: 1.2,
            move_speed: 0.5,
        },
        CaptureMode::Outdoor => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            lod_splat_scale: 0.5,
            lod_render_scale: 3.0,
            behind_foveate: 0.1,
            cone_foveate: 0.3,
            min_alpha: SPARK_MIN_ALPHA,
            max_std_dev: "Math.sqrt(4)",
            clip_xy: 1.1,
            move_speed: 2.0,
        },
        CaptureMode::Object => SparkViewerProfile {
            lod_above: LOD_ABOVE,
            lod_splat_scale: 1.0,
            lod_render_scale: 1.5,
            behind_foveate: 0.2,
            cone_foveate: 0.5,
            min_alpha: SPARK_MIN_ALPHA,
            max_std_dev: "Math.sqrt(5)",
            clip_xy: 1.2,
            move_speed: 0.8,
        },
    }
}

/// Matches `src/viewerMode.ts`: Splats keep the profile; dots clamp; discs drop falloff.
fn view_mode_js(max_std_dev: &str) -> String {
    format!(
        r#"const MODES = ["splats", "dots", "discs"];
    const LABELS = ["Splats", "Dots", "Discs"];
    let viewMode = 0;
    const modeBtn = document.getElementById("mode");
    const splatMaxStdDev = {max_std_dev};
    function applyMode() {{
      const name = MODES[viewMode];
      modeBtn.textContent = LABELS[viewMode];
      if (name === "dots") {{
        spark.maxStdDev = 0.15;
        spark.minPixelRadius = 1.5;
        spark.maxPixelRadius = 2;
        spark.falloff = 0;
        return;
      }}
      spark.maxStdDev = splatMaxStdDev;
      spark.minPixelRadius = 1;
      spark.maxPixelRadius = 512;
      spark.falloff = name === "discs" ? 0 : 1;
    }}
    modeBtn.addEventListener("click", () => {{
      viewMode = (viewMode + 1) % MODES.length;
      applyMode();
    }});
    applyMode();"#
    )
}

/// Places the camera at the first capture pose when known; no orbit target.
fn camera_setup_js(view: Option<&ViewPose>) -> String {
    match view {
        Some(pose) => {
            let [px, py, pz] = pose.position;
            let [qx, qy, qz, qw] = pose.quaternion;
            format!(
                "camera.position.set({px:.8}, {py:.8}, {pz:.8});\n    camera.quaternion.set({qx:.8}, {qy:.8}, {qz:.8}, {qw:.8});"
            )
        }
        None => "camera.position.set(0, 0.4, 2.4);".into(),
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

/// Classic script so `file://` pages can load the splat without CORS-blocked fetch.
fn scene_js_payload(spz: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(spz);
    format!("globalThis.SCENE_SPZ_B64 = \"{b64}\";\n")
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
    fn html_embeds_spz_name_and_osm_link() {
        let html = sample_html(true);
        assert!(html.contains("./scene.js"));
        assert!(html.contains("fileBytes: sceneBytes"));
        assert!(html.contains("fileType: \"spz\""));
        assert!(!html.contains("url: \"./scene.spz\""));
        assert!(!html.contains("./scene.ply"));
        assert!(html.contains("openstreetmap.org"));
        assert!(html.contains("52.52000"));
        assert!(html.contains("@sparkjsdev/spark"));
        assert!(html.contains("spark.module.min.js"));
        assert!(html.contains("three.module.min.js"));
        assert!(html.contains("quaternion.set(1, 0, 0, 0)"));
        assert!(html.contains("event.code !== \"Space\""));
        assert!(html.contains("lod: true"));
        assert!(html.contains("nonLod: true"));
        assert!(html.contains("enableLod: true"));
        assert!(html.contains("lodSplatCount: 1500000"));
        assert!(html.contains("lodAbove: 100000"));
        assert!(html.contains("raycastable: false"));
        assert!(html.contains("lodSplatScale: 1"));
        assert!(html.contains("lodRenderScale: 1.5"));
        assert!(html.contains("behindFoveate: 0.2"));
        assert!(html.contains("coneFoveate: 0.5"));
        assert!(html.contains("maxStdDev: Math.sqrt(5)"));
        assert!(html.contains("clipXY: 1.2"));
        assert!(html.contains("minPixelRadius: 1"));
        assert!(html.contains("minSortIntervalMs: 8"));
        assert!(html.contains("antialias: false"));
        assert!(html.contains("powerPreference: \"high-performance\""));
        assert!(html.contains("const cap = sharp ? 1920 : 1280"));
        assert!(html.contains("setSize("));
        assert!(html.contains("false)"));
        assert!(html.contains("id=\"scale\""));
        assert!(html.contains(">Sharp</button>"));
        assert!(html.contains("id=\"mode\""));
        assert!(html.contains(">Splats</button>"));
        assert!(html.contains("function applyMode()"));
        assert!(html.contains("spark.falloff = name === \"discs\" ? 0 : 1"));
        assert!(html.contains("spark.maxPixelRadius = 512"));
        assert!(html.contains("spark.maxStdDev = 0.15"));
        assert!(html.contains("spark.minPixelRadius = 1.5"));
        assert!(html.contains("spark.maxPixelRadius = 2"));
        assert!(html.contains("spark.falloff = 0"));
        assert!(html.contains("const splatMaxStdDev = Math.sqrt(5)"));
        assert!(html.contains("new SparkControls"));
        assert!(html.contains("controls.update(camera)"));
        assert!(html.contains("keycodeMoveMapping"));
        assert!(html.contains("KeyQ: new THREE.Vector3(0, 1, 0)"));
        assert!(html.contains("KeyE: new THREE.Vector3(0, -1, 0)"));
        assert!(html.contains("controls.fpsMovement.moveSpeed = 0.8"));
        assert!(html.contains("three/addons/"));
        assert!(html.contains("three@0.180.0/examples/jsm/"));
        assert!(!html.contains("OrbitControls"));
        assert!(!html.contains("controls.target"));
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
                has_ply: true,
            },
            None,
        );
        assert!(html.contains("lodSplatScale: 0.7"));
        assert!(html.contains("lodRenderScale: 2"));
        assert!(html.contains("behindFoveate: 0.1"));
        assert!(html.contains("coneFoveate: 0.4"));
        assert!(html.contains("clipXY: 1.2"));
        assert!(html.contains(&format!("minAlpha: {:.8}", ROOM_MIN_ALPHA)));
        assert!(html.contains("controls.fpsMovement.moveSpeed = 0.5"));
    }

    #[test]
    fn html_outdoor_profile_lowers_splat_budget() {
        let mut settings = PipelineSettings::from_preset(Preset::Balanced);
        settings.capture_mode = CaptureMode::Outdoor;
        let html = viewer_html(
            &ArchiveEntry {
                meta: crate::archive::ArchiveMeta {
                    id: "x".into(),
                    title: "Park".into(),
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
                has_ply: true,
            },
            None,
        );
        assert!(html.contains("lodSplatScale: 0.5"));
        assert!(html.contains("lodRenderScale: 3"));
        assert!(html.contains("coneFoveate: 0.3"));
        assert!(html.contains("maxStdDev: Math.sqrt(4)"));
        assert!(html.contains("clipXY: 1.1"));
        assert!(html.contains("controls.fpsMovement.moveSpeed = 2"));
        assert!(html.contains("const splatMaxStdDev = Math.sqrt(4)"));
    }

    #[test]
    fn html_omits_map_without_geo() {
        let html = sample_html(false);
        assert!(!html.contains("openstreetmap.org"));
        assert!(html.contains("./scene.js"));
    }

    #[test]
    fn html_cycles_view_modes() {
        let html = sample_html(false);
        assert!(html.contains(r#"const MODES = ["splats", "dots", "discs"]"#));
        assert!(html.contains(r#"const LABELS = ["Splats", "Dots", "Discs"]"#));
        assert!(html.contains("viewMode = (viewMode + 1) % MODES.length"));
        assert!(html.contains("spark.maxStdDev = splatMaxStdDev"));
        assert!(html.contains("spark.minPixelRadius = 1"));
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
        let err = export_html(&entry, &dest).unwrap_err();
        assert!(err.to_string().contains("scene.spz is missing"));
        fs::write(Path::new(&entry.dir).join("scene.spz"), b"spz\n").unwrap();
        export_html(&entry, &dest).unwrap();
        assert!(dest.join("index.html").is_file());
        assert!(dest.join("scene.spz").is_file());
        assert!(dest.join("scene.js").is_file());
        assert!(!dest.join("scene.ply").is_file());
        assert!(dest.join("meta.json").is_file());
        let html = fs::read_to_string(dest.join("index.html")).unwrap();
        assert!(html.contains("openstreetmap.org"));
        assert!(html.contains("./scene.js"));
        assert!(html.contains("fileBytes: sceneBytes"));
        assert_eq!(fs::read(dest.join("scene.spz")).unwrap(), b"spz\n");
        assert_eq!(
            fs::read_to_string(dest.join("scene.js")).unwrap(),
            "globalThis.SCENE_SPZ_B64 = \"c3B6Cg==\";\n"
        );
    }

    #[test]
    fn scene_js_payload_is_classic_script_base64() {
        assert_eq!(
            scene_js_payload(b"spz\n"),
            "globalThis.SCENE_SPZ_B64 = \"c3B6Cg==\";\n"
        );
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
            has_ply: true,
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
                has_ply: true,
            },
            Some(&ViewPose {
                position: [1.0, 2.0, 3.0],
                quaternion: [0.0, 0.0, 0.0, 1.0],
            }),
        );
        assert!(html.contains("camera.position.set(1.00000000, 2.00000000, 3.00000000)"));
        assert!(html.contains("camera.quaternion.set(0.00000000, 0.00000000, 0.00000000, 1.00000000)"));
        assert!(!html.contains("controls.target"));
    }
}
