import { describe, expect, it } from "vitest";
import { isLogWindowSearch, logText, logWindowLabel, logWindowUrl } from "./logWindow";

describe("isLogWindowSearch", () => {
  it("detects the log query param", () => {
    expect(isLogWindowSearch("?log=1")).toBe(true);
    expect(isLogWindowSearch("log=1")).toBe(true);
    expect(isLogWindowSearch("?log=")).toBe(true);
  });

  it("returns false when missing or disabled", () => {
    expect(isLogWindowSearch("")).toBe(false);
    expect(isLogWindowSearch("?splat=abc")).toBe(false);
    expect(isLogWindowSearch("?log=0")).toBe(false);
    expect(isLogWindowSearch("?log=false")).toBe(false);
  });
});

describe("logWindowLabel", () => {
  it("uses a stable Tauri label", () => {
    expect(logWindowLabel()).toBe("pipeline-log");
  });
});

describe("logWindowUrl", () => {
  it("points at the log webview", () => {
    expect(logWindowUrl()).toBe("/?log=1");
  });
});

describe("logText", () => {
  it("joins lines for the clipboard", () => {
    expect(logText(["a", "b"])).toBe("a\nb");
    expect(logText([])).toBe("");
  });
});
