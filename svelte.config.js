// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { i18nMarkupPreprocess } from "./scripts/i18n-markup-preprocess.js";

const base = process.env.TEPUB_BASE_PATH || "";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: [vitePreprocess(), i18nMarkupPreprocess()],
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    paths: {
      base,
    },
  },
};

export default config;
