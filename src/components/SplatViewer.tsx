import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as THREE from "three";
import { SparkControls, SparkRenderer, SplatFileType, SplatMesh } from "@sparkjsdev/spark";
import { applyPointerLook, applyPointerRoll, isLookDragClick, levelFpsCamera } from "../pointerLook";
import { jpegBase64FromCanvas, scaledJpegDataUrl } from "../previewCapture";
import { loadPlyBytes, splatBytesFromInvoke } from "../spzTranscode";
import { splatFileName, splatKindFromPath, splatLoadHint, splatSidecarPath } from "../splatFile";
import { readSplatFile } from "../api";
import { parseMenuMode } from "../appMenu";
import type { CaptureMode } from "../types";
import { MOVE_SPEED_MAX, MOVE_SPEED_MIN, viewerKnobsFor, type ViewerKnobs } from "../viewerKnobs";
import {
  applyViewerMode,
  nextViewerMode,
  viewerModeLabel,
  type ViewerMode,
} from "../viewerMode";
import { sparkTuning, splatLoadFlags, viewerProfileFromKnobs } from "../viewerProfile";
import {
  viewerPixelRatio,
  viewerScaleForSession,
  type ViewerScale,
} from "../viewerPixelRatio";

type Props = {
  plyPath: string | null;
  captureMode?: CaptureMode;
  viewer?: ViewerKnobs;
  live?: boolean;
  fullscreen?: boolean;
  onToggleFullscreen?: () => void;
  onSetPreview?: (jpegBase64: string) => void | Promise<void>;
};

type ViewPose = {
  position: [number, number, number];
  quaternion: [number, number, number, number];
};

type World = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  spark: SparkRenderer;
  controls: SparkControls;
  splat: SplatMesh | null;
  framed: boolean;
  view: ViewPose | null;
};

/** Places the camera at the first capture frame when known, otherwise by capture type. */
function frameSplat(
  camera: THREE.PerspectiveCamera,
  mesh: SplatMesh,
  mode: CaptureMode,
  profile: ReturnType<typeof viewerProfileFromKnobs>,
  view: ViewPose | null,
) {
  mesh.updateMatrixWorld(true);
  const box = mesh.getBoundingBox(true);
  const size = box.isEmpty()
    ? new THREE.Vector3(1, 1, 1)
    : box.clone().applyMatrix4(mesh.matrixWorld).getSize(new THREE.Vector3());
  const extent = Math.max(size.length(), 0.5);
  const radius = Math.max(size.x, size.y, size.z, 0.5) * 0.5;
  camera.fov = profile.fov;

  if (view) {
    camera.position.set(view.position[0], view.position[1], view.position[2]);
    camera.quaternion.set(view.quaternion[0], view.quaternion[1], view.quaternion[2], view.quaternion[3]);
    camera.near = Math.max(0.01, extent * 0.0001);
    camera.far = Math.max(1000, extent * profile.farMultiplier);
    camera.updateProjectionMatrix();
    levelFpsCamera(camera);
    return;
  }

  if (box.isEmpty()) {
    return;
  }
  const worldBox = box.clone().applyMatrix4(mesh.matrixWorld);
  const center = worldBox.getCenter(new THREE.Vector3());

  if (mode === "object") {
    const fit = radius / Math.tan(THREE.MathUtils.degToRad(camera.fov) * 0.5);
    const dist = fit * 2.6;
    camera.position.set(center.x, center.y + radius * 0.35, center.z + dist);
    camera.lookAt(center);
    camera.far = Math.max(1000, dist * 20);
    camera.updateProjectionMatrix();
    levelFpsCamera(camera);
    return;
  }

  camera.position.copy(center);
  const target = center.clone();
  target.z -= extent * 0.25;
  if (mode === "outdoor") {
    camera.position.y += extent * 0.05;
    target.y -= extent * 0.05;
  }
  camera.lookAt(target);
  camera.near = Math.max(0.01, extent * 0.0001);
  camera.far = Math.max(1000, extent * profile.farMultiplier);
  camera.updateProjectionMatrix();
  levelFpsCamera(camera);
}

/** Updates FOV and clip planes from the splat bounds without moving the camera. */
function applyViewerOptics(
  camera: THREE.PerspectiveCamera,
  mesh: SplatMesh,
  profile: ReturnType<typeof viewerProfileFromKnobs>,
) {
  mesh.updateMatrixWorld(true);
  const box = mesh.getBoundingBox(true);
  const size = box.isEmpty()
    ? new THREE.Vector3(1, 1, 1)
    : box.clone().applyMatrix4(mesh.matrixWorld).getSize(new THREE.Vector3());
  const extent = Math.max(size.length(), 0.5);
  camera.fov = profile.fov;
  camera.near = Math.max(0.01, extent * 0.0001);
  camera.far = Math.max(1000, extent * profile.farMultiplier);
  camera.updateProjectionMatrix();
}

/** Removes a Spark mesh from the scene and frees its GPU buffers. */
function dropSplat(mesh: SplatMesh) {
  mesh.removeFromParent();
  mesh.dispose();
}

async function loadViewPose(plyPath: string): Promise<ViewPose | null> {
  const sidecar = splatSidecarPath(plyPath, "view.json");
  if (!sidecar) {
    return null;
  }
  try {
    const bytes = splatBytesFromInvoke(await readSplatFile(sidecar));
    const data = JSON.parse(new TextDecoder().decode(bytes)) as ViewPose;
    if (!Array.isArray(data.position) || !Array.isArray(data.quaternion)) {
      return null;
    }
    return data;
  } catch {
    return null;
  }
}

export function SplatViewer({
  plyPath,
  captureMode = "object",
  viewer,
  live,
  fullscreen,
  onToggleFullscreen,
  onSetPreview,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<World | null>(null);
  const modeRef = useRef(captureMode);
  modeRef.current = captureMode;
  const knobsRef = useRef(viewer ?? viewerKnobsFor(captureMode));
  knobsRef.current = viewer ?? viewerKnobsFor(captureMode);
  const [ready, setReady] = useState(false);
  const [framed, setFramed] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [scale, setScale] = useState<ViewerScale>("fast");
  const [viewMode, setViewMode] = useState<ViewerMode>("splats");
  const [loadError, setLoadError] = useState<string | null>(null);
  const liveRef = useRef(!!live);
  liveRef.current = !!live;
  const scaleRef = useRef(scale);
  scaleRef.current = scale;
  const viewModeRef = useRef(viewMode);
  viewModeRef.current = viewMode;
  const pendingPathRef = useRef<string | null>(null);
  const shownPathRef = useRef<string | null>(null);
  const loadingRef = useRef(false);
  const [flySpeed, setFlySpeed] = useState(
    () => (viewer ?? viewerKnobsFor(captureMode)).moveSpeed,
  );

  useEffect(() => {
    let cancelled = false;
    let stop: (() => void) | undefined;
    void listen<string>("menu-mode", (event) => {
      const next = parseMenuMode(event.payload);
      if (next) {
        setViewMode(next);
      }
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  /** Renders the current view to JPEG base64, or null if the splat is not framed. */
  function capturePreview(): string | null {
    const world = worldRef.current;
    if (!world?.splat || !world.framed) {
      return null;
    }
    const profile = viewerProfileFromKnobs(knobsRef.current, liveRef.current);
    applyViewerMode(world.spark, "splats", profile);
    try {
      world.renderer.render(world.scene, world.camera);
      const canvas = world.renderer.domElement;
      return jpegBase64FromCanvas(canvas, (_source, width, height) =>
        scaledJpegDataUrl(canvas, width, height),
      );
    } finally {
      applyViewerMode(world.spark, viewModeRef.current, profile);
    }
  }

  async function setPreviewFromView() {
    if (!onSetPreview || previewBusy) {
      return;
    }
    const jpeg = capturePreview();
    if (!jpeg) {
      return;
    }
    setPreviewBusy(true);
    try {
      await onSetPreview(jpeg);
    } finally {
      setPreviewBusy(false);
    }
  }

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return;
    }

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x101114);
    const camera = new THREE.PerspectiveCamera(
      knobsRef.current.fov,
      host.clientWidth / Math.max(host.clientHeight, 1),
      0.01,
      1000,
    );
    camera.position.set(0, 0.4, 2.4);

    const renderer = new THREE.WebGLRenderer({
      antialias: false,
      alpha: false,
      powerPreference: "high-performance",
    });
    renderer.setPixelRatio(
      viewerPixelRatio(viewerScaleForSession(scaleRef.current, liveRef.current), window.devicePixelRatio),
    );
    renderer.setSize(host.clientWidth, host.clientHeight, false);
    renderer.domElement.style.touchAction = "none";
    renderer.domElement.tabIndex = 0;
    renderer.domElement.style.outline = "none";
    host.tabIndex = 0;
    host.appendChild(renderer.domElement);

    const profile = viewerProfileFromKnobs(knobsRef.current, liveRef.current);
    const spark = new SparkRenderer({
      renderer,
      ...sparkTuning(profile),
      enableLod: true,
      lodSplatCount: knobsRef.current.webviewLodSplatCount,
    });
    applyViewerMode(spark, viewModeRef.current, profile);
    scene.add(spark);

    const controls = new SparkControls({ canvas: renderer.domElement });
    controls.fpsMovement.keycodeMoveMapping = {
      KeyW: new THREE.Vector3(0, 0, -1),
      KeyS: new THREE.Vector3(0, 0, 1),
      KeyA: new THREE.Vector3(-1, 0, 0),
      KeyD: new THREE.Vector3(1, 0, 0),
      KeyQ: new THREE.Vector3(0, 1, 0),
      KeyE: new THREE.Vector3(0, -1, 0),
    };
    controls.fpsMovement.keycodeRotateMapping = {};
    controls.fpsMovement.capsMultiplier = 1;
    controls.fpsMovement.moveSpeed = knobsRef.current.moveSpeed;
    controls.pointerControls.enable = false;

    worldRef.current = { scene, camera, renderer, spark, controls, splat: null, framed: false, view: null };
    setReady(true);

    let looking = false;
    const canvas = renderer.domElement;
    const onPointerDown = (event: PointerEvent) => {
      canvas.focus();
      if (!isLookDragClick(event)) {
        return;
      }
      looking = true;
      canvas.setPointerCapture(event.pointerId);
    };
    const onPointerMove = (event: PointerEvent) => {
      if (!looking) {
        return;
      }
      applyPointerLook(camera, event.movementX, event.movementY);
    };
    const onPointerUp = (event: PointerEvent) => {
      if (!looking) {
        return;
      }
      looking = false;
      if (canvas.hasPointerCapture(event.pointerId)) {
        canvas.releasePointerCapture(event.pointerId);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.code !== "Space" || event.repeat || event.metaKey || event.ctrlKey || event.altKey) {
        return;
      }
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.tagName === "BUTTON" || target.tagName === "INPUT" || target.tagName === "TEXTAREA")
      ) {
        return;
      }
      const world = worldRef.current;
      const mesh = world?.splat;
      if (!world || !mesh || !world.framed) {
        return;
      }
      event.preventDefault();
      frameSplat(world.camera, mesh, modeRef.current, viewerProfileFromKnobs(knobsRef.current, liveRef.current), world.view);
    };
    canvas.addEventListener("pointerdown", onPointerDown);
    canvas.addEventListener("pointermove", onPointerMove);
    canvas.addEventListener("pointerup", onPointerUp);
    canvas.addEventListener("pointercancel", onPointerUp);
    host.addEventListener("keydown", onKeyDown);

    let frame = 0;
    let lastTime = performance.now();
    const animate = () => {
      frame = requestAnimationFrame(animate);
      const now = performance.now();
      const dt = (now - lastTime) / 1000;
      lastTime = now;
      const keys = controls.fpsMovement.keycode;
      let roll = 0;
      if (keys.KeyY || keys.KeyZ) {
        roll += 1;
      }
      if (keys.KeyC) {
        roll -= 1;
      }
      applyPointerRoll(camera, roll, dt);
      controls.update(camera);
      renderer.render(scene, camera);
    };
    animate();

    const onResize = () => {
      if (!host.clientWidth || !host.clientHeight) {
        return;
      }
      camera.aspect = host.clientWidth / host.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(host.clientWidth, host.clientHeight, false);
    };
    const observer = new ResizeObserver(onResize);
    observer.observe(host);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerup", onPointerUp);
      canvas.removeEventListener("pointercancel", onPointerUp);
      host.removeEventListener("keydown", onKeyDown);
      controls.fpsMovement.enable = false;
      controls.pointerControls.enable = false;
      const splat = worldRef.current?.splat;
      if (splat) {
        dropSplat(splat);
      }
      spark.removeFromParent();
      spark.dispose();
      renderer.dispose();
      worldRef.current = null;
      setReady(false);
      if (renderer.domElement.parentElement === host) {
        host.removeChild(renderer.domElement);
      }
    };
  }, []);

  useEffect(() => {
    hostRef.current?.focus();
    worldRef.current?.renderer.domElement.focus();
    const host = hostRef.current;
    const world = worldRef.current;
    if (!host || !world) {
      return;
    }
    const syncSize = () => {
      if (!host.clientWidth || !host.clientHeight) {
        return;
      }
      world.camera.aspect = host.clientWidth / host.clientHeight;
      world.camera.updateProjectionMatrix();
      world.renderer.setSize(host.clientWidth, host.clientHeight, false);
    };
    syncSize();
    const frame = requestAnimationFrame(syncSize);
    return () => cancelAnimationFrame(frame);
  }, [fullscreen]);

  useEffect(() => {
    if (!ready) {
      return;
    }
    if (!worldRef.current?.splat) {
      shownPathRef.current = null;
    }
    pendingPathRef.current = plyPath;
    if (!plyPath) {
      setLoadError(null);
      return;
    }
    void loadLatest();

    /** Loads the newest pending PLY; skips start if a decode is already running. */
    async function loadLatest() {
      const world = worldRef.current;
      if (!world || loadingRef.current) {
        return;
      }
      const path = pendingPathRef.current;
      if (!path || path === shownPathRef.current) {
        return;
      }
      loadingRef.current = true;
      const posePromise = loadViewPose(path);
      let next: SplatMesh | null = null;
      try {
        const bytes = await loadPlyBytes(path);
        if (worldRef.current !== world || pendingPathRef.current !== path) {
          return;
        }
        const profile = viewerProfileFromKnobs(knobsRef.current, liveRef.current);
        const kind = splatKindFromPath(path);
        next = new SplatMesh({
          fileBytes: bytes,
          fileType: kind === "spz" ? SplatFileType.SPZ : SplatFileType.PLY,
          fileName: splatFileName(path),
          ...splatLoadFlags(profile, liveRef.current),
        });
        next.quaternion.set(1, 0, 0, 0);
        world.scene.add(next);
        const mesh = await next.initialized;
        next = mesh;
        if (worldRef.current !== world || pendingPathRef.current === null) {
          dropSplat(mesh);
          return;
        }
        const previous = world.splat;
        world.splat = mesh;
        shownPathRef.current = path;
        world.framed = false;
        setFramed(false);
        const view = await posePromise;
        if (worldRef.current?.splat !== mesh) {
          return;
        }
        world.view = view;
        frameSplat(world.camera, mesh, modeRef.current, profile, view);
        world.framed = true;
        setFramed(true);
        setLoadError(null);
        if (previous && previous !== mesh) {
          dropSplat(previous);
        }
      } catch (err) {
        if (next && worldRef.current?.splat !== next) {
          dropSplat(next);
        }
        if (pendingPathRef.current === path) {
          pendingPathRef.current = shownPathRef.current;
        }
        console.error("Checkpoint failed to load", path, err);
        setLoadError(splatLoadHint(err));
      } finally {
        loadingRef.current = false;
        if (worldRef.current === world && pendingPathRef.current !== shownPathRef.current) {
          void loadLatest();
        }
      }
    }
  }, [plyPath, ready]);

  useEffect(() => {
    const world = worldRef.current;
    const mesh = world?.splat;
    if (!ready || !world || !mesh || !world.framed) {
      return;
    }
    const profile = viewerProfileFromKnobs(viewer ?? viewerKnobsFor(captureMode), live);
    Object.assign(world.spark, sparkTuning(profile));
    applyViewerMode(world.spark, viewModeRef.current, profile);
    applyViewerOptics(world.camera, mesh, profile);
  }, [viewer, captureMode, ready, live]);

  useEffect(() => {
    const world = worldRef.current;
    const mesh = world?.splat;
    if (!ready || !world || !mesh || !world.framed) {
      return;
    }
    const profile = viewerProfileFromKnobs(knobsRef.current, liveRef.current);
    void mesh.initialized.then((readyMesh) => {
      if (worldRef.current?.splat !== readyMesh) {
        return;
      }
      frameSplat(world.camera, readyMesh, captureMode, profile, world.view);
    });
  }, [captureMode, ready]);

  const knobSpeed = (viewer ?? viewerKnobsFor(captureMode)).moveSpeed;
  useEffect(() => {
    setFlySpeed(knobSpeed);
  }, [knobSpeed]);

  useEffect(() => {
    const world = worldRef.current;
    if (!ready || !world) {
      return;
    }
    world.controls.fpsMovement.moveSpeed = flySpeed;
  }, [ready, flySpeed]);

  useEffect(() => {
    const world = worldRef.current;
    const host = hostRef.current;
    if (!ready || !world || !host) {
      return;
    }
    const nextScale = viewerScaleForSession(scale, !!live);
    world.renderer.setPixelRatio(viewerPixelRatio(nextScale, window.devicePixelRatio));
    if (host.clientWidth && host.clientHeight) {
      world.renderer.setSize(host.clientWidth, host.clientHeight, false);
    }
  }, [ready, scale, live]);

  useEffect(() => {
    const world = worldRef.current;
    if (!ready || !world) {
      return;
    }
    applyViewerMode(world.spark, viewMode, viewerProfileFromKnobs(knobsRef.current, liveRef.current));
  }, [ready, viewMode]);

  const hint = loadError
    ? loadError
    : live
      ? "Live preview — Hold left mouse to look · WASD fly · Q up · E down · Y/C roll · Space start · Shift faster"
      : "Hold left mouse to look · WASD fly · Q up · E down · Y/C roll · Space start · Shift faster";

  return (
    <div className="viewer" ref={hostRef}>
      {onSetPreview ? (
        <button
          type="button"
          className="viewer-preview"
          disabled={!framed || previewBusy}
          onClick={() => void setPreviewFromView()}
        >
          {previewBusy ? "Saving preview…" : "Set as preview"}
        </button>
      ) : null}
      <div className="viewer-actions">
        <label className="viewer-speed">
          Speed
          <input
            type="range"
            min={MOVE_SPEED_MIN}
            max={MOVE_SPEED_MAX}
            step={0.05}
            value={flySpeed}
            aria-label="Move speed"
            onChange={(event) => setFlySpeed(Number(event.currentTarget.value))}
          />
        </label>
        <button
          type="button"
          className="viewer-mode"
          onClick={() => setViewMode((current) => nextViewerMode(current))}
        >
          {viewerModeLabel(viewMode)}
        </button>
        <button
          type="button"
          className="viewer-scale"
          disabled={!!live}
          onClick={() => setScale((current) => (current === "fast" ? "sharp" : "fast"))}
        >
          {viewerScaleForSession(scale, !!live) === "fast" ? "Sharp" : "Fast"}
        </button>
        {onToggleFullscreen ? (
          <button type="button" className="viewer-expand" onClick={onToggleFullscreen}>
            {fullscreen ? "Exit fullscreen" : "Fullscreen"}
          </button>
        ) : null}
      </div>
      <p className="viewer-hint">{hint}</p>
    </div>
  );
}
