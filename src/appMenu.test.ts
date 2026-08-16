import { describe, expect, it } from "vitest";
import { parseMenuMode, parseMenuProject, parseMenuView } from "./appMenu";

describe("parseMenuView", () => {
  it("accepts the three shell views", () => {
    expect(parseMenuView("easy")).toBe("easy");
    expect(parseMenuView("expert")).toBe("expert");
    expect(parseMenuView("archive")).toBe("archive");
  });

  it("rejects unknown payloads", () => {
    expect(parseMenuView("settings")).toBeNull();
    expect(parseMenuView("")).toBeNull();
  });
});

describe("parseMenuProject", () => {
  it("accepts new and open", () => {
    expect(parseMenuProject("new")).toBe("new");
    expect(parseMenuProject("open")).toBe("open");
  });

  it("rejects unknown payloads", () => {
    expect(parseMenuProject("save")).toBeNull();
  });
});

describe("parseMenuMode", () => {
  it("accepts splat display modes", () => {
    expect(parseMenuMode("splats")).toBe("splats");
    expect(parseMenuMode("dots")).toBe("dots");
    expect(parseMenuMode("discs")).toBe("discs");
  });

  it("rejects unknown payloads", () => {
    expect(parseMenuMode("fast")).toBeNull();
  });
});
