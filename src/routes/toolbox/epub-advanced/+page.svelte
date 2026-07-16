<script lang="ts">
  import { page } from "$app/stores";
  import { onDestroy } from "svelte";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import {
    listWebEpubChapterTargets,
    processWebEpubAdvanced,
    type WebEpubAdvancedAction,
    type WebEpubAdvancedOptions,
    type WebEpubAdvancedResult,
    type WebEpubChapterTarget,
  } from "$lib/webEpubAdvanced";

  const CONFIG: Record<WebEpubAdvancedAction, { title: string; icon: string; detail: string; multiple: boolean }> = {
    "epub-to-txt": { title: "EPUB 转 TXT", icon: "TXT", detail: "按 EPUB 阅读顺序提取章节正文，输出 UTF-8 纯文本。", multiple: false },
    "epub-version": { title: "EPUB 版本互转", icon: "2/3", detail: "在 EPUB 2.0 与 3.0 间转换，并自动补齐 NCX 或 NAV 导航。", multiple: false },
    "epub-chinese": { title: "EPUB 简繁互转", icon: "繁简", detail: "转换正文、目录和 OPF 元数据中的中文，不改动标签与样式。", multiple: false },
    "epub-ad-clean": { title: "EPUB 广告清理", icon: "CLEAN", detail: "按逐行正则从正文中移除短广告段落，保留正常长段落和复杂容器。", multiple: false },
    "epub-phonetic": { title: "EPUB 拼音标注", icon: "PIN", detail: "使用标准 ruby/rt 标记为正文汉字添加带声调拼音，跳过代码、样式和已有注音。", multiple: false },
    "epub-footnote": { title: "EPUB 批注与脚注", icon: "NOTE", detail: "增强标准 EPUB3 脚注的弹窗数据，或将带文本属性的弹窗批注转为标准脚注。", multiple: false },
    "image-compress": { title: "EPUB 图片压缩", icon: "IMG", detail: "浏览器本地重编码并按需缩放 JPEG、PNG 和 WebP，较大结果不会写回。", multiple: false },
    "image-watermark": { title: "EPUB 图片水印", icon: "WM", detail: "把带校验的 UTF-8 文本写入图片 RGB 最低位，或扫描读取 TEpub 水印。", multiple: false },
    "epub-merge": { title: "EPUB 合并", icon: "MERGE", detail: "保留各书资源目录和相对引用，重新生成合集 manifest、spine、NAV 与 NCX。", multiple: true },
    "epub-split": { title: "EPUB 拆分", icon: "SPLIT", detail: "预览阅读顺序条目，按指定章节数生成多本结构完整的 EPUB。", multiple: false },
  };
  const ACTIONS = new Set<WebEpubAdvancedAction>(Object.keys(CONFIG) as WebEpubAdvancedAction[]);

  let fileInput: HTMLInputElement | null = null;
  let files: File[] = [];
  let busy = false;
  let dragActive = false;
  let status = "";
  let errorText = "";
  let result: WebEpubAdvancedResult | null = null;
  let activeController: AbortController | null = null;
  let outputUrls: Array<{ name: string; message: string; url: string }> = [];
  let chapterTargets: WebEpubChapterTarget[] = [];
  let options: WebEpubAdvancedOptions = {
    targetVersion: "3",
    chineseDirection: "s2t",
    imageQuality: 0.78,
    maxImageDimension: 2400,
    watermarkMode: "embed",
    watermarkText: "",
    outputTitle: "",
    splitEvery: 20,
    adPatterns: "",
    footnoteMode: "standard-to-popup",
  };

  $: rawAction = $page.url.searchParams.get("tool") as WebEpubAdvancedAction | null;
  $: action = rawAction && ACTIONS.has(rawAction) ? rawAction : "epub-to-txt";
  $: config = CONFIG[action];

  function clearOutputs() {
    for (const item of outputUrls) URL.revokeObjectURL(item.url);
    outputUrls = [];
    result = null;
  }

  async function setFiles(next: File[]) {
    const accepted = next.filter((file) => file.name.toLowerCase().endsWith(".epub"));
    files = config.multiple ? accepted : accepted.slice(0, 1);
    errorText = accepted.length ? "" : "请选择 EPUB 文件。";
    status = "";
    chapterTargets = [];
    clearOutputs();
    if (action === "epub-split" && files[0]) {
      status = "正在读取章节目标...";
      try {
        chapterTargets = await listWebEpubChapterTargets(files[0]);
        status = `已识别 ${chapterTargets.length} 个阅读顺序条目`;
      } catch (error) {
        errorText = error instanceof Error ? error.message : String(error);
        status = "";
      }
    }
  }

  function handleFileChange(event: Event) {
    void setFiles(Array.from((event.currentTarget as HTMLInputElement).files || []));
    if (fileInput) fileInput.value = "";
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (!busy) void setFiles(Array.from(event.dataTransfer?.files || []));
  }

  async function run() {
    if (!files.length || busy) return;
    clearOutputs();
    busy = true;
    activeController = new AbortController();
    errorText = "";
    status = `正在执行${config.title}...`;
    try {
      result = await processWebEpubAdvanced(files, action, {
        ...options,
        signal: activeController.signal,
        onProgress: (completed, total) => { status = `正在执行${config.title}：${completed} / ${total}`; },
      });
      outputUrls = result.outputs.map((output) => ({ ...output, url: URL.createObjectURL(output.blob) }));
      status = result.message;
    } catch (error) {
      errorText = error instanceof Error ? error.message : String(error);
      status = "处理未完成";
    } finally {
      busy = false;
      activeController = null;
    }
  }

  function cancelRun() {
    activeController?.abort();
  }

  function download(item: { name: string; url: string }) {
    const anchor = document.createElement("a");
    anchor.href = item.url;
    anchor.download = item.name;
    anchor.click();
  }

  async function downloadAll() {
    for (const item of outputUrls) {
      download(item);
      await new Promise((resolve) => setTimeout(resolve, 180));
    }
  }

  onDestroy(() => {
    activeController?.abort();
    clearOutputs();
  });
</script>

<svelte:head>
  <title>{config.title} - TEpub Editor</title>
  <meta name="description" content={config.detail} />
</svelte:head>

<div class="advanced-page">
  <input bind:this={fileInput} class="file-input" type="file" accept=".epub,application/epub+zip" multiple={config.multiple} on:change={handleFileChange} />

  <main class="content">
    <section class="intro">
      <span class="mark">{config.icon}</span>
      <div><span class="kicker">DESKTOP + WEB</span><h2>{config.title}</h2><p>{config.detail}</p></div>
      <div class="privacy"><strong>本地处理</strong><span>文件不会上传服务器</span></div>
    </section>

    {#if action === "epub-version"}
      <section class="options"><div><strong>目标版本</strong><span>转换时保留兼容导航文件</span></div><div class="segments"><button class:active={options.targetVersion === "2"} on:click={() => options = { ...options, targetVersion: "2" }}>EPUB 2.0</button><button class:active={options.targetVersion === "3"} on:click={() => options = { ...options, targetVersion: "3" }}>EPUB 3.0</button></div></section>
    {:else if action === "epub-chinese"}
      <section class="options"><div><strong>转换方向</strong><span>同时处理正文、目录与元数据</span></div><div class="segments"><button class:active={options.chineseDirection === "s2t"} on:click={() => options = { ...options, chineseDirection: "s2t" }}>简体转繁体</button><button class:active={options.chineseDirection === "t2s"} on:click={() => options = { ...options, chineseDirection: "t2s" }}>繁体转简体</button></div></section>
    {:else if action === "epub-ad-clean"}
      <section class="options pattern-options"><label class="wide"><span>广告段落正则（每行一条）</span><textarea rows="5" value={options.adPatterns} placeholder="留空使用内置安全规则；自定义时每行一条正则" on:input={(event) => options = { ...options, adPatterns: (event.currentTarget as HTMLTextAreaElement).value }}></textarea></label></section>
    {:else if action === "epub-footnote"}
      <section class="options"><div><strong>转换方式</strong><span>标准脚注保留 aside 作为兼容兜底</span></div><div class="segments"><button class:active={options.footnoteMode === "standard-to-popup"} on:click={() => options = { ...options, footnoteMode: "standard-to-popup" }}>脚注弹窗增强</button><button class:active={options.footnoteMode === "popup-to-standard"} on:click={() => options = { ...options, footnoteMode: "popup-to-standard" }}>弹窗转标准脚注</button></div></section>
    {:else if action === "image-compress"}
      <section class="options image-options"><label><span>编码质量 {Math.round((options.imageQuality || 0.78) * 100)}%</span><input type="range" min="0.35" max="0.95" step="0.01" value={options.imageQuality} on:input={(event) => options = { ...options, imageQuality: Number((event.currentTarget as HTMLInputElement).value) }} /></label><label><span>最长边限制</span><CustomSelect value={String(options.maxImageDimension ?? 2400)} options={[{ value: "0", label: "不缩放" }, { value: "1600", label: "1600 px" }, { value: "2400", label: "2400 px" }, { value: "3200", label: "3200 px" }]} ariaLabel="图片最长边限制" on:change={(event) => options = { ...options, maxImageDimension: Number(event.detail) }} /></label></section>
    {:else if action === "image-watermark"}
      <section class="options watermark-options"><div class="segments"><button class:active={options.watermarkMode === "embed"} on:click={() => options = { ...options, watermarkMode: "embed" }}>写入水印</button><button class:active={options.watermarkMode === "inspect"} on:click={() => options = { ...options, watermarkMode: "inspect" }}>读取水印</button></div>{#if options.watermarkMode === "embed"}<label><span>水印文本</span><input type="text" value={options.watermarkText} maxlength="500" placeholder="输入需要隐形写入图片的信息" on:input={(event) => options = { ...options, watermarkText: (event.currentTarget as HTMLInputElement).value }} /></label>{/if}</section>
    {:else if action === "epub-merge"}
      <section class="options"><label class="wide"><span>合集书名</span><input type="text" value={options.outputTitle} placeholder="默认使用第一本书名 + 合集" on:input={(event) => options = { ...options, outputTitle: (event.currentTarget as HTMLInputElement).value }} /></label></section>
    {:else if action === "epub-split"}
      <section class="options"><label><span>每个输出包含的章节数</span><input type="number" min="1" max="500" value={options.splitEvery} on:input={(event) => options = { ...options, splitEvery: Math.max(1, Number((event.currentTarget as HTMLInputElement).value) || 1) }} /></label><div><strong>{chapterTargets.length} 个可拆分条目</strong><span>预计生成 {chapterTargets.length ? Math.ceil(chapterTargets.length / Math.max(1, options.splitEvery || 1)) : 0} 个文件</span></div></section>
    {/if}

    {#if !files.length}
      <button class="drop-zone" class:drag-active={dragActive} type="button" disabled={busy} on:click={() => fileInput?.click()} on:dragenter={(event) => { event.preventDefault(); dragActive = true; }} on:dragover={(event) => event.preventDefault()} on:dragleave={() => dragActive = false} on:drop={handleDrop}>
        <span class="drop-icon">+</span><strong>{config.multiple ? "选择或拖入多个 EPUB" : "选择或拖入 EPUB"}</strong><span>{config.multiple ? "文件顺序将作为合并后的阅读顺序" : "安装版和 Web 使用同一浏览器本地处理内核"}</span><b>选择文件</b>
      </button>
    {:else}
      <section class="selection">
        <div><span class="kicker">SELECTED FILES</span><h3>{files.length} 个 EPUB</h3></div>
        <div class="file-list">{#each files as file, index}<span><b>{index + 1}</b>{file.name}<small>{(file.size / 1024 / 1024).toFixed(2)} MB</small></span>{/each}</div>
        {#if busy}
          <button class="run" type="button" on:click={cancelRun}>取消处理</button>
        {:else}
          <button class="primary run" type="button" disabled={action === "epub-merge" && files.length < 2} on:click={run}>{options.watermarkMode === "inspect" && action === "image-watermark" ? "扫描水印" : "开始处理"}</button>
        {/if}
      </section>
    {/if}

    {#if chapterTargets.length && action === "epub-split"}
      <section class="chapters"><header><strong>拆分目标预览</strong><span>{status}</span></header><div>{#each chapterTargets as chapter}<span><b>{chapter.index + 1}</b><em>{chapter.title}</em><small>{chapter.path}</small></span>{/each}</div></section>
    {/if}

    {#if status || errorText || result}
      <section class:error={Boolean(errorText)} class="result" aria-live="polite"><header><div><span class="kicker">PROCESS RESULT</span><h3>{errorText || status}</h3></div></header>
        {#if result?.report}<pre>{result.report}</pre>{/if}
        {#if outputUrls.length}<div class="outputs">{#each outputUrls as output}<article><div><strong>{output.name}</strong><span>{output.message}</span></div><button type="button" on:click={() => download(output)}>下载</button></article>{/each}</div>{/if}
      </section>
    {/if}
  </main>
</div>

<!-- Legacy header selectors share the compact one-line stylesheet; the header markup was intentionally removed. -->
<!-- svelte-ignore css_unused_selector -->
<style>
  :global(body){margin:0;background:#edf1f5;color:#172033;font-family:Inter,"Microsoft YaHei",sans-serif}button,input,textarea,a{font:inherit}button{color:inherit}.advanced-page{min-height:100dvh}.topbar{min-height:68px;display:flex;align-items:center;justify-content:space-between;gap:18px;padding:0 24px;border-bottom:1px solid #d8e0e9;background:rgba(255,255,255,.96)}.heading,.head-actions{display:flex;align-items:center;gap:12px}.back{width:36px;height:36px;display:grid;place-items:center;border:1px solid #d6dee8;border-radius:8px;color:#43536b;background:#fff}.back svg{width:20px;fill:none;stroke:currentColor;stroke-width:2}.heading span,.kicker{color:#7b899d;font-size:9px;font-weight:800;letter-spacing:.15em}.heading h1{margin:2px 0 0;font-size:18px}.head-actions button,.primary{min-height:36px;padding:0 16px;border:1px solid #d1dae5;border-radius:7px;background:#fff;font-size:11px;font-weight:800;cursor:pointer}.head-actions .primary,.primary{border-color:#17699a;background:#17699a;color:#fff}button:disabled{cursor:wait;opacity:.55}.file-input{position:fixed;width:1px;height:1px;opacity:0;pointer-events:none}.content{width:min(1040px,calc(100% - 36px));margin:26px auto 40px;display:grid;gap:14px}.intro{display:grid;grid-template-columns:70px minmax(0,1fr) auto;align-items:center;gap:18px;padding:22px;border:1px solid #d8e1eb;border-radius:10px;background:#fff}.mark{width:64px;height:64px;display:grid;place-items:center;border-radius:12px;background:#e4f1f8;color:#17699a;font-size:10px;font-weight:900}.intro h2{margin:4px 0 7px;font-size:22px}.intro p{margin:0;color:#4d5d72;font-size:13px;line-height:1.55}.privacy{display:grid;gap:4px;padding-left:20px;border-left:1px solid #e0e6ed}.privacy strong{color:#27704b;font-size:11px}.privacy span{color:#8491a2;font-size:9px}.options{display:flex;align-items:center;justify-content:space-between;gap:20px;padding:15px 18px;border:1px solid #d8e1eb;border-radius:8px;background:#fff}.options>div,.options label{display:grid;gap:4px}.options strong,.options label>span{font-size:11px}.options div>span{color:#7e8c9f;font-size:9px}.segments{display:flex!important;grid-auto-flow:column;gap:3px!important;padding:3px;border:1px solid #d5dee8;border-radius:7px;background:#f5f7fa}.segments button{min-height:31px;padding:0 12px;border:0;border-radius:4px;background:transparent;font-size:10px;cursor:pointer}.segments button.active{background:#fff;color:#17699a;box-shadow:0 1px 4px rgba(28,58,84,.12);font-weight:800}.options input,.options textarea{box-sizing:border-box;min-height:34px;border:1px solid #d1dbe6;border-radius:6px;padding:0 10px;background:#fff}.options textarea{padding:9px;resize:vertical;font:10px/1.5 ui-monospace,Consolas,monospace}.options label.wide{width:100%}.image-options{display:grid;grid-template-columns:minmax(0,1fr) 180px}.image-options input[type="range"]{width:100%;padding:0;border:0}.watermark-options{align-items:end}.watermark-options label{flex:1}.drop-zone{min-height:280px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;border:1px dashed #9eafc4;border-radius:10px;background:rgba(255,255,255,.8);cursor:pointer}.drop-zone:hover,.drop-zone.drag-active{border-color:#17699a;background:#fff}.drop-icon{width:48px;height:48px;display:grid;place-items:center;border-radius:50%;background:#e4f1f8;color:#17699a;font-size:27px}.drop-zone strong{font-size:17px}.drop-zone>span:not(.drop-icon){color:#748298;font-size:11px}.drop-zone b{margin-top:6px;padding:8px 16px;border-radius:6px;background:#17699a;color:#fff;font-size:10px}.selection{display:grid;grid-template-columns:150px minmax(0,1fr) auto;align-items:start;gap:16px;padding:17px;border:1px solid #d8e1eb;border-radius:9px;background:#fff}.selection h3{margin:4px 0 0;font-size:14px}.file-list{max-height:180px;display:grid;gap:5px;overflow:auto}.file-list>span{min-width:0;display:grid;grid-template-columns:24px minmax(0,1fr) auto;gap:8px;align-items:center;padding:7px 9px;border-radius:5px;background:#f5f7fa;font-size:10px}.file-list b{width:20px;height:20px;display:grid;place-items:center;border-radius:4px;background:#e0ebf2;color:#17699a;font-size:8px}.file-list small{color:#8390a2}.run{align-self:center}.chapters,.result{border:1px solid #d8e1eb;border-radius:9px;background:#fff;overflow:hidden}.chapters>header,.result>header{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:13px 16px;border-bottom:1px solid #e2e7ed}.chapters>header strong{font-size:12px}.chapters>header span{color:#79879a;font-size:9px}.chapters>div{max-height:310px;overflow:auto}.chapters>div>span{display:grid;grid-template-columns:34px minmax(120px,.6fr) minmax(180px,1fr);gap:9px;padding:8px 15px;border-bottom:1px solid #edf1f4;font-size:9px}.chapters b{color:#17699a}.chapters em{font-style:normal;font-weight:700}.chapters small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#8491a2}.result.error{border-color:#e7bebe;background:#fffafa}.result h3{margin:4px 0 0;font-size:13px}.result pre{max-height:280px;margin:0;padding:16px;overflow:auto;white-space:pre-wrap;font:11px/1.6 ui-monospace,Consolas,monospace}.outputs{display:grid;gap:7px;padding:11px}.outputs article{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:11px 13px;border:1px solid #dce4ec;border-radius:7px}.outputs article div{min-width:0;display:grid;gap:3px}.outputs strong{overflow-wrap:anywhere;font-size:11px}.outputs span{color:#7b899b;font-size:9px}.outputs button{min-height:33px;padding:0 13px;border:1px solid #26704b;border-radius:5px;background:#26704b;color:#fff;font-size:10px;font-weight:800;cursor:pointer}
  @media(max-width:720px){.topbar{min-height:62px;padding:0 13px}.heading span{display:none}.heading h1{font-size:15px}.content{width:calc(100% - 24px);margin:14px auto 28px}.intro{grid-template-columns:52px minmax(0,1fr);padding:16px;gap:12px}.mark{width:50px;height:50px}.intro h2{font-size:18px}.privacy{grid-column:1/-1;padding:10px 0 0;border-left:0;border-top:1px solid #e0e6ed}.options,.watermark-options{align-items:stretch;flex-direction:column}.image-options{grid-template-columns:1fr}.segments{align-self:stretch}.segments button{flex:1}.selection{grid-template-columns:1fr}.run{width:100%}.chapters>div>span{grid-template-columns:28px minmax(0,1fr)}.chapters small{grid-column:2}.head-actions button:not(.primary){display:none}.drop-zone{text-align:center;padding:18px}.outputs article{align-items:flex-start;flex-direction:column}.outputs button{width:100%}}
</style>
