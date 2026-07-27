import { PlatformUnsupportedError, type PlatformAdapter, type PlatformWindowHandle } from "./types";
import { getWebAccountSession } from "$lib/webAccount";

function getWebApiBaseUrl(): string {
  const env = import.meta.env.PUBLIC_TEPUB_API_BASE;
  return String(env || "/api").replace(/\/+$/, "");
}

function webAuthHeaders(): Record<string, string> {
  const session = getWebAccountSession();
  return session?.token && !session.localOnly ? { authorization: `Bearer ${session.token}` } : {};
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
    let message = detail;
    try {
      const parsed = JSON.parse(detail);
      message = parsed?.error || parsed?.message || detail;
    } catch {
      // Keep the raw response text when it is not JSON.
    }
    throw new Error(message || `Request failed with ${response.status}`);
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
        headers: webAuthHeaders(),
        body: JSON.stringify(args),
      });
    },

    async listen() {
      // Web 端没有 Tauri 事件总线，后端推送事件（如解压进度阶段）不会到达
      // 浏览器。监听器都是纯通知消费者（更新 loading 文案等），事件永不触发
      // 没有副作用，因此这里安全地返回 no-op unlisten。
      return () => {};
    },

    async getFileSrcConverter() {
      // 浏览器无法把本地文件系统路径映射为可加载 URL（无 asset 协议）。
      // 返回恒等转换：调用方 fetch 该"URL"会快速失败，走它们自己的回退路径
      // （如 reader 回退到 platform.invoke 的文本读取）。
      return (filePath: string) => filePath;
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

    async readFile(path) {
      if (/^https?:\/\//i.test(path) || path.startsWith("/")) {
        const response = await fetch(path);
        if (!response.ok) throw new Error(`Request failed with ${response.status}`);
        return new Uint8Array(await response.arrayBuffer());
      }
      throw new PlatformUnsupportedError("Direct filesystem read", "web");
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
