<script lang="ts">
  import { base } from "$app/paths";
  import { page } from "$app/stores";
  import { onDestroy } from "svelte";
  import {
    processWebEpub,
    type WebEpubProcessAction,
    type WebEpubProcessResult,
    type WebImageFormat,
  } from "$lib/webEpubProcess";

  type ToolConfig = {
    title: string;
    kicker: string;
    description: string;
    detail: string;
    output: string;
  };

  type ProcessedItem = WebEpubProcessResult & { url: string };
  type FailedItem = { sourceName: string; error: string };

  const CONFIGS: Record<WebEpubProcessAction, ToolConfig> = {
    "file-encrypt": {
      title: "文件加密",
      kicker: "FILE OBFUSCATION",
      description: "混淆 EPUB manifest 资源文件名，并同步改写 OPF、HTML 与 CSS 引用。",
      detail: "此功能用于降低 EPUB 内部资源的直接可读性，不属于密码学加密。",
      output: "_encrypt.epub",
    },
    "file-decrypt": {
      title: "文件解密",
      kicker: "FILE RESTORE",
      description: "依据 OPF manifest ID 恢复混淆文件名，并修复内部资源引用。",
      detail: "同时清理 Windows 不兼容字符，便于后续编辑和解压。",
      output: "_decrypt.epub",
    },
    "epub-reformat": {
      title: "EPUB 重构",
      kicker: "PACKAGE REFORMAT",
      description: "将 EPUB 整理为标准 OEBPS 目录结构，并补登记未写入 manifest 的资源。",
      detail: "文本、样式、图片、字体、音视频会归入对应目录，所有引用同步更新。",
      output: "_reformat.epub",
    },
    "image-convert": {
      title: "图片转换",
      kicker: "WEBP CONVERTER",
      description: "把 EPUB 内的 WebP 图片转换为 PNG 或 JPEG，并同步更新资源路径与媒体类型。",
      detail: "自动模式会为含透明通道的图片选择 PNG，其余图片选择 JPEG。",
      output: "_transfer.epub",
    },
  };

  const ACTIONS = new Set<WebEpubProcessAction>(["file-encrypt", "file-decrypt", "epub-reformat", "image-convert"]);

  let fileInput: HTMLInputElement | null = null;
  let busy = false;
  let dragActive = false;
  let progress = "";
  let results: ProcessedItem[] = [];
  let failures: FailedItem[] = [];
  let imageFormat: WebImageFormat = "auto";

  $: rawAction = $page.url.searchParams.get("tool") as WebEpubProcessAction | null;
  $: action = rawAction && ACTIONS.has(rawAction) ? rawAction : "epub-reformat";
  $: config = CONFIGS[action];

  function appPath(path: string) {
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  function clearResults() {
    for (const result of results) URL.revokeObjectURL(result.url);
    results = [];
    failures = [];
  }

  async function processFiles(files: File[]) {
    const epubFiles = files.filter((file) => file.name.toLowerCase().endsWith(".epub"));
    if (!epubFiles.length) {
      failures = [{ sourceName: "未选择文件", error: "请选择扩展名为 .epub 的文件。" }];
      return;
    }
    clearResults();
    busy = true;
    const completed: ProcessedItem[] = [];
    const failed: FailedItem[] = [];
    try {
      for (let index = 0; index < epubFiles.length; index += 1) {
        const file = epubFiles[index];
        progress = `正在处理 ${index + 1} / ${epubFiles.length}：${file.name}`;
        try {
          const result = await processWebEpub(file, action, { imageFormat });
          completed.push({ ...result, url: URL.createObjectURL(result.blob) });
          results = [...completed];
        } catch (error) {
          failed.push({ sourceName: file.name, error: error instanceof Error ? error.message : String(error) });
          failures = [...failed];
        }
      }
      progress = `处理完成：${completed.length} 成功 / ${failed.length} 失败`;
    } finally {
      busy = false;
      if (fileInput) fileInput.value = "";
    }
  }

  function handleFileChange(event: Event) {
    void processFiles(Array.from((event.currentTarget as HTMLInputElement).files || []));
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (!busy) void processFiles(Array.from(event.dataTransfer?.files || []));
  }

  function downloadResult(result: ProcessedItem) {
    const anchor = document.createElement("a");
    anchor.href = result.url;
    anchor.download = result.outputName;
    anchor.click();
  }

  async function downloadAll() {
    for (const result of results) {
      downloadResult(result);
      await new Promise((resolve) => setTimeout(resolve, 180));
    }
  }

  onDestroy(clearResults);
</script>

<svelte:head>
  <title>Web {config.title} - TEpub Editor</title>
  <meta name="description" content={config.description} />
</svelte:head>

<div class="process-page">
  <header class="topbar">
    <div class="heading">
      <a class="back" href={appPath("/")} aria-label="返回工具箱">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
      </a>
      <div><span>{config.kicker}</span><h1>{config.title}</h1></div>
    </div>
    <div class="head-actions">
      {#if results.length > 1}<button class="secondary" type="button" on:click={downloadAll}>下载全部</button>{/if}
      <button class="primary" type="button" disabled={busy} on:click={() => fileInput?.click()}>{results.length || failures.length ? "重新选择" : "选择 EPUB"}</button>
    </div>
  </header>

  <input bind:this={fileInput} class="file-input" type="file" accept=".epub,application/epub+zip" multiple on:change={handleFileChange} />

  <main class="content">
    <section class="intro-card">
      <div class="tool-mark">{action === "image-convert" ? "IMG" : action === "epub-reformat" ? "REF" : action === "file-encrypt" ? "LOCK" : "OPEN"}</div>
      <div><span class="kicker">{config.kicker}</span><h2>{config.title}</h2><p>{config.description}</p><small>{config.detail}</small></div>
      <div class="output-note"><span>输出文件</span><strong>*{config.output}</strong></div>
    </section>

    {#if action === "image-convert"}
      <section class="format-panel">
        <div><strong>输出格式</strong><span>仅转换 EPUB 内的 WebP 图片</span></div>
        <div class="segments" aria-label="图片输出格式">
          <button class:active={imageFormat === "auto"} type="button" disabled={busy} on:click={() => { imageFormat = "auto"; }}>自动</button>
          <button class:active={imageFormat === "png"} type="button" disabled={busy} on:click={() => { imageFormat = "png"; }}>PNG</button>
          <button class:active={imageFormat === "jpeg"} type="button" disabled={busy} on:click={() => { imageFormat = "jpeg"; }}>JPEG</button>
        </div>
      </section>
    {/if}

    {#if !results.length && !failures.length}
      <button
        class="drop-zone"
        class:drag-active={dragActive}
        type="button"
        disabled={busy}
        on:click={() => fileInput?.click()}
        on:dragenter={(event) => { event.preventDefault(); dragActive = true; }}
        on:dragover={(event) => event.preventDefault()}
        on:dragleave={() => { dragActive = false; }}
        on:drop={handleDrop}
      >
        {#if busy}<span class="spinner"></span><strong>正在处理 EPUB</strong><span>{progress}</span>
        {:else}<span class="upload-icon">+</span><strong>选择或拖入 EPUB 文件</strong><span>支持一次处理多个文件 · 文件不会上传服务器</span><span class="choose">选择文件</span>{/if}
      </button>
    {:else}
      <section class="results-panel" aria-live="polite">
        <div class="results-head"><div><span class="kicker">PROCESS RESULTS</span><h3>{progress}</h3></div><button class="secondary" type="button" disabled={busy} on:click={() => fileInput?.click()}>添加新文件</button></div>
        <div class="result-list">
          {#each results as result}
            <article class:unchanged={!result.changed}>
              <span class="state">{result.changed ? "✓" : "–"}</span>
              <div class="result-copy"><strong>{result.sourceName}</strong><p>{result.message}</p><small>{result.processedEntries} 个相关条目 · {result.outputName}</small></div>
              <button type="button" on:click={() => downloadResult(result)}>{result.changed ? "下载结果" : "下载原文件"}</button>
            </article>
          {/each}
          {#each failures as failure}
            <article class="failed"><span class="state">×</span><div class="result-copy"><strong>{failure.sourceName}</strong><p>{failure.error}</p></div></article>
          {/each}
          {#if busy}<article class="pending"><span class="spinner small"></span><div class="result-copy"><strong>处理中</strong><p>{progress}</p></div></article>{/if}
        </div>
      </section>
    {/if}
  </main>
</div>

<style>
  :global(body) { margin: 0; background: #edf1f5; color: #172033; font-family: Inter, "Microsoft YaHei", sans-serif; }
  button, a { font: inherit; }
  button { color: inherit; }
  .process-page { min-height: 100dvh; }
  .topbar { min-height: 68px; display: flex; align-items: center; justify-content: space-between; gap: 18px; padding: 0 24px; border-bottom: 1px solid #d8e0e9; background: rgba(255,255,255,.96); }
  .heading, .head-actions { display: flex; align-items: center; gap: 12px; }
  .back { width: 36px; height: 36px; display: grid; place-items: center; border: 1px solid #d6dee8; border-radius: 8px; color: #43536b; background: #fff; }
  .back svg { width: 20px; fill: none; stroke: currentColor; stroke-width: 2; }
  .heading span, .kicker { color: #7b899d; font-size: 9px; font-weight: 800; letter-spacing: .15em; }
  .heading h1 { margin: 2px 0 0; font-size: 18px; }
  .primary, .secondary { min-height: 36px; padding: 0 16px; border-radius: 7px; font-size: 12px; font-weight: 800; cursor: pointer; }
  .primary { border: 1px solid #17699a; background: #17699a; color: #fff; }
  .secondary { border: 1px solid #d1dae5; background: #fff; }
  button:disabled { cursor: wait; opacity: .6; }
  .file-input { position: fixed; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
  .content { width: min(1040px, calc(100% - 36px)); margin: 26px auto; display: grid; gap: 14px; }
  .intro-card { display: grid; grid-template-columns: 70px minmax(0, 1fr) auto; align-items: center; gap: 18px; padding: 22px; border: 1px solid #d8e1eb; border-radius: 12px; background: #fff; box-shadow: 0 8px 25px rgba(42,62,84,.05); }
  .tool-mark { width: 64px; height: 64px; display: grid; place-items: center; border-radius: 14px; background: #e4f1f8; color: #17699a; font-size: 12px; font-weight: 900; letter-spacing: .08em; }
  .intro-card h2 { margin: 4px 0 7px; font-size: 22px; }
  .intro-card p { margin: 0; color: #46566c; font-size: 13px; line-height: 1.55; }
  .intro-card small { display: block; margin-top: 5px; color: #8491a2; font-size: 11px; }
  .output-note { min-width: 132px; display: grid; gap: 5px; padding-left: 20px; border-left: 1px solid #e0e6ed; }
  .output-note span { color: #8491a2; font-size: 10px; }
  .output-note strong { color: #31516b; font-family: ui-monospace, Consolas, monospace; font-size: 11px; }
  .format-panel { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 15px 18px; border: 1px solid #d8e1eb; border-radius: 9px; background: #fff; }
  .format-panel > div:first-child { display: grid; gap: 3px; }
  .format-panel strong { font-size: 12px; }
  .format-panel span { color: #8290a2; font-size: 10px; }
  .segments { display: flex; gap: 3px; padding: 3px; border: 1px solid #d5dee8; border-radius: 7px; background: #f5f7fa; }
  .segments button { min-width: 66px; padding: 6px 10px; border: 0; border-radius: 4px; background: transparent; color: #6b7b90; font-size: 11px; cursor: pointer; }
  .segments button.active { background: #fff; color: #17699a; box-shadow: 0 1px 4px rgba(28,58,84,.12); font-weight: 800; }
  .drop-zone { min-height: 290px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; border: 1px dashed #9eb1c7; border-radius: 12px; background: rgba(255,255,255,.78); cursor: pointer; transition: .18s ease; }
  .drop-zone:hover, .drop-zone.drag-active { border-color: #17699a; background: #fff; box-shadow: 0 10px 32px rgba(42,74,103,.1); }
  .drop-zone strong { font-size: 18px; }
  .drop-zone > span:not(.upload-icon):not(.choose):not(.spinner) { color: #728197; font-size: 12px; }
  .upload-icon { width: 48px; height: 48px; display: grid; place-items: center; border-radius: 50%; background: #e4f1f8; color: #17699a; font-size: 28px; font-weight: 300; }
  .choose { margin-top: 8px; padding: 8px 17px; border-radius: 6px; background: #17699a; color: #fff; font-size: 11px; font-weight: 800; }
  .spinner { width: 28px; height: 28px; border: 3px solid #d7e2ec; border-top-color: #17699a; border-radius: 50%; animation: spin .75s linear infinite; }
  .spinner.small { width: 18px; height: 18px; border-width: 2px; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .results-panel { border: 1px solid #d8e1eb; border-radius: 10px; background: #fff; overflow: hidden; }
  .results-head { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 16px 18px; border-bottom: 1px solid #e1e7ee; }
  .results-head h3 { margin: 4px 0 0; font-size: 14px; }
  .result-list { display: grid; gap: 8px; padding: 12px; }
  .result-list article { display: grid; grid-template-columns: 34px minmax(0, 1fr) auto; align-items: center; gap: 11px; padding: 13px; border: 1px solid #cce5d7; border-radius: 8px; background: #f9fffb; }
  .result-list article.unchanged { border-color: #d9e1e9; background: #fafbfd; }
  .result-list article.failed { border-color: #eccaca; background: #fffafa; }
  .result-list article.pending { border-color: #d8e1e9; background: #fafcfe; }
  .state { width: 30px; height: 30px; display: grid; place-items: center; border-radius: 50%; background: #def5e8; color: #177544; font-weight: 900; }
  .failed .state { background: #ffe2e2; color: #c32b2b; }
  .unchanged .state { background: #edf1f5; color: #68778b; }
  .result-copy { min-width: 0; display: grid; gap: 3px; }
  .result-copy strong { overflow-wrap: anywhere; font-size: 12px; }
  .result-copy p { margin: 0; color: #526176; font-size: 11px; line-height: 1.45; }
  .result-copy small { color: #8693a4; font-size: 9px; overflow-wrap: anywhere; }
  .result-list article > button { min-height: 32px; padding: 0 12px; border: 1px solid #b9cfdd; border-radius: 6px; background: #fff; color: #17699a; font-size: 10px; font-weight: 800; cursor: pointer; }
  @media (max-width: 700px) {
    .topbar { min-height: 62px; padding: 0 13px; }
    .heading span { display: none; }
    .heading h1 { font-size: 16px; }
    .head-actions .secondary { display: none; }
    .content { width: calc(100% - 24px); margin: 14px auto 28px; }
    .intro-card { grid-template-columns: 52px minmax(0, 1fr); padding: 16px; gap: 12px; }
    .tool-mark { width: 50px; height: 50px; border-radius: 11px; }
    .intro-card h2 { font-size: 18px; }
    .output-note { grid-column: 1 / -1; padding: 10px 0 0; border-left: 0; border-top: 1px solid #e0e6ed; }
    .format-panel { align-items: flex-start; flex-direction: column; }
    .segments { width: 100%; box-sizing: border-box; }
    .segments button { flex: 1; }
    .drop-zone { min-height: 250px; padding: 18px; }
    .drop-zone > span:not(.upload-icon):not(.choose):not(.spinner) { text-align: center; line-height: 1.5; }
    .result-list article { grid-template-columns: 32px minmax(0, 1fr); }
    .result-list article > button { grid-column: 2; justify-self: start; }
  }
</style>
