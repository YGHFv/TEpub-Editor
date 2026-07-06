import { PlatformUnsupportedError, type PlatformAdapter, type PlatformWindowHandle } from "./types";

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

function createBrowserWindowHandle(target: Window | null = null): PlatformWindowHandle {
  return {
    label: null,
    async show() {
      target?.focus();
    },
    async setFocus() {
      target?.focus();
    },
    async hide() {},
    async close() {
      target?.close();
    },
    async destroy() {
      target?.close();
    },
    async once() {
      return () => {};
    },
  };
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

    async saveDialog(options = {}) {
      const { defaultPath } = options as { defaultPath?: string };
      return defaultPath || "download";
    },

    async message(text) {
      if (typeof window !== "undefined") window.alert(text);
    },

    async ask(text) {
      if (typeof window === "undefined") return false;
      return window.confirm(text);
    },

    async writeFile(path, data) {
      if (typeof window === "undefined" || typeof document === "undefined") return;
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      const blob = new Blob([bytes], { type: "application/octet-stream" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = path.split(/[\\/]/).pop() || "download";
      document.body.appendChild(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
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

    async getWindowByLabel() {
      return null;
    },

    getCurrentWindow() {
      return createBrowserWindowHandle(typeof window === "undefined" ? null : window);
    },

    async onCurrentWindowCloseRequested() {
      return () => {};
    },

    async closeCurrentWindow() {
      if (typeof window !== "undefined") window.close();
    },

    async createWebviewWindow(_label, url) {
      const target = typeof window === "undefined" ? null : window.open(url, "_blank", "noopener,noreferrer");
      return createBrowserWindowHandle(target);
    },
  };
}
