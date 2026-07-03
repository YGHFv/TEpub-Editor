<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";

  type BatchEvent = {
    taskId: string;
    event: string;
    level: "info" | "warning" | "error" | string;
    index: number;
    total: number;
    inputPath?: string | null;
    outputPath?: string | null;
    message: string;
  };

  type QueueStatus = "waiting" | "running" | "done" | "warning" | "error";

  type QueueRow = {
    inputPath: string;
    outputPath: string;
    status: QueueStatus;
    message: string;
  };

  type BatchTaskConfig = {
    taskId: string;
    tool: string;
    toolTitle: string;
    inputPaths: string[];
    sourceDirectories?: string[];
    outputDir?: string;
    resolvedOutputDir?: string;
    imageFormat?: string;
  };

  type BatchSummary = {
    taskId: string;
    total: number;
    succeeded: number;
    failed: number;
  };

  type BatchScanResult = {
    inputPaths: string[];
    outputDir: string;
  };

  const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);
  const taskId = params.get("taskId") ?? "";
  const toolTitleParam = params.get("tool") ?? "批量处理";
  const BATCH_TASK_PREFIX = "tepub-editor-batch-task:";

  const TOOL_META: Record<string, { title: string; detail: string }> = {
    "file-encrypt": { title: "文件加密", detail: "混淆 EPUB 内部文件名并同步引用" },
    "file-decrypt": { title: "文件解密", detail: "还原混淆文件名，支持单本、多本和目录扫描" },
    "font-encrypt": { title: "字体加密", detail: "对 EPUB 正文字形进行不可逆混淆处理" },
    "font-decrypt": { title: "字体解密", detail: "使用保存的映射或 TXT 对齐还原字体混淆" },
    "epub-reformat": { title: "EPUB 重构", detail: "整理 EPUB 目录结构并重写内部引用" },
    "image-convert": { title: "图片转换", detail: "转换 EPUB 内 WebP 图片并更新引用" },
  };

  let tool = "";
  let inputPaths: string[] = [];
  let sourceDirectories: string[] = [];
  let outputDir = "";
  let resolvedOutputDir = "";
  let imageFormat: string | undefined;
  let total = 0;
  let current = 0;
  let running = false;
  let scanning = false;
  let done = false;
  let summary = "请选择 EPUB 文件或扫描目录";
  let rows: QueueRow[] = [];
  let logs: string[] = [];

  $: currentMeta = TOOL_META[tool] ?? { title: toolTitleParam, detail: "批量执行 EPUB 工具" };
  $: percent = total > 0 ? Math.round((current / total) * 100) : 0;
  $: canStart = inputPaths.length > 0 && !running && !scanning;
  $: outputLabel = outputDir || resolvedOutputDir || "默认：所选文件夹下 TEpub-batch-output";

  function basename(path: string) {
    return path.split(/[\\/]/).pop() || path;
  }

  function statusLabel(status: QueueStatus) {
    switch (status) {
      case "running":
        return "处理中";
      case "done":
        return "完成";
      case "warning":
        return "跳过";
      case "error":
        return "失败";
      default:
        return "等待";
    }
  }

  function logLine(message: string, prefix = currentMeta.title) {
    const time = new Date().toLocaleTimeString();
    logs = [`[${time}] ${prefix}: ${message}`, ...logs].slice(0, 400);
  }

  function pushEventLog(event: BatchEvent) {
    logLine(event.message, event.inputPath ? basename(event.inputPath) : currentMeta.title);
  }

  function upsertRow(event: BatchEvent, status: QueueStatus) {
    if (!event.inputPath) return;
    const row: QueueRow = {
      inputPath: event.inputPath,
      outputPath: event.outputPath ?? "",
      status,
      message: event.message,
    };
    const existing = rows.findIndex((item) => item.inputPath === event.inputPath);
    if (existing >= 0) {
      rows = rows.map((item, index) => (index === existing ? { ...item, ...row } : item));
    } else {
      rows = [...rows, row];
    }
  }

  function addInputRows(paths: string[], message: string) {
    const existing = new Set(rows.map((row) => row.inputPath.toLowerCase()));
    const nextRows = [...rows];
    for (const path of paths) {
      const key = path.toLowerCase();
      if (existing.has(key)) continue;
      existing.add(key);
      nextRows.push({
        inputPath: path,
        outputPath: "",
        status: "waiting",
        message,
      });
    }
    rows = nextRows;
  }

  function addInputPaths(paths: string[], message: string) {
    const seen = new Set(inputPaths.map((path) => path.toLowerCase()));
    const added: string[] = [];
    for (const path of paths) {
      if (!path || seen.has(path.toLowerCase())) continue;
      seen.add(path.toLowerCase());
      added.push(path);
    }
    if (added.length === 0) return;
    inputPaths = [...inputPaths, ...added];
    addInputRows(added, message);
    summary = `已加入 ${inputPaths.length} 个输入源`;
    logLine(`加入 ${added.length} 个输入源`);
    saveTaskConfig();
  }

  function applyEvent(event: BatchEvent) {
    if (event.taskId !== taskId) return;
    total = event.total || total;
    if (event.outputPath && event.event === "scan-start") {
      resolvedOutputDir = event.outputPath;
    }
    if (event.event === "scan-start") {
      rows = [];
      current = 0;
      total = 0;
      summary = event.message;
    } else if (event.event === "scan-file") {
      summary = event.message;
      upsertRow(event, "waiting");
    } else if (event.event === "started") {
      summary = event.message;
    } else if (event.event === "file-start") {
      current = Math.max(current, event.index - 1);
      summary = event.message;
      upsertRow(event, "running");
    } else if (event.event === "file-done") {
      current = Math.max(current, event.index);
      summary = event.message;
      upsertRow(event, event.level === "warning" ? "warning" : "done");
    } else if (event.event === "file-error") {
      current = Math.max(current, event.index);
      summary = event.message;
      upsertRow(event, "error");
    } else if (event.event === "finished") {
      current = event.total;
      running = false;
      done = true;
      summary = event.message;
    }
    pushEventLog(event);
  }

  function readTaskConfig() {
    if (typeof window === "undefined") return null;
    const raw = localStorage.getItem(`${BATCH_TASK_PREFIX}${taskId}`);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as BatchTaskConfig;
    } catch {
      return null;
    }
  }

  function saveTaskConfig() {
    if (typeof window === "undefined") return;
    localStorage.setItem(
      `${BATCH_TASK_PREFIX}${taskId}`,
      JSON.stringify({
        taskId,
        tool,
        toolTitle: currentMeta.title,
        inputPaths,
        sourceDirectories,
        outputDir: outputDir || undefined,
        resolvedOutputDir: resolvedOutputDir || undefined,
        imageFormat,
      }),
    );
  }

  async function chooseFiles() {
    if (running || scanning) return;
    const selected = await open({
      multiple: true,
      filters: [{ name: "EPUB 文件", extensions: ["epub"] }],
    });
    if (!selected) return;
    addInputPaths(Array.isArray(selected) ? selected : [selected], "文件待处理");
  }

  async function scanDirectory() {
    if (running || scanning) return;
    const selected = await open({
      directory: true,
      multiple: true,
      title: "选择 EPUB 目录",
    });
    if (!selected) return;
    const directories = Array.isArray(selected) ? selected : [selected];
    sourceDirectories = directories;
    await scanInputDirectories(directories);
  }

  async function scanInputDirectories(directories: string[]) {
    if (directories.length === 0 || running || scanning) return;
    scanning = true;
    done = false;
    total = 0;
    current = 0;
    summary = "正在扫描目录中的 EPUB 文件";
    rows = directories.map((path) => ({
      inputPath: path,
      outputPath: "",
      status: "running",
      message: "正在扫描目录",
    }));
    logLine(`开始扫描 ${directories.length} 个目录`);
    try {
      const result = await invoke<BatchScanResult>("toolbox_scan_batch_inputs", {
        inputPaths: directories,
        outputDir: outputDir || undefined,
      });
      resolvedOutputDir = result.outputDir || "";
      inputPaths = [];
      rows = [];
      addInputPaths(result.inputPaths, "扫描完成，等待执行");
      total = result.inputPaths.length;
      current = 0;
      summary = result.inputPaths.length > 0 ? `扫描到 ${result.inputPaths.length} 个 EPUB 文件` : "目录中未找到 EPUB 文件";
      logLine(summary);
      saveTaskConfig();
    } catch (e: any) {
      summary = `目录扫描失败: ${e}`;
      rows = directories.map((path) => ({
        inputPath: path,
        outputPath: "",
        status: "error",
        message: summary,
      }));
      logLine(summary);
    } finally {
      scanning = false;
    }
  }

  async function rescanDirectories() {
    if (sourceDirectories.length === 0 || running || scanning) {
      summary = "没有可重新扫描的目录";
      return;
    }
    await scanInputDirectories(sourceDirectories);
  }

  async function chooseOutputDir() {
    if (running || scanning) return;
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择输出目录",
    });
    if (!selected || Array.isArray(selected)) return;
    outputDir = selected;
    resolvedOutputDir = "";
    summary = "已选择输出目录";
    logLine(`输出目录: ${outputDir}`);
    saveTaskConfig();
  }

  function resetOutputDir() {
    if (running || scanning) return;
    outputDir = "";
    resolvedOutputDir = "";
    summary = "已恢复默认输出目录";
    logLine("已恢复默认输出目录");
    saveTaskConfig();
  }

  function clearQueue() {
    if (running || scanning) return;
    inputPaths = [];
    sourceDirectories = [];
    rows = [];
    total = 0;
    current = 0;
    done = false;
    resolvedOutputDir = "";
    summary = "队列已清空";
    logLine("队列已清空");
    saveTaskConfig();
  }

  function removeInputPath(path: string) {
    if (running || scanning) return;
    const key = path.toLowerCase();
    inputPaths = inputPaths.filter((item) => item.toLowerCase() !== key);
    rows = rows.filter((row) => row.inputPath.toLowerCase() !== key);
    total = rows.length;
    current = Math.min(current, total);
    summary = inputPaths.length > 0 ? `已加入 ${inputPaths.length} 个输入源` : "队列已清空";
    logLine(`移除队列项: ${basename(path)}`);
    saveTaskConfig();
  }

  async function revealPath(path: string) {
    if (!path) return;
    try {
      await invoke("reveal_in_explorer", { path });
    } catch (e: any) {
      summary = `无法打开位置: ${e}`;
      logLine(summary);
    }
  }

  async function openOutputDir() {
    const path = outputDir || resolvedOutputDir;
    if (!path) {
      summary = "还没有可打开的输出目录";
      return;
    }
    await revealPath(path);
  }

  function clearLog() {
    logs = [];
  }

  async function startBatch() {
    if (!canStart) {
      summary = "请先加入 EPUB 文件或目录";
      return;
    }
    running = true;
    done = false;
    total = 0;
    current = 0;
    resolvedOutputDir = "";
    rows = [];
    summary = "正在启动批量任务";
    logLine("开始执行批量任务");
    saveTaskConfig();

    try {
      const result = await invoke<BatchSummary>("toolbox_run_batch", {
        taskId,
        tool,
        inputPaths,
        imageFormat,
        outputDir: outputDir || resolvedOutputDir || undefined,
      });
      running = false;
      done = true;
      total = result.total;
      current = result.total;
      if (result.total === 0) {
        summary = "未找到 EPUB 文件";
      }
    } catch (e: any) {
      running = false;
      done = true;
      summary = `批量任务失败: ${e}`;
      logLine(summary);
    }
  }

  function closeWindow() {
    getCurrentWindow().close();
  }

  onMount(() => {
    const config = readTaskConfig();
    if (config) {
      tool = config.tool;
      inputPaths = config.inputPaths ?? [];
      sourceDirectories = config.sourceDirectories ?? [];
      outputDir = config.outputDir ?? "";
      resolvedOutputDir = config.resolvedOutputDir ?? "";
      imageFormat = config.imageFormat;
      if (inputPaths.length > 0) {
        addInputRows(inputPaths, "等待执行");
        summary = `已加入 ${inputPaths.length} 个输入源`;
      }
    } else {
      summary = "未找到批量任务配置，请从工具箱重新打开";
    }

    let unlisten: UnlistenFn | undefined;
    listen<BatchEvent>("toolbox-batch-event", (event) => applyEvent(event.payload)).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  });
</script>

<main class="batch-app">
  <header class="batch-head">
    <div>
      <div class="eyebrow">功能执行</div>
      <h1>{currentMeta.title}</h1>
      <p>{currentMeta.detail}</p>
    </div>
    <button class="ghost-btn" type="button" on:click={closeWindow}>关闭</button>
  </header>

  <section class="source-panel" aria-label="输入源">
    <div class="source-copy">
      <div class="eyebrow">输入源</div>
      <h2>选择目录或添加 EPUB 文件</h2>
      <p>当前队列 {inputPaths.length} 个输入源。</p>
    </div>
    <div class="source-actions">
      <button class="primary-btn" type="button" on:click={scanDirectory} disabled={running || scanning}>
        {scanning ? "扫描中" : "选择目录"}
      </button>
      <button class="strong-btn" type="button" on:click={chooseFiles} disabled={running || scanning}>添加 EPUB</button>
      <button class="ghost-btn" type="button" on:click={clearQueue} disabled={running || scanning || inputPaths.length === 0}>清空队列</button>
    </div>
  </section>

  <section class="workspace">
    <section class="panel execute-panel" aria-label="任务配置">
      <div class="panel-head">
        <div>
          <div class="eyebrow">任务配置</div>
          <h2>输出与执行</h2>
        </div>
        <div class="panel-actions">
          <button class="ghost-btn" type="button" on:click={chooseOutputDir} disabled={running || scanning}>选择输出目录</button>
          <button class="ghost-btn" type="button" on:click={openOutputDir} disabled={running || scanning || !(outputDir || resolvedOutputDir)}>打开输出目录</button>
          <button class="ghost-btn" type="button" on:click={resetOutputDir} disabled={running || scanning || !outputDir}>重置输出路径</button>
        </div>
      </div>

      <div class="field-block">
        <span>输出目录</span>
        <strong title={outputLabel}>{outputLabel}</strong>
      </div>

      <div class="tool-run-row">
        <div class="tool-summary">
          <strong>{currentMeta.title}</strong>
          <span>{summary}</span>
        </div>
        <button class="start-btn" type="button" on:click={startBatch} disabled={!canStart}>
          {running ? "执行中" : scanning ? "扫描中" : "开始执行"}
        </button>
      </div>

      <div class="progress-area" aria-label="批量进度">
        <div class="progress-meta">
          <span>{current} / {total}</span>
          <strong>{percent}%</strong>
        </div>
        <div class="progress-track">
          <div class="progress-bar" style={`width: ${percent}%`}></div>
        </div>
      </div>
    </section>

    <section class="panel queue-panel" aria-label="文件队列">
      <div class="panel-head">
        <div>
          <div class="eyebrow">文件队列</div>
          <h2>待处理列表</h2>
        </div>
        <div class="queue-head-actions">
          <button class="ghost-btn compact-btn" type="button" on:click={rescanDirectories} disabled={running || scanning || sourceDirectories.length === 0}>重新扫描</button>
          <span class="count-pill">{rows.length}</span>
        </div>
      </div>
      <div class="queue-list">
        {#if rows.length === 0}
          <div class="empty">还没有加入 EPUB 文件或目录。</div>
        {:else}
          {#each rows as row}
            <div class={`queue-row status-${row.status}`}>
              <div class="queue-main">
                <strong>{basename(row.inputPath)}</strong>
                <span>{row.message}</span>
                <small title={row.outputPath || row.inputPath}>{row.outputPath || row.inputPath}</small>
              </div>
              <div class="queue-actions">
                <span class="queue-status">{statusLabel(row.status)}</span>
                <button class="icon-text-btn" type="button" on:click={() => revealPath(row.outputPath || row.inputPath)} title="打开位置">定位</button>
                <button class="icon-text-btn danger" type="button" on:click={() => removeInputPath(row.inputPath)} disabled={running || scanning} title="从队列移除">移除</button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </section>
  </section>

  <section class="panel log-panel" aria-label="处理日志">
    <div class="panel-head">
      <div>
        <div class="eyebrow">过程</div>
        <h2>处理日志</h2>
      </div>
      <button class="ghost-btn" type="button" on:click={clearLog} disabled={logs.length === 0}>清空日志</button>
    </div>
    <div class="log-list">
      {#if logs.length === 0}
        <div class="empty">尚未执行任务。</div>
      {:else}
        {#each logs as log}
          <div>{log}</div>
        {/each}
      {/if}
    </div>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    overflow: hidden;
    background: var(--color-canvas);
  }

  .batch-app {
    box-sizing: border-box;
    height: 100vh;
    display: grid;
    grid-template-rows: auto auto minmax(260px, 1fr) minmax(150px, 190px);
    gap: 18px;
    padding: 28px;
    overflow: hidden;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
  }

  .batch-head,
  .source-panel,
  .panel-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .batch-head h1,
  .source-copy h2,
  .panel-head h2 {
    margin: 0;
    color: var(--color-text);
    font-weight: 800;
    letter-spacing: 0;
  }

  .batch-head h1 {
    font-size: 22px;
    line-height: 1.25;
  }

  .source-copy h2,
  .panel-head h2 {
    font-size: 16px;
    line-height: 1.35;
  }

  .batch-head p,
  .source-copy p {
    margin: 8px 0 0;
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.45;
  }

  .eyebrow {
    margin-bottom: 6px;
    color: var(--color-accent-deep);
    font-size: 12px;
    line-height: 1.2;
    font-weight: 800;
  }

  .source-panel,
  .panel {
    box-sizing: border-box;
    min-width: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    box-shadow: var(--shadow-xs);
  }

  .source-panel {
    align-items: center;
    padding: 18px 20px;
  }

  .source-actions,
  .panel-actions,
  .queue-head-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 10px;
  }

  button {
    box-sizing: border-box;
    min-height: 36px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 7px 14px;
    font: inherit;
    font-size: 13px;
    font-weight: 800;
    line-height: 1.2;
    cursor: pointer;
    transition: background var(--transition-fast), border-color var(--transition-fast), color var(--transition-fast), opacity var(--transition-fast);
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .primary-btn,
  .start-btn {
    border-color: color-mix(in srgb, var(--color-accent) 42%, var(--color-border));
    background: var(--color-accent);
    color: var(--color-accent-contrast, #fff);
  }

  .strong-btn {
    border-color: var(--color-border-strong);
    background: var(--color-text);
    color: var(--color-surface);
  }

  .ghost-btn {
    background: var(--color-surface);
    color: var(--color-text);
  }

  button:hover:not(:disabled) {
    border-color: var(--color-border-strong);
  }

  .workspace {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(320px, 1fr) minmax(360px, 1fr);
    align-items: stretch;
    gap: 18px;
    overflow: hidden;
  }

  .panel {
    min-height: 0;
    padding: 16px;
    overflow: hidden;
  }

  .execute-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: hidden;
  }

  .field-block {
    display: grid;
    gap: 6px;
    padding: 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
  }

  .execute-panel .panel-actions button {
    min-height: 32px;
    padding: 6px 10px;
    font-size: 12px;
  }

  .field-block span {
    color: var(--color-muted);
    font-size: 12px;
    font-weight: 700;
  }

  .field-block strong {
    min-width: 0;
    color: var(--color-text);
    font-size: 13px;
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tool-summary {
    min-width: 0;
    display: grid;
    gap: 6px;
  }

  .tool-summary strong {
    font-size: 15px;
    line-height: 1.3;
  }

  .tool-summary span {
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .tool-run-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
  }

  .start-btn {
    min-width: 108px;
    min-height: 36px;
    padding-inline: 16px;
    flex: 0 0 auto;
  }

  .progress-area {
    margin-top: 0;
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
    color: var(--color-muted);
    font-size: 12px;
    font-weight: 800;
  }

  .progress-meta strong {
    color: var(--color-text);
  }

  .progress-track {
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--color-canvas);
    box-shadow: inset 0 0 0 1px var(--color-border);
  }

  .progress-bar {
    height: 100%;
    border-radius: inherit;
    background: var(--color-accent);
    transition: width var(--transition-fast);
  }

  .queue-panel,
  .log-panel {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 12px;
  }

  .count-pill {
    min-width: 34px;
    padding: 4px 8px;
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 12px;
    font-weight: 800;
    text-align: center;
  }

  .compact-btn {
    min-height: 30px;
    padding: 5px 10px;
    font-size: 12px;
  }

  .queue-list,
  .log-list {
    box-sizing: border-box;
    min-height: 0;
    height: 100%;
    overflow-x: hidden;
    overflow-y: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-canvas);
  }

  .queue-row {
    min-height: 64px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-border);
  }

  .queue-row:last-child {
    border-bottom: 0;
  }

  .queue-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .queue-main strong,
  .queue-main span,
  .queue-main small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .queue-main strong {
    font-size: 13px;
    line-height: 1.3;
  }

  .queue-main span,
  .queue-main small {
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.35;
  }

  .queue-status {
    padding: 4px 8px;
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 800;
    line-height: 1.2;
  }

  .queue-actions {
    justify-self: end;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .icon-text-btn {
    min-height: 26px;
    padding: 4px 8px;
    border-color: transparent;
    background: transparent;
    color: var(--color-muted);
    font-size: 11px;
  }

  .icon-text-btn:hover:not(:disabled) {
    background: var(--color-surface);
    color: var(--color-text);
  }

  .icon-text-btn.danger:hover:not(:disabled) {
    color: #9b1c1c;
  }

  .status-running .queue-status {
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
  }

  .status-error .queue-status {
    background: color-mix(in srgb, #d14343 12%, transparent);
    color: #9b1c1c;
  }

  .status-warning .queue-status {
    background: color-mix(in srgb, #b7791f 14%, transparent);
    color: #8a4b08;
  }

  .log-list {
    max-height: 100%;
    padding: 10px 12px;
    color: var(--color-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
  }

  .empty {
    padding: 16px;
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.5;
    text-align: center;
  }

  @media (max-width: 820px) {
    :global(body) {
      overflow: auto;
    }

    .batch-app {
      grid-template-rows: auto auto auto minmax(160px, 1fr);
      height: auto;
      min-height: 100vh;
      padding: 16px;
      overflow: auto;
    }

    .source-panel,
    .batch-head,
    .panel-head {
      flex-direction: column;
      align-items: stretch;
    }

    .workspace {
      grid-template-columns: 1fr;
      overflow: visible;
    }

    .queue-panel {
      min-height: 320px;
    }

    .log-panel {
      min-height: 220px;
    }

    .source-actions,
    .panel-actions {
      justify-content: flex-start;
    }
  }
</style>
