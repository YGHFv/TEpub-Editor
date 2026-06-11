<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, message } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  type ToolId =
    | "txt-epub"
    | "epub-edit"
    | "epub-read"
    | "font-encrypt"
    | "font-decrypt"
    | "file-encrypt"
    | "file-decrypt";

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

  const tools: Tool[] = [
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
  ];

  const toolGroups: ToolGroup[] = [
    {
      id: "open",
      title: "打开工具",
      meta: "新窗口",
      tools: tools.filter((tool) => tool.id === "txt-epub" || tool.id === "epub-edit" || tool.id === "epub-read"),
      gridClass: "open-grid",
    },
    {
      id: "process",
      title: "EPUB 处理",
      meta: "生成新文件",
      tools: tools.filter((tool) => tool.id === "font-encrypt" || tool.id === "font-decrypt" || tool.id === "file-encrypt" || tool.id === "file-decrypt"),
      gridClass: "process-grid",
    },
  ];

  let busyTool: ToolId | "" = "";
  let statusText = "";

  function windowLabel(prefix: string) {
    return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  }

  async function pickFile(tool: Tool) {
    busyTool = tool.id;
    statusText = "";
    try {
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

  function isProcessingTool(id: ToolId) {
    return id === "font-encrypt" || id === "font-decrypt" || id === "file-encrypt" || id === "file-decrypt";
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
      default:
        throw new Error(`不支持的工具: ${id}`);
    }
  }

  async function runProcessingTool(tool: Tool, filePath: string, txtPath?: string) {
    return await invoke<ToolboxResult>(commandForTool(tool.id), { epubPath: filePath, txtPath });
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
</script>

<svelte:window on:keydown={(e) => { if (e.key === "Escape") closeWindow(); }} />

<main class="toolbox-app">
  <header class="toolbox-header">
    <div class="header-inner">
      <div>
        <h1>工具箱</h1>
        <div class="toolbox-subtitle">常用工具</div>
      </div>
      <button class="close-btn" type="button" on:click={closeWindow} title="关闭" aria-label="关闭">×</button>
    </div>
  </header>

  <section class="toolbox-content" aria-label="工具列表">
    {#each toolGroups as group}
      <section class="tool-section" aria-labelledby={`toolbox-section-${group.id}`}>
        <div class="section-head">
          <h2 id={`toolbox-section-${group.id}`}>{group.title}</h2>
          <span>{group.meta}</span>
        </div>
        <div class={`tool-grid ${group.gridClass}`}>
          {#each group.tools as tool}
            <button
              class="tool-card"
              type="button"
              on:click={() => pickFile(tool)}
              disabled={busyTool !== ""}
              aria-label={tool.title}
            >
              <span class="tool-icon">{tool.icon}</span>
              <span class="tool-copy">
                <span class="tool-title">{tool.title}</span>
                <span class="tool-detail">{busyTool === tool.id ? "处理中..." : tool.detail}</span>
              </span>
              <span class="tool-action">{tool.action}</span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  </section>

  <footer class="toolbox-status" aria-live="polite"><span>{statusText}</span></footer>
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

  .toolbox-header {
    min-height: 64px;
    box-sizing: border-box;
    padding: 0 24px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    flex-shrink: 0;
  }

  .header-inner {
    width: min(1100px, 100%);
    height: 64px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .toolbox-header h1 {
    margin: 0;
    font-size: 18px;
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

  .close-btn {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-muted);
    cursor: pointer;
    font-size: 24px;
    line-height: 1;
    transition: background var(--transition-fast), color var(--transition-fast);
  }

  .close-btn:hover {
    background: var(--color-hover);
    color: var(--color-text);
  }

  .toolbox-content {
    box-sizing: border-box;
    width: min(1100px, calc(100% - 48px));
    flex: 1;
    min-height: 0;
    margin: 0 auto;
    padding: 26px 0 30px;
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
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .process-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .tool-card {
    position: relative;
    min-height: 104px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    padding: 16px 18px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
    text-align: left;
    box-shadow: var(--shadow-xs);
    transition: border-color var(--transition-fast), background var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tool-card:hover:not(:disabled) {
    border-color: var(--color-border-strong);
    background: var(--color-hover);
    box-shadow: var(--shadow-sm);
  }

  .tool-card:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .tool-card:disabled {
    cursor: wait;
    opacity: 0.68;
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

  .toolbox-status {
    min-height: 38px;
    box-sizing: border-box;
    padding: 0 24px;
    display: flex;
    align-items: center;
    border-top: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.4;
    flex-shrink: 0;
  }

  .toolbox-status span {
    width: min(1100px, 100%);
    min-height: 18px;
    margin: 0 auto;
  }

  @media (max-width: 980px) {
    .open-grid,
    .process-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .toolbox-header {
      padding: 0 14px;
    }

    .toolbox-content {
      width: calc(100% - 24px);
      padding: 16px 0 20px;
    }

    .open-grid,
    .process-grid {
      grid-template-columns: 1fr;
    }

    .tool-card {
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
      padding-right: 0;
    }

    .toolbox-status {
      padding: 0 14px;
    }
  }
</style>
