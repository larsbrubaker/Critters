// LAN profiling server: serves the built demo (`demo/dist`) to a phone and
// collects the timing lines the game posts back with `?report`.
//
//   wasm-pack build ../critters-wasm --target web --out-dir ../demo/public/pkg --no-typescript
//   bun run build
//   bun run debug-server.ts            # then open http://<lan-ip>:8791/?report on the phone
//
// Every line the game logs (see `critters-core/src/debug.rs`) is appended to
// `demo/stats.log` and echoed here. Debug tooling only — not part of the
// deployed site.

import { appendFileSync, existsSync, writeFileSync } from "node:fs";
import { join, normalize } from "node:path";

const PORT = Number(process.env.PORT ?? 8791);
const ROOT = join(import.meta.dir, "dist");
const LOG = join(import.meta.dir, "stats.log");

writeFileSync(LOG, "");

const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".css": "text/css",
  ".png": "image/png",
  ".json": "application/json",
};

Bun.serve({
  port: PORT,
  hostname: "0.0.0.0",
  async fetch(req) {
    const url = new URL(req.url);
    if (url.pathname === "/__stats") {
      const line = (await req.text()).trim();
      if (line) {
        const stamped = `${new Date().toISOString()} ${line}\n`;
        appendFileSync(LOG, stamped);
        process.stdout.write(stamped);
      }
      return new Response("ok");
    }
    let path = normalize(decodeURIComponent(url.pathname)).replace(/^([/\\])+/, "");
    if (path === "" || path.endsWith("/") || path.endsWith("\\")) path += "index.html";
    const file = join(ROOT, path);
    if (!file.startsWith(ROOT) || !existsSync(file)) {
      return new Response("not found", { status: 404 });
    }
    const ext = file.slice(file.lastIndexOf("."));
    return new Response(Bun.file(file), {
      headers: {
        "content-type": TYPES[ext] ?? "application/octet-stream",
        "cache-control": "no-store",
      },
    });
  },
});

console.log(`critters debug server on http://0.0.0.0:${PORT} — log: ${LOG}`);
