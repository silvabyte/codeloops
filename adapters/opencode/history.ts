import { spawnSync } from "node:child_process";
import type { Hooks, Plugin } from "@opencode-ai/plugin";

// OpenCode 1.18.30 invokes event hooks without awaiting their promises. Keep the
// durable enqueue synchronous so process disposal cannot strand pending promises.
// Boundary commands archive files before enqueue; draining never resamples files.
const HistoryPlugin: Plugin = ({ client, project, directory }) => {
  const executable = process.env.CODELOOPS_BIN ?? "codeloops";
  // The v1 plugin SDK has no runtime-version API. Setup supplies the tested
  // version; absence is reported honestly rather than inferring from SDK types.
  const sourceVersion = process.env.CODELOOPS_OPENCODE_VERSION ?? "unknown";

  const capture = async (event: unknown) => {
    const result = spawnSync(executable, ["capture-opencode", "--json"], {
      input: JSON.stringify({
        source_version: sourceVersion,
        observed_at: Date.now(),
        directory,
        project: project.id,
        event,
      }),
      encoding: "utf8",
      timeout: 60_000,
      maxBuffer: 64 * 1024,
      windowsHide: true,
    });
    if (result.error || result.status !== 0) {
      const message = `History capture failed: ${result.error?.message ?? result.stderr}`;
      process.stderr.write(`${message}\n`);
      try {
        await client.app.log({
          body: { service: "codeloops-history", level: "error", message },
        });
      } catch {
        process.stderr.write("History capture: client logging unavailable\n");
      }
    }
  };

  const hooks: Hooks = {
    "chat.message": async (input) => {
      await capture({ type: "history.prompt", properties: input });
    },
    "tool.execute.before": async (input, output) => {
      await capture({
        type: "history.tool.before",
        properties: { ...input, args: output.args },
      });
    },
    "tool.execute.after": async (input, output) => {
      await capture({
        type: "history.tool.after",
        properties: { ...input, ...output },
      });
    },
    event: async ({ event }) => {
      if (
        !(
          event.type.startsWith("message.") || event.type.startsWith("session.")
        )
      ) {
        return;
      }
      // Global session errors have no conversation to attach to.
      const properties = event.properties as Record<string, unknown>;
      if (event.type === "session.error" && !properties.sessionID) {
        return;
      }
      await capture(event);
    },
  };
  return Promise.resolve(hooks);
};

export default HistoryPlugin;
