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
fs.writeFileSync(${JSON.stringify(`${saved}.args`)}, JSON.stringify(process.argv.slice(2)));
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
    const input = { sessionID: "session", callID: "call", tool: "bash" };
    const output = {
      args: { command: "write then fail", unknown: "preserved" },
    };
    const before = JSON.stringify(output);
    const starting = hooks["tool.execute.before"]?.(input, output);
    const queued = JSON.parse(readFileSync(saved, "utf8"));
    assert.equal(queued.event.type, "history.tool.before");
    assert.deepEqual(queued.event.properties.args, output.args);
    assert.equal(JSON.stringify(output), before);
    await starting;
    const result = { title: "done", output: "literal result", metadata: {} };
    await hooks["tool.execute.after"]?.(
      { ...input, args: output.args },
      result
    );
    assert.deepEqual(result, {
      title: "done",
      output: "literal result",
      metadata: {},
    });
    assert.equal(
      JSON.parse(readFileSync(saved, "utf8")).event.type,
      "history.tool.after"
    );
    const configured = await HistoryPlugin(
      {
        client: {},
        project: { id: "project-id" },
        directory,
      } as PluginInput,
      {
        executable,
        dataDir: join(directory, "isolated data"),
        address: "127.0.0.1:47899",
        sourceVersion: "configured-version",
      }
    );
    await configured.event?.({ event });
    assert.equal(
      JSON.parse(readFileSync(saved, "utf8")).source_version,
      "configured-version"
    );
    assert.deepEqual(JSON.parse(readFileSync(`${saved}.args`, "utf8")), [
      "capture-opencode",
      "--json",
      "--data-dir",
      join(directory, "isolated data"),
      "--address",
      "127.0.0.1:47899",
    ]);
  } finally {
    if (previous === undefined) {
      Reflect.deleteProperty(process.env, "CODELOOPS_BIN");
    } else {
      process.env.CODELOOPS_BIN = previous;
    }
    rmSync(directory, { recursive: true, force: true });
  }
});
