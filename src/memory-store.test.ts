import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { pino } from "pino";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { MemoryStore } from "./memory-store.ts";

// Create a silent logger for tests
const testLogger = pino({ level: "silent" });

describe("MemoryStore", () => {
  let store: MemoryStore;
  let testDir: string;

  beforeEach(async () => {
    // Create a unique temp directory for each test
    testDir = path.join(
      os.tmpdir(),
      `codeloops-test-${Date.now()}-${Math.random().toString(36).slice(2)}`
    );
    await fs.mkdir(testDir, { recursive: true });

    // Create store with test logger and custom data directory
    store = new MemoryStore(testLogger, { dataDir: testDir });
    await store.init();
  });

  afterEach(async () => {
    // Clean up test directory
    try {
      await fs.rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe("append", () => {
    it("should append a memory entry and return it with generated id", async () => {
      const entry = await store.append({
        content: "Test memory content",
        project: "test-project",
        tags: ["test", "example"],
      });

      expect(entry.id).toBeDefined();
      expect(entry.content).toBe("Test memory content");
      expect(entry.project).toBe("test-project");
      expect(entry.tags).toEqual(["test", "example"]);
      expect(entry.createdAt).toBeDefined();
    });

    it("should append entries with optional fields", async () => {
      const entry = await store.append({
        content: "Memory with session",
        project: "test-project",
        sessionId: "session-123",
        source: "test-source",
      });

      expect(entry.sessionId).toBe("session-123");
      expect(entry.source).toBe("test-source");
    });
  });

  describe("getById", () => {
    it("should retrieve an entry by id", async () => {
      const created = await store.append({
        content: "Find me",
        project: "test-project",
      });

      const found = await store.getById(created.id);

      expect(found).toBeDefined();
      expect(found?.id).toBe(created.id);
      expect(found?.content).toBe("Find me");
    });

    it("should return undefined for non-existent id", async () => {
      const found = await store.getById("non-existent-id");
      expect(found).toBeUndefined();
    });
  });

  describe("query", () => {
    beforeEach(async () => {
      // Add test entries
      await store.append({
        content: "Alpha entry",
        project: "project-a",
        tags: ["important"],
      });
      await store.append({
        content: "Beta entry",
        project: "project-a",
        tags: ["test"],
      });
      await store.append({
        content: "Gamma entry",
        project: "project-b",
        tags: ["important", "test"],
      });
      await store.append({ content: "Delta entry", project: "project-b" });
    });

    it("should query by project", async () => {
      const results = await store.query({ project: "project-a" });
      expect(results).toHaveLength(2);
      expect(results.every((e) => e.project === "project-a")).toBe(true);
    });

    it("should query by tags", async () => {
      const results = await store.query({ tags: ["important"] });
      expect(results).toHaveLength(2);
    });

    it("should query by multiple tags (AND)", async () => {
      const results = await store.query({ tags: ["important", "test"] });
      expect(results).toHaveLength(1);
      expect(results[0].content).toBe("Gamma entry");
    });

    it("should query by text search", async () => {
      const results = await store.query({ query: "gamma" });
      expect(results).toHaveLength(1);
      expect(results[0].content).toBe("Gamma entry");
    });

    it("should respect limit", async () => {
      const results = await store.query({ limit: 2 });
      expect(results).toHaveLength(2);
      // Should return the most recent entries
      expect(results[0].content).toBe("Gamma entry");
      expect(results[1].content).toBe("Delta entry");
    });

    it("should combine filters", async () => {
      const results = await store.query({
        project: "project-b",
        tags: ["important"],
      });
      expect(results).toHaveLength(1);
      expect(results[0].content).toBe("Gamma entry");
    });
  });

  describe("listProjects", () => {
    it("should list all unique projects", async () => {
      await store.append({ content: "Entry 1", project: "project-a" });
      await store.append({ content: "Entry 2", project: "project-b" });
      await store.append({ content: "Entry 3", project: "project-a" });

      const projects = await store.listProjects();

      expect(projects).toHaveLength(2);
      expect(projects).toContain("project-a");
      expect(projects).toContain("project-b");
    });

    it("should return empty array when no entries", async () => {
      const projects = await store.listProjects();
      expect(projects).toEqual([]);
    });
  });

  describe("forget", () => {
    it("should soft delete an entry", async () => {
      const entry = await store.append({
        content: "To be deleted",
        project: "test-project",
      });

      const deleted = await store.forget(entry.id, "test reason");

      expect(deleted).toBeDefined();
      expect(deleted?.id).toBe(entry.id);
      expect(deleted?.deletedAt).toBeDefined();
      expect(deleted?.deletedReason).toBe("test reason");

      // Entry should no longer be findable
      const found = await store.getById(entry.id);
      expect(found).toBeUndefined();
    });

    it("should return undefined when deleting non-existent entry", async () => {
      const deleted = await store.forget("non-existent-id");
      expect(deleted).toBeUndefined();
    });

    it("should not affect other entries when deleting", async () => {
      const entry1 = await store.append({
        content: "Keep me",
        project: "test",
      });
      const entry2 = await store.append({
        content: "Delete me",
        project: "test",
      });
      const entry3 = await store.append({
        content: "Keep me too",
        project: "test",
      });

      await store.forget(entry2.id);

      const found1 = await store.getById(entry1.id);
      const found2 = await store.getById(entry2.id);
      const found3 = await store.getById(entry3.id);

      expect(found1).toBeDefined();
      expect(found2).toBeUndefined();
      expect(found3).toBeDefined();
    });
  });
});
