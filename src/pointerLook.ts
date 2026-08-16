import * as THREE from "three";

export const POINTER_LOOK_SPEED = 0.002;
export const POINTER_ROLL_SPEED = 2;

const _dir = new THREE.Vector3();
const _camUp = new THREE.Vector3();
const _worldUp = new THREE.Vector3();
const _x = new THREE.Vector3();
const _y = new THREE.Vector3();
const _z = new THREE.Vector3();
const _basis = new THREE.Matrix4();

/** True for a primary press on the splat canvas, not overlay UI. */
export function isLookDragClick(event: { button: number; target: unknown }): boolean {
  return event.button === 0 && elementTag(event.target) === "CANVAS";
}

function elementTag(target: unknown): string | undefined {
  if (target && typeof target === "object" && "tagName" in target && typeof target.tagName === "string") {
    return target.tagName;
  }
  return undefined;
}

function fpsWorldUp(camera: THREE.Object3D): THREE.Vector3 {
  _camUp.set(0, 1, 0).applyQuaternion(camera.quaternion);
  return _worldUp.set(0, _camUp.y < 0 ? -1 : 1, 0);
}

/**
 * Drops roll around the current up hemisphere. Used when framing a capture pose,
 * not during look — mouse look must keep roll and pass the zenith.
 */
export function levelFpsCamera(camera: THREE.Object3D): void {
  _dir.set(0, 0, -1).applyQuaternion(camera.quaternion);
  const worldUp = fpsWorldUp(camera);
  _z.copy(_dir).negate();
  _x.crossVectors(worldUp, _z);
  if (_x.lengthSq() < 1e-10) {
    return;
  }
  _x.normalize();
  _y.crossVectors(_z, _x).normalize();
  _basis.makeBasis(_x, _y, _z);
  camera.quaternion.setFromRotationMatrix(_basis);
}

/**
 * Applies unclamped local yaw/pitch from mouse deltas so look can pass the zenith.
 * Existing roll is left intact.
 */
export function applyPointerLook(
  camera: THREE.Object3D,
  dx: number,
  dy: number,
  rotateSpeed = POINTER_LOOK_SPEED,
): void {
  if (dx === 0 && dy === 0) {
    return;
  }
  camera.rotateY(-dx * rotateSpeed);
  camera.rotateX(-dy * rotateSpeed);
}

/** Rolls around the camera's local Z. Direction is +1 or -1; dt is seconds. */
export function applyPointerRoll(
  camera: THREE.Object3D,
  direction: number,
  dt: number,
  rollSpeed = POINTER_ROLL_SPEED,
): void {
  if (direction === 0 || dt === 0) {
    return;
  }
  camera.rotateZ(direction * rollSpeed * dt);
}
