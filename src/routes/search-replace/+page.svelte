<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let searchQuery = "";
  let replaceQuery = "";

  // 选项
  let matchCase = false;
  let wholeWord = false;
  let wrapAround = true; 

  // 模式: "normal", "extended", "regex"
  let searchMode = "normal";

  let resultCount = 0;
  let currentMatch = 0;
  let hasSearched = false;

  let searchHistory: string[] = [];
  let replaceHistory: string[] = [];

  let showSearchHistory = false;
  let showReplaceHistory = false;

  let unlistenSync: UnlistenFn | null = null;
  let unlistenFocus: UnlistenFn | null = null;

  let syncTimer: any;
  async function syncState() {
    // 立即发送 sync-only 保存参数设置（但后端并不会触发高亮扫描导致卡顿）
    await emit("search-action", {
      type: "sync-only",
      search: searchQuery,
      replace: replaceQuery,
      matchCase,
      wholeWord,
      searchMode,
      wrapAround
    });

    // 防抖发送真正的渲染查询（比如用户停下手 500ms 后才开始高亮背景）
    clearTimeout(syncTimer);
    syncTimer = setTimeout(async () => {
      await emit("search-action", {
        type: "update-highlight",
        search: searchQuery,
        replace: replaceQuery,
        matchCase,
        wholeWord,
        searchMode,
        wrapAround
      });
    }, 500);
  }

  // 仅保存历史记录，并执行特定动作
  function saveHistory() {
    if (searchQuery && !searchHistory.includes(searchQuery)) {
      searchHistory = [searchQuery, ...searchHistory].slice(0, 8);
    }
    if (replaceQuery && !replaceHistory.includes(replaceQuery)) {
      replaceHistory = [replaceQuery, ...replaceHistory].slice(0, 8);
    }
  }

  async function performAction(actionType: string) {
    saveHistory();
    // 把当前状态全部带上发送
    await emit("search-action", {
      type: actionType,
      search: searchQuery,
      replace: replaceQuery,
      matchCase,
      wholeWord,
      searchMode,
      wrapAround
    });
  }

  async function findNext() {
    await performAction("find-next");
  }

  async function findPrev() {
    await performAction("find-prev");
  }

  async function replace() {
    await performAction("replace");
  }

  async function replaceAll() {
    await performAction("replace-all");
  }

  function closeWindow() {
    getCurrentWindow().close();
  }

  async function handleKeyDown(e: KeyboardEvent, isSearchInput: boolean) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (isSearchInput && showSearchHistory) {
         showSearchHistory = false;
      } else if (!isSearchInput && showReplaceHistory) {
         showReplaceHistory = false;
      } else {
         await findNext();
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      await findPrev();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      await findNext();
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeWindow();
    }
  }

  function handleBodyClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (!target.closest('.input-wrapper')) {
          showSearchHistory = false;
          showReplaceHistory = false;
      }
  }

  onMount(async () => {
    document.body.addEventListener('click', handleBodyClick);

    unlistenSync = await listen("search-status", (event: any) => {
      const payload = event.payload;
      if (payload) {
        resultCount = payload.count || 0;
        currentMatch = payload.current || 0;
        hasSearched = true;
      }
    });

    unlistenFocus = await listen("search-focus", (event: any) => {
      const payload = event.payload;
      if (payload && payload.selection) {
        searchQuery = payload.selection;
        syncState();
      }
      setTimeout(() => {
        const input = document.getElementById("search-input") as HTMLInputElement;
        if (input) {
          input.focus();
          input.select();
        }
      }, 50);
    });

    setTimeout(syncState, 100);
  });

  onDestroy(() => {
    document.body.removeEventListener('click', handleBodyClick);
    if (unlistenSync) unlistenSync();
    if (unlistenFocus) unlistenFocus();
  });

  function selectSearchHistory(item: string) {
      searchQuery = item;
      showSearchHistory = false;
      syncState();
  }
  
  function selectReplaceHistory(item: string) {
      replaceQuery = item;
      showReplaceHistory = false;
      syncState();
  }
</script>

<svelte:window on:keydown={(e) => { if (e.key === 'Escape') closeWindow(); }} />

<div class="search-window">
  <!-- 左侧控制区 -->
  <div class="left-panel">
    
    <div class="input-group">
      <div class="row">
        <label for="search-input">查找:</label>
        <div class="input-wrapper">
          <input
            id="search-input"
            type="text"
            bind:value={searchQuery}
            on:input={() => { showSearchHistory = true; syncState(); }}
            on:keydown={(e) => handleKeyDown(e, true)}
            on:focus={() => { showSearchHistory = searchHistory.length > 0; showReplaceHistory = false; }}
            autocomplete="off"
          />
          {#if showSearchHistory && searchHistory.length > 0}
          <ul class="history-dropdown">
             {#each searchHistory as item}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
                <li on:click={() => selectSearchHistory(item)}>{item}</li>
             {/each}
          </ul>
          {/if}
        </div>
      </div>
      
      <div class="row">
        <label for="replace-input">替换:</label>
        <div class="input-wrapper">
          <input
            id="replace-input"
            type="text"
            bind:value={replaceQuery}
            on:input={() => { showReplaceHistory = true; syncState(); }}
            on:keydown={(e) => handleKeyDown(e, false)}
            on:focus={() => { showReplaceHistory = replaceHistory.length > 0; showSearchHistory = false; }}
            autocomplete="off"
          />
          {#if showReplaceHistory && replaceHistory.length > 0}
          <ul class="history-dropdown">
             {#each replaceHistory as item}
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
                <li on:click={() => selectReplaceHistory(item)}>{item}</li>
             {/each}
          </ul>
          {/if}
        </div>
      </div>
    </div>

    <!-- 中间的复选框 -->
    <div class="checkbox-row">
      <label class="checkbox" title="必须匹配完整的单词"><input type="checkbox" bind:checked={wholeWord} on:change={syncState} /> 全词匹配</label>
      <label class="checkbox" title="区分英文字母大小写"><input type="checkbox" bind:checked={matchCase} on:change={syncState} /> 区分大小写</label>
      <label class="checkbox" title="到达文件末尾时从头继续"><input type="checkbox" bind:checked={wrapAround} /> 循环查找</label>
    </div>

    <!-- 底部的查找模式 -->
    <fieldset class="mode-group">
      <legend>查找模式</legend>
      <div class="radios">
        <label class="radio"><input type="radio" value="normal" bind:group={searchMode} on:change={syncState} /> 普通</label>
        <label class="radio" title="支持 \n, \r, \t 等"><input type="radio" value="extended" bind:group={searchMode} on:change={syncState} /> 扩展(\n,\r,\t)</label>
        <label class="radio"><input type="radio" value="regex" bind:group={searchMode} on:change={syncState} /> 正则表达式</label>
      </div>
    </fieldset>

    <div class="status-bar">
      {#if hasSearched}
         {resultCount > 0 ? '' : '无结果'}
      {/if}
    </div>
  </div>

  <!-- 右侧按钮堆叠区 -->
  <div class="right-panel">
    <button on:click={findNext} disabled={!searchQuery}>查找下一个</button>
    <button on:click={findPrev} disabled={!searchQuery}>查找上一个</button>
    <button on:click={replace} disabled={!searchQuery}>替换</button>
    <button on:click={replaceAll} disabled={!searchQuery}>全部替换</button>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background-color: #f0f0f0;
    color: #333;
    overflow: hidden; 
    font-size: 13px;
    user-select: none;
  }

  .search-window {
    display: flex;
    padding: 12px;
    gap: 16px;
    height: 100vh;
    box-sizing: border-box;
    align-items: stretch;
  }

  .left-panel {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100px;
    flex-shrink: 0;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .row label {
    width: 36px;
    text-align: right;
    white-space: nowrap;
  }

  .input-wrapper {
    flex-grow: 1;
    position: relative;
    display: flex;
  }

  input[type="text"] {
    width: 100%;
    padding: 5px 6px;
    border: 1px solid #999;
    box-sizing: border-box;
    font-size: 13px;
    outline: none;
    border-radius: 2px;
  }

  input[type="text"]:focus {
    border-color: #0066b8;
  }

  .history-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: #fff;
    border: 1px solid #999;
    border-top: none;
    max-height: 120px;
    overflow-y: auto;
    margin: 0;
    padding: 0;
    list-style: none;
    z-index: 1000;
    box-shadow: 0 4px 6px rgba(0,0,0,0.15);
  }

  .history-dropdown li {
    padding: 5px 8px;
    cursor: pointer;
  }

  .history-dropdown li:hover {
    background-color: #0066b8;
    color: white;
  }

  .checkbox-row {
    display: flex;
    flex-wrap: nowrap;
    gap: 12px;
    padding-left: 40px;
  }

  .checkbox {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
  }

  .mode-group {
    border: 1px solid #ccc;
    padding: 6px 12px;
    margin-top: 2px;
    margin-left: 40px; 
    border-radius: 2px;
  }

  .mode-group legend {
    color: #0066b8;
    padding: 0 4px;
    font-size: 12px;
  }

  .radios {
    display: flex;
    flex-direction: row;
    gap: 12px;
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }

  button {
    padding: 6px 12px;
    border: 1px solid #aaa;
    background-color: #f8f8f8;
    border-radius: 2px;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.1s;
    width: 100%;
    text-align: center;
  }

  button:hover:not(:disabled) {
    background-color: #e5f1fb;
    border-color: #0066b8;
  }

  button:active:not(:disabled) {
    background-color: #cce4f7;
    border-color: #005499;
  }

  button:disabled {
    color: #999;
    background-color: #f0f0f0;
    border-color: #ccc;
    cursor: default;
  }

  .status-bar {
    margin-top: auto;
    font-size: 12px;
    color: #666;
    padding-left: 40px;
    padding-bottom: 2px;
  }
</style>
