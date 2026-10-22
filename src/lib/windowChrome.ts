/**
 * 自绘标题行的三种形态。
 *
 * - none：浏览器里没有可自绘的窗口边框，页面不画窗口按钮
 * - custom：Windows / Linux，decorations: false，最小化/最大化/关闭全部自己画
 * - mac-overlay：macOS 用 titleBarStyle: "Overlay"，红黄绿按钮仍由系统绘制并
 *   浮在页面左上角，所以只要留出左侧安全区，不能自己再画一套按钮
 */
import { isMobileUserAgent } from "./deviceProfile";

export type WindowChromeMode = "none" | "custom" | "mac-overlay";

function isTauriRuntime() {
  if (typeof window === "undefined") return false;
  const global = window as any;
  return Boolean(global.__TAURI__ || global.__TAURI_INTERNALS__);
}

export function isMacOS() {
  // 用 userAgent 而不是 navigator.platform：新版 Chrome 已冻结 platform，
  // 且 electron/wry 里 userAgent 更可靠
  const nav = typeof navigator === "undefined" ? null : navigator;
  if (!nav) return false;
  return /Macintosh|Mac OS X/i.test(nav.userAgent || "");
}

function isLinux() {
  const nav = typeof navigator === "undefined" ? null : navigator;
  if (!nav) return false;
  return /Linux/i.test(nav.userAgent || "") && !/Android/i.test(nav.userAgent);
}

/**
 * Android / iOS 的 Tauri 壳：只有一个全屏 Activity，没有可以最小化、
 * 最大化、关闭的窗口框。以前这里落到 "custom"，于是在手机上画出三个
 * 完全无效的桌面窗口按钮，还占掉 40px 高度，并被状态栏压住。
 */
export function isMobileRuntime() {
  return isMobileUserAgent();
}

export function detectWindowChrome(): WindowChromeMode {
  // 不依赖 $app/environment：这个函数在 +layout 顶层和 onMount 里都会调用，
  // 服务端渲染时 window 不存在，直接判 typeof 更省事
  if (typeof window === "undefined" || !isTauriRuntime()) return "none";
  // 手机壳没有窗口框（见上）
  if (isMobileRuntime()) return "none";
  if (isMacOS()) return "mac-overlay";
  // Linux：tao 在无边框窗口上没法拖动边缘缩放（Windows 靠 WM_NCHITTEST 才行），
  // 关掉系统边框会让窗口既不能改大小也没法最大化。所以 Linux 保留系统标题栏，
  // 页面这边只画一条标签栏（见 tauri.linux.conf.json）
  if (isLinux()) return "none";
  return "custom";
}

/** 当前窗口的 label（Tauri 注入的 metadata 是同步可读的，不用等 IPC） */
export function currentWindowLabel(): string | null {
  if (typeof window === "undefined") return null;
  const internals = (window as any).__TAURI_INTERNALS__;
  const label = internals?.metadata?.currentWindow?.label;
  return typeof label === "string" ? label : null;
}

/**
 * 工具型小窗：查找替换、EPUB 元数据。
 *
 * 这些窗口不是"应用主界面"，而是主界面弹出的一个面板：
 * - 不能画应用标题行（否则系统标题栏 + 自绘标题行两条，窗口内容被挤没）；
 * - 不能显示二级页面标签（点一下就会把面板导航走，主窗口的搜索桥接直接断掉）。
 *
 * 主界面的窗口（main / editor-* / reader-* / toolbox / batch-progress-* …）
 * 都是 decorations:false 自绘标题行，标签栏只在同窗口模式下出现。
 */
export function isUtilityWindowLabel(label: string | null): boolean {
  return label === "search-replace" || label === "epub-metadata";
}
