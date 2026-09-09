import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import type { PluginInput } from "@opencode-ai/plugin";
import HistoryPlugin from "./history.ts";

test("forwards unchanged native JSON to a durable command before the hook yields", async () => {
  const directory = mkdtempSync(join(tmpdir(), "history bridge "));
  const previous = process.env.CODELOOPS_BIN;
  const executable = join(directory, "capture");
  const saved = join(directory, "input.json");
  writeFileSync(
    executable,
    `#!/usr/bin/env node
const fs = require('node:fs');
fs.writeFileSync(${JSON.stringify(saved)}, fs.readFileSync(0));
`,
    { mode: 0o700 }
  );
  process.env.CODELOOPS_BIN = executable;
  try {
    const hooks = await HistoryPlugin({
      client: {},
      project: { id: "project-id" },
      directory,
    } as PluginInput);
    const event = {
      type: "message.updated" as const,
      properties: {
        info: {
          id: "msg",
          sessionID: "session",
          role: "user" as const,
          time: { created: 1 },
          agent: "build",
          model: { providerID: "fixture", modelID: "fixture" },
          unknown: "$(literal)",
        },
      },
    };
    const original = JSON.stringify(event);
    const pending = hooks.event?.({ event });
    // Deliberately inspect before awaiting: OpenCode does not await event hooks.
    const forwarded = JSON.parse(readFileSync(saved, "utf8"));
    assert.deepEqual(forwarded.event, event);
    assert.equal(forwarded.directory, directory);
    assert.equal(JSON.stringify(event), original);
    await pending;
  } finally {
    if (previous === undefined) {
      Reflect.deleteProperty(process.env, "CODELOOPS_BIN");
    } else {
      process.env.CODELOOPS_BIN = previous;
    }
    rmSync(directory, { recursive: true, force: true });
  }
});
