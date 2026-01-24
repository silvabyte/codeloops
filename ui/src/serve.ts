import { serve } from "bun";
import { join } from "path";

const PORT = parseInt(process.env.PORT || "3101");
const DIST = join(import.meta.dir, "../dist");

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
