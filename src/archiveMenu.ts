export type MenuPoint = { x: number; y: number };
export type MenuSize = { width: number; height: number };
export type Viewport = { width: number; height: number };

const PAD = 8;

/** Keeps a context menu inside the viewport with a small inset. */
export function clampMenuPosition(point: MenuPoint, size: MenuSize, viewport: Viewport): MenuPoint {
  const maxX = Math.max(PAD, viewport.width - size.width - PAD);
  const maxY = Math.max(PAD, viewport.height - size.height - PAD);
  return {
    x: Math.min(Math.max(PAD, point.x), maxX),
    y: Math.min(Math.max(PAD, point.y), maxY),
  };
}
