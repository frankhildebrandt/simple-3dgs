import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import * as THREE from "three";
import { SparkControls, SparkRenderer, SplatFileType, SplatMesh } from "@sparkjsdev/spark";
import { LookCapture, isLookCaptureClick, isLookReleaseKey } from "../lookCapture";
import { applyPointerLook } from "../pointerLook";
import { jpegBase64FromCanvas, scaledJpegDataUrl } from "../previewCapture";
import { loadPlyBytes } from "../spzTranscode";
import { splatFileName, splatKindFromPath } from "../splatFile";
import type { CaptureMode } from "../types";
import {
  applyViewerMode,
  nextViewerMode,
  viewerModeLabel,
  type ViewerMode,
} from "../viewerMode";
import { sparkTuning, splatLoadFlags, viewerProfile } from "../viewerProfile";
import {
  viewerPixelRatio,
  viewerScaleForSession,
  type ViewerScale,
} from "../viewerPixelRatio";

type Props = {
  plyPath: string | null;
  captureMode?: CaptureMode;
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
  controls: SparkControls,
  mesh: SplatMesh,
  mode: CaptureMode,
  view: ViewPose | null,
) {
  mesh.updateMatrixWorld(true);
  const box = mesh.getBoundingBox(true);
  const size = box.isEmpty()
    ? new THREE.Vector3(1, 1, 1)
    : box.clone().applyMatrix4(mesh.matrixWorld).getSize(new THREE.Vector3());
  const extent = Math.max(size.length(), 0.5);
  const radius = Math.max(size.x, size.y, size.z, 0.5) * 0.5;

  if (view) {
    camera.position.set(view.position[0], view.position[1], view.position[2]);
    camera.quaternion.set(view.quaternion[0], view.quaternion[1], view.quaternion[2], view.quaternion[3]);
    camera.near = Math.max(0.01, extent * 0.0001);
    camera.far = Math.max(1000, extent * (mode === "outdoor" ? 80 : 40));
    camera.updateProjectionMatrix();
    controls.fpsMovement.moveSpeed =
      mode === "object"
        ? Math.max(radius * 0.8, 0.8)
        : mode === "outdoor"
          ? Math.max(extent * 0.15, 2)
          : Math.max(extent * 0.08, 0.5);
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
    controls.fpsMovement.moveSpeed = Math.max(radius * 0.8, 0.8);
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
  camera.far = Math.max(1000, extent * (mode === "outdoor" ? 80 : 40));
  camera.updateProjectionMatrix();
    controls.fpsMovement.moveSpeed =
    mode === "outdoor" ? Math.max(extent * 0.15, 2) : Math.max(extent * 0.08, 0.5);
}

/** Removes a Spark mesh from the scene and frees its GPU buffers. */
function dropSplat(mesh: SplatMesh) {
  mesh.removeFromParent();
  mesh.dispose();
}

async function loadViewPose(plyPath: string): Promise<ViewPose | null> {
  const slash = plyPath.lastIndexOf("/");
  if (slash < 0) {
    return null;
  }
  const url = convertFileSrc(`${plyPath.slice(0, slash)}/view.json`);
  try {
    const response = await fetch(url);
    if (!response.ok) {
      return null;
    }
    const data = (await response.json()) as ViewPose;
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
  live,
  fullscreen,
  onToggleFullscreen,
  onSetPreview,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<World | null>(null);
  const modeRef = useRef(captureMode);
  modeRef.current = captureMode;
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

  /** Renders the current view to JPEG base64, or null if the splat is not framed. */
  function capturePreview(): string | null {
    const world = worldRef.current;
    if (!world?.splat || !world.framed) {
      return null;
    }
    const profile = viewerProfile(modeRef.current, liveRef.current);
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
      60,
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
    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.domElement.style.touchAction = "none";
    renderer.domElement.tabIndex = 0;
    renderer.domElement.style.outline = "none";
    host.tabIndex = 0;
    host.appendChild(renderer.domElement);

    const profile = viewerProfile(modeRef.current, liveRef.current);
    const spark = new SparkRenderer({
      renderer,
      ...sparkTuning(profile),
      enableLod: true,
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
    controls.fpsMovement.moveSpeed = 2;
    controls.pointerControls.enable = false;

    worldRef.current = { scene, camera, renderer, spark, controls, splat: null, framed: false, view: null };
    setReady(true);

    const look = new LookCapture(getCurrentWindow());
    const onPointerDown = (event: PointerEvent) => {
      renderer.domElement.focus();
      if (!isLookCaptureClick(event)) {
        return;
      }
      void look.enter();
    };
    const onMouseMove = (event: MouseEvent) => {
      if (!look.captured) {
        return;
      }
      applyPointerLook(camera, event.movementX, event.movementY);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (isLookReleaseKey(event) && look.captured) {
        event.preventDefault();
        void look.exit();
        return;
      }
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
      frameSplat(world.camera, world.controls, mesh, modeRef.current, world.view);
    };
    const onBlur = () => {
      void look.exit();
    };
    host.addEventListener("pointerdown", onPointerDown);
    host.addEventListener("keydown", onKeyDown);
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("blur", onBlur);

    let frame = 0;
    const animate = () => {
      frame = requestAnimationFrame(animate);
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
      renderer.setSize(host.clientWidth, host.clientHeight);
    };
    const observer = new ResizeObserver(onResize);
    observer.observe(host);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      host.removeEventListener("pointerdown", onPointerDown);
      host.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("blur", onBlur);
      void look.exit();
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
        const profile = viewerProfile(modeRef.current, liveRef.current);
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
        frameSplat(world.camera, world.controls, mesh, modeRef.current, view);
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
        setLoadError("Checkpoint failed to load");
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
    const profile = viewerProfile(captureMode, live);
    Object.assign(world.spark, sparkTuning(profile));
    applyViewerMode(world.spark, viewModeRef.current, profile);
    void mesh.initialized.then((readyMesh) => {
      if (worldRef.current?.splat !== readyMesh) {
        return;
      }
      frameSplat(world.camera, world.controls, readyMesh, captureMode, world.view);
    });
  }, [captureMode, ready, live]);

  useEffect(() => {
    const world = worldRef.current;
    const host = hostRef.current;
    if (!ready || !world || !host) {
      return;
    }
    const nextScale = viewerScaleForSession(scale, !!live);
    world.renderer.setPixelRatio(viewerPixelRatio(nextScale, window.devicePixelRatio));
    if (host.clientWidth && host.clientHeight) {
      world.renderer.setSize(host.clientWidth, host.clientHeight);
    }
  }, [ready, scale, live]);

  useEffect(() => {
    const world = worldRef.current;
    if (!ready || !world) {
      return;
    }
    applyViewerMode(world.spark, viewMode, viewerProfile(modeRef.current, liveRef.current));
  }, [ready, viewMode]);

  const hint = loadError
    ? loadError
    : live
      ? "Live preview — Click to look · Esc release · WASD fly · Q up · E down · Space start · Shift faster"
      : "Click to look · Esc release · WASD fly · Q up · E down · Space start · Shift faster";

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
