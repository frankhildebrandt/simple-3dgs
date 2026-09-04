import { useEffect, useRef } from "react";
import * as THREE from "three";
import type { SparsePreview } from "../types";

type Props = { preview: SparsePreview; active?: boolean };

type World = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  renderer: THREE.WebGLRenderer;
  root: THREE.Group;
  points: THREE.Points<THREE.BufferGeometry, THREE.PointsMaterial>;
  cameras: THREE.InstancedMesh<THREE.ConeGeometry, THREE.MeshBasicMaterial>;
  cameraCapacity: number;
  pointCapacity: number;
  target: THREE.Vector3;
  size: number;
  yaw: number;
  pitch: number;
  distance: number;
  framed: boolean;
};

function render(world: World, active: boolean) {
  if (active && !document.hidden) world.renderer.render(world.scene, world.camera);
}

function applyOrbit(world: World) {
  const { target, yaw, pitch, distance } = world;
  world.camera.position.set(
    target.x + Math.sin(yaw) * Math.cos(pitch) * distance,
    target.y + Math.sin(pitch) * distance,
    target.z + Math.cos(yaw) * Math.cos(pitch) * distance,
  );
  world.camera.lookAt(target);
}

function cameraMesh(capacity: number, material?: THREE.MeshBasicMaterial) {
  const geometry = new THREE.ConeGeometry(0.06, 0.18, 4);
  geometry.rotateX(-Math.PI / 2);
  const mesh = new THREE.InstancedMesh(
    geometry,
    material ?? new THREE.MeshBasicMaterial({ color: 0xd98a32, wireframe: true, depthTest: true }),
    Math.max(1, capacity),
  );
  mesh.count = 0;
  return mesh;
}

/** Sparse COLMAP cloud and camera frustums with one persistent, on-demand WebGL scene. */
export function CameraPreview({ preview, active = true }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const worldRef = useRef<World | null>(null);
  const activeRef = useRef(active);
  activeRef.current = active;

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x101114);
    const camera = new THREE.PerspectiveCamera(60, host.clientWidth / Math.max(host.clientHeight, 1), 0.05, 500);
    const renderer = new THREE.WebGLRenderer({ antialias: false, alpha: false, powerPreference: "high-performance" });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.5));
    renderer.setSize(host.clientWidth, host.clientHeight, false);
    host.appendChild(renderer.domElement);

    const root = new THREE.Group();
    scene.add(root);
    const points = new THREE.Points(
      new THREE.BufferGeometry(),
      new THREE.PointsMaterial({ size: 0.04, vertexColors: true, sizeAttenuation: true }),
    );
    root.add(points);
    const cameras = cameraMesh(1);
    root.add(cameras);
    const world: World = {
      scene, camera, renderer, root, points, cameras, cameraCapacity: 1, pointCapacity: 0,
      target: new THREE.Vector3(), size: 1, yaw: 0.6, pitch: 0.35, distance: 1.2, framed: false,
    };
    worldRef.current = world;

    let dragging = false;
    let lastX = 0;
    let lastY = 0;
    const onDown = (event: PointerEvent) => {
      dragging = true;
      lastX = event.clientX;
      lastY = event.clientY;
      renderer.domElement.setPointerCapture(event.pointerId);
    };
    const onMove = (event: PointerEvent) => {
      if (!dragging) return;
      world.yaw -= (event.clientX - lastX) * 0.005;
      world.pitch = Math.max(-1.2, Math.min(1.2, world.pitch + (event.clientY - lastY) * 0.005));
      lastX = event.clientX;
      lastY = event.clientY;
      applyOrbit(world);
      render(world, activeRef.current);
    };
    const onUp = () => { dragging = false; };
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      world.distance = Math.max(world.size * 0.2, world.distance * (event.deltaY > 0 ? 1.08 : 0.92));
      applyOrbit(world);
      render(world, activeRef.current);
    };
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointermove", onMove);
    renderer.domElement.addEventListener("pointerup", onUp);
    renderer.domElement.addEventListener("pointercancel", onUp);
    renderer.domElement.addEventListener("wheel", onWheel, { passive: false });

    const onResize = () => {
      if (!activeRef.current || !host.clientWidth || !host.clientHeight) return;
      camera.aspect = host.clientWidth / host.clientHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(host.clientWidth, host.clientHeight, false);
      render(world, activeRef.current);
    };
    const observer = new ResizeObserver(onResize);
    observer.observe(host);

    return () => {
      observer.disconnect();
      renderer.domElement.removeEventListener("pointerdown", onDown);
      renderer.domElement.removeEventListener("pointermove", onMove);
      renderer.domElement.removeEventListener("pointerup", onUp);
      renderer.domElement.removeEventListener("pointercancel", onUp);
      renderer.domElement.removeEventListener("wheel", onWheel);
      points.geometry.dispose();
      points.material.dispose();
      world.cameras.geometry.dispose();
      world.cameras.material.dispose();
      renderer.dispose();
      worldRef.current = null;
      if (renderer.domElement.parentElement === host) host.removeChild(renderer.domElement);
    };
  }, []);

  useEffect(() => {
    const world = worldRef.current;
    if (!world || !active) return;
    if (world.pointCapacity === 0 || preview.points.length > world.pointCapacity) {
      world.pointCapacity = Math.max(10_000, preview.points.length);
      world.points.geometry.setAttribute("position", new THREE.BufferAttribute(new Float32Array(world.pointCapacity * 3), 3));
      world.points.geometry.setAttribute("color", new THREE.BufferAttribute(new Float32Array(world.pointCapacity * 3), 3));
    }
    const positionAttribute = world.points.geometry.getAttribute("position") as THREE.BufferAttribute;
    const colorAttribute = world.points.geometry.getAttribute("color") as THREE.BufferAttribute;
    const positions = positionAttribute.array as Float32Array;
    const colors = colorAttribute.array as Float32Array;
    const pointBounds = new THREE.Box3();
    const point = new THREE.Vector3();
    for (let i = 0; i < preview.points.length; i++) {
      positions.set(preview.points[i], i * 3);
      pointBounds.expandByPoint(point.fromArray(preview.points[i]));
      const rgb = preview.colors[i] ?? [200, 180, 120];
      colors.set([rgb[0] / 255, rgb[1] / 255, rgb[2] / 255], i * 3);
    }
    positionAttribute.needsUpdate = true;
    colorAttribute.needsUpdate = true;
    world.points.geometry.setDrawRange(0, preview.points.length);
    world.points.geometry.boundingBox = pointBounds.isEmpty() ? null : pointBounds;
    world.points.geometry.boundingSphere = pointBounds.isEmpty()
      ? null
      : pointBounds.getBoundingSphere(new THREE.Sphere());

    if (preview.cameras.length > world.cameraCapacity) {
      const previous = world.cameras;
      const next = cameraMesh(preview.cameras.length, previous.material);
      world.root.remove(previous);
      previous.geometry.dispose();
      world.root.add(next);
      world.cameras = next;
      world.cameraCapacity = preview.cameras.length;
    }
    const matrix = new THREE.Matrix4();
    const position = new THREE.Vector3();
    const quaternion = new THREE.Quaternion();
    const scale = new THREE.Vector3(1, 1, 1);
    preview.cameras.forEach((cam, index) => {
      position.fromArray(cam.position);
      quaternion.fromArray(cam.quaternion);
      matrix.compose(position, quaternion, scale);
      world.cameras.setMatrixAt(index, matrix);
    });
    world.cameras.count = preview.cameras.length;
    world.cameras.instanceMatrix.needsUpdate = true;

    if (!world.framed && (preview.points.length || preview.cameras.length)) {
      const box = new THREE.Box3().setFromObject(world.root);
      box.getCenter(world.target);
      world.size = Math.max(box.getSize(new THREE.Vector3()).length(), 1);
      world.distance = world.size * 1.2;
      world.framed = true;
      applyOrbit(world);
    }
    render(world, activeRef.current);
  }, [active, preview]);

  useEffect(() => {
    const world = worldRef.current;
    const host = hostRef.current;
    if (world && host && active && host.clientWidth && host.clientHeight) {
      world.camera.aspect = host.clientWidth / host.clientHeight;
      world.camera.updateProjectionMatrix();
      world.renderer.setSize(host.clientWidth, host.clientHeight, false);
      render(world, true);
    }
  }, [active]);

  const empty = preview.cameras.length === 0 && preview.points.length === 0;
  return (
    <div className="viewer stage-preview" ref={hostRef}>
      <p className="viewer-hint">
        {empty ? "Waiting for cameras" : `Drag to orbit · Scroll to zoom · ${preview.cameras.length} cameras`}
      </p>
    </div>
  );
}
