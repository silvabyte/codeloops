/**
 * Build script for the CodeLoops OpenCode plugin.
 *
 * Uses Bun's bundler to create a single-file bundle that includes
 * all dependencies except @opencode-ai/plugin (provided by OpenCode runtime).
 *
 * Output: dist/codeloops.js
 *
 * Best practices from https://bun.sh/docs/bundler:
 * - target: "bun" - Optimized for Bun runtime (OpenCode uses tsx/Bun)
 * - format: "esm" - ES modules (required for OpenCode plugins)
 * - external: ["@opencode-ai/plugin"] - Don't bundle the plugin SDK
 * - minify: false - Keep readable for debugging plugin issues
 */

const result = await Bun.build({
  entrypoints: ["./plugin/index.ts"],
  outdir: "./dist",
  target: "bun",
  format: "esm",
  minify: false,
  // Keep the bundle human-readable
  naming: "codeloops.[ext]",
  // External dependencies provided by OpenCode runtime
  external: ["@opencode-ai/plugin", "@opencode-ai/plugin/tool"],
});

if (!result.success) {
  console.error("Build failed:");
  for (const log of result.logs) {
    console.error(log);
  }
  process.exit(1);
}

console.log("Build successful!");
console.log("Output files:");
for (const output of result.outputs) {
  console.log(`  - ${output.path} (${(output.size / 1024).toFixed(2)} KB)`);
}
