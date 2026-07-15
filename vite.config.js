import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// Vite configuration runs in Node.js.
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async ({ mode }) => {
  const isWebMode = mode === "web" || process.env.TEPUB_TARGET === "web";

  return {
  plugins: [sveltekit()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: isWebMode ? 5233 : 1420,
    strictPort: !isWebMode,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    chunkSizeWarningLimit: 900,
    rollupOptions: {
      output: {
        manualChunks(/** @type {string} */ id) {
          if (typeof id !== "string" || !id.includes("node_modules")) return;
          if (id.includes("@codemirror") || id.includes("codemirror")) {
            return "codemirror";
          }
          if (id.includes("opencc-js")) {
            return "opencc";
          }
        },
      },
    },
  },
  define: {
    __TEPUB_TARGET__: JSON.stringify(isWebMode ? "web" : "tauri"),
  },
  };
});
