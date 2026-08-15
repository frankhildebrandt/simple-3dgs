import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import * as THREE from "three";
import { SparkControls, SparkRenderer, SplatMesh } from "@sparkjsdev/spark";
import { LookCapture, isLookCaptureClick, isLookReleaseKey } from "../lookCapture";
import { applyPointerLook } from "../pointerLook";
import type { CaptureMode } from "../types";
import { viewerProfile } from "../viewerProfile";

type Props = {
  plyPath: string | null;
  captureMode?: CaptureMode;
  live?: boolean;
  fullscreen?: boolean;
  onToggleFullscreen?: () => void;
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
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<World | null>(null);
  const modeRef = useRef(captureMode);
  modeRef.current = captureMode;
  const [ready, setReady] = useState(false);

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

    const renderer = new THREE.WebGLRenderer({ antialias: false, alpha: false });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(host.clientWidth, host.clientHeight);
    renderer.domElement.style.touchAction = "none";
    renderer.domElement.tabIndex = 0;
    renderer.domElement.style.outline = "none";
    host.tabIndex = 0;
    host.appendChild(renderer.domElement);

    const profile = viewerProfile(modeRef.current);
    const spark = new SparkRenderer({
      renderer,
      minAlpha: profile.minAlpha,
      lodRenderScale: profile.lodRenderScale,
      behindFoveate: profile.behindFoveate,
    });
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
      worldRef.current?.splat?.removeFromParent();
      spark.removeFromParent();
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
    const world = worldRef.current;
    if (!ready || !world || !plyPath) {
      return;
    }
    const url = `${convertFileSrc(plyPath)}?t=${Date.now()}`;
    const profile = viewerProfile(modeRef.current);
    const next = new SplatMesh({
      url,
      lod: profile.lod,
      lodAbove: profile.lodAbove,
    });
    next.quaternion.set(1, 0, 0, 0);
    world.scene.add(next);
    const previous = world.splat;
    world.splat = next;
    if (previous) {
      previous.removeFromParent();
    }
    const posePromise = loadViewPose(plyPath);
    void next.initialized.then(async (mesh) => {
      if (worldRef.current?.splat !== mesh || world.framed) {
        return;
      }
      const view = await posePromise;
      if (worldRef.current?.splat !== mesh || world.framed) {
        return;
      }
      world.view = view;
      frameSplat(world.camera, world.controls, mesh, modeRef.current, view);
      world.framed = true;
    });
  }, [plyPath, ready]);

  useEffect(() => {
    const world = worldRef.current;
    const mesh = world?.splat;
    if (!ready || !world || !mesh || !world.framed) {
      return;
    }
    const profile = viewerProfile(captureMode);
    world.spark.minAlpha = profile.minAlpha;
    world.spark.lodRenderScale = profile.lodRenderScale;
    world.spark.behindFoveate = profile.behindFoveate;
    void mesh.initialized.then((readyMesh) => {
      if (worldRef.current?.splat !== readyMesh) {
        return;
      }
      frameSplat(world.camera, world.controls, readyMesh, captureMode, world.view);
    });
  }, [captureMode, ready]);

  const hint = live
    ? "Live preview — Click to look · Esc release · WASD fly · Q up · E down · Space start · Shift faster"
    : "Click to look · Esc release · WASD fly · Q up · E down · Space start · Shift faster";

  return (
    <div className="viewer" ref={hostRef}>
      {onToggleFullscreen ? (
        <button type="button" className="viewer-expand" onClick={onToggleFullscreen}>
          {fullscreen ? "Exit fullscreen" : "Fullscreen"}
        </button>
      ) : null}
      <p className="viewer-hint">{hint}</p>
    </div>
  );
}
