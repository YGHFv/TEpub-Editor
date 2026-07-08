import { platform } from "$lib/platform";

const MOBILE_UA_PATTERN =
  /Android|webOS|iPhone|iPod|BlackBerry|IEMobile|Opera Mini|Mobile/i;

function queryViewOverride() {
  if (typeof window === "undefined") return "";
  return new URLSearchParams(window.location.search).get("view")?.toLowerCase() ?? "";
}

export function isMobileUserAgent() {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  const iPadLikeMac = navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1;
  return MOBILE_UA_PATTERN.test(ua) || iPadLikeMac;
}

export function isWebMobileClient() {
  if (!platform.isWeb || typeof window === "undefined") return false;
  const view = queryViewOverride();
  if (view === "desktop") return false;
  if (view === "mobile") return true;

  const coarsePointer = window.matchMedia?.("(pointer: coarse)")?.matches ?? false;
  const narrowViewport = window.matchMedia?.("(max-width: 760px)")?.matches ?? false;
  return isMobileUserAgent() || narrowViewport || coarsePointer;
}

export function applyClientProfile() {
  if (typeof document === "undefined") return () => {};
  const root = document.documentElement;
  const apply = () => {
    const client = platform.isWeb ? (isWebMobileClient() ? "web-mobile" : "web-desktop") : "tauri";
    root.dataset.tepubClient = client;
  };

  apply();
  window.addEventListener("resize", apply);
  window.addEventListener("orientationchange", apply);
  window.addEventListener("popstate", apply);
  return () => {
    window.removeEventListener("resize", apply);
    window.removeEventListener("orientationchange", apply);
    window.removeEventListener("popstate", apply);
  };
}
