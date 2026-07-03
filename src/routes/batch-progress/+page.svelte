<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

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

  type QueueRow = {
    inputPath: string;
    outputPath: string;
    status: "waiting" | "running" | "done" | "warning" | "error";
    message: string;
  };

  const params = new URLSearchParams(typeof window === "undefined" ? "" : window.location.search);
  const taskId = params.get("taskId") ?? "";
  const toolTitle = params.get("tool") ?? "批量处理";

  let total = 0;
  let current = 0;
  let done = false;
  let summary = "等待任务开始";
  let rows: QueueRow[] = [];
  let logs: string[] = [];

  $: percent = total > 0 ? Math.round((current / total) * 100) : 0;

  function basename(path: string) {
    return path.split(/[\\/]/).pop() || path;
  }

  function pushLog(event: BatchEvent) {
    const time = new Date().toLocaleTimeString();
    const prefix = event.inputPath ? basename(event.inputPath) : toolTitle;
    logs = [`[${time}] ${prefix}: ${event.message}`, ...logs].slice(0, 300);
  }

  function upsertRow(event: BatchEvent, status: QueueRow["status"]) {
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

  function applyEvent(event: BatchEvent) {
    if (event.taskId !== taskId) return;
    total = event.total || total;
    if (event.event === "started") {
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
      done = true;
      summary = event.message;
    }
    pushLog(event);
  }

  function closeWindow() {
    getCurrentWindow().close();
  }

  onMount(() => {
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
      <h1>{toolTitle}</h1>
      <p>{summary}</p>
    </div>
    <button type="button" on:click={closeWindow} disabled={!done && current < total}>关闭</button>
  </header>

  <section class="progress-area" aria-label="批量进度">
    <div class="progress-meta">
      <span>{current} / {total}</span>
      <strong>{percent}%</strong>
    </div>
    <div class="progress-track">
      <div class="progress-bar" style={`width: ${percent}%`}></div>
    </div>
  </section>

  <section class="queue-area" aria-label="处理队列">
    <div class="section-title">队列</div>
    <div class="queue-list">
      {#if rows.length === 0}
        <div class="empty">等待后端发送文件队列</div>
      {:else}
        {#each rows as row}
          <div class={`queue-row status-${row.status}`}>
            <div class="queue-main">
              <strong>{basename(row.inputPath)}</strong>
              <span>{row.message}</span>
              {#if row.outputPath}
                <small>{row.outputPath}</small>
              {/if}
            </div>
            <span class="queue-status">{row.status}</span>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <section class="log-area" aria-label="处理日志">
    <div class="section-title">日志</div>
    <div class="log-list">
      {#if logs.length === 0}
        <div class="empty">暂无日志</div>
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
    grid-template-rows: auto auto minmax(0, 1fr) 160px;
    gap: 14px;
    padding: 24px;
    background: var(--color-canvas);
    color: var(--color-text);
    font-family: var(--font-ui);
  }

  .batch-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .batch-head h1 {
    margin: 0;
    font-size: 20px;
    line-height: 1.25;
    font-weight: 800;
    letter-spacing: 0;
  }

  .batch-head p {
    margin: 6px 0 0;
    color: var(--color-muted);
    font-size: 13px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .batch-head button {
    min-width: 72px;
    padding: 7px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }

  .batch-head button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .progress-area,
  .queue-area,
  .log-area {
    min-width: 0;
  }

  .progress-meta,
  .section-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
    color: var(--color-muted);
    font-size: 12px;
    line-height: 1.2;
    font-weight: 700;
  }

  .progress-meta strong {
    color: var(--color-text);
  }

  .progress-track {
    height: 10px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--color-surface);
    box-shadow: inset 0 0 0 1px var(--color-border);
  }

  .progress-bar {
    height: 100%;
    border-radius: inherit;
    background: var(--color-accent);
    transition: width var(--transition-fast);
  }

  .queue-list,
  .log-list {
    height: 100%;
    box-sizing: border-box;
    overflow: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .queue-row {
    min-height: 58px;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 74px;
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
    line-height: 1.3;
  }

  .queue-status {
    justify-self: end;
    padding: 3px 7px;
    border-radius: 999px;
    background: var(--color-accent-quiet);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 800;
    line-height: 1.2;
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
    padding: 10px 12px;
    color: var(--color-muted);
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
  }

  .empty {
    padding: 14px;
    color: var(--color-muted);
    font-size: 12px;
  }

  @media (max-width: 640px) {
    .batch-app {
      grid-template-rows: auto auto minmax(0, 1fr) 140px;
      padding: 16px;
    }

    .batch-head {
      flex-direction: column;
    }

    .batch-head button {
      width: 100%;
    }
  }
</style>
