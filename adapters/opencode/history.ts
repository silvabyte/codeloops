import { spawnSync } from "node:child_process";
import type { Hooks, Plugin } from "@opencode-ai/plugin";

// OpenCode 1.18.30 invokes event hooks without awaiting their promises. Keep the
// durable enqueue synchronous so process disposal cannot strand pending promises.
// This command only writes the local spool; the service drains it independently.
const HistoryPlugin: Plugin = ({ client, project, directory }) => {
  const executable = process.env.CODELOOPS_BIN ?? "codeloops";
  // The v1 plugin SDK has no runtime-version API. Setup supplies the tested
  // version; absence is reported honestly rather than inferring from SDK types.
  const sourceVersion = process.env.CODELOOPS_OPENCODE_VERSION ?? "unknown";

  const hooks: Hooks = {
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
      const result = spawnSync(executable, ["capture-opencode", "--json"], {
        input: JSON.stringify({
          source_version: sourceVersion,
          observed_at: Date.now(),
          directory,
          project: project.id,
          event,
        }),
        encoding: "utf8",
        timeout: 15_000,
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
    },
  };
  return Promise.resolve(hooks);
};

export default HistoryPlugin;
