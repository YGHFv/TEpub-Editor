import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// 以 package.json 为唯一真源注入版本号（与 Cargo.toml / tauri.conf.json 由
// release.yml 的版本一致性门禁保持一致）。用 import attributes 读 json 会
// 依赖 Node 版本特性，这里直接读文件更省事。
const version = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
).version;

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
      // 版本号从 package.json 注入，不要在各页面里手写：此前「关于」页写死
      // 0.6.1，之后每次升版都漏改，界面显示的版本和实际发的包对不上。
      __TEPUB_VERSION__: JSON.stringify(version),
    },
    test: {
      // Node by default; DOM-dependent suites opt in via `// @vitest-environment happy-dom`.
      environment: "node",
      include: ["src/**/*.test.ts"],
      coverage: {
        provider: "v8",
        reporter: ["text", "html"],
        include: ["src/lib/**/*.ts"],
        exclude: ["src/lib/**/*.test.ts", "src/lib/**/*.d.ts"],
      },
    },
  };
});
