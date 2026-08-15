import { describe, expect, it } from "vitest";
import * as THREE from "three";
import { POINTER_LOOK_SPEED, applyPointerLook } from "./pointerLook";

function yawPitch(camera: THREE.Object3D): { yaw: number; pitch: number } {
  const eulers = new THREE.Euler().setFromQuaternion(camera.quaternion, "YXZ");
  return { yaw: eulers.y, pitch: eulers.x };
}

describe("applyPointerLook", () => {
  it("yaws right when the mouse moves right", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 100, 0);
    const { yaw, pitch } = yawPitch(camera);
    expect(yaw).toBeCloseTo(-100 * POINTER_LOOK_SPEED);
    expect(pitch).toBeCloseTo(0);
  });

  it("accumulates yaw across moves", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 50, 0);
    applyPointerLook(camera, 50, 0);
    expect(yawPitch(camera).yaw).toBeCloseTo(-100 * POINTER_LOOK_SPEED);
  });

  it("pitches down when the mouse moves down", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 0, 80);
    const { yaw, pitch } = yawPitch(camera);
    expect(yaw).toBeCloseTo(0);
    expect(pitch).toBeCloseTo(-80 * POINTER_LOOK_SPEED);
  });

  it("clamps pitch to plus or minus half pi", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 0, 10_000);
    expect(yawPitch(camera).pitch).toBeCloseTo(-Math.PI / 2);
    applyPointerLook(camera, 0, -20_000);
    expect(yawPitch(camera).pitch).toBeCloseTo(Math.PI / 2);
  });

  it("ignores a zero delta", () => {
    const camera = new THREE.Object3D();
    camera.quaternion.set(0, 0.1, 0, 1).normalize();
    const before = camera.quaternion.clone();
    applyPointerLook(camera, 0, 0);
    expect(camera.quaternion.equals(before)).toBe(true);
  });
});
