import { describe, expect, it } from "vitest";
import { extractProjectName } from "./project.ts";

describe("extractProjectName", () => {
  it("returns project name for a valid path", () => {
    expect(extractProjectName("/Users/foo/bar/my_project")).toBe("my_project");
    expect(extractProjectName("C:/dev/my-project")).toBe("my-project");
  });

  it("returns null for empty or invalid input", () => {
    expect(extractProjectName("")).toBeNull();
    expect(extractProjectName("   ")).toBeNull();
    expect(extractProjectName(undefined as unknown as string)).toBeNull();
    expect(extractProjectName(null as unknown as string)).toBeNull();
  });

  it("replaces special characters with underscores", () => {
    expect(extractProjectName("/foo/bar/!@#my$%^proj&*()")).toBe("my_proj_");
  });

  it("returns null if only invalid characters", () => {
    expect(extractProjectName("/foo/bar/!@#$%^&*()")).toBeNull();
  });

  it("truncates long names to 50 chars", () => {
    const longName = "a".repeat(60);
    expect(extractProjectName(`/foo/bar/${longName}`)).toBe("a".repeat(50));
  });

  it("handles trailing slashes and mixed separators", () => {
    expect(extractProjectName("/foo/bar/project/")).toBe("project");
    expect(extractProjectName("foo\\bar\\proj")).toBe("proj");
  });
});
