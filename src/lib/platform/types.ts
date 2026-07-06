export type PlatformKind = "tauri" | "web";

export type PlatformUnlisten = () => void;
export type PlatformCloseEvent = {
  preventDefault(): void;
};

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
  openDialog<T = string | string[] | null>(options?: Record<string, unknown>): Promise<T>;
  saveDialog(options?: Record<string, unknown>): Promise<string | null>;
  message(text: string, options?: Record<string, unknown>): Promise<void>;
  ask(text: string, options?: Record<string, unknown>): Promise<boolean>;
  writeFile(path: string, data: Uint8Array | number[]): Promise<void>;
  revealPath(path: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  getCurrentWindowLabel(): Promise<string | null>;
  getWindowByLabel(label: string): Promise<PlatformWindowHandle | null>;
  getCurrentWindow(): PlatformWindowHandle;
  onCurrentWindowCloseRequested(handler: (event: PlatformCloseEvent) => void | Promise<void>): Promise<PlatformUnlisten>;
  closeCurrentWindow(): Promise<void>;
  createWebviewWindow(label: string, url: string, options?: Record<string, unknown>): Promise<PlatformWindowHandle>;
}

export class PlatformUnsupportedError extends Error {
  constructor(feature: string, platform: PlatformKind) {
    super(`${feature} is not available on ${platform}`);
    this.name = "PlatformUnsupportedError";
  }
}
