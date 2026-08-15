export type CursorWindow = {
  setCursorGrab: (grab: boolean) => Promise<void>;
  setCursorVisible: (visible: boolean) => Promise<void>;
};

/** Native cursor grab + hide. WKWebView has no Pointer Lock on macOS. */
export async function setLookCaptured(win: CursorWindow, captured: boolean): Promise<void> {
  await win.setCursorGrab(captured);
  await win.setCursorVisible(!captured);
}

/** Tracks whether look is captured so enter/exit stay idempotent. */
export class LookCapture {
  captured = false;
  private token = 0;

  constructor(private readonly win: CursorWindow) {}

  /** Grabs and hides the cursor if look is not already captured. */
  async enter(): Promise<void> {
    if (this.captured) {
      return;
    }
    const token = ++this.token;
    await setLookCaptured(this.win, true);
    if (token !== this.token) {
      await setLookCaptured(this.win, false);
      return;
    }
    this.captured = true;
  }

  /** Releases the cursor even if the native call fails. */
  async exit(): Promise<void> {
    this.token += 1;
    if (!this.captured) {
      return;
    }
    try {
      await setLookCaptured(this.win, false);
    } finally {
      this.captured = false;
    }
  }
}

/** True for a primary click on the splat canvas, not overlay UI. */
export function isLookCaptureClick(event: { button: number; target: unknown }): boolean {
  return event.button === 0 && elementTag(event.target) === "CANVAS";
}

function elementTag(target: unknown): string | undefined {
  if (target && typeof target === "object" && "tagName" in target && typeof target.tagName === "string") {
    return target.tagName;
  }
  return undefined;
}

/** True when Escape should drop look capture rather than native fullscreen. */
export function isLookReleaseKey(event: {
  code: string;
  repeat: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
}): boolean {
  return event.code === "Escape" && !event.repeat && !event.metaKey && !event.ctrlKey && !event.altKey;
}
