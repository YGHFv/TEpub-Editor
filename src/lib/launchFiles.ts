import type { PlatformAdapter, PlatformUnlisten } from "./platform/types";
import { loadAppSettings } from "./appSettings";

export type LaunchInfo = {
  filePath?: string | null;
  filePaths?: string[];
  action?: string | null;
};

export function launchFileTargets(info: LaunchInfo, base = "") {
  const paths = [...new Set([...(info.filePaths ?? []), info.filePath ?? ""])];
  return paths.flatMap((path) => {
    const ext = path.split(".").pop()?.toLowerCase();
    if (ext !== "txt" && ext !== "epub") return [];
    const reader = ext === "epub" && info.action === "reader";
    const route = ext === "txt" ? "editor" : reader ? "reader" : "epub-editor";
    return [
      {
        path,
        isEpub: ext === "epub",
        route,
        url: `${base}/${route}?file=${encodeURIComponent(path)}${ext === "txt" ? "&fromLibrary=1" : ""}`,
        title: `TEpub-Editor-${ext === "txt" ? "TXT" : reader ? "Reader" : "EPUB"}`,
      },
    ];
  });
}

// The root layout survives tool/library navigation and is the only request consumer.
export async function listenForLaunchFiles(
  platform: PlatformAdapter,
  base = "",
): Promise<PlatformUnlisten> {
  if (
    !platform.isTauri ||
    (await platform.getCurrentWindowLabel()) !== "main"
  ) {
    return () => {};
  }

  let disposed = false;
  let draining = false;
  let pending = false;
  const mainWindow = platform.getCurrentWindow();

  async function drain() {
    pending = true;
    if (draining || disposed) return;
    draining = true;
    try {
      while (pending && !disposed) {
        pending = false;
        const requests = await platform.invoke<LaunchInfo[]>(
          "take_launch_requests",
        );
        for (const request of requests) {
          const targets = launchFileTargets(request, base);
          for (const target of targets) {
            try {
              const win = await platform.createWebviewWindow(
                `${target.route}-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
                target.url,
                {
                  title: target.title,
                  width: 1200,
                  height: 740,
                  minWidth: 1200,
                  minHeight: 740,
                  dragDropEnabled: target.route !== "reader",
                  center: true,
                },
              );
              // Window construction is asynchronous; never hide the main window on failure.
              await new Promise<void>((resolve, reject) => {
                let created: PlatformUnlisten | undefined;
                let failed: PlatformUnlisten | undefined;
                const cleanup = () => {
                  created?.();
                  failed?.();
                };
                void win
                  .once("tauri://created", () => {
                    cleanup();
                    resolve();
                  })
                  .then((unlisten) => {
                    created = unlisten;
                  }, reject);
                void win
                  .once("tauri://error", (event) => {
                    cleanup();
                    reject(event.payload);
                  })
                  .then((unlisten) => {
                    failed = unlisten;
                  }, reject);
              });
              await win.setFocus();
              const settings = loadAppSettings();
              const inLibrary = window.location.pathname === `${base}/library`;
              const hideMain =
                targets.length === 1 &&
                inLibrary &&
                (target.isEpub
                  ? settings.closeLibraryOnEpubOpen
                  : settings.closeLibraryOnTxtOpen) !== false;
              if (hideMain) {
                await win.once("tauri://destroyed", async () => {
                  await mainWindow.show();
                  await mainWindow.setFocus();
                });
                await mainWindow.hide();
              }
            } catch (error) {
              console.error(
                `Failed to open launch file: ${target.path}`,
                error,
              );
              await platform.message(
                `打开文件失败：${target.path}\n${String(error)}`,
                {
                  title: "打开失败",
                  kind: "error",
                },
              );
            }
          }
        }
      }
    } catch (error) {
      console.error("Failed to process launch requests:", error);
    } finally {
      draining = false;
    }
  }

  // Subscribe before draining so a launch racing with initialization is not lost.
  const unlisten = await mainWindow.listen("tepub-launch-requested", () => {
    void drain();
  });
  void drain();
  return () => {
    disposed = true;
    unlisten();
  };
}
