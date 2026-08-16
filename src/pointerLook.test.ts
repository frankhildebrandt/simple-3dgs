import { describe, expect, it } from "vitest";
import * as THREE from "three";
import {
  POINTER_LOOK_SPEED,
  POINTER_ROLL_SPEED,
  applyPointerLook,
  applyPointerRoll,
  isLookDragClick,
  levelFpsCamera,
} from "./pointerLook";

function yawPitchRoll(camera: THREE.Object3D): { yaw: number; pitch: number; roll: number } {
  const eulers = new THREE.Euler().setFromQuaternion(camera.quaternion, "YXZ");
  return { yaw: eulers.y, pitch: eulers.x, roll: eulers.z };
}

function worldUpY(camera: THREE.Object3D): number {
  return new THREE.Vector3(0, 1, 0).applyQuaternion(camera.quaternion).y;
}

function lookDir(camera: THREE.Object3D): THREE.Vector3 {
  return new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion);
}

describe("applyPointerLook", () => {
  it("yaws right when the mouse moves right", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 100, 0);
    const { yaw, pitch } = yawPitchRoll(camera);
    expect(yaw).toBeCloseTo(-100 * POINTER_LOOK_SPEED);
    expect(pitch).toBeCloseTo(0);
    expect(lookDir(camera).x).toBeGreaterThan(0);
  });

  it("accumulates yaw across moves", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 50, 0);
    applyPointerLook(camera, 50, 0);
    expect(yawPitchRoll(camera).yaw).toBeCloseTo(-100 * POINTER_LOOK_SPEED);
  });

  it("pitches down when the mouse moves down", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 0, 80);
    const { yaw, pitch } = yawPitchRoll(camera);
    expect(yaw).toBeCloseTo(0);
    expect(pitch).toBeCloseTo(-80 * POINTER_LOOK_SPEED);
    expect(lookDir(camera).y).toBeLessThan(0);
  });

  it("pitches past straight down instead of clamping", () => {
    const camera = new THREE.Object3D();
    applyPointerLook(camera, 0, Math.PI / POINTER_LOOK_SPEED);
    expect(lookDir(camera).z).toBeCloseTo(1);
  });

  it("ignores a zero delta", () => {
    const camera = new THREE.Object3D();
    camera.quaternion.set(0, 0.1, 0, 1).normalize();
    const before = camera.quaternion.clone();
    applyPointerLook(camera, 0, 0);
    expect(camera.quaternion.equals(before)).toBe(true);
  });

  it("keeps existing roll while looking", () => {
    const camera = new THREE.Object3D();
    camera.quaternion.setFromEuler(new THREE.Euler(0, 0, 0.4, "YXZ"));
    applyPointerLook(camera, 10, 0);
    expect(Math.abs(yawPitchRoll(camera).roll)).toBeGreaterThan(0.2);
  });
});

describe("applyPointerRoll", () => {
  it("rolls around the local Z axis", () => {
    const camera = new THREE.Object3D();
    applyPointerRoll(camera, 1, 0.5);
    expect(yawPitchRoll(camera).roll).toBeCloseTo(POINTER_ROLL_SPEED * 0.5);
  });

  it("ignores a zero direction", () => {
    const camera = new THREE.Object3D();
    const before = camera.quaternion.clone();
    applyPointerRoll(camera, 0, 1);
    expect(camera.quaternion.equals(before)).toBe(true);
  });
});

describe("levelFpsCamera", () => {
  it("keeps yaw and pitch and drops roll", () => {
    const camera = new THREE.Object3D();
    camera.quaternion.setFromEuler(new THREE.Euler(0.2, 0.3, 0.4, "YXZ"));
    levelFpsCamera(camera);
    const { yaw, pitch, roll } = yawPitchRoll(camera);
    expect(roll).toBeCloseTo(0);
    expect(yaw).toBeCloseTo(0.3);
    expect(pitch).toBeCloseTo(0.2);
  });

  it("keeps the up hemisphere of an Rx(180) capture pose", () => {
    const camera = new THREE.Object3D();
    camera.quaternion.set(1, 0, 0, 0);
    levelFpsCamera(camera);
    expect(worldUpY(camera)).toBeLessThan(0);
    expect(lookDir(camera).z).toBeCloseTo(1);
  });
});

describe("isLookDragClick", () => {
  it("accepts a primary click on the canvas", () => {
    expect(isLookDragClick({ button: 0, target: { tagName: "CANVAS" } })).toBe(true);
  });

  it("ignores overlay buttons and non-primary clicks", () => {
    expect(isLookDragClick({ button: 0, target: { tagName: "BUTTON" } })).toBe(false);
    expect(isLookDragClick({ button: 2, target: { tagName: "CANVAS" } })).toBe(false);
  });
});
