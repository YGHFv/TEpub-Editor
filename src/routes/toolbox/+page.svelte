<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open, message } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  type ToolId =
    | "library"
    | "txt-epub"
    | "epub-edit"
    | "epub-read"
    | "font-encrypt"
    | "font-decrypt"
    | "file-encrypt"
    | "file-decrypt"
    | "epub-reformat"
    | "image-convert";

  type Tool = {
    id: ToolId;
    icon: string;
    title: string;
    detail: string;
    action: string;
  };

  type ToolGroup = {
    id: string;
    title: string;
    meta: string;
    tools: Tool[];
    gridClass: string;
  };

  type ToolboxResult = {
    sourcePath: string;
    outputPath: string;
    changed: boolean;
    action: string;
    message: string;
  };

  type LaunchInfo = {
    filePath?: string | null;
    filePaths?: string[];
    action?: string | null;
  };

  const LAUNCH_SESSION_KEY = "tepub-editor-launch-files";

  const tools: Tool[] = [
    {
      id: "library",
      icon: "LIB",
      title: "书库",
      detail: "管理图书与元数据",
      action: "进入",
    },
    {
      id: "txt-epub",
      icon: "TXT",
      title: "TXT 制作 EPUB",
      detail: "选择 TXT 文件",
      action: "打开",
    },
    {
      id: "epub-edit",
      icon: "EPUB",
      title: "EPUB 编辑器",
      detail: "选择 EPUB 文件",
      action: "编辑",
    },
    {
      id: "epub-read",
      icon: "READ",
      title: "EPUB 阅读器",
      detail: "选择 EPUB 文件",
      action: "阅读",
    },
    {
      id: "font-encrypt",
      icon: "FONT+",
      title: "字体加密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "font-decrypt",
      icon: "FONT-",
      title: "字体解密",
      detail: "选择 EPUB 和对应 TXT",
      action: "处理",
    },
    {
      id: "file-encrypt",
      icon: "LOCK",
      title: "文件加密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "file-decrypt",
      icon: "OPEN",
      title: "文件解密",
      detail: "选择 EPUB 文件",
      action: "处理",
    },
    {
      id: "epub-reformat",
      icon: "REFIT",
      title: "EPUB 重构",
      detail: "整理目录与引用",
      action: "处理",
    },
    {
      id: "image-convert",
      icon: "IMG",
      title: "图片转换",
      detail: "转换 EPUB 内 WebP",
      action: "处理",
    },
  ];

  const toolGroups: ToolGroup[] = [
    {
      id: "open",
      title: "常用入口",
      meta: "书库 / 新窗口",
      tools: tools.filter((tool) => tool.id === "library" || tool.id === "txt-epub" || tool.id === "epub-edit" || tool.id === "epub-read"),
      gridClass: "open-grid",
    },
    {
      id: "process",
      title: "EPUB 处理",
      meta: "生成新文件",
      tools: tools.filter((tool) => tool.id === "font-encrypt" || tool.id === "font-decrypt" || tool.id === "file-encrypt" || tool.id === "file-decrypt" || tool.id === "epub-reformat" || tool.id === "image-convert"),
      gridClass: "process-grid",
    },
  ];

  let busyTool: ToolId | "" = "";
  let statusText = "";

  function windowLabel(prefix: string) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  }

  function collectLaunchPaths(launchInfo: LaunchInfo | null | undefined) {
    const candidates = [...(launchInfo?.filePaths ?? []), launchInfo?.filePath ?? ""];
    const seen = new Set<string>();
    return candidates
      .filter((path): path is string => typeof path === "string" && path.trim().length > 0)
      .filter((path) => {
        const key = path.toLowerCase();
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
  }

  function launchSessionKey(paths: string[], action: string) {
    return JSON.stringify({ action, paths });
  }

  function supportedLaunchPath(path: string) {
    const ext = path.split(".").pop()?.toLowerCase();
    return ext === "epub" || ext === "txt";
  }

  async function openLaunchPath(filePath: string, action: string) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const encoded = encodeURIComponent(filePath);

    if (ext === "txt") {
      new WebviewWindow(windowLabel("editor"), {
        url: `/editor?file=${encoded}&fromLibrary=1`,
        title: "TEpub-Editor-TXT",
        width: 1200,
        height: 740,
        dragDropEnabled: true,
        center: true,
      });
      return;
    }

    if (ext !== "epub") return;

    if (action === "reader") {
      new WebviewWindow(windowLabel("reader"), {
        url: `/reader?file=${encoded}`,
        title: "TEpub-Editor-Reader",
        width: 500,
        height: 800,
        dragDropEnabled: false,
        center: true,
      });
      return;
    }

    new WebviewWindow(windowLabel("epub-editor"), {
      url: `/epub-editor?file=${encoded}`,
      title: "TEpub-Editor-EPUB",
      width: 1200,
      height: 740,
      dragDropEnabled: true,
      center: true,
    });
  }

  async function openLaunchFiles() {
    if (!isRootToolbox()) return;

    try {
      const launchInfo = await invoke<LaunchInfo>("get_launch_info");
      const action = launchInfo?.action ?? "";
      const paths = collectLaunchPaths(launchInfo).filter(supportedLaunchPath);
      if (paths.length === 0) return;

      const key = launchSessionKey(paths, action);
      if (sessionStorage.getItem(LAUNCH_SESSION_KEY) === key) return;
      sessionStorage.setItem(LAUNCH_SESSION_KEY, key);

      for (const path of paths) {
        await openLaunchPath(path, action);
      }
      statusText = paths.length > 1 ? `${paths.length} files opened` : "File opened";
    } catch (e) {
      console.warn("Failed to process launch files:", e);
    }
  }

  async function pickFile(tool: Tool) {
    busyTool = tool.id;
    statusText = "";
    try {
      if (tool.id === "library") {
        await openLibrary();
        return;
      }

      const selected = await open({
        multiple: false,
        filters: [
          tool.id === "txt-epub"
            ? { name: "TXT 文件", extensions: ["txt"] }
            : { name: "EPUB 文件", extensions: ["epub"] },
        ],
      });

      if (!selected || Array.isArray(selected)) return;
      if (isProcessingTool(tool.id)) {
        let txtPath: string | undefined;
        if (tool.id === "font-decrypt") {
          const selectedTxt = await open({
            multiple: false,
            filters: [{ name: "TXT 文件", extensions: ["txt"] }],
            title: "选择与 EPUB 对应的明文 TXT",
          });
          if (!selectedTxt || Array.isArray(selectedTxt)) return;
          txtPath = selectedTxt;
        }
        const result = await runProcessingTool(tool, selected, txtPath);
        statusText = result.changed ? `${tool.title} 已完成` : result.message;
        await message(`${result.message}\n\n${result.outputPath}`, {
          title: tool.title,
          kind: result.changed ? "info" : "warning",
        });
      } else {
        await openToolForPath(tool, selected);
        statusText = `${tool.title} 已打开`;
      }
    } catch (e: any) {
      console.error("工具箱打开失败:", e);
      statusText = "打开失败";
      await message(`打开失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      busyTool = "";
    }
  }

  function isRootToolbox() {
    return typeof window !== "undefined" && window.location.pathname === "/";
  }

  async function openLibrary() {
    if (isRootToolbox()) {
      window.location.href = "/library";
      return;
    }

    try {
      const mainWin = await WebviewWindow.getByLabel("main");
      if (mainWin) {
        await mainWin.show();
        await mainWin.setFocus();
        await getCurrentWindow().close();
        return;
      }
    } catch (e) {
      console.warn("唤起主窗口失败，改为在当前窗口打开书库:", e);
    }

    window.location.href = "/library";
  }

  function isProcessingTool(id: ToolId) {
    return id === "font-encrypt" || id === "font-decrypt" || id === "file-encrypt" || id === "file-decrypt" || id === "epub-reformat" || id === "image-convert";
  }

  function commandForTool(id: ToolId) {
    switch (id) {
      case "font-encrypt":
        return "toolbox_font_encrypt";
      case "font-decrypt":
        return "toolbox_font_decrypt";
      case "file-encrypt":
        return "toolbox_file_encrypt";
      case "file-decrypt":
        return "toolbox_file_decrypt";
      case "epub-reformat":
        return "toolbox_epub_reformat";
      case "image-convert":
        return "toolbox_image_convert";
      default:
        throw new Error(`不支持的工具: ${id}`);
    }
  }

  async function runProcessingTool(tool: Tool, filePath: string, txtPath?: string) {
    return await invoke<ToolboxResult>(commandForTool(tool.id), {
      epubPath: filePath,
      txtPath,
      imageFormat: tool.id === "image-convert" ? "auto" : undefined,
    });
  }

  async function runBatchForFolder(tool: Tool) {
    if (!isProcessingTool(tool.id)) return;
    busyTool = tool.id;
    statusText = "";
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 EPUB 文件夹",
      });
      if (!selected || Array.isArray(selected)) return;

      const taskId = `batch-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const progressWindow = new WebviewWindow(windowLabel("batch-progress"), {
        url: `/batch-progress?taskId=${encodeURIComponent(taskId)}&tool=${encodeURIComponent(tool.title)}`,
        title: `${tool.title} 批量处理`,
        width: 900,
        height: 640,
        dragDropEnabled: false,
        center: true,
      });
      await new Promise((resolve) => setTimeout(resolve, 250));
      const summary = await invoke<{ total: number; succeeded: number; failed: number }>("toolbox_run_batch", {
        taskId,
        tool: tool.id,
        inputPaths: [selected],
        imageFormat: tool.id === "image-convert" ? "auto" : undefined,
      });
      statusText = `${tool.title} 批量完成：${summary.succeeded}/${summary.total}`;
      await progressWindow.setFocus().catch(() => {});
    } catch (e: any) {
      console.error("批量处理失败:", e);
      statusText = "批量处理失败";
      await message(`批量处理失败: ${e}`, { title: "错误", kind: "error" });
    } finally {
      busyTool = "";
    }
  }

  async function openToolForPath(tool: Tool, filePath: string) {
    const encoded = encodeURIComponent(filePath);
    if (tool.id === "txt-epub") {
      new WebviewWindow(windowLabel("editor"), {
        url: `/editor?file=${encoded}&fromLibrary=1`,
        title: "TEpub-Editor-TXT",
        width: 1200,
        height: 740,
        dragDropEnabled: true,
        center: true,
      });
      return;
    }

    if (tool.id === "epub-edit") {
      new WebviewWindow(windowLabel("epub-editor"), {
        url: `/epub-editor?file=${encoded}`,
        title: "TEpub-Editor-EPUB",
        width: 1200,
        height: 740,
        dragDropEnabled: true,
        center: true,
      });
      return;
    }

    new WebviewWindow(windowLabel("reader"), {
      url: `/reader?file=${encoded}`,
      title: "TEpub-Editor-Reader",
      width: 500,
      height: 800,
      dragDropEnabled: false,
      center: true,
    });
  }

  function closeWindow() {
    getCurrentWindow().close();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && !isRootToolbox()) {
      closeWindow();
    }
  }

  onMount(() => {
    openLaunchFiles();
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="toolbox-app">
  <section class="toolbox-content" aria-label="工具列表">
    <div class="toolbox-title-row">
      <div>
        <h1>工具箱</h1>
        <div class="toolbox-subtitle">常用工具</div>
      </div>
      {#if statusText}
        <span class="toolbox-inline-status" aria-live="polite">{statusText}</span>
      {/if}
    </div>

    {#each toolGroups as group}
      <section class="tool-section" aria-labelledby={`toolbox-section-${group.id}`}>
        <div class="section-head">
          <h2 id={`toolbox-section-${group.id}`}>{group.title}</h2>
          <span>{group.meta}</span>
        </div>
        <div class={`tool-grid ${group.gridClass}`}>
          {#each group.tools as tool}
            <div
              class="tool-card"
              class:tool-card-disabled={busyTool !== ""}
              aria-label={tool.title}
            >
              <button
                class="tool-main"
                type="button"
                on:click={() => pickFile(tool)}
                disabled={busyTool !== ""}
              >
              <span class="tool-icon">{tool.icon}</span>
              <span class="tool-copy">
                <span class="tool-title">{tool.title}</span>
                <span class="tool-detail">{busyTool === tool.id ? "处理中..." : tool.detail}</span>
              </span>
              <span class="tool-action">{tool.action}</span>
              </button>
              {#if isProcessingTool(tool.id)}
                <button
                  class="tool-batch"
                  type="button"
                  on:click={() => runBatchForFolder(tool)}
                  disabled={busyTool !== ""}
                  title="选择文件夹批量处理"
                >
                  批量
                </button>
              {/if}
            </div>
          {/each}
        </div>
      </section>
    {/each}
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    overflow: hidden;
    background: var(--color-canvas);
  }

  .toolbox-app {
    box-sizing: border-box;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
  }

  .toolbox-title-row {
    min-height: 50px;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 20px;
    margin-bottom: 28px;
  }

  .toolbox-title-row h1 {
    margin: 0;
    font-size: 22px;
    line-height: 1.2;
    font-weight: 800;
    letter-spacing: 0;
  }

  .toolbox-subtitle {
    margin-top: 4px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.2;
  }

  .toolbox-inline-status {
    flex-shrink: 0;
    max-width: min(420px, 42vw);
    margin-top: 4px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.4;
    text-align: right;
    overflow-wrap: anywhere;
  }

  .toolbox-content {
    box-sizing: border-box;
    width: min(1100px, calc(100% - 48px));
    flex: 1;
    min-height: 0;
    margin: 0 auto;
    padding: 30px 0 34px;
    overflow: auto;
  }

  .tool-section + .tool-section {
    margin-top: 26px;
  }

  .section-head {
    height: 24px;
    margin-bottom: 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .section-head h2 {
    margin: 0;
    color: var(--color-text);
    font-size: 14px;
    line-height: 1.3;
    font-weight: 800;
    letter-spacing: 0;
  }

  .section-head span {
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.2;
    white-space: nowrap;
  }

  .tool-grid {
    display: grid;
    gap: 12px;
  }

  .open-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .process-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .tool-card {
    position: relative;
    min-height: 104px;
    box-sizing: border-box;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    text-align: left;
    box-shadow: var(--shadow-xs);
    overflow: hidden;
    transition: border-color var(--transition-fast), background var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tool-card:hover:not(.tool-card-disabled) {
    border-color: var(--color-border-strong);
    background: var(--color-hover);
    box-shadow: var(--shadow-sm);
  }

  .tool-card-disabled {
    opacity: 0.68;
  }

  .tool-main {
    width: 100%;
    min-height: 104px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    padding: 16px 18px;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    text-align: left;
    font: inherit;
  }

  .tool-main:focus-visible,
  .tool-batch:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .tool-main:disabled,
  .tool-batch:disabled {
    cursor: wait;
  }

  .tool-icon {
    width: 48px;
    height: 48px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0;
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--color-accent) 10%, transparent);
  }

  .tool-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-right: 58px;
  }

  .tool-title {
    font-size: 15px;
    font-weight: 800;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .tool-detail {
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .tool-action {
    position: absolute;
    top: 14px;
    right: 14px;
    min-width: 40px;
    box-sizing: border-box;
    padding: 3px 8px;
    border: 1px solid color-mix(in srgb, var(--color-accent) 22%, transparent);
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 12px;
    font-weight: 800;
    line-height: 1.25;
    text-align: center;
  }

  .tool-batch {
    position: absolute;
    right: 14px;
    bottom: 12px;
    min-width: 40px;
    box-sizing: border-box;
    padding: 3px 8px;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-surface);
    color: var(--color-muted);
    font: inherit;
    font-size: 12px;
    font-weight: 800;
    line-height: 1.25;
    cursor: pointer;
  }

  .tool-batch:hover:not(:disabled) {
    border-color: var(--color-border-strong);
    color: var(--color-text);
    background: var(--color-hover);
  }

  @media (max-width: 980px) {
    .open-grid,
    .process-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .toolbox-content {
      width: calc(100% - 24px);
      padding: 16px 0 20px;
    }

    .toolbox-title-row {
      min-height: 0;
      flex-direction: column;
      gap: 8px;
      margin-bottom: 18px;
    }

    .toolbox-inline-status {
      max-width: 100%;
      text-align: left;
    }

    .open-grid,
    .process-grid {
      grid-template-columns: 1fr;
    }

    .tool-card {
      min-height: 88px;
    }

    .tool-main {
      grid-template-columns: 44px minmax(0, 1fr);
      min-height: 88px;
      padding: 14px;
    }

    .tool-icon {
      width: 44px;
      height: 44px;
    }

    .tool-action {
      position: static;
      grid-column: 2;
      justify-self: start;
      margin-top: -2px;
    }

    .tool-copy {
      padding-right: 52px;
    }

    .tool-batch {
      right: 14px;
      bottom: 12px;
    }
  }
</style>
