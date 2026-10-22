import {
  PlatformUnsupportedError,
  type PlatformAdapter,
  type PlatformWindowHandle,
} from "./types";
import { getWebAccountSession } from "$lib/webAccount";
import { describeCommandFailure } from "./webErrors";

function getWebApiBaseUrl(): string {
  const env = import.meta.env.PUBLIC_TEPUB_API_BASE;
  return String(env || "/api").replace(/\/+$/, "");
}

function webAuthHeaders(): Record<string, string> {
  const session = getWebAccountSession();
  return session?.token && !session.localOnly
    ? { authorization: `Bearer ${session.token}` }
    : {};
}

async function requestJson<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const response = await fetch(`${getWebApiBaseUrl()}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init.headers || {}),
    },
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => "");
    throw new Error(
      describeCommandFailure(
        path.replace(/^\/commands\//, ""),
        response.status,
        detail,
      ),
    );
  }

  if (response.status === 204) return undefined as T;

  // 成功分支也必须防一手：把不存在的 /api/* 用 200 重写回 index.html 的
  // 静态托管（Cloudflare Pages 的 SPA 回退、Netlify `/* /index.html 200`、
  // nginx `try_files … /index.html`）会给出 2xx + 整页 HTML。直接
  // response.json() 会抛 `Unexpected token '<', "<!doctype html>…"`，
  // 页面把这句话原样拼进 toast，用户看到的是一条浏览器解析错误。
  const body = await response.text();
  try {
    return JSON.parse(body) as T;
  } catch {
    throw new Error(
      describeCommandFailure(
        path.replace(/^\/commands\//, ""),
        response.status,
        body,
      ),
    );
  }
}

function createBrowserWindowHandle(
  target: Window | null = null,
): PlatformWindowHandle {
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
    async listen() {
      // 浏览器窗口句柄没有跨窗口事件通道（window.open 的标签页与本页无事件
      // 总线）。消费方均为纯通知回调（如主窗口等待"restore-main-window"后
      // show/setFocus），事件永不触发没有副作用，返回 no-op unlisten。
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

    async invoke<T = unknown>(
      command: string,
      args: Record<string, unknown> = {},
    ) {
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

    async emit() {
      // Web 端没有跨窗口事件总线。emit 的消费者都是其他 Tauri 窗口
      // （search-replace 窗口的命中计数、epub-metadata 窗口的初始化数据），
      // 这些窗口在 web 端要么不存在、要么是无法互通的独立标签页。
      // 事件丢失只影响跨窗口通知展示，不改变本页状态，no-op 无副作用。
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

    async confirm(text) {
      // 浏览器原生双钮对话框只有 window.confirm，其"确定/取消"按钮语义
      // 与桌面 plugin-dialog confirm（OK/Cancel）一致，是真实等价降级。
      if (typeof window === "undefined") return false;
      return window.confirm(text);
    },

    async readFile(path) {
      if (/^https?:\/\//i.test(path) || path.startsWith("/")) {
        const response = await fetch(path);
        if (!response.ok)
          throw new Error(`Request failed with ${response.status}`);
        return new Uint8Array(await response.arrayBuffer());
      }
      throw new PlatformUnsupportedError("Direct filesystem read", "web");
    },

    async writeFile(path, data) {
      if (typeof window === "undefined" || typeof document === "undefined")
        return;
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      const blob = new Blob([bytes], { type: "application/octet-stream" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = path.split(/[\\/]/).pop() || "download";
      document.body.appendChild(link);
      link.click();
      link.remove();
      // 推迟释放：移动端 WebView 里下载是异步接管的，同步 revoke 之后
      // 文件还没开始读，URL 就已经失效（表现为"点了保存没反应"）。
      setTimeout(() => URL.revokeObjectURL(url), 30_000);
    },

    async revealPath() {
      throw new PlatformUnsupportedError(
        "Reveal in system file manager",
        "web",
      );
    },

    async openExternal(url) {
      if (typeof window !== "undefined")
        window.open(url, "_blank", "noopener,noreferrer");
    },

    async getCurrentWindowLabel() {
      return null;
    },

    async getWindowByLabel() {
      return null;
    },

    getCurrentWindow() {
      return createBrowserWindowHandle(
        typeof window === "undefined" ? null : window,
      );
    },

    async onCurrentWindowCloseRequested() {
      return () => {};
    },

    async onCurrentWindowFileDrop() {
      // 浏览器沙箱拿不到拖入文件的本地路径（DataTransferItem 只给 File 内容），
      // 而消费方（书库入库）需要把"本地路径"交给后端命令处理，语义在 web 端
      // 不成立。监听器永不触发没有副作用（HTML5 拖放仍由页面自身的
      // dragover/drop 处理器兜底），返回 no-op unlisten。
      return () => {};
    },

    async setCurrentWindowTitle(title) {
      // 浏览器等价物：改标签页标题。
      if (typeof document !== "undefined") document.title = title;
    },

    async setCurrentWindowPosition() {
      // 浏览器标签页无法移动宿主窗口（window.moveTo 仅对脚本自开窗口生效，
      // 且标签页没有"窗口位置"语义）。调用方只做位置恢复这一增强体验，
      // 跳过不影响任何后续逻辑，no-op 无副作用。
    },

    async setCurrentWindowSize() {
      // 同上：浏览器标签页无法调整宿主窗口尺寸（window.resizeTo 限制同
      // setCurrentWindowPosition），布局由浏览器窗口自身决定，no-op 无副作用。
    },

    async onCurrentWindowMoved() {
      // 浏览器没有窗口移动事件；该回调仅用于把位置持久化到 localStorage，
      // 而 web 端位置恢复（setCurrentWindowPosition）本身就是 no-op，
      // 不保存也不会产生行为差异，返回 no-op unlisten。
      return () => {};
    },

    async closeCurrentWindow() {
      if (typeof window !== "undefined") window.close();
    },

    // 浏览器没有原生窗口边框：这些调用没有对应语义，返回 no-op 让调用方
    // 不必到处判断运行时（自绘标题行在 web 端不画窗口按钮）
    async minimizeCurrentWindow() {},
    async toggleMaximizeCurrentWindow() {},
    async isCurrentWindowMaximized() {
      return false;
    },
    async onCurrentWindowResized() {
      return () => {};
    },

    async createWebviewWindow(_label, url) {
      const target =
        typeof window === "undefined"
          ? null
          : window.open(url, "_blank", "noopener,noreferrer");
      return createBrowserWindowHandle(target);
    },
  };
}
