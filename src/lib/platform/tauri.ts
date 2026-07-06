import type { PlatformAdapter } from "./types";

export function createTauriPlatform(): PlatformAdapter {
  return {
    kind: "tauri",
    isTauri: true,
    isWeb: false,
    apiBaseUrl: "",

    async invoke<T = unknown>(command: string, args: Record<string, unknown> = {}) {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<T>(command, args);
    },

    async openDialog<T>(options = {}) {
      const { open } = await import("@tauri-apps/plugin-dialog");
      return open(options as any) as Promise<T>;
    },

    async saveDialog(options = {}) {
      const { save } = await import("@tauri-apps/plugin-dialog");
      return save(options as any);
    },

    async message(text, options = {}) {
      const { message } = await import("@tauri-apps/plugin-dialog");
      await message(text, options as any);
    },

    async ask(text, options = {}) {
      const { ask } = await import("@tauri-apps/plugin-dialog");
      return ask(text, options as any);
    },

    async writeFile(path, data) {
      const { writeFile } = await import("@tauri-apps/plugin-fs");
      const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
      await writeFile(path, bytes);
    },

    async revealPath(path) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("reveal_in_explorer", { path });
    },

    async openExternal(url) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    },

    async getCurrentWindowLabel() {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      return getCurrentWindow().label;
    },

    async closeCurrentWindow() {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    },

    async createWebviewWindow(label, url, options = {}) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      new WebviewWindow(label, { url, ...(options as any) });
    },
  };
}
