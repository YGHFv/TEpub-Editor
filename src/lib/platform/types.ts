export type PlatformKind = "tauri" | "web";

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
  closeCurrentWindow(): Promise<void>;
  createWebviewWindow(label: string, url: string, options?: Record<string, unknown>): Promise<void>;
}

export class PlatformUnsupportedError extends Error {
  constructor(feature: string, platform: PlatformKind) {
    super(`${feature} is not available on ${platform}`);
    this.name = "PlatformUnsupportedError";
  }
}
