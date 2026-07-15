<script lang="ts">
  import { base } from "$app/paths";
  import {
    diagnoseWebEpub,
    formatWebEpubDiagnosticReport,
    type WebEpubDiagnosticIssue,
    type WebEpubDiagnosticResult,
  } from "$lib/webEpubDiagnose";

  type IssueFilter = "all" | "error" | "warning";

  let fileInput: HTMLInputElement | null = null;
  let results: WebEpubDiagnosticResult[] = [];
  let activeResultIndex = 0;
  let issueFilter: IssueFilter = "all";
  let busy = false;
  let dragActive = false;
  let progressText = "";
  let errorText = "";

  $: activeResult = results[activeResultIndex] || null;
  $: visibleIssues = activeResult
    ? activeResult.issues.filter((issue) => issueFilter === "all" || issue.level === issueFilter)
    : [];
  $: totalErrors = results.reduce((sum, result) => sum + result.errorCount, 0);
  $: totalWarnings = results.reduce((sum, result) => sum + result.warningCount, 0);

  function appPath(path: string) {
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function isEpub(file: File) {
    return file.name.toLowerCase().endsWith(".epub");
  }

  async function diagnoseFiles(files: File[]) {
    const epubFiles = files.filter(isEpub);
    if (!epubFiles.length) {
      errorText = "请选择扩展名为 .epub 的文件。";
      return;
    }

    busy = true;
    errorText = "";
    results = [];
    activeResultIndex = 0;
    issueFilter = "all";
    try {
      const nextResults: WebEpubDiagnosticResult[] = [];
      for (let index = 0; index < epubFiles.length; index += 1) {
        progressText = `正在诊断 ${index + 1} / ${epubFiles.length}：${epubFiles[index].name}`;
        nextResults.push(await diagnoseWebEpub(epubFiles[index]));
        results = [...nextResults];
      }
      progressText = `已完成 ${epubFiles.length} 个 EPUB 文件的诊断。`;
    } catch (error) {
      errorText = `诊断失败：${String(error)}`;
    } finally {
      busy = false;
      if (fileInput) fileInput.value = "";
    }
  }

  function handleFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    void diagnoseFiles(Array.from(input.files || []));
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (busy) return;
    void diagnoseFiles(Array.from(event.dataTransfer?.files || []));
  }

  function openPicker() {
    if (!busy) fileInput?.click();
  }

  function selectResult(index: number) {
    activeResultIndex = index;
    issueFilter = "all";
  }

  function downloadReport() {
    if (!results.length) return;
    const report = formatWebEpubDiagnosticReport(results);
    const url = URL.createObjectURL(new Blob([report], { type: "text/plain;charset=utf-8" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `tepub-epub-diagnostic-${new Date().toISOString().slice(0, 10)}.txt`;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  function issueLabel(issue: WebEpubDiagnosticIssue) {
    const labels: Record<string, string> = {
      "invalid-zip": "压缩包无效",
      "missing-mimetype": "缺少 mimetype",
      "missing-container": "缺少 container.xml",
      "container-parse": "container.xml 无法解析",
      "missing-opf": "缺少 OPF",
      "manifest-parse": "manifest 无法解析",
      "manifest-missing": "manifest 资源缺失",
      "case-mismatch": "路径大小写不一致",
      "unregistered-resource": "资源未登记",
      "ref-case-mismatch": "引用大小写不一致",
      "missing-reference": "引用目标缺失",
      "read-entry": "资源读取失败",
    };
    return labels[issue.kind] || issue.kind;
  }
</script>

<svelte:head>
  <title>Web EPUB 诊断 - TEpub Editor</title>
  <meta name="description" content="在浏览器本地检查 EPUB 的 OPF、manifest 与内部资源引用。" />
</svelte:head>

<div class="diagnose-page">
  <header class="topbar">
    <div class="heading">
      <a class="back-link" href={appPath("/")} aria-label="返回工具箱">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m15 18-6-6 6-6" /></svg>
      </a>
      <div>
        <span class="eyebrow">WEB TOOL</span>
        <h1>EPUB 诊断</h1>
      </div>
    </div>
    <div class="top-actions">
      {#if results.length}
        <button class="secondary" type="button" on:click={downloadReport}>下载报告</button>
      {/if}
      <button class="primary" type="button" disabled={busy} on:click={openPicker}>
        {results.length ? "重新选择" : "选择 EPUB"}
      </button>
    </div>
  </header>

  <input
    bind:this={fileInput}
    class="file-input"
    type="file"
    accept=".epub,application/epub+zip"
    multiple
    on:change={handleFileChange}
  />

  {#if !results.length && !busy}
    <main class="empty-shell">
      <button
        class:drag-active={dragActive}
        class="drop-zone"
        type="button"
        on:click={openPicker}
        on:dragenter={(event) => { event.preventDefault(); dragActive = true; }}
        on:dragover={(event) => event.preventDefault()}
        on:dragleave={() => { dragActive = false; }}
        on:drop={handleDrop}
      >
        <span class="drop-icon" aria-hidden="true">CHK</span>
        <strong>选择或拖入 EPUB 文件</strong>
        <span>支持一次诊断多个文件，检查过程完全在当前浏览器中完成</span>
        <span class="choose-pill">选择文件</span>
      </button>
      <section class="check-grid" aria-label="诊断项目">
        <article><b>01</b><strong>容器结构</strong><span>检查 mimetype、container.xml 与 OPF 路径</span></article>
        <article><b>02</b><strong>资源清单</strong><span>检查 manifest 缺失项、未登记资源与路径大小写</span></article>
        <article><b>03</b><strong>内部引用</strong><span>扫描 HTML、CSS、SVG、NCX 中的断链引用</span></article>
      </section>
      {#if errorText}<p class="error-banner">{errorText}</p>{/if}
    </main>
  {:else if busy && !results.length}
    <main class="loading-shell" aria-live="polite">
      <span class="spinner"></span>
      <strong>正在解包并检查 EPUB</strong>
      <span>{progressText}</span>
    </main>
  {:else}
    <main class="result-workspace">
      <aside class="file-sidebar">
        <div class="sidebar-summary">
          <span>{results.length} 个文件</span>
          <strong>{totalErrors} 错误 · {totalWarnings} 警告</strong>
        </div>
        <div class="file-list">
          {#each results as result, index}
            <button class:active={index === activeResultIndex} type="button" on:click={() => selectResult(index)}>
              <span class="file-state" class:clean={!result.errorCount && !result.warningCount} class:error={result.errorCount > 0}>
                {result.errorCount ? "×" : result.warningCount ? "!" : "✓"}
              </span>
              <span class="file-copy">
                <strong title={result.sourceName}>{result.sourceName}</strong>
                <small>{result.errorCount} 错误 · {result.warningCount} 警告</small>
              </span>
            </button>
          {/each}
          {#if busy}
            <div class="pending-file"><span class="spinner small"></span>{progressText}</div>
          {/if}
        </div>
      </aside>

      {#if activeResult}
        <section class="report-panel">
          <div class="report-head">
            <div>
              <span class="eyebrow">DIAGNOSTIC REPORT</span>
              <h2>{activeResult.sourceName}</h2>
              <p>{formatBytes(activeResult.sourceSize)} · OPF：{activeResult.opfPath || "未发现"}</p>
            </div>
            <span class:clean={!activeResult.errorCount && !activeResult.warningCount} class="result-badge">
              {activeResult.errorCount ? "发现错误" : activeResult.warningCount ? "需要注意" : "检查通过"}
            </span>
          </div>

          <div class="metrics">
            <article><span>ZIP 条目</span><strong>{activeResult.totalEntries}</strong></article>
            <article><span>Manifest</span><strong>{activeResult.manifestItems}</strong></article>
            <article class="error-metric"><span>错误</span><strong>{activeResult.errorCount}</strong></article>
            <article class="warning-metric"><span>警告</span><strong>{activeResult.warningCount}</strong></article>
          </div>

          <div class="issue-toolbar">
            <h3>问题列表</h3>
            <div class="filter-tabs" aria-label="筛选诊断问题">
              <button class:active={issueFilter === "all"} type="button" on:click={() => { issueFilter = "all"; }}>全部</button>
              <button class:active={issueFilter === "error"} type="button" on:click={() => { issueFilter = "error"; }}>错误 {activeResult.errorCount}</button>
              <button class:active={issueFilter === "warning"} type="button" on:click={() => { issueFilter = "warning"; }}>警告 {activeResult.warningCount}</button>
            </div>
          </div>

          <div class="issue-list">
            {#if !activeResult.issues.length}
              <div class="clean-state"><span>✓</span><strong>未发现明显结构问题</strong><p>EPUB 的容器、资源清单与内部引用均通过当前检查。</p></div>
            {:else if !visibleIssues.length}
              <div class="no-filter-result">当前筛选下没有问题。</div>
            {:else}
              {#each visibleIssues as issue}
                <article class:error={issue.level === "error"} class="issue-row">
                  <span class="level-icon">{issue.level === "error" ? "×" : "!"}</span>
                  <div>
                    <div class="issue-title"><strong>{issueLabel(issue)}</strong><code>{issue.kind}</code></div>
                    <p>{issue.message}</p>
                    {#if issue.path}<span class="issue-path">{issue.path}</span>{/if}
                  </div>
                </article>
              {/each}
            {/if}
          </div>
        </section>
      {/if}
    </main>
  {/if}

  {#if errorText && results.length}<div class="floating-error">{errorText}</div>{/if}
</div>

<style>
  :global(body) { margin: 0; overflow: hidden; background: #eef2f6; color: #172033; font-family: Inter, "Microsoft YaHei", sans-serif; }
  button, a { font: inherit; }
  button { color: inherit; }
  .diagnose-page { min-height: 100vh; height: 100vh; display: grid; grid-template-rows: 68px minmax(0, 1fr); background: #eef2f6; }
  .topbar { z-index: 2; display: flex; align-items: center; justify-content: space-between; gap: 20px; padding: 0 24px; border-bottom: 1px solid #d8e0ea; background: rgba(255,255,255,.96); }
  .heading, .top-actions { display: flex; align-items: center; gap: 12px; }
  .back-link { width: 36px; height: 36px; display: grid; place-items: center; border: 1px solid #d7dee8; border-radius: 8px; color: #40516a; background: #fff; }
  .back-link svg { width: 20px; fill: none; stroke: currentColor; stroke-width: 2; }
  .heading h1 { margin: 1px 0 0; font-size: 18px; line-height: 1.1; }
  .eyebrow { color: #7b8aa0; font-size: 9px; font-weight: 800; letter-spacing: .16em; }
  .primary, .secondary { min-height: 36px; padding: 0 16px; border-radius: 7px; font-size: 13px; font-weight: 700; cursor: pointer; }
  .primary { border: 1px solid #17699a; background: #17699a; color: #fff; }
  .secondary { border: 1px solid #ced8e4; background: #fff; color: #34445b; }
  button:disabled { cursor: wait; opacity: .6; }
  .file-input { position: fixed; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
  .empty-shell { min-height: 0; display: grid; align-content: center; justify-items: center; gap: 24px; padding: 32px; overflow: auto; }
  .drop-zone { width: min(720px, 100%); min-height: 260px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; border: 1px dashed #9eb1c7; border-radius: 14px; background: #fff; cursor: pointer; transition: .18s ease; }
  .drop-zone:hover, .drop-zone.drag-active { border-color: #17699a; box-shadow: 0 12px 35px rgba(36,71,103,.12); transform: translateY(-2px); }
  .drop-zone strong { font-size: 22px; }
  .drop-zone > span:not(.drop-icon):not(.choose-pill) { color: #718096; font-size: 13px; }
  .drop-icon { width: 58px; height: 58px; display: grid; place-items: center; margin-bottom: 6px; border-radius: 13px; background: #e4f1f8; color: #17699a; font-size: 13px; font-weight: 900; letter-spacing: .08em; }
  .choose-pill { margin-top: 10px; padding: 9px 18px; border-radius: 7px; background: #17699a; color: #fff; font-size: 13px; font-weight: 700; }
  .check-grid { width: min(900px, 100%); display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
  .check-grid article { min-width: 0; display: grid; grid-template-columns: 36px 1fr; gap: 2px 10px; padding: 14px; border: 1px solid #dce4ed; border-radius: 9px; background: rgba(255,255,255,.72); }
  .check-grid b { grid-row: 1 / 3; color: #17699a; font-size: 11px; }
  .check-grid strong { font-size: 13px; }
  .check-grid span { color: #718096; font-size: 11px; line-height: 1.45; }
  .error-banner, .floating-error { padding: 10px 14px; border: 1px solid #efb3b3; border-radius: 7px; background: #fff1f1; color: #b42323; font-size: 13px; }
  .loading-shell { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; color: #617188; }
  .loading-shell strong { color: #26364d; font-size: 18px; }
  .spinner { width: 28px; height: 28px; border: 3px solid #d9e4ed; border-top-color: #17699a; border-radius: 50%; animation: spin .75s linear infinite; }
  .spinner.small { width: 14px; height: 14px; border-width: 2px; flex: 0 0 auto; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .result-workspace { min-height: 0; display: grid; grid-template-columns: 270px minmax(0, 1fr); overflow: hidden; }
  .file-sidebar { min-height: 0; display: grid; grid-template-rows: auto minmax(0, 1fr); border-right: 1px solid #d5dee8; background: #f8fafc; }
  .sidebar-summary { display: grid; gap: 3px; padding: 16px; border-bottom: 1px solid #dde5ed; }
  .sidebar-summary span { color: #728197; font-size: 11px; text-transform: uppercase; letter-spacing: .08em; }
  .sidebar-summary strong { font-size: 13px; }
  .file-list { min-height: 0; overflow: auto; padding: 8px; }
  .file-list button { width: 100%; display: grid; grid-template-columns: 30px minmax(0, 1fr); align-items: center; gap: 9px; padding: 10px; border: 1px solid transparent; border-radius: 7px; background: transparent; text-align: left; cursor: pointer; }
  .file-list button:hover { background: #eef3f7; }
  .file-list button.active { border-color: #b9d2e2; background: #e6f1f7; }
  .file-state { width: 26px; height: 26px; display: grid; place-items: center; border-radius: 50%; background: #fff1d6; color: #a86600; font-weight: 900; }
  .file-state.error { background: #ffe5e5; color: #c52a2a; }
  .file-state.clean { background: #ddf6e8; color: #187848; }
  .file-copy { min-width: 0; display: grid; gap: 3px; }
  .file-copy strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .file-copy small { color: #78879b; font-size: 10px; }
  .pending-file { display: flex; align-items: center; gap: 8px; padding: 12px 10px; color: #64748b; font-size: 11px; line-height: 1.4; }
  .report-panel { min-width: 0; min-height: 0; display: grid; grid-template-rows: auto auto auto minmax(0, 1fr); gap: 14px; padding: 22px 26px 24px; overflow: hidden; }
  .report-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }
  .report-head h2 { margin: 4px 0; font-size: 21px; overflow-wrap: anywhere; }
  .report-head p { margin: 0; color: #6d7c91; font-size: 12px; overflow-wrap: anywhere; }
  .result-badge { flex: 0 0 auto; padding: 7px 11px; border: 1px solid #edc77e; border-radius: 999px; background: #fff6df; color: #925c00; font-size: 11px; font-weight: 800; }
  .result-badge.clean { border-color: #91d0ad; background: #e7f8ee; color: #176d42; }
  .metrics { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; }
  .metrics article { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; padding: 13px 15px; border: 1px solid #d9e2ec; border-radius: 8px; background: #fff; }
  .metrics span { color: #758399; font-size: 11px; }
  .metrics strong { font-size: 20px; }
  .metrics .error-metric strong { color: #c52a2a; }
  .metrics .warning-metric strong { color: #a86600; }
  .issue-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .issue-toolbar h3 { margin: 0; font-size: 14px; }
  .filter-tabs { display: flex; gap: 4px; padding: 3px; border: 1px solid #d5dee8; border-radius: 7px; background: #fff; }
  .filter-tabs button { padding: 5px 9px; border: 0; border-radius: 4px; background: transparent; color: #68778d; font-size: 11px; cursor: pointer; }
  .filter-tabs button.active { background: #e4f0f7; color: #155f8a; font-weight: 800; }
  .issue-list { min-height: 0; display: grid; align-content: start; gap: 8px; overflow: auto; padding-right: 4px; }
  .issue-row { display: grid; grid-template-columns: 30px minmax(0, 1fr); gap: 10px; padding: 13px; border: 1px solid #ead8b9; border-left: 3px solid #d39228; border-radius: 7px; background: #fffdfa; }
  .issue-row.error { border-color: #eccaca; border-left-color: #cc3b3b; background: #fffafa; }
  .level-icon { width: 26px; height: 26px; display: grid; place-items: center; border-radius: 6px; background: #fff0ce; color: #9c6509; font-weight: 900; }
  .issue-row.error .level-icon { background: #ffe3e3; color: #bd2929; }
  .issue-title { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .issue-title strong { font-size: 12px; }
  .issue-title code { padding: 2px 5px; border-radius: 3px; background: #edf1f5; color: #67758a; font-size: 9px; }
  .issue-row p { margin: 5px 0 0; color: #4e5d72; font-size: 12px; line-height: 1.5; }
  .issue-path { display: block; margin-top: 7px; color: #7a889b; font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 10px; overflow-wrap: anywhere; }
  .clean-state, .no-filter-result { display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 220px; padding: 30px; border: 1px solid #d9e5de; border-radius: 10px; background: #f9fffb; text-align: center; }
  .clean-state > span { width: 46px; height: 46px; display: grid; place-items: center; margin-bottom: 12px; border-radius: 50%; background: #dbf5e6; color: #187848; font-size: 22px; font-weight: 900; }
  .clean-state p { margin: 7px 0 0; color: #718096; font-size: 12px; }
  .no-filter-result { min-height: 120px; color: #77869a; font-size: 12px; }
  .floating-error { position: fixed; right: 18px; bottom: 18px; z-index: 10; max-width: min(440px, calc(100vw - 36px)); box-shadow: 0 8px 24px rgba(80,30,30,.14); }
  @media (max-width: 760px) {
    :global(body) { overflow: auto; }
    .diagnose-page { height: auto; min-height: 100dvh; grid-template-rows: auto minmax(calc(100dvh - 62px), auto); }
    .topbar { min-height: 62px; padding: 0 14px; }
    .eyebrow { display: none; }
    .heading h1 { font-size: 16px; }
    .secondary { display: none; }
    .empty-shell { align-content: start; padding: 24px 14px; }
    .drop-zone { min-height: 220px; }
    .drop-zone strong { font-size: 18px; }
    .drop-zone > span:not(.drop-icon):not(.choose-pill) { max-width: 260px; text-align: center; line-height: 1.5; }
    .check-grid { grid-template-columns: 1fr; }
    .result-workspace { display: block; overflow: visible; }
    .file-sidebar { display: block; border-right: 0; border-bottom: 1px solid #d5dee8; }
    .file-list { display: flex; gap: 6px; padding: 8px 12px; overflow-x: auto; }
    .file-list button { min-width: 210px; }
    .pending-file { min-width: 210px; }
    .report-panel { display: grid; min-height: 0; padding: 18px 14px 28px; overflow: visible; }
    .report-head { display: grid; }
    .result-badge { justify-self: start; }
    .metrics { grid-template-columns: repeat(2, 1fr); }
    .issue-toolbar { align-items: flex-start; flex-direction: column; }
    .issue-list { overflow: visible; }
  }
</style>
