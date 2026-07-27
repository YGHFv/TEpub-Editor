import type { PlatformAdapter, PlatformWindowHandle } from "./types";

function wrapTauriWindow(win: any): PlatformWindowHandle {
  return {
    label: typeof win.label === "string" ? win.label : null,
    async show() {
      await win.show?.();
    },
    async setFocus() {
      await win.setFocus?.();
    },
    async hide() {
      await win.hide?.();
    },
    async close() {
      await win.close?.();
    },
    async destroy() {
      await (win.destroy ? win.destroy() : win.close?.());
    },
    async once(event, handler) {
      if (!win.once) return () => {};
      return await win.once(event, handler);
    },
    async listen(event, handler) {
      if (!win.listen) return () => {};
      return await win.listen(event, handler);
    },
  };
}

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

    async listen(event, handler) {
      const { listen } = await import("@tauri-apps/api/event");
      // Tauri 的 Event<T>（含 event / payload / id）结构上兼容 PlatformEvent<T>，
      // 直接透传 handler。
      return await listen(event, handler);
    },

    async emit(event, payload) {
      const { emit } = await import("@tauri-apps/api/event");
      await emit(event, payload);
    },

    async getFileSrcConverter() {
      const { convertFileSrc } = await import("@tauri-apps/api/core");
      return convertFileSrc;
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

    async readFile(path) {
      const { readFile } = await import("@tauri-apps/plugin-fs");
      return await readFile(path);
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

    async getWindowByLabel(label) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const win = await WebviewWindow.getByLabel(label);
      return win ? wrapTauriWindow(win) : null;
    },

    getCurrentWindow() {
      const lazyWindow = {
        get value() {
          return import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow());
        },
      };
      return {
        label: null,
        async show() {
          await (await lazyWindow.value).show();
        },
        async setFocus() {
          await (await lazyWindow.value).setFocus();
        },
        async hide() {
          await (await lazyWindow.value).hide();
        },
        async close() {
          await (await lazyWindow.value).close();
        },
        async destroy() {
          const win = await lazyWindow.value;
          await (win.destroy ? win.destroy() : win.close());
        },
        async once(event, handler) {
          return await (await lazyWindow.value).once(event, handler);
        },
        async listen(event, handler) {
          return await (await lazyWindow.value).listen(event, handler);
        },
      };
    },

    async onCurrentWindowCloseRequested(handler) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      return await getCurrentWindow().onCloseRequested(handler);
    },

    async onCurrentWindowFileDrop(handler) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      return await getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === "drop") handler(event.payload.paths);
      });
    },

    async setCurrentWindowTitle(title) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setTitle(title);
    },

    async setCurrentWindowPosition(x, y) {
      const { getCurrentWindow, LogicalPosition } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setPosition(new LogicalPosition(x, y));
    },

    async setCurrentWindowSize(width, height) {
      const { getCurrentWindow, LogicalSize } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setSize(new LogicalSize(width, height));
    },

    async onCurrentWindowMoved(handler) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      return await win.listen("tauri://move", async () => {
        try {
          const pos = await win.outerPosition();
          handler({ x: pos.x, y: pos.y });
        } catch {
          // 窗口销毁瞬间 outerPosition 可能失败：静默跳过，与原调用点
          // （editor 页 try/catch 包裹）语义一致。
        }
      });
    },

    async closeCurrentWindow() {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    },

    async createWebviewWindow(label, url, options = {}) {
      const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      return wrapTauriWindow(new WebviewWindow(label, { url, ...(options as any) }));
    },
  };
}
