import { PlatformUnsupportedError, type PlatformAdapter } from "./types";

function getWebApiBaseUrl(): string {
  const env = import.meta.env.PUBLIC_TEPUB_API_BASE;
  return String(env || "/api").replace(/\/+$/, "");
}

async function requestJson<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${getWebApiBaseUrl()}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init.headers || {}),
    },
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => "");
    throw new Error(detail || `Request failed with ${response.status}`);
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export function createWebPlatform(): PlatformAdapter {
  return {
    kind: "web",
    isTauri: false,
    isWeb: true,
    apiBaseUrl: getWebApiBaseUrl(),

    async invoke<T = unknown>(command: string, args: Record<string, unknown> = {}) {
      return requestJson<T>(`/commands/${encodeURIComponent(command)}`, {
        method: "POST",
        body: JSON.stringify(args),
      });
    },

    async openDialog() {
      throw new PlatformUnsupportedError("Native file picker", "web");
    },

    async saveDialog() {
      throw new PlatformUnsupportedError("Native save dialog", "web");
    },

    async message(text) {
      if (typeof window !== "undefined") window.alert(text);
    },

    async ask(text) {
      if (typeof window === "undefined") return false;
      return window.confirm(text);
    },

    async writeFile() {
      throw new PlatformUnsupportedError("Direct filesystem write", "web");
    },

    async revealPath() {
      throw new PlatformUnsupportedError("Reveal in system file manager", "web");
    },

    async openExternal(url) {
      if (typeof window !== "undefined") window.open(url, "_blank", "noopener,noreferrer");
    },

    async getCurrentWindowLabel() {
      return null;
    },

    async closeCurrentWindow() {
      if (typeof window !== "undefined") window.close();
    },

    async createWebviewWindow(_label, url) {
      if (typeof window !== "undefined") window.open(url, "_blank", "noopener,noreferrer");
    },
  };
}
