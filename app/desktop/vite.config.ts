import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed port; if that's not available it will try the next one.
const host = process.env.TAURI_DEV_HOST;

// Repo root (two levels above app/desktop). The in-app plugin authoring
// guide imports the routing crate's docs with `?raw` (kata bprp), which live
// OUTSIDE the Vite root — `vite dev` only serves files under `server.fs.allow`,
// so the guide's raw imports need the repo root allowed (rollup/`vite build`
// resolves relative imports regardless).
const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    host: host || false,
    port: 1420,
    strictPort: true,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
    fs: {
      allow: [repoRoot],
    },
  },
});
