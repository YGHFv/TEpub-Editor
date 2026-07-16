import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// Vite configuration runs in Node.js.
const host = process.env.TAURI_DEV_HOST;

const optimizedDependencies = [
  "@codemirror/commands",
  "@codemirror/lang-css",
  "@codemirror/lang-html",
  "@codemirror/lang-xml",
  "@codemirror/language",
  "@codemirror/search",
  "@codemirror/state",
  "@codemirror/theme-one-dark",
  "@codemirror/view",
  "@tauri-apps/api/core",
  "@tauri-apps/api/event",
  "@tauri-apps/api/path",
  "@tauri-apps/api/webviewWindow",
  "@tauri-apps/api/window",
  "@tauri-apps/plugin-dialog",
  "@tauri-apps/plugin-fs",
  "@tauri-apps/plugin-opener",
  "codemirror",
  "fonteditor-core",
  "jszip",
  "opencc-js",
  "pako",
  "pinyin-pro",
];

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
      // 3. generated native and static build output must not reload the Web dev page
      ignored: ["**/src-tauri/**", "**/build/**"],
    },
  },
  optimizeDeps: {
    // Scan every toolbox dependency up front instead of reloading once per first-used route.
    include: optimizedDependencies,
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
