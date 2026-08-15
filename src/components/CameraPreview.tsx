import { useEffect, useRef } from "react";
import * as THREE from "three";
import type { SparsePreview } from "../types";

type Props = {
  preview: SparsePreview;
};

/** Sparse COLMAP cloud and camera frustums in Spark/Three space. */
export function CameraPreview({ preview }: Props) {
  const hostRef = useRef<HTMLDivElement>(null);

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
      0.05,
      500,
    );
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(host.clientWidth, host.clientHeight);
    host.appendChild(renderer.domElement);

    const root = new THREE.Group();
    scene.add(root);

    if (preview.points.length > 0) {
      const positions = new Float32Array(preview.points.length * 3);
      const colors = new Float32Array(preview.points.length * 3);
      for (let i = 0; i < preview.points.length; i++) {
        positions.set(preview.points[i], i * 3);
        const rgb = preview.colors[i] ?? [200, 180, 120];
        colors.set([rgb[0] / 255, rgb[1] / 255, rgb[2] / 255], i * 3);
      }
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
      geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
      root.add(
        new THREE.Points(
          geometry,
          new THREE.PointsMaterial({ size: 0.04, vertexColors: true, sizeAttenuation: true }),
        ),
      );
    }

    const frustumMat = new THREE.MeshBasicMaterial({
      color: 0xd98a32,
      wireframe: true,
      depthTest: true,
    });
    for (const cam of preview.cameras) {
      const geom = new THREE.ConeGeometry(0.06, 0.18, 4);
      geom.rotateX(-Math.PI / 2);
      const mesh = new THREE.Mesh(geom, frustumMat);
      mesh.position.set(cam.position[0], cam.position[1], cam.position[2]);
      mesh.quaternion.set(cam.quaternion[0], cam.quaternion[1], cam.quaternion[2], cam.quaternion[3]);
      root.add(mesh);
    }

    const box = new THREE.Box3().setFromObject(root);
    const center = box.getCenter(new THREE.Vector3());
    const size = Math.max(box.getSize(new THREE.Vector3()).length(), 1);
    let yaw = 0.6;
    let pitch = 0.35;
    let distance = size * 1.2;
    const target = center.clone();

    function applyOrbit() {
      camera.position.set(
        target.x + Math.sin(yaw) * Math.cos(pitch) * distance,
        target.y + Math.sin(pitch) * distance,
        target.z + Math.cos(yaw) * Math.cos(pitch) * distance,
      );
      camera.lookAt(target);
    }
    applyOrbit();

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
      if (!dragging) {
        return;
      }
      yaw -= (event.clientX - lastX) * 0.005;
      pitch = Math.max(-1.2, Math.min(1.2, pitch + (event.clientY - lastY) * 0.005));
      lastX = event.clientX;
      lastY = event.clientY;
      applyOrbit();
    };
    const onUp = () => {
      dragging = false;
    };
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      distance = Math.max(size * 0.2, distance * (event.deltaY > 0 ? 1.08 : 0.92));
      applyOrbit();
    };
    renderer.domElement.addEventListener("pointerdown", onDown);
    renderer.domElement.addEventListener("pointermove", onMove);
    renderer.domElement.addEventListener("pointerup", onUp);
    renderer.domElement.addEventListener("wheel", onWheel, { passive: false });

    let frame = 0;
    const tick = () => {
      frame = requestAnimationFrame(tick);
      renderer.render(scene, camera);
    };
    tick();

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
      renderer.domElement.removeEventListener("pointerdown", onDown);
      renderer.domElement.removeEventListener("pointermove", onMove);
      renderer.domElement.removeEventListener("pointerup", onUp);
      renderer.domElement.removeEventListener("wheel", onWheel);
      renderer.dispose();
      host.removeChild(renderer.domElement);
    };
  }, [preview]);

  const empty = preview.cameras.length === 0 && preview.points.length === 0;
  return (
    <div className="viewer stage-preview" ref={hostRef}>
      {empty ? <p className="viewer-hint">Waiting for cameras</p> : (
        <p className="viewer-hint">Drag to orbit · Scroll to zoom · {preview.cameras.length} cameras</p>
      )}
    </div>
  );
}
