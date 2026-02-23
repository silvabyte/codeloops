import { serve } from "bun";
import { join, dirname } from "path";

const PORT = parseInt(process.env.PORT || "3101");
// When compiled with `bun build --compile`, import.meta.dir points to a virtual
// filesystem (/$bunfs/). Use the real binary location to find dist/ alongside it.
const BIN_DIR = dirname(process.execPath);
const DIST = join(BIN_DIR, "dist");

serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname === "/" ? "/index.html" : url.pathname;
    const file = Bun.file(join(DIST, path));
    if (await file.exists()) return new Response(file);
    // SPA fallback
    return new Response(Bun.file(join(DIST, "index.html")));
  },
});

console.log(`UI server running on http://localhost:${PORT}`);
