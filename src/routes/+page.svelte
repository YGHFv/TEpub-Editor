<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open, save, message, ask } from '@tauri-apps/plugin-dialog';
  import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Editor from "$lib/Editor.svelte";
  import ContextMenu from "$lib/ContextMenu.svelte";

  // --- 接口定义 ---
  interface RawChapter { title: string; line_number: number; toc_type: "Volume" | "Chapter" | "Meta"; word_count: number; }
  interface TocNode { id: string; title: string; line_number: number; type: "Volume" | "Chapter" | "Meta"; word_count: number; children: TocNode[]; expanded: boolean; parentId?: string; }
  interface MatchLocation { line: number; start_char: number; end_char: number; }
  interface SearchResult { found: boolean; count: number; matches: MatchLocation[]; }
  interface FlatNode { id: string; line: number; parentId?: string; title: string; type: "Volume" | "Chapter" | "Meta"; word_count: number; }
  interface CheckItem { id: string; title: string; line: number; msg: string; val: number | string; parentId?: string; }
  interface HistoryMeta { filename: string; path: string; timestamp: number; size: number; }

  // --- 状态变量 ---
  let showEpubModal = false;
  let epubMeta = {
      title: "书名", creator: "作者", publisher: "出版社",
      date: new Date().toISOString().split('T')[0],
      uuid: crypto.randomUUID(), md5: "", cover_path: ""
  };
  let isEpubGenerating = false;
  let isEpubSuccess = false;

  const DEFAULT_SETTINGS = {
      volRegex: "^\\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*",
      chapRegex: "^\\s*(第[一二三四五六七八九十百千万0-9]+[章回]|Chapter\\s*\\d+).*",
      metaRegex: "^\\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*",
      wordCountThreshold: 8000,
      clearHistoryOnSave: false
  };

  let filePath = "请打开一本小说...";
  let fileContent = "";
  let isLoading = false;
  let isScanning = false;
  let tocTree: TocNode[] = [];
  let flatToc: FlatNode[] = []; 
  let editorComponent: Editor;
  
  let isModified = false;
  let isLoadingFile = false;
  let isSaving = false;
  let hasInitialized = false;
  let stats = { volumes: 0, chapters: 0 };

  let isAllCollapsed = false;
  let activeChapterId = "";
  let showSidebar = true; 
  let isMobile = false; 

  let isCheckModeOn = false; 
  let invalidSequenceIds = new Set<string>(); 
  let appSettings = { ...DEFAULT_SETTINGS };
  let showSettingsPanel = false;

  let showCheckPanel = false;
  let sequenceErrors: CheckItem[] = [];
  let titleErrors: CheckItem[] = [];
  let wordCountErrors: CheckItem[] = [];
  let expandSeq = true; let expandTitle = true; let expandWord = true;
  let longPressTimer: any;

  let showHistoryPanel = false;
  let historyList: HistoryMeta[] = [];

  let showFindReplace = false;
  let findPattern = ""; let replacePattern = ""; let replaceMsg = ""; let isRegex = false;
  let allMatches: MatchLocation[] = []; let currentMatchIndex = -1; let lastSearchKey = "";

  let dialogPos = { x: 0, y: 0 };
  let checkPanelPos = { x: 0, y: 0 };
  let settingsPanelPos = { x: 0, y: 0 };
  let historyPanelPos = { x: 0, y: 0 };
  let epubModalPos = { x: 0, y: 0 };
  let isDragging = false;
  let draggingTarget = ""; 
  let dragOffset = { x: 0, y: 0 };

  let autoRefreshTimer: any; 

  onMount(async () => {
    if (window.innerWidth < 768) { isMobile = true; showSidebar = false; }
    const storedSettings = localStorage.getItem("app-settings");
    if (storedSettings) try { appSettings = { ...DEFAULT_SETTINGS, ...JSON.parse(storedSettings) }; } catch(e){}

    const savedState = localStorage.getItem("app-crash-recovery");
    if (savedState) {
        try {
            const state = JSON.parse(savedState);
            if (state.filePath && state.filePath !== "请打开一本小说...") {
                filePath = state.filePath;
                if (state.isModified && state.content) { 
                    fileContent = state.content; isModified = true; 
                } else { 
                    try { fileContent = await readTextFile(filePath); } catch(e) {}
                }
                if (fileContent) {
                    editorComponent?.resetDoc(fileContent);
                    await scanToc(fileContent);
                    updateMd5(fileContent);
                    if (state.scrollLine) setTimeout(() => editorComponent?.scrollToLine(state.scrollLine), 200);
                }
            }
        } catch (e) {}
    }
    setTimeout(() => { hasInitialized = true; }, 500);

    const appWindow = getCurrentWindow();
    const unlisten = await appWindow.onCloseRequested(async (event) => {
        if (isModified) {
            event.preventDefault();
            const confirmed = await ask('当前文件有未保存的修改，确定要退出吗？', { title: '未保存警告', kind: 'warning' });
            if (confirmed) { 
                localStorage.removeItem("app-crash-recovery"); 
                await invoke("exit_app");
            }
        } else { 
            localStorage.removeItem("app-crash-recovery"); 
            await invoke("exit_app"); 
        }
    });
    return () => unlisten();
  });

  // --- 核心工具 ---
  function closeAllPanels() {
      showFindReplace = false; showSettingsPanel = false; showEpubModal = false; showCheckPanel = false; showHistoryPanel = false;
  }

  async function updateMd5(content: string) {
      try { epubMeta.md5 = await invoke("calculate_md5", { content }); } catch(e) {}
  }

  function saveStateToCache(line?: number) {
    if (isLoadingFile) return;
    const safeContent = fileContent.length > 2000000 ? null : fileContent;
    const state = { filePath, isModified, scrollLine: line || 0, content: isModified ? safeContent : null };
    try { localStorage.setItem("app-crash-recovery", JSON.stringify(state)); } catch (e) {}
  }

  // --- 文件操作 ---
  async function selectFile() {
    try {
      const selected = await open({ multiple: false, filters: [{ name: 'Text', extensions: ['txt', 'md'] }] });
      if (selected) {
        isLoading = true; isLoadingFile = true;
        filePath = selected as string;
        try {
            const content = await readTextFile(filePath);
            fileContent = content;
            editorComponent?.resetDoc(content);
            isModified = false; 
            updateMd5(content); 
            await scanToc(content); 
        } catch (e) { await message("读取失败: " + e, { kind: 'error' }); }
        isLoading = false;
        localStorage.removeItem("app-crash-recovery");
        setTimeout(() => { isLoadingFile = false; }, 100);
      }
    } catch (error) { isLoading = false; }
  }

  async function saveFile() {
    if (!fileContent || isSaving) return;
    isSaving = true;
    try {
        if (filePath.startsWith("请打开")) {
            const savedPath = await save({ filters: [{ name: 'Text', extensions: ['txt'] }] });
            if (!savedPath) { isSaving = false; return; }
            filePath = savedPath;
        }
        await writeTextFile(filePath, fileContent);
        await invoke("save_history", { originalPath: filePath, content: fileContent }).catch(()=>{});
        if (appSettings.clearHistoryOnSave) editorComponent?.clearHistory();
        isModified = false;
        saveStateToCache();
        updateMd5(fileContent);
        await scanToc(fileContent);
    } catch (e) { await message(`保存失败: ${e}`, { kind: 'error' }); } 
    finally { isSaving = false; }
  }

  // --- EPUB 导出 ---
  function openEpubModal() {
      if (filePath.startsWith("请打开")) { message("请先打开小说文件", {kind:'warning'}); return; }
      closeAllPanels(); 
      showEpubModal = true;
      isEpubGenerating = false; isEpubSuccess = false; 
      epubModalPos = isMobile ? { x: 0, y: 0 } : { x: window.innerWidth / 2 - 200, y: 80 };
      try {
          let decoded = decodeURIComponent(filePath).replace(/^.*primary:/, ''); 
          const name = decoded.split(/[\\/]/).pop()?.replace(/\.(txt|md)$/i, "") || "书名";
          epubMeta.title = name;
      } catch (e) { epubMeta.title = "书名"; }
      updateMd5(fileContent);
  }
  
  async function selectCover() {
      const selected = await open({ multiple: false, filters: [{ name: 'Image', extensions: ['jpg', 'jpeg', 'png'] }] });
      if (selected) epubMeta.cover_path = selected as string;
  }

  async function generateEpub() {
      if (isEpubGenerating) return;
      isEpubGenerating = true;
      try {
          const savePath = await save({ filters: [{ name: 'EPUB', extensions: ['epub'] }], defaultPath: epubMeta.title + ".epub" });
          if (!savePath) { isEpubGenerating = false; return; }

          let chapters = await invoke<RawChapter[]>("scan_chapters", { 
              content: fileContent, volreg: appSettings.volRegex, chapreg: appSettings.chapRegex, metareg: appSettings.metaRegex
          });

          // 标题冒号清洗
          const smartCleanRegex = /^(\s*(?:第[零一二三四五六七八九十百千万0-9]+[卷部章回]|Chapter\s*\d+|楔子|序[章言]?))\s*[:：]\s*/;
          chapters = chapters.map(c => {
              c.title = c.title.replace(smartCleanRegex, "$1 "); 
              return c;
          });

          await invoke("export_epub", { savePath, content: fileContent, chapters, metadata: epubMeta });
          isEpubSuccess = true;
      } catch (e) { await message("EPUB 生成失败: " + e, { kind: "error" }); } 
      finally { isEpubGenerating = false; }
  }

  // --- 目录扫描与同步 ---
  async function scanToc(textOverride?: string) {
    const text = textOverride ?? fileContent;
    if (!text) return;
    isScanning = true;
    try {
      const rawList = await invoke<RawChapter[]>("scan_chapters", { 
          content: text, volreg: appSettings.volRegex, chapreg: appSettings.chapRegex, metareg: appSettings.metaRegex
      });
      tocTree = buildTocTree(rawList);
      updateStats();
      if (isCheckModeOn) runFullCheck(); 
    } catch (e) {} finally { isScanning = false; }
  }

  function buildTocTree(list: RawChapter[]): TocNode[] {
    const tree: TocNode[] = []; flatToc = []; let curVol: TocNode | null = null; let uid = 0;
    for (const item of list) {
      const nodeId = `n-${uid++}`;
      const node: TocNode = { id: nodeId, title: item.title, line_number: item.line_number, type: item.toc_type, word_count: item.word_count, children: [], expanded: !isAllCollapsed };
      const flatItem: FlatNode = { id: nodeId, line: item.line_number, title: item.title, type: item.toc_type, word_count: item.word_count };
      if (item.toc_type === "Volume") { curVol = node; tree.push(node); flatToc.push(flatItem); } 
      else if (item.toc_type === "Chapter" && curVol) {
        node.parentId = curVol.id; curVol.children.push(node); flatToc.push({ ...flatItem, parentId: curVol.id });
      } else { tree.push(node); flatToc.push(flatItem); }
    }
    return tree;
  }

  function updateStats() {
    let v = 0, c = 0;
    tocTree.forEach(n => { if (n.type === "Volume") { v++; c += n.children.length; } else if (n.type === "Chapter") c++; });
    stats = { volumes: v, chapters: c };
  }

  async function handleScroll(line: number) {
    saveStateToCache(line);
    if (flatToc.length === 0) return;
    let found: FlatNode | null = null;
    for (let i = flatToc.length - 1; i >= 0; i--) { if (flatToc[i].line <= line) { found = flatToc[i]; break; } }
    if (found && found.id !== activeChapterId) {
        activeChapterId = found.id;
        if (found.parentId) {
            const p = tocTree.find(n => n.id === found!.parentId);
            if (p && !p.expanded) { p.expanded = true; tocTree = [...tocTree]; await tick(); }
        }
        const el = document.getElementById(`toc-${activeChapterId}`);
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  function handleEditorChange(newText: string) { 
    fileContent = newText; 
    if(!isLoadingFile && hasInitialized) { 
        isModified = true; saveStateToCache();
        clearTimeout(autoRefreshTimer);
        autoRefreshTimer = setTimeout(() => scanToc(newText), 600); 
    } 
  }

  // --- 检查与报告 ---
  function toggleCheckMode() { 
      isCheckModeOn = !isCheckModeOn; 
      if (isCheckModeOn) { scanToc(); runFullCheck(); } 
      else { invalidSequenceIds.clear(); tocTree = [...tocTree]; }
  }
  function startLongPress(e: Event) { 
      if(isMobile) { e.preventDefault(); (document.activeElement as HTMLElement)?.blur(); }
      longPressTimer = setTimeout(() => { closeAllPanels(); showCheckPanel = true; runFullCheck(); }, 600); 
  }
  function runFullCheck() {
    sequenceErrors = []; titleErrors = []; wordCountErrors = []; invalidSequenceIds.clear();
    let lastNum = -1;
    for (const node of flatToc) {
        if (node.type === "Chapter") {
            const num = parseInt(node.title.match(/\d+/)?.[0] || "-1");
            if (num !== -1) { 
                if (lastNum !== -1 && num !== lastNum + 1) { 
                    invalidSequenceIds.add(node.id); 
                    sequenceErrors.push({ id: node.id, title: node.title, line: node.line, msg: `断序: ${lastNum}->${num}`, val: num }); 
                } 
                lastNum = num; 
            }
            if (node.word_count > appSettings.wordCountThreshold) wordCountErrors.push({ id: node.id, title: node.title, line: node.line, msg: `字数超标`, val: node.word_count });
        }
    }
    tocTree = [...tocTree];
  }
  function jumpToError(item: CheckItem) { editorComponent?.scrollToLine(item.line); if (isMobile) showSidebar = false; }

  // --- 查找替换逻辑 ---
  async function performFind() {
    if (!fileContent || !findPattern) return;
    if (isMobile) (document.activeElement as HTMLElement)?.blur();
    try {
      const res = await invoke<SearchResult>("advanced_search", { content: fileContent, pattern: findPattern, isRegex });
      if (res.found) { allMatches = res.matches; currentMatchIndex = 0; replaceMsg = `找到 ${res.count} 处`; highlightMatch(allMatches[0]); } 
      else { allMatches = []; replaceMsg = "未找到"; }
    } catch (e) { replaceMsg = "查找错误"; }
  }
  function nextMatch() { if (allMatches.length === 0) return; currentMatchIndex = (currentMatchIndex + 1) % allMatches.length; highlightMatch(allMatches[currentMatchIndex]); }
  function prevMatch() { if (allMatches.length === 0) return; currentMatchIndex = (currentMatchIndex - 1 + allMatches.length) % allMatches.length; highlightMatch(allMatches[currentMatchIndex]); }
  function highlightMatch(m: MatchLocation) { editorComponent?.selectMatch(m.line, m.start_char, m.end_char); }

  async function performReplaceAll() {
    if (!fileContent || !findPattern) return;
    try { 
        const newContent = await invoke<string>("advanced_replace", { 
            content: fileContent, pattern: findPattern, replacement: replacePattern, isRegex 
        }); 
        fileContent = newContent; replaceMsg = "全部替换完成"; allMatches = []; 
    } catch (e) { replaceMsg = "替换失败"; }
  }

  // --- 拖拽处理 ---
  function startDrag(e: MouseEvent, target: string) {
    if (isMobile) return;
    isDragging = true; draggingTarget = target;
    const p = document.getElementById(target + '-panel');
    if(p) { const r = p.getBoundingClientRect(); dragOffset = { x: e.clientX - r.left, y: e.clientY - r.top }; }
  }
  function onMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    let x = e.clientX - dragOffset.x; let y = e.clientY - dragOffset.y;
    // 简单的边界限制
    x = Math.max(0, Math.min(x, window.innerWidth - 100));
    y = Math.max(0, Math.min(y, window.innerHeight - 100));
    if (draggingTarget === 'find') dialogPos = { x, y };
    else if (draggingTarget === 'check') checkPanelPos = { x, y };
    else if (draggingTarget === 'settings') settingsPanelPos = { x, y };
    else if (draggingTarget === 'epub') epubModalPos = { x, y };
  }
</script>

<svelte:head>
    <meta name="theme-color" content="#f3f3f3">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
</svelte:head>

<ContextMenu />
<svelte:window on:mousemove={onMouseMove} on:mouseup={() => isDragging = false} />

<main class="app-container" on:contextmenu|preventDefault>
  <header class="toolbar">
    <div class="btn-group">
        <button class="btn-primary" on:click={selectFile}>📂</button>
        <button class="{isModified ? 'btn-save-modified' : 'btn-save-default'}" on:click={saveFile}>💾</button>
        <button class="btn-secondary" on:click={() => editorComponent?.triggerUndo()}>↩️</button>
        <button class="btn-secondary" on:click={() => showSidebar = !showSidebar}>📖</button>
        <button class="btn-secondary" on:click={openEpubModal}>📚</button>
        <button class="btn-secondary" on:click={() => { closeAllPanels(); showSettingsPanel = true; settingsPanelPos = isMobile ? {x:0,y:0} : { x: window.innerWidth / 2 - 200, y: 100 }; }}>⚙️</button>
    </div>
    <button class="btn-secondary" on:click={() => { closeAllPanels(); showFindReplace = !showFindReplace; dialogPos = { x: window.innerWidth - 350, y: 80 }; }}>🔍</button>
  </header>

  <div class="main-body">
    {#if showSidebar && isMobile}
      <div class="sidebar-mask" role="button" tabindex="0" on:click={() => showSidebar = false} on:keydown={e => e.key==='Escape' && (showSidebar=false)}></div>
    {/if}
    
    {#if showSidebar}
        <aside class="sidebar">
          <div class="sidebar-header">
            <span class="stats-info">{stats.volumes}卷 {stats.chapters}章</span>
            <button class="mini-btn {isCheckModeOn ? 'active' : ''}" on:touchstart={startLongPress} on:touchend={() => clearTimeout(longPressTimer)} on:click={toggleCheckMode}>内容检查</button>
          </div>
          <div class="toc-list">
            <div class="toc-item vol-title" role="button" tabindex="0" on:click={() => { isAllCollapsed = !isAllCollapsed; tocTree.forEach(n => n.expanded = !isAllCollapsed); tocTree = [...tocTree]; }} on:keydown={e => e.key==='Enter' && (isAllCollapsed = !isAllCollapsed)}>
                <span>{isAllCollapsed ? '展开全部' : '折叠全部'}</span>
            </div>
            {#each tocTree as node (node.id)}
                <div id={`toc-${node.id}`} class="toc-item {node.type === 'Volume' ? 'vol-title' : ''} {activeChapterId === node.id ? 'active' : ''}" role="button" tabindex="0" on:click={() => node.type === 'Volume' ? (node.expanded = !node.expanded, tocTree=[...tocTree]) : editorComponent.scrollToLine(node.line_number)} on:keydown={e => e.key==='Enter' && editorComponent.scrollToLine(node.line_number)}>
                    {#if node.type === 'Volume'}<span class="arrow">{node.expanded ? '▼' : '▶'}</span>{/if}
                    <span class="toc-title {invalidSequenceIds.has(node.id) ? 'text-error' : ''}">{node.title}</span>
                    <span class="toc-count">{node.word_count}</span>
                </div>
                {#if node.expanded}
                    {#each node.children as child (child.id)}
                        <div id={`toc-${child.id}`} class="toc-item indent {activeChapterId === child.id ? 'active' : ''}" role="button" tabindex="0" on:click={() => editorComponent.scrollToLine(child.line_number)} on:keydown={e => e.key==='Enter' && editorComponent.scrollToLine(child.line_number)}>
                            <span class="toc-title {invalidSequenceIds.has(child.id) ? 'text-error' : ''}">{child.title}</span>
                            <span class="toc-count">{child.word_count}</span>
                        </div>
                    {/each}
                {/if}
            {/each}
          </div>
        </aside>
    {/if}
    <section class="editor-wrapper">
      <Editor bind:this={editorComponent} doc={fileContent} onChange={handleEditorChange} onScroll={handleScroll} />
    </section>
  </div>

  {#if showFindReplace}
    <div id="find-panel" class="find-panel mobile-drawer">
        <div class="drawer-header" on:mousedown={e => startDrag(e, 'find')}><span>查找与替换</span><button on:click={() => showFindReplace = false}>✕</button></div>
        <div class="drawer-body">
            <div class="input-row">
                <input type="text" bind:value={findPattern} placeholder="查找内容" on:keydown={e => e.key==='Enter' && performFind()}>
                <label class="regex-check"><input type="checkbox" bind:checked={isRegex}> 正则</label>
            </div>
            <input type="text" bind:value={replacePattern} placeholder="替换为">
            <div class="msg-bar">{replaceMsg}</div>
            <div class="grid-btns">
                <button class="grid-btn" on:click={prevMatch}>上一个</button><button class="grid-btn" on:click={nextMatch}>下一个</button>
                <button class="grid-btn blue" on:click={performFind}>查找全部</button><button class="grid-btn orange" on:click={() => editorComponent.replaceSelection(replacePattern)}>替换当前</button>
                <button class="grid-btn red full-row" on:click={performReplaceAll}>⚠️ 全部替换</button>
            </div>
        </div>
    </div>
  {/if}

  {#if showSettingsPanel}
    <div id="settings-panel" class="find-panel modal-center">
      <div class="panel-header" on:mousedown={e => startDrag(e, 'settings')}><span>偏好设置</span><button on:click={() => showSettingsPanel = false}>✕</button></div>
      <div class="panel-body">
          <div class="set-row"><label for="vol-reg">卷正则:</label><input id="vol-reg" type="text" bind:value={appSettings.volRegex}></div>
          <div class="set-row"><label for="chap-reg">章正则:</label><input id="chap-reg" type="text" bind:value={appSettings.chapRegex}></div>
          <div class="set-row"><label for="word-th">字数阈值:</label><input id="word-th" type="number" bind:value={appSettings.wordCountThreshold}></div>
          <div class="set-row"><label for="clear-hist">保存清空历史:</label><input id="clear-hist" type="checkbox" bind:checked={appSettings.clearHistoryOnSave}></div>
          <div class="actions">
              <button class="grid-btn" on:click={async () => { await fetchHistory(); showHistoryPanel = true; showSettingsPanel = false; }}>🕒 历史</button>
              <button class="grid-btn blue" on:click={() => { localStorage.setItem("app-settings", JSON.stringify(appSettings)); showSettingsPanel = false; scanToc(); }}>应用</button>
          </div>
      </div>
    </div>
  {/if}

  {#if showEpubModal}
    <div id="epub-panel" class="find-panel modal-center">
      <div class="panel-header" on:mousedown={e => startDrag(e, 'epub')}><span>制作 EPUB</span><button on:click={() => showEpubModal = false}>✕</button></div>
      <div class="panel-body">
          <div class="set-row"><label for="epub-title">书名:</label><input id="epub-title" type="text" bind:value={epubMeta.title}></div>
          <div class="set-row"><label for="epub-creator">作者:</label><input id="epub-creator" type="text" bind:value={epubMeta.creator}></div>
          <div class="set-row"><label for="epub-pub">出版社:</label><input id="epub-pub" type="text" bind:value={epubMeta.publisher}></div>
          <div class="set-row"><label for="epub-uuid">UUID:</label><input id="epub-uuid" type="text" bind:value={epubMeta.uuid} readonly style="font-size:10px; background:#f9f9f9"></div>
          <div class="set-row"><label for="epub-md5">MD5:</label><input id="epub-md5" type="text" bind:value={epubMeta.md5} readonly style="font-size:10px; background:#f9f9f9"></div>
          <div class="set-row"><label for="epub-date">日期:</label><input id="epub-date" type="text" bind:value={epubMeta.date} readonly style="background:#f9f9f9"></div>
          <div class="set-row"><label>封面:</label><button class="mini-btn" style="flex:1; margin-left:10px;" on:click={selectCover}>{epubMeta.cover_path ? '已选' : '选择图片'}</button></div>
          <button class="grid-btn blue full-row" style="height:48px; margin-top:10px;" on:click={generateEpub}>{isEpubGenerating ? '正在生成...' : '开始生成'}</button>
      </div>
    </div>
  {/if}

  {#if showCheckPanel}
    <div id="check-panel" class="find-panel modal-center">
        <div class="panel-header" on:mousedown={e => startDrag(e, 'check')}><span>检查报告</span><button on:click={() => showCheckPanel = false}>✕</button></div>
        <div class="panel-body scroll-body" style="max-height:60vh; overflow-y:auto;">
            <div class="check-sec">
                <div class="check-title">断序章节 ({sequenceErrors.length})：</div>
                <div class="tag-list">{#each sequenceErrors as e}<button class="err-tag" on:click={() => jumpToError(e)}>{e.title}</button>{:else}无{/each}</div>
            </div>
            <div class="check-sec" style="margin-top:15px">
                <div class="check-title">字数超标章节 ({wordCountErrors.length})：</div>
                <div class="tag-list">{#each wordCountErrors as e}<button class="err-tag" on:click={() => jumpToError(e)}>{e.title} ({e.val}字)</button>{:else}无{/each}</div>
            </div>
        </div>
    </div>
  {/if}

  {#if showHistoryPanel}
    <div id="history-panel" class="find-panel modal-center">
        <div class="panel-header" on:mousedown={e => startDrag(e, 'history')}><span>历史快照</span><button on:click={() => showHistoryPanel = false}>✕</button></div>
        <div class="panel-body scroll-body" style="max-height:50vh; overflow-y:auto; padding:0;">
            {#each historyList as h}<button class="hist-item" on:click={() => restoreSnapshot(h)}><span>{new Date(h.timestamp*1000).toLocaleString()}</span><span>{formatSize(h.size)}</span></button>{:else}<div class="empty">暂无备份记录</div>{/each}
        </div>
    </div>
  {/if}
</main>

<style>
  :global(body) { margin: 0; background: #fff; overflow: hidden; -webkit-touch-callout: none; -webkit-user-select: none; font-family: 'Segoe UI', system-ui, sans-serif; }
  .app-container { display: flex; flex-direction: column; height: 100vh; width: 100vw; }
  
  .toolbar { 
      padding-top: env(safe-area-inset-top); background: #f3f3f3; border-bottom: 1px solid #ddd;
      display: flex; align-items: center; justify-content: space-between;
      padding-left: 10px; padding-right: 10px; height: 44px; flex-shrink: 0; z-index: 100;
  }
  .btn-group { display: flex; gap: 6px; align-items: center; }
  button { height: 34px; min-width: 42px; border-radius: 6px; border: 1px solid #ccc; background: #fff; font-size: 18px; display: flex; align-items: center; justify-content: center; outline: none; transition: 0.2s; -webkit-tap-highlight-color: transparent; }
  button:active { background: #eee; }
  .btn-primary { background: #0066b8; color: #fff; border: none; }
  .btn-save-modified { background: #d32f2f; color: #fff; border: none; font-weight: bold; animation: pulse 2s infinite; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.7; } }
  
  .main-body { flex: 1; display: flex; overflow: hidden; position: relative; }
  .sidebar { width: 280px; background: #f8f8f8; border-right: 1px solid #ddd; display: flex; flex-direction: column; flex-shrink: 0; }
  .sidebar-header { padding: 10px 12px; display: flex; justify-content: space-between; background: #eee; align-items: center; font-size: 12px; font-weight: bold; }
  .toc-list { flex: 1; overflow-y: auto; }
  .toc-item { padding: 12px 14px; font-size: 14px; border-bottom: 1px solid #eee; display: flex; justify-content: space-between; align-items: center; cursor: pointer; }
  .vol-title { background: #eaeaea; font-weight: bold; position: sticky; top: 0; z-index: 10; }
  .indent { padding-left: 28px; background: #fafafa; }
  .toc-item.active { background: #d4e8fa; color: #0066b8; border-left: 4px solid #0066b8; }
  .text-error { color: #d32f2f; font-weight: bold; }
  .toc-count { color: #999; font-size: 12px; font-family: monospace; }
  .arrow { font-size: 10px; margin-right: 8px; color: #888; width: 12px; display: inline-block; }
  
  .mini-btn { font-size: 11px; height: 26px; padding: 0 10px; border-radius: 4px; border: 1px solid #ccc; background: #fff; width: auto; min-width: auto; }
  .mini-btn.active { background: #0066b8; color: #fff; }

  .editor-wrapper { flex: 1; overflow: hidden; }

  /* 查找面板 - 对称居中布局 */
  .mobile-drawer {
      position: fixed; bottom: 0; left: 0; right: 0; background: #fff; z-index: 1000;
      border-radius: 24px 24px 0 0; box-shadow: 0 -5px 30px rgba(0,0,0,0.25);
      padding-bottom: calc(25px + env(safe-area-inset-bottom));
  }
  .drawer-header { padding: 16px 20px; border-bottom: 1px solid #eee; display: flex; justify-content: space-between; font-weight: bold; font-size: 16px; }
  .drawer-body { padding: 20px; display: flex; flex-direction: column; gap: 14px; }
  .input-row { display: flex; gap: 10px; align-items: center; width: 100%; }
  .input-row input { flex: 1; }
  .regex-check { font-size: 14px; color: #555; display: flex; align-items: center; gap: 6px; white-space: nowrap; cursor: pointer; }
  input[type="text"], input[type="number"] { height: 46px; padding: 0 16px; border: 1px solid #ddd; border-radius: 12px; font-size: 16px; outline: none; width: 100%; box-sizing: border-box; }
  input:focus { border-color: #0066b8; box-shadow: 0 0 0 2px rgba(0,102,184,0.1); }
  
  /* 弹窗居中核心样式 */
  .modal-center {
      position: fixed; top: 50% !important; left: 50% !important; transform: translate(-50%, -50%) !important;
      width: 92vw !important; max-width: 440px; max-height: 85vh; overflow: hidden; display: flex; flex-direction: column;
      border-radius: 20px; box-shadow: 0 20px 60px rgba(0,0,0,0.4); z-index: 1100;
  }
  .find-panel { background: #fff; border: 1px solid #ccc; position: fixed; }
  .panel-header { background: #f0f0f0; padding: 14px 18px; border-bottom: 1px solid #ddd; display: flex; justify-content: space-between; font-weight: bold; font-size: 15px; }
  .panel-body { padding: 20px; display: flex; flex-direction: column; gap: 16px; }
  .set-row { display: flex; align-items: center; justify-content: space-between; font-size: 15px; gap: 12px; }
  .set-row label { font-weight: bold; color: #444; }
  .set-row input { width: 65%; }
  .set-row input[type="checkbox"] { width: 22px; height: 22px; }

  .grid-btns { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .grid-btn { height: 50px; border-radius: 12px; font-weight: bold; font-size: 15px; border: 1px solid #ddd; background: #fdfdfd; width: 100%; }
  .blue { background: #e3f2fd; color: #0066b8; border-color: #bbdefb; }
  .orange { background: #fff3e0; color: #e65100; border-color: #ffe0b2; }
  .red { background: #ffebee; color: #c62828; border-color: #ffcdd2; }
  .full-row { grid-column: span 2; }
  .msg-bar { font-size: 13px; color: #e65100; min-height: 18px; text-align: center; font-weight: bold; }

  .check-sec { margin-bottom: 10px; }
  .check-title { font-weight: bold; font-size: 14px; color: #333; margin-bottom: 10px; border-left: 4px solid #0066b8; padding-left: 8px; }
  .tag-list { display: flex; flex-wrap: wrap; gap: 8px; }
  .err-tag { margin: 0; padding: 8px 16px; background: #fee; color: #c00; border: 1px solid #fcc; border-radius: 25px; font-size: 13px; text-align: left; transition: 0.2s; }
  .err-tag:active { background: #fcc; }
  
  .hist-item { display: flex; justify-content: space-between; padding: 16px 20px; border: none; border-bottom: 1px solid #eee; background: #fff; font-size: 14px; text-align: left; width: 100%; }
  .hist-item:active { background: #f5f5f5; }
  .empty { padding: 50px; text-align: center; color: #999; font-size: 15px; }

  .sidebar-mask { position: absolute; inset: 0; background: rgba(0,0,0,0.5); z-index: 90; backdrop-filter: blur(3px); }

  @media (max-width: 768px) {
      .sidebar { position: absolute; z-index: 1000; left: 0; top: 0; bottom: 0; width: 85%; box-shadow: 15px 0 45px rgba(0,0,0,0.3); }
      .toolbar { height: 42px; }
  }
</style>