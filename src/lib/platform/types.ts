export type PlatformKind = "tauri" | "web";

export type PlatformUnlisten = () => void;
export type PlatformCloseEvent = {
  preventDefault(): void;
};

// 后端 → 前端事件（Tauri event bus 的最小结构子集）。
export type PlatformEvent<T> = {
  event: string;
  payload: T;
};

// 把本地文件路径转换为 webview 可加载 URL 的同步函数（Tauri convertFileSrc 的
// 抽象）。以"异步取同步转换器"的形式暴露：调用方常在同步字符串替换回调
// （如 CSS url() 改写）里逐次转换，无法逐次 await。
export type FileSrcConverter = (filePath: string, protocol?: string) => string;

export interface PlatformWindowHandle {
  label: string | null;
  show(): Promise<void>;
  setFocus(): Promise<void>;
  hide(): Promise<void>;
  close(): Promise<void>;
  destroy(): Promise<void>;
  once(event: string, handler: () => void | Promise<void>): Promise<PlatformUnlisten>;
}

export interface PlatformAdapter {
  kind: PlatformKind;
  isTauri: boolean;
  isWeb: boolean;
  apiBaseUrl: string;

  invoke<T = unknown>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T = unknown>(event: string, handler: (event: PlatformEvent<T>) => void): Promise<PlatformUnlisten>;
  getFileSrcConverter(): Promise<FileSrcConverter>;
  openDialog<T = string | string[] | null>(options?: Record<string, unknown>): Promise<T>;
  saveDialog(options?: Record<string, unknown>): Promise<string | null>;
  message(text: string, options?: Record<string, unknown>): Promise<void>;
  ask(text: string, options?: Record<string, unknown>): Promise<boolean>;
  readFile(path: string): Promise<Uint8Array>;
  writeFile(path: string, data: Uint8Array | number[]): Promise<void>;
  revealPath(path: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  getCurrentWindowLabel(): Promise<string | null>;
  getWindowByLabel(label: string): Promise<PlatformWindowHandle | null>;
  getCurrentWindow(): PlatformWindowHandle;
  onCurrentWindowCloseRequested(handler: (event: PlatformCloseEvent) => void | Promise<void>): Promise<PlatformUnlisten>;
  // 系统级文件拖放：仅在文件真正落下（drop）时回调本地路径列表。
  onCurrentWindowFileDrop(handler: (paths: string[]) => void): Promise<PlatformUnlisten>;
  setCurrentWindowTitle(title: string): Promise<void>;
  closeCurrentWindow(): Promise<void>;
  createWebviewWindow(label: string, url: string, options?: Record<string, unknown>): Promise<PlatformWindowHandle>;
}

export class PlatformUnsupportedError extends Error {
  constructor(feature: string, platform: PlatformKind) {
    super(`${feature} is not available on ${platform}`);
    this.name = "PlatformUnsupportedError";
  }
}
