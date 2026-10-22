// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import "../app.css";
// 手机外壳样式单独一个文件：只在 [data-tepub-layout="phone"] 下有规则，
// 桌面首屏不会被它多拖累一点，也不会被媒体查询误伤。
import "../mobile.css";

export const ssr = false;
