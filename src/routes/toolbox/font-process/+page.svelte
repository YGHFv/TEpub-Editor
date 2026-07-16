<script lang="ts">
  import { page } from "$app/stores";
  import { onDestroy } from "svelte";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import { processWebEpubFont, type WebEpubFontAction, type WebEpubFontProcessResult } from "$lib/webEpubFontProcess";

  let epubInput: HTMLInputElement | null = null;
  let txtInput: HTMLInputElement | null = null;
  let epubFile: File | null = null;
  let plainTxtFile: File | null = null;
  let txtEncoding = "utf-8";
  let busy = false;
  let dragActive = false;
  let progress = "";
  let errorText = "";
  let result: (WebEpubFontProcessResult & { url: string }) | null = null;

  $: rawAction = $page.url.searchParams.get("tool");
  $: action = (rawAction === "font-decrypt" || rawAction === "font-subset" ? rawAction : "font-encrypt") as WebEpubFontAction;
  $: decryptMode = action === "font-decrypt";
  $: subsetMode = action === "font-subset";
  $: title = decryptMode ? "EPUB 字体解密" : subsetMode ? "EPUB 字体精简" : "EPUB 字体加密";
  $: description = decryptMode
    ? "恢复 EPUB 中通过私用区字体映射混淆的正文；优先读取内置映射或字体 cmap，也可使用同版本明文 TXT 对齐。"
    : subsetMode
      ? "扫描整本 EPUB 正文实际使用的 Unicode 码点，裁剪未使用字形以减小内嵌字体体积。"
      : "将正文汉字替换为私用区字符，并把对应 cmap 写入 EPUB 内嵌字体；映射随文件保存，可跨端恢复。";

  function clearResult() {
    if (result) URL.revokeObjectURL(result.url);
    result = null;
  }

  async function readPlainText() {
    if (!plainTxtFile) return undefined;
    const bytes = await plainTxtFile.arrayBuffer();
    if (txtEncoding === "auto") {
      try { return new TextDecoder("utf-8", { fatal: true }).decode(bytes); }
      catch { return new TextDecoder("gb18030").decode(bytes); }
    }
    return new TextDecoder(txtEncoding).decode(bytes);
  }

  async function run(file = epubFile) {
    if (!file) return;
    epubFile = file;
    clearResult();
    busy = true;
    errorText = "";
    progress = decryptMode ? "正在分析字体映射并恢复正文…" : subsetMode ? "正在收集正文码点并裁剪字体…" : "正在分析正文与内嵌字体…";
    try {
      const processed = await processWebEpubFont(file, action, await readPlainText());
      result = { ...processed, url: URL.createObjectURL(processed.blob) };
      progress = processed.message;
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
      progress = "处理未完成";
    } finally {
      busy = false;
      if (epubInput) epubInput.value = "";
    }
  }

  function handleEpubChange(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0] || null;
    if (file) void run(file);
  }

  function handleTxtChange(event: Event) {
    plainTxtFile = (event.currentTarget as HTMLInputElement).files?.[0] || null;
    errorText = "";
    if (epubFile && plainTxtFile) void run();
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (busy) return;
    const files = Array.from(event.dataTransfer?.files || []);
    const epub = files.find((file) => file.name.toLowerCase().endsWith(".epub"));
    const txt = files.find((file) => file.name.toLowerCase().endsWith(".txt"));
    if (txt) plainTxtFile = txt;
    if (epub) void run(epub);
    else errorText = "请拖入 EPUB 文件。";
  }

  function download() {
    if (!result) return;
    const anchor = document.createElement("a");
    anchor.href = result.url;
    anchor.download = result.outputName;
    anchor.click();
  }

  onDestroy(clearResult);
</script>

<svelte:head>
  <title>Web {title} - TEpub Editor</title>
  <meta name="description" content={description} />
</svelte:head>

<div class="font-page">
  <input bind:this={epubInput} class="file-input" type="file" accept=".epub,application/epub+zip" on:change={handleEpubChange} />
  <input bind:this={txtInput} class="file-input" type="file" accept=".txt,text/plain" on:change={handleTxtChange} />

  <main class="content">
    <section class="intro-card">
      <span class="font-mark">FONT{decryptMode ? "−" : subsetMode ? "S" : "+"}</span>
      <div><span class="kicker">BROWSER LOCAL FONT TOOL</span><h2>{title}</h2><p>{description}</p></div>
      <div class="privacy"><strong>本地处理</strong><span>EPUB 与字体不会上传</span></div>
    </section>

    {#if decryptMode}
      <section class="txt-assist">
        <div><strong>明文 TXT 辅助对齐</strong><span>仅在 EPUB 没有映射且字体 cmap 无法直接恢复时需要</span></div>
        <div class="txt-controls">
          <CustomSelect
            value={txtEncoding}
            options={[{ value: "auto", label: "自动识别" }, { value: "utf-8", label: "UTF-8" }, { value: "gb18030", label: "GB18030" }, { value: "big5", label: "Big5" }]}
            ariaLabel="TXT 编码"
            on:change={(event) => (txtEncoding = event.detail)}
          />
          <button type="button" disabled={busy} on:click={() => txtInput?.click()}>{plainTxtFile ? plainTxtFile.name : "选择 TXT"}</button>
          {#if plainTxtFile}<button class="clear" type="button" disabled={busy} title="移除 TXT" on:click={() => { plainTxtFile = null; }}>×</button>{/if}
        </div>
      </section>
    {/if}

    {#if !epubFile && !busy}
      <button class="drop-zone" class:drag-active={dragActive} type="button" on:click={() => epubInput?.click()} on:dragenter={(event) => { event.preventDefault(); dragActive = true; }} on:dragover={(event) => event.preventDefault()} on:dragleave={() => { dragActive = false; }} on:drop={handleDrop}>
        <span class="drop-icon">Aa</span><strong>选择或拖入 EPUB 文件</strong><span>{decryptMode ? "也可以同时拖入对应的明文 TXT" : subsetMode ? "自动保留正文所需字形和基础 ASCII 字符" : "支持 TTF、OTF、WOFF 与 WOFF2 内嵌字体"}</span><span class="choose">选择文件</span>
      </button>
    {:else}
      <section class="status-card" class:error={Boolean(errorText)} class:success={Boolean(result)} aria-live="polite">
        {#if busy}<span class="spinner"></span>{:else if result}<span class="state">✓</span>{:else}<span class="state">×</span>{/if}
        <div class="status-copy"><span class="kicker">PROCESS STATUS</span><h3>{epubFile?.name || "EPUB"}</h3><p>{errorText || progress}</p>
          {#if result}<small>{result.mappedCharacters} 个{subsetMode ? "保留码点" : "字符映射"} · {result.changedFonts} 个字体 · {result.changedFiles} 个正文 · {result.mode}</small>{/if}
        </div>
        {#if result}<button class="download" type="button" on:click={download}>下载结果</button>{/if}
      </section>
      {#if errorText && decryptMode && !plainTxtFile}
        <button class="txt-callout" type="button" on:click={() => txtInput?.click()}><strong>使用明文 TXT 重试</strong><span>请选择与该 EPUB 内容、章节顺序一致的 TXT 文件</span></button>
      {/if}
    {/if}

    <section class="notes">
      <article><b>01</b><div><strong>{decryptMode ? "优先自动恢复" : subsetMode ? "整书扫描" : "正文映射"}</strong><p>{decryptMode ? "先读取 TEpub 映射，再检查字体 cmap，无需 TXT 时不会要求上传。" : subsetMode ? "从 HTML/XHTML 正文收集码点，并额外保留基础 ASCII 字符。" : "仅替换正文文本节点中的汉字，不修改标签、脚本、样式或 HTML 实体。"}</p></div></article>
      <article><b>02</b><div><strong>{decryptMode ? "TXT 对齐兜底" : subsetMode ? "仅在变小时写回" : "字体写回"}</strong><p>{decryptMode ? "旧式第三方混淆无法从 cmap 反推时，以同版本 TXT 生成可靠映射。" : subsetMode ? "裁剪结果不比原字体小时保持原文件，避免无意义增大 EPUB。" : "将私用区编码追加到对应字形 cmap，保留原始编码以兼容阅读器。"}</p></div></article>
      <article><b>03</b><div><strong>格式兼容</strong><p>支持 TTF、OTF、WOFF、WOFF2；OTF 修改后会转换为 TTF 并同步更新 EPUB 引用。</p></div></article>
    </section>
  </main>
</div>

<style>
  :global(body) { margin: 0; background: #edf1f5; color: #172033; font-family: Inter, "Microsoft YaHei", sans-serif; }
  button { font: inherit; }
  button { color: inherit; }
  .font-page { min-height: 100dvh; }
  .kicker { color: #7b899d; font-size: 9px; font-weight: 800; letter-spacing: .15em; }
  button:disabled { cursor: wait; opacity: .6; }
  .file-input { position: fixed; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
  .content { width: min(960px, calc(100% - 36px)); margin: 26px auto; display: grid; gap: 14px; }
  .intro-card { display: grid; grid-template-columns: 72px minmax(0, 1fr) auto; align-items: center; gap: 18px; padding: 22px; border: 1px solid #d8e1eb; border-radius: 12px; background: #fff; }
  .font-mark { width: 66px; height: 66px; display: grid; place-items: center; border-radius: 14px; background: #e7ecfa; color: #4059a1; font-size: 12px; font-weight: 900; }
  .intro-card h2 { margin: 4px 0 7px; font-size: 22px; }
  .intro-card p { margin: 0; color: #4c5c72; font-size: 13px; line-height: 1.55; }
  .privacy { display: grid; gap: 4px; padding-left: 20px; border-left: 1px solid #e0e6ed; }
  .privacy strong { color: #28704b; font-size: 11px; }
  .privacy span { color: #8491a2; font-size: 9px; }
  .txt-assist { display: flex; align-items: center; justify-content: space-between; gap: 18px; padding: 15px 18px; border: 1px solid #d8e1eb; border-radius: 9px; background: #fff; }
  .txt-assist > div:first-child { display: grid; gap: 4px; }
  .txt-assist strong { font-size: 12px; }
  .txt-assist span { color: #7e8c9f; font-size: 10px; }
  .txt-controls { display: flex; gap: 6px; min-width: 0; }
  .txt-controls :global(.custom-select) { width: 132px; flex: 0 0 132px; }
  .txt-controls :global(.custom-select-trigger), .txt-controls button { min-height: 34px; border: 1px solid #d1dbe6; border-radius: 6px; background: #fff; font-size: 10px; }
  .txt-controls button { padding: 0 10px; }
  .txt-controls button { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; }
  .txt-controls .clear { width: 34px; padding: 0; color: #a33; }
  .drop-zone { min-height: 285px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; border: 1px dashed #9eafc4; border-radius: 12px; background: rgba(255,255,255,.8); cursor: pointer; transition: .18s ease; }
  .drop-zone:hover, .drop-zone.drag-active { border-color: #4059a1; background: #fff; box-shadow: 0 10px 30px rgba(50,66,110,.1); }
  .drop-icon { width: 52px; height: 52px; display: grid; place-items: center; border-radius: 13px; background: #e7ecfa; color: #4059a1; font-family: Georgia, serif; font-size: 20px; font-weight: 700; }
  .drop-zone strong { font-size: 18px; }
  .drop-zone > span:not(.drop-icon):not(.choose) { color: #738298; font-size: 11px; }
  .choose { margin-top: 8px; padding: 8px 17px; border-radius: 6px; background: #4059a1; color: #fff; font-size: 11px; font-weight: 800; }
  .status-card { display: grid; grid-template-columns: 44px minmax(0, 1fr) auto; align-items: center; gap: 14px; min-height: 130px; padding: 20px; border: 1px solid #d8e1eb; border-radius: 11px; background: #fff; }
  .status-card.success { border-color: #bddfcb; background: #fbfffc; }
  .status-card.error { border-color: #e9c6c6; background: #fffafa; }
  .state { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; background: #e1f5e9; color: #207447; font-size: 18px; font-weight: 900; }
  .error .state { background: #ffe3e3; color: #c42d2d; }
  .status-copy { min-width: 0; }
  .status-copy h3 { margin: 4px 0; font-size: 15px; overflow-wrap: anywhere; }
  .status-copy p { margin: 0; color: #526176; font-size: 11px; line-height: 1.5; }
  .status-copy small { display: block; margin-top: 6px; color: #8390a2; font-size: 9px; }
  .download { min-height: 36px; padding: 0 15px; border: 1px solid #28704b; border-radius: 6px; background: #28704b; color: #fff; font-size: 11px; font-weight: 800; cursor: pointer; }
  .spinner { width: 30px; height: 30px; border: 3px solid #d8e1eb; border-top-color: #4059a1; border-radius: 50%; animation: spin .75s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .txt-callout { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 14px 17px; border: 1px solid #e1c78f; border-radius: 8px; background: #fff9e9; color: #805b13; cursor: pointer; }
  .txt-callout strong { font-size: 11px; }.txt-callout span { font-size: 9px; }
  .notes { display: grid; grid-template-columns: repeat(3, 1fr); gap: 9px; }
  .notes article { display: grid; grid-template-columns: 28px 1fr; gap: 8px; padding: 13px; border: 1px solid #dbe3ec; border-radius: 8px; background: rgba(255,255,255,.72); }
  .notes b { color: #4059a1; font-size: 10px; }.notes strong { font-size: 11px; }.notes p { margin: 4px 0 0; color: #758398; font-size: 9px; line-height: 1.5; }
  @media (max-width: 700px) {
    .content { width: calc(100% - 24px); margin: 14px auto 28px; }
    .intro-card { grid-template-columns: 52px minmax(0, 1fr); padding: 16px; gap: 12px; }.font-mark { width: 50px; height: 50px; }.intro-card h2 { font-size: 18px; }
    .privacy { grid-column: 1 / -1; padding: 10px 0 0; border-left: 0; border-top: 1px solid #e0e6ed; }
    .txt-assist { align-items: flex-start; flex-direction: column; }.txt-controls { width: 100%; flex-wrap: wrap; }.txt-controls button:not(.clear) { flex: 1; }
    .drop-zone { min-height: 250px; padding: 18px; }.drop-zone > span:not(.drop-icon):not(.choose) { text-align: center; line-height: 1.5; }
    .status-card { grid-template-columns: 40px minmax(0, 1fr); }.download { grid-column: 2; justify-self: start; }
    .txt-callout { align-items: flex-start; flex-direction: column; }.notes { grid-template-columns: 1fr; }
  }
</style>
