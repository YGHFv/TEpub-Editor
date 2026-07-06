import { createTauriPlatform } from "./tauri";
import { createWebPlatform } from "./web";
import type { PlatformAdapter, PlatformKind } from "./types";

declare const __TEPUB_TARGET__: PlatformKind | undefined;

function hasTauriRuntime(): boolean {
  if (typeof window === "undefined") return false;
  const maybeWindow = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(maybeWindow.__TAURI_INTERNALS__ || maybeWindow.__TAURI__);
}

function resolvePlatformKind(): PlatformKind {
  if (typeof __TEPUB_TARGET__ !== "undefined") return __TEPUB_TARGET__;
  return hasTauriRuntime() ? "tauri" : "web";
}

export const platform: PlatformAdapter =
  resolvePlatformKind() === "web" ? createWebPlatform() : createTauriPlatform();

export type { PlatformAdapter, PlatformKind } from "./types";
export { PlatformUnsupportedError } from "./types";
