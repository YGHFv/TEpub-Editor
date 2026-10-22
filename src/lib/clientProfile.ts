import { writable, get } from "svelte/store";
import { platform } from "$lib/platform";
import { isMobileUserAgent } from "./deviceProfile";
import { readAndroidSafeAreaInsets } from "./mobile/safeArea";
import { WEB_VIEW_OVERRIDE_KEY } from "./storageKeys";

// UA 判定只有一份实现，放在 deviceProfile.ts（零依赖，windowChrome 也要用）。
// 这里再导出一次，是为了让既有的 `from "$lib/clientProfile"` 导入继续可用。
export { isMobileUserAgent };

/**
 * 客户端形态判定 —— 全应用唯一检测源。
 *
 * 历史包袱：以前这里只有 `platform.isWeb` 分支，所以 Tauri Android 的
 * `data-tepub-client` 永远是 `tauri`，所有 `web-mobile` 的 CSS 在 App 里
 * 全是死代码。现在把「手机不手机」和「web 还是 tauri」拆成两个维度：
 *
 *   <html data-tepub-layout="phone|desktop">   是否按手机布局渲染（新的 CSS 只认它）
 *   <html data-tepub-client="web-mobile|web-desktop|tauri">  旧钩子，值域不变
 *   <html data-tepub-runtime="web|tauri">       运行时，需要区分「能否用原生能力」时用
 *
 * 新写的手机样式一律挂 `[data-tepub-layout="phone"]`，不要再用
 * `data-tepub-client="web-mobile"`（安卓永远拿不到那个值）。
 */

const CLIENT_PROFILE_EVENT = "tepub-client-profile-updated";
const DEFAULT_VIEWPORT_CONTENT = "width=device-width, initial-scale=1";
const DESKTOP_VIEWPORT_CONTENT = "width=1200, initial-scale=1";

/**
 * 唯一的数值断点，且只被检测函数使用一次。
 * 手机样式不要自己写 `@media (max-width: …)`——横屏手机宽 844px 会整块失效。
 */
export const PHONE_MAX_WIDTH = 760;

export type ClientLayout = "phone" | "desktop";
export type ClientRuntime = "web" | "tauri";
export type WebClientViewOverride = "desktop" | "mobile" | "";

export type ClientSignal = {
  /** UA 看起来像手机/平板 */
  mobileUA: boolean;
  /** 布局视口宽度（CSS px） */
  width: number;
  /** 用户/查询串显式覆盖 */
  override: WebClientViewOverride;
  /** 运行时：桌面 Tauri 窗口的宽度与手机无关，不能拿宽度判形态 */
  runtime: ClientRuntime;
};

/**
 * 纯函数，可单测。优先级：显式覆盖 > 移动 UA > 窄视口（仅 web） > 桌面。
 *
 * 两个容易踩的点：
 * 1. 不再把 `pointer: coarse` 单独当成手机信号：触屏笔记本会被误判，
 *    而真正的手机会同时命中 UA 或窄视口。
 * 2. 宽度信号只在 web 下参与判定。桌面 Tauri 有一堆小窗（查找替换 560×360、
 *    EPUB 元数据 540×620），它们窄但绝不是手机；以前它们拿不到 web-mobile，
 *    现在若按宽度判就会突然套上手机密度（44px 控件、16px 输入框）。
 */
export function resolveClientLayout({
  mobileUA,
  width,
  override,
  runtime,
}: ClientSignal): ClientLayout {
  if (override === "mobile") return "phone";
  if (override === "desktop") return "desktop";
  if (mobileUA) return "phone";
  if (runtime === "web" && Number.isFinite(width) && width > 0 && width <= PHONE_MAX_WIDTH)
    return "phone";
  return "desktop";
}

function queryViewOverride(): WebClientViewOverride {
  if (typeof window === "undefined") return "";
  const value = new URLSearchParams(window.location.search)
    .get("view")
    ?.toLowerCase();
  return value === "desktop" || value === "mobile" ? value : "";
}

export function getWebClientViewOverride(): WebClientViewOverride {
  if (typeof localStorage === "undefined") return "";
  const value = localStorage.getItem(WEB_VIEW_OVERRIDE_KEY);
  return value === "desktop" || value === "mobile" ? value : "";
}

export function setWebClientViewOverride(view: WebClientViewOverride) {
  if (typeof localStorage === "undefined") return;
  if (view) localStorage.setItem(WEB_VIEW_OVERRIDE_KEY, view);
  else localStorage.removeItem(WEB_VIEW_OVERRIDE_KEY);
  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent(CLIENT_PROFILE_EVENT, { detail: view }),
    );
  }
}

function resolvedViewOverride(): WebClientViewOverride {
  const queryView = queryViewOverride();
  if (queryView) return queryView;
  return getWebClientViewOverride();
}

/** 当前环境信号（浏览器/WebView 都可用） */
export function readClientSignal(): ClientSignal {
  return {
    mobileUA: isMobileUserAgent(),
    width: typeof window === "undefined" ? 0 : window.innerWidth || 0,
    override: resolvedViewOverride(),
    runtime: platform.isWeb ? "web" : "tauri",
  };
}

/** 首帧值：app.html 的内联脚本会在样式表之前写好这个属性 */
function initialLayout(): ClientLayout {
  if (typeof document === "undefined") return "desktop";
  return document.documentElement.dataset.tepubLayout === "phone"
    ? "phone"
    : "desktop";
}

/** 响应式镜像，组件里 `$clientLayout` 即可跟着变 */
export const clientLayout = writable<ClientLayout>(initialLayout());

/**
 * 是否「真机手机/平板」（只看设备，不看窗口宽度）。
 *
 * 外壳（要不要收掉顶部标签条、换底部导航）用它，而不是 $clientLayout：
 * 桌面浏览器被临时拖到 700px 时 layout 也会是 phone（沿用旧 web-mobile 语义，
 * 页面 CSS 照旧响应），但那还是"一台电脑"，不该把它的标签条换成手机底部导航。
 * 只有 UA 判定为移动设备、或用户显式 ?view=mobile / 存了 mobile 覆盖，才切手机外壳。
 */
export function resolvePhoneDevice(signal: ClientSignal): boolean {
  if (signal.override === "desktop") return false;
  if (signal.override === "mobile") return true;
  return signal.mobileUA;
}

export const phoneDevice = writable<boolean>(
  typeof window === "undefined"
    ? false
    : resolvePhoneDevice(readClientSignal()),
);

/**
 * 安卓壳没有把系统状态栏高度告诉 WebView：`env(safe-area-inset-top)` 在
 * edge-to-edge 的 Android WebView 里通常只反映刘海裁切，竖屏状态栏那 24-28dp
 * 拿不到。没有这条兜底，首屏内容会被状态栏压住。
 *
 * app.html 的内联脚本已经按同一条件写过一次（首帧就要生效），这里给的是
 * 挂载后的同一份保证：万一内联脚本被 CSP 或存储异常打断，也不会漏。
 */
function applySafeAreaFallback(signal: ClientSignal) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (signal.runtime === "tauri" && signal.mobileUA) {
    if (!readAndroidSafeAreaInsets()) {
      root.style.setProperty("--safe-top-min", "24px");
      root.style.setProperty("--safe-bottom-min", "0px");
      root.style.setProperty("--safe-left-min", "0px");
      root.style.setProperty("--safe-right-min", "0px");
    }
  } else {
    root.style.removeProperty("--safe-top-min");
    root.style.removeProperty("--safe-bottom-min");
    root.style.removeProperty("--safe-left-min");
    root.style.removeProperty("--safe-right-min");
  }
}

/** 组件里判断手机布局（同步读，首帧就对） */
export function isPhoneLayout(): boolean {
  return get(clientLayout) === "phone";
}

/** 兼容旧 API：web 端的「手机」= 手机布局 */
export function isWebMobileClient() {
  return platform.isWeb && isPhoneLayout();
}

export function applyClientProfile() {
  if (typeof document === "undefined") return () => {};
  const root = document.documentElement;
  const viewportMeta = document.querySelector<HTMLMetaElement>(
    'meta[name="viewport"]',
  );
  const originalViewport =
    viewportMeta?.getAttribute("content") || DEFAULT_VIEWPORT_CONTENT;

  const apply = () => {
    const signal = readClientSignal();
    const layout = resolveClientLayout(signal);
    const runtime: ClientRuntime = signal.runtime;
    root.dataset.tepubLayout = layout;
    root.dataset.tepubRuntime = runtime;
    // 旧钩子：值域保持三值，免得 30 多处存量 CSS 一起失效。
    // 页面迁移时逐个把这些块改挂 data-tepub-layout。
    root.dataset.tepubClient = platform.isWeb
      ? layout === "phone"
        ? "web-mobile"
        : "web-desktop"
      : "tauri";
    clientLayout.set(layout);
    phoneDevice.set(resolvePhoneDevice(signal));
    applySafeAreaFallback(signal);
    if (viewportMeta) {
      viewportMeta.setAttribute(
        "content",
        platform.isWeb && resolvedViewOverride() === "desktop"
          ? DESKTOP_VIEWPORT_CONTENT
          : originalViewport,
      );
    }
  };

  apply();
  window.addEventListener("resize", apply);
  window.addEventListener("orientationchange", apply);
  window.addEventListener("popstate", apply);
  window.addEventListener(CLIENT_PROFILE_EVENT, apply);
  window.addEventListener("storage", apply);
  return () => {
    window.removeEventListener("resize", apply);
    window.removeEventListener("orientationchange", apply);
    window.removeEventListener("popstate", apply);
    window.removeEventListener(CLIENT_PROFILE_EVENT, apply);
    window.removeEventListener("storage", apply);
  };
}
