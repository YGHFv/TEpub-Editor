import { platform } from "$lib/platform";

const MOBILE_UA_PATTERN =
  /Android|webOS|iPhone|iPod|BlackBerry|IEMobile|Opera Mini|Mobile/i;
const WEB_VIEW_OVERRIDE_KEY = "tepub-web-view-mode";
const CLIENT_PROFILE_EVENT = "tepub-client-profile-updated";
const DEFAULT_VIEWPORT_CONTENT = "width=device-width, initial-scale=1";
const DESKTOP_VIEWPORT_CONTENT = "width=1200, initial-scale=1";

export type WebClientViewOverride = "desktop" | "mobile" | "";

function queryViewOverride() {
  if (typeof window === "undefined") return "";
  return new URLSearchParams(window.location.search).get("view")?.toLowerCase() ?? "";
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
    window.dispatchEvent(new CustomEvent(CLIENT_PROFILE_EVENT, { detail: view }));
  }
}

function resolvedViewOverride() {
  const queryView = queryViewOverride();
  if (queryView === "desktop" || queryView === "mobile") return queryView;
  return getWebClientViewOverride();
}

export function isMobileUserAgent() {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  const iPadLikeMac = navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1;
  return MOBILE_UA_PATTERN.test(ua) || iPadLikeMac;
}

export function isWebMobileClient() {
  if (!platform.isWeb || typeof window === "undefined") return false;
  const view = resolvedViewOverride();
  if (view === "desktop") return false;
  if (view === "mobile") return true;

  const coarsePointer = window.matchMedia?.("(pointer: coarse)")?.matches ?? false;
  const narrowViewport = window.matchMedia?.("(max-width: 760px)")?.matches ?? false;
  return isMobileUserAgent() || narrowViewport || coarsePointer;
}

export function applyClientProfile() {
  if (typeof document === "undefined") return () => {};
  const root = document.documentElement;
  const viewportMeta = document.querySelector<HTMLMetaElement>('meta[name="viewport"]');
  const originalViewport = viewportMeta?.getAttribute("content") || DEFAULT_VIEWPORT_CONTENT;

  const apply = () => {
    const client = platform.isWeb ? (isWebMobileClient() ? "web-mobile" : "web-desktop") : "tauri";
    root.dataset.tepubClient = client;
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
