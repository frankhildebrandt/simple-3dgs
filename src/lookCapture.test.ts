import { describe, expect, it, vi } from "vitest";
import {
  LookCapture,
  isLookCaptureClick,
  isLookReleaseKey,
  setLookCaptured,
} from "./lookCapture";

function mockWindow() {
  return {
    setCursorGrab: vi.fn(async () => undefined),
    setCursorVisible: vi.fn(async () => undefined),
  };
}

describe("setLookCaptured", () => {
  it("grabs and hides the cursor when entering", async () => {
    const win = mockWindow();
    await setLookCaptured(win, true);
    expect(win.setCursorGrab).toHaveBeenCalledWith(true);
    expect(win.setCursorVisible).toHaveBeenCalledWith(false);
  });

  it("releases grab and shows the cursor when exiting", async () => {
    const win = mockWindow();
    await setLookCaptured(win, false);
    expect(win.setCursorGrab).toHaveBeenCalledWith(false);
    expect(win.setCursorVisible).toHaveBeenCalledWith(true);
  });
});

describe("LookCapture", () => {
  it("enters once and exits by restoring the cursor", async () => {
    const win = mockWindow();
    const look = new LookCapture(win);
    await look.enter();
    await look.enter();
    expect(look.captured).toBe(true);
    expect(win.setCursorGrab).toHaveBeenCalledTimes(1);
    await look.exit();
    expect(look.captured).toBe(false);
    expect(win.setCursorGrab).toHaveBeenLastCalledWith(false);
    expect(win.setCursorVisible).toHaveBeenLastCalledWith(true);
    await look.exit();
    expect(win.setCursorGrab).toHaveBeenCalledTimes(2);
  });

  it("releases if exit runs while enter is in flight", async () => {
    let finishGrab: () => void = () => undefined;
    let grabCalls = 0;
    const win = {
      setCursorGrab: vi.fn(
        () =>
          new Promise<void>((resolve) => {
            grabCalls += 1;
            if (grabCalls === 1) {
              finishGrab = resolve;
              return;
            }
            resolve();
          }),
      ),
      setCursorVisible: vi.fn(async () => undefined),
    };
    const look = new LookCapture(win);
    const entering = look.enter();
    await look.exit();
    expect(look.captured).toBe(false);
    finishGrab();
    await entering;
    expect(look.captured).toBe(false);
    expect(win.setCursorGrab).toHaveBeenLastCalledWith(false);
    expect(win.setCursorVisible).toHaveBeenLastCalledWith(true);
  });
});

describe("isLookCaptureClick", () => {
  it("accepts a primary click on the canvas", () => {
    expect(isLookCaptureClick({ button: 0, target: { tagName: "CANVAS" } })).toBe(true);
  });

  it("ignores overlay buttons and non-primary clicks", () => {
    expect(isLookCaptureClick({ button: 0, target: { tagName: "BUTTON" } })).toBe(false);
    expect(isLookCaptureClick({ button: 2, target: { tagName: "CANVAS" } })).toBe(false);
  });
});

describe("isLookReleaseKey", () => {
  it("matches a clean Escape press", () => {
    expect(
      isLookReleaseKey({
        code: "Escape",
        repeat: false,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(true);
  });

  it("ignores repeats and modified Escape", () => {
    expect(
      isLookReleaseKey({
        code: "Escape",
        repeat: true,
        metaKey: false,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(false);
    expect(
      isLookReleaseKey({
        code: "Escape",
        repeat: false,
        metaKey: true,
        ctrlKey: false,
        altKey: false,
      }),
    ).toBe(false);
  });
});
