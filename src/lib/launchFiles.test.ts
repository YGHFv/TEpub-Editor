// @vitest-environment happy-dom
import { describe, expect, it, vi } from "vitest";
import {
  launchFileTargets,
  listenForLaunchFiles,
  type LaunchInfo,
} from "./launchFiles";
import type { PlatformAdapter, PlatformEvent } from "./platform/types";

function fakePlatform(initial: LaunchInfo[] = []) {
  let queued = initial;
  let notify: (() => void) | undefined;
  const stop = vi.fn();
  const created = vi.fn();
  const destroyed = vi.fn();
  const platform = {
    isTauri: true,
    getCurrentWindowLabel: vi.fn(async () => "main"),
    getCurrentWindow: () => ({
      listen: vi.fn(async (_event: string, callback: () => void) => {
        notify = callback;
        return stop;
      }),
      hide: vi.fn(),
      show: vi.fn(),
      setFocus: vi.fn(),
    }),
    invoke: vi.fn(async () => {
      const result = queued;
      queued = [];
      return result;
    }),
    createWebviewWindow: vi.fn(async () => ({
      setFocus: vi.fn(),
      once: vi.fn(
        async (
          event: string,
          callback: (event: PlatformEvent<unknown>) => void,
        ) => {
          if (event === "tauri://created") {
            created();
            setTimeout(() => callback({ event, payload: null }), 0);
          }
          if (event === "tauri://destroyed") destroyed();
          return vi.fn();
        },
      ),
    })),
    message: vi.fn(),
  };
  return {
    platform,
    adapter: platform as unknown as PlatformAdapter,
    stop,
    push(request: LaunchInfo) {
      queued.push(request);
      notify?.();
    },
  };
}

describe("launch file dispatch", () => {
  it("routes EPUB editing, reading and TXT without losing special path characters", () => {
    const path = "C:\\books\\a & b #1.epub";
    expect(launchFileTargets({ filePaths: [path] }, "/app")[0].url).toBe(
      `/app/epub-editor?file=${encodeURIComponent(path)}`,
    );
    expect(
      launchFileTargets({ filePath: path, action: "reader" })[0].route,
    ).toBe("reader");
    expect(
      launchFileTargets({ filePath: "notes.TXT", action: "make-epub" })[0].url,
    ).toBe("/editor?file=notes.TXT&fromLibrary=1");
  });

  it("deduplicates within a request and rejects unsupported paths", () => {
    expect(
      launchFileTargets({
        filePaths: ["one.epub", "two.epub", "image.png", ""],
        filePath: "one.epub",
      }).map((target) => target.path),
    ).toEqual(["one.epub", "two.epub"]);
  });

  it("does not consume launches in Web mode or child windows", async () => {
    const fake = fakePlatform();
    fake.platform.isTauri = false;
    await listenForLaunchFiles(fake.adapter);
    expect(fake.platform.invoke).not.toHaveBeenCalled();
    fake.platform.isTauri = true;
    fake.platform.getCurrentWindowLabel.mockResolvedValue("epub-editor-1");
    await listenForLaunchFiles(fake.adapter);
    expect(fake.platform.invoke).not.toHaveBeenCalled();
  });

  it("opens every cold-start file and later requests in independent windows", async () => {
    const fake = fakePlatform([{ filePaths: ["first.epub", "second.epub"] }]);
    const cleanup = await listenForLaunchFiles(fake.adapter);
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(2),
    );
    fake.push({ filePath: "third.epub", action: "reader" });
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(3),
    );
    const calls = fake.platform.createWebviewWindow.mock.calls as unknown as [
      string,
      string,
    ][];
    expect(new Set(calls.map(([label]) => label)).size).toBe(3);
    expect(calls[2][1]).toContain("/reader?file=third.epub");
    cleanup();
    expect(fake.stop).toHaveBeenCalledOnce();
  });

  it("does not suppress reopening the same file in a later launch", async () => {
    const fake = fakePlatform([{ filePath: "same.epub" }]);
    await listenForLaunchFiles(fake.adapter);
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(1),
    );
    fake.push({ filePath: "same.epub" });
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(2),
    );
  });

  it("drains a request arriving while an earlier window is being created", async () => {
    const fake = fakePlatform([{ filePath: "first.epub" }]);
    await listenForLaunchFiles(fake.adapter);
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(1),
    );
    fake.push({ filePaths: ["second.epub", "third.epub"] });
    fake.push({ filePath: "fourth.epub" });
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(4),
    );
  });

  it("continues opening other files after a window fails", async () => {
    const fake = fakePlatform([{ filePaths: ["bad.epub", "good.epub"] }]);
    fake.platform.createWebviewWindow.mockRejectedValueOnce(
      new Error("window failed"),
    );
    await listenForLaunchFiles(fake.adapter);
    await vi.waitFor(() =>
      expect(fake.platform.createWebviewWindow).toHaveBeenCalledTimes(2),
    );
    expect(fake.platform.message).toHaveBeenCalledOnce();
  });
});
