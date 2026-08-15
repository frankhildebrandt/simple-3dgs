import * as THREE from "three";

export const POINTER_LOOK_SPEED = 0.002;
const PITCH_LIMIT = Math.PI / 2;

/**
 * Applies FPS yaw/pitch from mouse deltas. Matches Spark PointerControls:
 * yaw subtracts dx, pitch subtracts dy, pitch is clamped to ±π/2.
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
  const eulers = new THREE.Euler().setFromQuaternion(camera.quaternion, "YXZ");
  eulers.y -= dx * rotateSpeed;
  eulers.x = Math.max(-PITCH_LIMIT, Math.min(PITCH_LIMIT, eulers.x - dy * rotateSpeed));
  camera.quaternion.setFromEuler(eulers);
}
