<script lang="ts">
  import { base } from "$app/paths";
  import { goto } from "$app/navigation";
  import { onDestroy, onMount } from "svelte";
  import { loadAppSettings, providerToProofingConfig } from "$lib/appSettings";
  import { runWebAiText } from "$lib/webAiText";
  import {
    addWebLibraryBook,
    listWebLibraryBooks,
    removeWebLibraryBook,
    replaceWebLibraryBookCover,
    requestWebLibraryPersistence,
    updateWebLibraryBookMetadata,
    type WebLibraryBook,
  } from "$lib/webLibrary";

  type ViewMode = "grid" | "list";
  type SortMode = "added-desc" | "modified-desc" | "title-asc" | "author-asc";

  let bookInput: HTMLInputElement | null = null;
  let coverInput: HTMLInputElement | null = null;
  let books: WebLibraryBook[] = [];
  let selectedId = "";
  let searchQuery = "";
  let activeTag = "";
  let viewMode: ViewMode = "grid";
  let sortMode: SortMode = "added-desc";
  let busy = false;
  let status = "";
  let importProgress = "";
  let coverUrls = new Map<string, string>();
  let showMetadata = false;
  let showStorage = false;
  let storageUsage = 0;
  let storageQuota = 0;
  let persistenceGranted: boolean | null = null;
  let aiMatchRunning = false;
  let aiMatchMessage = "";
  let metadataDraft = { title: "", author: "", subtitle: "", description: "", publisher: "", language: "zh-CN", identifier: "", tags: "", series: "", maker: "" };

  $: selectedBook = books.find((book) => book.id === selectedId) || null;
  $: allTags = [...new Set(books.flatMap((book) => book.tags))].sort((a, b) => a.localeCompare(b, "zh-CN"));
  $: filteredBooks = books
    .filter((book) => {
      const query = searchQuery.trim().toLowerCase();
      const matchesQuery = !query || [book.title, book.author, book.fileName, book.description, book.tags.join(" ")].some((value) => value.toLowerCase().includes(query));
      return matchesQuery && (!activeTag || book.tags.includes(activeTag));
    })
    .sort((a, b) => {
      if (sortMode === "modified-desc") return b.modifiedAt - a.modifiedAt;
      if (sortMode === "title-asc") return a.title.localeCompare(b.title, "zh-CN");
      if (sortMode === "author-asc") return a.author.localeCompare(b.author, "zh-CN");
      return b.addedAt - a.addedAt;
    });

  function appPath(path: string) {
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function formatDate(timestamp: number) {
    return new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" }).format(timestamp);
  }

  function revokeCoverUrls() {
    for (const url of coverUrls.values()) URL.revokeObjectURL(url);
    coverUrls = new Map();
  }

  function rebuildCoverUrls() {
    revokeCoverUrls();
    const next = new Map<string, string>();
    for (const book of books) if (book.coverBlob) next.set(book.id, URL.createObjectURL(book.coverBlob));
    coverUrls = next;
  }

  async function loadBooks(preferredId = selectedId) {
    books = await listWebLibraryBooks();
    rebuildCoverUrls();
    if (preferredId && books.some((book) => book.id === preferredId)) selectedId = preferredId;
    else if (!selectedId || !books.some((book) => book.id === selectedId)) selectedId = books[0]?.id || "";
  }

  async function updateStorageEstimate() {
    if (!navigator.storage?.estimate) return;
    const estimate = await navigator.storage.estimate();
    storageUsage = estimate.usage || 0;
    storageQuota = estimate.quota || 0;
    if (navigator.storage.persisted) persistenceGranted = await navigator.storage.persisted();
  }

  async function importBooks(files: File[]) {
    const accepted = files.filter((file) => /\.(?:epub|txt)$/i.test(file.name));
    if (!accepted.length) { status = "请选择 EPUB 或 TXT 文件"; return; }
    busy = true;
    let added = 0;
    let lastId = "";
    const failures: string[] = [];
    try {
      for (let index = 0; index < accepted.length; index += 1) {
        importProgress = `正在导入 ${index + 1} / ${accepted.length}：${accepted[index].name}`;
        try {
          const book = await addWebLibraryBook(accepted[index]);
          lastId = book.id;
          added += 1;
        } catch (error) {
          failures.push(`${accepted[index].name}: ${error instanceof Error ? error.message : String(error)}`);
        }
      }
      await loadBooks(lastId);
      await updateStorageEstimate();
      status = `已导入 ${added} 本${failures.length ? `，${failures.length} 本失败` : ""}`;
      if (failures.length) console.warn("Web 书库导入失败", failures);
    } finally {
      busy = false;
      importProgress = "";
      if (bookInput) bookInput.value = "";
    }
  }

  function handleBookInput(event: Event) {
    void importBooks(Array.from((event.currentTarget as HTMLInputElement).files || []));
  }

  function selectBook(book: WebLibraryBook) {
    selectedId = book.id;
  }

  async function openBook(book: WebLibraryBook, mode: "read" | "edit") {
    if (book.kind === "txt") {
      await goto(appPath(`/toolbox/text-editor?library=${encodeURIComponent(book.id)}`));
      return;
    }
    const reader = mode === "read" ? "&mode=reader" : "";
    await goto(appPath(`/toolbox/epub-editor?library=${encodeURIComponent(book.id)}${reader}`));
  }

  function exportBook(book: WebLibraryBook) {
    const url = URL.createObjectURL(book.blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = book.fileName;
    anchor.click();
    URL.revokeObjectURL(url);
  }

  async function deleteBook(book: WebLibraryBook) {
    if (!confirm(`从 Web 书库移除《${book.title}》？\n这会删除浏览器中保存的图书副本。`)) return;
    await removeWebLibraryBook(book.id);
    await loadBooks();
    await updateStorageEstimate();
    status = `已移除《${book.title}》`;
  }

  function editMetadata(book: WebLibraryBook) {
    selectedId = book.id;
    aiMatchMessage = "";
    metadataDraft = {
      title: book.title,
      author: book.author,
      subtitle: book.subtitle,
      description: book.description,
      publisher: book.publisher,
      language: book.language || "zh-CN",
      identifier: book.identifier,
      tags: book.tags.join("，"),
      series: book.series,
      maker: book.maker,
    };
    showMetadata = true;
  }

  async function saveMetadata() {
    if (!selectedBook || busy) return;
    busy = true;
    try {
      const updated = await updateWebLibraryBookMetadata(selectedBook.id, {
        ...metadataDraft,
        tags: metadataDraft.tags.split(/[,，;；]/).map((tag) => tag.trim()).filter(Boolean),
      });
      await loadBooks(updated.id);
      showMetadata = false;
      status = "元数据已写入书库和 EPUB";
    } catch (error) {
      status = `保存失败：${error instanceof Error ? error.message : String(error)}`;
    } finally { busy = false; }
  }

  function pickCover(book: WebLibraryBook) {
    selectedId = book.id;
    coverInput?.click();
  }

  async function handleCoverInput(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file || !selectedBook) return;
    busy = true;
    try {
      const updated = await replaceWebLibraryBookCover(selectedBook.id, file);
      await loadBooks(updated.id);
      status = "封面已更新";
    } catch (error) {
      status = `封面更新失败：${error instanceof Error ? error.message : String(error)}`;
    } finally {
      busy = false;
      if (coverInput) coverInput.value = "";
    }
  }

  async function enablePersistence() {
    persistenceGranted = await requestWebLibraryPersistence();
    status = persistenceGranted ? "浏览器已允许持久保存书库数据" : "浏览器未授予持久存储；请避免主动清理站点数据";
  }

  function setView(mode: ViewMode) {
    viewMode = mode;
    localStorage.setItem("tepub-web-library-view", mode);
  }

  function setSort(mode: SortMode) {
    sortMode = mode;
    localStorage.setItem("tepub-web-library-sort", mode);
  }

  async function runLibraryAiMatch() {
    if (!selectedBook || aiMatchRunning) return;
    const settings = loadAppSettings();
    const provider = settings.aiProviders.find((item) => item.id === settings.txtAiProofing.providerId && item.kind === "text")
      || settings.aiProviders.find((item) => item.kind === "text");
    const config = providerToProofingConfig(provider, settings.aiProofing);
    aiMatchRunning = true;
    aiMatchMessage = "智能匹配中...";
    try {
      const response = await runWebAiText(
        config,
        "你是中文图书元数据编辑。只返回 JSON，不要 Markdown。格式：{\"description\":\"80到300字简介\",\"tags\":[\"标签1\",\"标签2\"],\"reason\":\"40字内说明\"}。标签给出3到8个，避免重复书名和作者。",
        [`书名：${metadataDraft.title}`, `作者：${metadataDraft.author || "未知"}`, `副标题：${metadataDraft.subtitle}`, `当前简介：${metadataDraft.description}`, `当前标签：${metadataDraft.tags || "无"}`].join("\n"),
      );
      const clean = response.content.trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/i, "");
      const result = JSON.parse(clean);
      if (result.description) metadataDraft.description = String(result.description);
      if (Array.isArray(result.tags)) metadataDraft.tags = result.tags.map((tag: unknown) => String(tag).trim()).filter(Boolean).slice(0, 10).join(", ");
      metadataDraft = { ...metadataDraft };
      aiMatchMessage = `匹配完成${result.reason ? `：${String(result.reason)}` : ""}`;
    } catch (error) {
      aiMatchMessage = `匹配失败：${error instanceof Error ? error.message : String(error)}`;
    } finally {
      aiMatchRunning = false;
    }
  }

  onMount(async () => {
    viewMode = localStorage.getItem("tepub-web-library-view") === "list" ? "list" : "grid";
    const storedSort = localStorage.getItem("tepub-web-library-sort") as SortMode | null;
    if (storedSort && ["added-desc", "modified-desc", "title-asc", "author-asc"].includes(storedSort)) sortMode = storedSort;
    try {
      await loadBooks();
      await updateStorageEstimate();
    } catch (error) {
      status = `书库加载失败：${error instanceof Error ? error.message : String(error)}`;
    }
  });

  onDestroy(revokeCoverUrls);
</script>

<svelte:head>
  <title>Web 书库 - TEpub Editor</title>
  <meta name="description" content="保存在浏览器本地的 EPUB 与 TXT 书库。" />
</svelte:head>

<div class="library-page">
  <header class="topbar">
    <div class="heading">
      <a class="back" href={appPath("/")} aria-label="返回工具箱"><svg viewBox="0 0 24 24"><path d="m15 18-6-6 6-6" /></svg></a>
      <div><span>WEB LIBRARY</span><h1>书库</h1></div>
    </div>
    <div class="head-actions">
      <button class="icon-button" type="button" title="存储状态" on:click={() => { showStorage = true; void updateStorageEstimate(); }}><span>DB</span></button>
      <button class="primary" type="button" disabled={busy} on:click={() => bookInput?.click()}>+ 添加图书</button>
    </div>
  </header>

  <input bind:this={bookInput} class="file-input" type="file" accept=".epub,.txt,application/epub+zip,text/plain" multiple on:change={handleBookInput} />
  <input bind:this={coverInput} class="file-input" type="file" accept="image/png,image/jpeg,image/webp,image/gif" on:change={handleCoverInput} />

  <div class="toolbar">
    <label class="search"><svg viewBox="0 0 24 24"><circle cx="11" cy="11" r="7" /><path d="m16 16 4 4" /></svg><input bind:value={searchQuery} placeholder="搜索书名、作者、标签或文件名" /></label>
    <select value={sortMode} on:change={(event) => setSort((event.currentTarget as HTMLSelectElement).value as SortMode)} aria-label="排序方式">
      <option value="added-desc">最近加入</option><option value="modified-desc">最近修改</option><option value="title-asc">书名</option><option value="author-asc">作者</option>
    </select>
    <div class="view-switch" aria-label="书架样式">
      <button class:active={viewMode === "grid"} type="button" title="网格" on:click={() => setView("grid")}>▦</button>
      <button class:active={viewMode === "list"} type="button" title="列表" on:click={() => setView("list")}>☷</button>
    </div>
  </div>

  {#if allTags.length}
    <nav class="tag-filter" aria-label="标签筛选">
      <button class:active={!activeTag} type="button" on:click={() => { activeTag = ""; }}>全部 <span>{books.length}</span></button>
      {#each allTags as tag}<button class:active={activeTag === tag} type="button" on:click={() => { activeTag = activeTag === tag ? "" : tag; }}>{tag}</button>{/each}
    </nav>
  {/if}

  <main class:with-preview={Boolean(selectedBook)} class="workspace">
    <section class="shelf">
      {#if busy && importProgress}<div class="progress"><span class="spinner"></span>{importProgress}</div>{/if}
      {#if !books.length && !busy}
        <button class="empty-state" type="button" on:click={() => bookInput?.click()}><span>LIB</span><strong>书库还是空的</strong><p>添加 EPUB 或 TXT 后，可直接阅读、编辑、管理元数据和封面。</p><b>添加图书</b></button>
      {:else if !filteredBooks.length}
        <div class="no-results">没有符合当前筛选条件的图书。</div>
      {:else if viewMode === "grid"}
        <div class="book-grid">
          {#each filteredBooks as book (book.id)}
            <button class:selected={book.id === selectedId} class="book-card" type="button" on:click={() => selectBook(book)} on:dblclick={() => openBook(book, book.kind === "epub" ? "read" : "edit")}>
              <span class="cover">
                {#if coverUrls.get(book.id)}<img src={coverUrls.get(book.id)} alt="" />{:else}<span class="cover-placeholder"><b>{book.title.slice(0, 1)}</b><small>{book.kind.toUpperCase()}</small></span>{/if}
                <i>{book.kind.toUpperCase()}</i>
              </span>
              <span class="book-copy"><strong title={book.title}>{book.title}</strong><small>{book.author || "未知作者"}</small></span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="book-list">
          <div class="list-head"><span>图书</span><span>作者</span><span>标签</span><span>大小</span><span>修改时间</span></div>
          {#each filteredBooks as book (book.id)}
            <button class:selected={book.id === selectedId} type="button" on:click={() => selectBook(book)} on:dblclick={() => openBook(book, book.kind === "epub" ? "read" : "edit")}>
              <span class="list-title">{#if coverUrls.get(book.id)}<img src={coverUrls.get(book.id)} alt="" />{:else}<i>{book.kind.toUpperCase()}</i>{/if}<strong>{book.title}</strong></span>
              <span>{book.author || "-"}</span><span class="list-tags">{book.tags.join(" · ") || "-"}</span><span>{formatBytes(book.fileSize)}</span><span>{formatDate(book.modifiedAt)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    {#if selectedBook}
      <aside class="preview">
        <button class="close-preview" type="button" on:click={() => { selectedId = ""; }}>返回书架</button>
        <div class="preview-cover" class:empty={!coverUrls.get(selectedBook.id)}>
          {#if coverUrls.get(selectedBook.id)}<img src={coverUrls.get(selectedBook.id)} alt="{selectedBook.title} 封面" />{:else}<span>{selectedBook.title.slice(0, 1)}</span>{/if}
          <button type="button" title="更换封面" on:click={() => pickCover(selectedBook!)}>更换封面</button>
        </div>
        <div class="preview-copy"><span class="type">{selectedBook.kind.toUpperCase()} · {formatBytes(selectedBook.fileSize)}</span><h2>{selectedBook.title}</h2><p class="author">{selectedBook.author || "未知作者"}</p>
          {#if selectedBook.tags.length}<div class="tags">{#each selectedBook.tags as tag}<button type="button" on:click={() => { activeTag = tag; }}>{tag}</button>{/each}</div>{/if}
          <p class="description">{selectedBook.description || "暂无简介"}</p>
        </div>
        <div class="preview-actions">
          <button class="primary" type="button" on:click={() => openBook(selectedBook!, selectedBook!.kind === "epub" ? "read" : "edit")}>{selectedBook.kind === "epub" ? "阅读" : "编辑"}</button>
          {#if selectedBook.kind === "epub"}<button type="button" on:click={() => openBook(selectedBook!, "edit")}>编辑 EPUB</button>{/if}
          <button type="button" on:click={() => editMetadata(selectedBook!)}>元数据</button>
          <button type="button" on:click={() => exportBook(selectedBook!)}>下载</button>
          <button class="danger" type="button" on:click={() => deleteBook(selectedBook!)}>移除</button>
        </div>
        <dl><div><dt>文件名</dt><dd>{selectedBook.fileName}</dd></div><div><dt>加入时间</dt><dd>{formatDate(selectedBook.addedAt)}</dd></div>{#if selectedBook.publisher}<div><dt>出版社</dt><dd>{selectedBook.publisher}</dd></div>{/if}{#if selectedBook.identifier}<div><dt>标识符</dt><dd>{selectedBook.identifier}</dd></div>{/if}</dl>
      </aside>
    {/if}
  </main>

  <footer><span>{status || `${filteredBooks.length} / ${books.length} 本图书`}</span><span>浏览器本地存储 · {formatBytes(storageUsage)}</span></footer>

  {#if showMetadata && selectedBook}
    <div class="modal-backdrop" role="presentation" on:click={(event) => { if (event.target === event.currentTarget && !busy) showMetadata = false; }}>
      <div class="modal" role="dialog" aria-modal="true" aria-labelledby="metadata-title">
        <header><div><span>BOOK METADATA</span><h2 id="metadata-title">编辑元数据</h2></div><button type="button" aria-label="关闭" on:click={() => { showMetadata = false; }}>×</button></header>
        <div class="form-grid">
          <label><span>书名</span><input bind:value={metadataDraft.title} /></label><label><span>作者</span><input bind:value={metadataDraft.author} /></label>
          <label><span>副标题</span><input bind:value={metadataDraft.subtitle} /></label><label><span>出版社</span><input bind:value={metadataDraft.publisher} /></label>
          <label><span>语言</span><input bind:value={metadataDraft.language} /></label><label><span>标识符</span><input bind:value={metadataDraft.identifier} /></label>
          <label><span>系列</span><input bind:value={metadataDraft.series} /></label><label><span>制作者</span><input bind:value={metadataDraft.maker} /></label>
          <label class="wide"><span>标签</span><input bind:value={metadataDraft.tags} placeholder="多个标签用逗号分隔" /></label>
          <label class="wide"><span>简介</span><textarea bind:value={metadataDraft.description} rows="5"></textarea></label>
        </div>
        <footer><span class="ai-match-status">{aiMatchMessage}</span><button type="button" disabled={busy || aiMatchRunning} on:click={runLibraryAiMatch}>{aiMatchRunning ? "匹配中..." : "智能匹配"}</button><button type="button" disabled={busy} on:click={() => { showMetadata = false; }}>取消</button><button class="primary" type="button" disabled={busy} on:click={saveMetadata}>{busy ? "保存中…" : "保存"}</button></footer>
      </div>
    </div>
  {/if}

  {#if showStorage}
    <div class="modal-backdrop" role="presentation" on:click={(event) => { if (event.target === event.currentTarget) showStorage = false; }}>
      <div class="modal storage-modal" role="dialog" aria-modal="true" aria-labelledby="storage-title">
        <header><div><span>STORAGE</span><h2 id="storage-title">本地存储</h2></div><button type="button" on:click={() => { showStorage = false; }}>×</button></header>
        <div class="storage-body"><div class="storage-meter"><span style={`width:${storageQuota ? Math.min(100, storageUsage / storageQuota * 100) : 0}%`}></span></div><strong>{formatBytes(storageUsage)} / {storageQuota ? formatBytes(storageQuota) : "未知配额"}</strong><p>图书文件保存在当前站点的 IndexedDB 中。清理浏览器站点数据会同时删除书库。</p><div class="persist-state"><span class:ok={persistenceGranted === true}>{persistenceGranted === true ? "已启用持久存储" : persistenceGranted === false ? "未启用持久存储" : "尚未确认持久存储"}</span><button type="button" on:click={enablePersistence}>请求持久保存</button></div></div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(body) { margin: 0; overflow: hidden; background: #eef2f5; color: #172033; font-family: Inter, "Microsoft YaHei", sans-serif; }
  button, input, select, textarea, a { font: inherit; } button { color: inherit; }.library-page { height: 100vh; display: grid; grid-template-rows: 66px auto auto minmax(0,1fr) 32px; }
  .topbar { display:flex; align-items:center; justify-content:space-between; gap:18px; padding:0 20px; border-bottom:1px solid #d8e0e8; background:#fff; }.heading,.head-actions{display:flex;align-items:center;gap:11px}.back{width:35px;height:35px;display:grid;place-items:center;border:1px solid #d4dde7;border-radius:7px;color:#46566c}.back svg,.search svg{width:19px;fill:none;stroke:currentColor;stroke-width:2}.heading span{color:#7d8a9d;font-size:9px;font-weight:800;letter-spacing:.15em}.heading h1{margin:2px 0 0;font-size:18px}.primary{border-color:#17699a!important;background:#17699a!important;color:#fff!important}.head-actions button{min-height:35px;padding:0 14px;border:1px solid #d1dbe5;border-radius:7px;background:#fff;font-size:11px;font-weight:800;cursor:pointer}.icon-button{width:35px;padding:0!important}.icon-button span{color:#51637b;font-size:9px}.file-input{position:fixed;width:1px;height:1px;opacity:0;pointer-events:none}
  .toolbar{display:grid;grid-template-columns:minmax(240px,1fr) 140px auto;gap:9px;padding:11px 18px;border-bottom:1px solid #dce3ea;background:#f8fafc}.search{display:flex;align-items:center;gap:8px;padding:0 11px;border:1px solid #d3dce6;border-radius:7px;background:#fff;color:#8290a3}.search input{width:100%;min-width:0;height:34px;border:0;outline:0;background:transparent;font-size:11px}.toolbar select{border:1px solid #d3dce6;border-radius:7px;background:#fff;padding:0 9px;font-size:10px}.view-switch{display:flex;padding:3px;border:1px solid #d3dce6;border-radius:7px;background:#fff}.view-switch button{width:32px;border:0;border-radius:4px;background:transparent;color:#748297;cursor:pointer}.view-switch button.active{background:#e4f0f7;color:#17699a}
  .tag-filter{display:flex;gap:6px;padding:8px 18px;border-bottom:1px solid #dce3ea;background:#fff;overflow-x:auto}.tag-filter button{flex:0 0 auto;padding:5px 9px;border:1px solid #d9e1e9;border-radius:999px;background:#fff;color:#5d6d82;font-size:9px;cursor:pointer}.tag-filter button.active{border-color:#a9c9dc;background:#e7f2f8;color:#17699a;font-weight:800}.tag-filter span{margin-left:4px;opacity:.7}
  .workspace{min-height:0;display:grid;grid-template-columns:minmax(0,1fr);overflow:hidden}.workspace.with-preview{grid-template-columns:minmax(0,1fr) 300px}.shelf{min-width:0;min-height:0;position:relative;padding:14px;overflow:auto}.progress{position:sticky;top:0;z-index:3;display:flex;align-items:center;gap:9px;margin-bottom:10px;padding:10px 12px;border:1px solid #bed4e1;border-radius:7px;background:#f4fbff;color:#315a75;font-size:10px}.spinner{width:15px;height:15px;border:2px solid #cedce5;border-top-color:#17699a;border-radius:50%;animation:spin .7s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
  .empty-state{width:min(560px,100%);min-height:260px;margin:10vh auto 0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:9px;border:1px dashed #9eb1c6;border-radius:12px;background:#fff;cursor:pointer}.empty-state>span{width:52px;height:52px;display:grid;place-items:center;border-radius:12px;background:#e4f1f8;color:#17699a;font-size:11px;font-weight:900}.empty-state strong{font-size:18px}.empty-state p{max-width:380px;margin:0;color:#76859a;font-size:11px;text-align:center;line-height:1.6}.empty-state b{margin-top:8px;padding:8px 14px;border-radius:6px;background:#17699a;color:#fff;font-size:10px}.no-results{padding:80px 20px;text-align:center;color:#78869a;font-size:11px}
  .book-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(130px,1fr));gap:15px}.book-card{min-width:0;display:grid;gap:8px;padding:8px;border:1px solid transparent;border-radius:8px;background:transparent;text-align:left;cursor:pointer}.book-card:hover{background:#fff}.book-card.selected{border-color:#aac9db;background:#e8f2f7}.cover{position:relative;width:100%;aspect-ratio:3/4;overflow:hidden;border-radius:6px;background:#dfe6ed;box-shadow:0 5px 14px rgba(35,49,66,.15)}.cover img{width:100%;height:100%;object-fit:cover}.cover>i{position:absolute;right:5px;bottom:5px;padding:3px 5px;border-radius:3px;background:rgba(20,30,42,.78);color:#fff;font-size:7px;font-style:normal;font-weight:800}.cover-placeholder{width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:8px;background:linear-gradient(145deg,#dce8ef,#b8cad7);color:#36566d}.cover-placeholder b{font-family:Georgia,"SimSun",serif;font-size:34px}.cover-placeholder small{font-size:8px;letter-spacing:.15em}.book-copy{min-width:0;display:grid;gap:3px}.book-copy strong,.book-copy small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.book-copy strong{font-size:11px}.book-copy small{color:#78869a;font-size:9px}
  .book-list{min-width:720px;border:1px solid #d8e1e9;border-radius:8px;background:#fff;overflow:hidden}.list-head,.book-list>button{display:grid;grid-template-columns:minmax(210px,1.5fr) minmax(100px,.7fr) minmax(130px,1fr) 75px 85px;align-items:center;gap:10px;padding:9px 12px}.list-head{border-bottom:1px solid #dce4eb;background:#f5f7f9;color:#7b899b;font-size:8px;font-weight:800;text-transform:uppercase}.book-list>button{width:100%;border:0;border-bottom:1px solid #edf1f4;background:#fff;text-align:left;color:#526176;font-size:9px;cursor:pointer}.book-list>button:hover,.book-list>button.selected{background:#eaf3f8}.list-title{min-width:0;display:flex;align-items:center;gap:9px}.list-title img,.list-title i{width:28px;height:36px;object-fit:cover;border-radius:3px}.list-title i{display:grid;place-items:center;background:#dce7ee;color:#426177;font-size:6px;font-style:normal}.list-title strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#26364a;font-size:10px}.list-tags{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .preview{min-height:0;display:grid;grid-template-rows:auto auto auto auto minmax(0,1fr);align-content:start;gap:14px;padding:18px;border-left:1px solid #d6dfe8;background:#fff;overflow:auto}.close-preview{display:none;border:0;background:transparent;color:#17699a;font-size:10px;font-weight:800;text-align:left;cursor:pointer}.preview-cover{position:relative;width:128px;aspect-ratio:3/4;margin:0 auto;overflow:hidden;border-radius:7px;background:#dfe7ed;box-shadow:0 8px 22px rgba(25,40,55,.18)}.preview-cover img{width:100%;height:100%;object-fit:cover}.preview-cover>span{width:100%;height:100%;display:grid;place-items:center;color:#45647a;font-family:Georgia,serif;font-size:42px}.preview-cover button{position:absolute;left:8px;right:8px;bottom:8px;padding:6px;border:0;border-radius:5px;background:rgba(20,30,42,.8);color:#fff;font-size:8px;opacity:0;cursor:pointer}.preview-cover:hover button,.preview-cover.empty button{opacity:1}.preview-copy{text-align:center}.type{color:#7890a1;font-size:8px;font-weight:800;letter-spacing:.08em}.preview-copy h2{margin:5px 0 3px;font-size:17px;overflow-wrap:anywhere}.author{margin:0;color:#65758a;font-size:10px}.tags{display:flex;justify-content:center;flex-wrap:wrap;gap:4px;margin-top:9px}.tags button{padding:3px 6px;border:0;border-radius:3px;background:#e7f1f6;color:#17699a;font-size:8px;cursor:pointer}.description{max-height:82px;margin:10px 0 0!important;overflow:auto;color:#68778a!important;font-size:9px!important;line-height:1.6;text-align:left}.preview-actions{display:flex;flex-wrap:wrap;gap:6px}.preview-actions button{flex:1 0 75px;min-height:31px;border:1px solid #d1dbe5;border-radius:5px;background:#fff;font-size:9px;font-weight:700;cursor:pointer}.preview-actions .danger{color:#b52d2d}.preview dl{min-height:0;margin:0;padding-top:12px;border-top:1px solid #e2e7ed;overflow:auto}.preview dl div{display:grid;grid-template-columns:62px minmax(0,1fr);gap:8px;margin-bottom:8px}.preview dt{color:#8793a4;font-size:8px}.preview dd{margin:0;color:#526176;font-size:8px;overflow-wrap:anywhere}
  .library-page>footer{display:flex;align-items:center;justify-content:space-between;padding:0 18px;border-top:1px solid #d9e1e9;background:#f8fafc;color:#78869a;font-size:8px}.modal-backdrop{position:fixed;inset:0;z-index:20;display:grid;place-items:center;padding:20px;background:rgba(18,28,40,.45)}.modal{width:min(680px,100%);max-height:calc(100vh - 40px);display:grid;grid-template-rows:auto minmax(0,1fr) auto;border-radius:8px;background:#fff;box-shadow:0 20px 60px rgba(10,20,32,.25);overflow:hidden}.modal>header,.modal>footer{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:15px 18px;border-bottom:1px solid #e0e6ec}.modal>footer{justify-content:flex-end;border-top:1px solid #e0e6ec;border-bottom:0}.modal>header span{color:#7c899b;font-size:8px;font-weight:800;letter-spacing:.14em}.modal>header h2{margin:3px 0 0;font-size:16px}.modal>header button{width:30px;height:30px;border:0;border-radius:5px;background:#eef2f5;font-size:18px;cursor:pointer}.modal>footer button{min-height:34px;padding:0 14px;border:1px solid #d1dbe5;border-radius:6px;background:#fff;font-size:10px;font-weight:700;cursor:pointer}.form-grid{min-height:0;display:grid;grid-template-columns:1fr 1fr;gap:12px;padding:18px;overflow:auto}.form-grid label{display:grid;gap:5px}.form-grid label.wide{grid-column:1/-1}.form-grid label>span{color:#68778c;font-size:9px;font-weight:700}.form-grid input,.form-grid textarea{box-sizing:border-box;width:100%;border:1px solid #d3dce6;border-radius:5px;padding:8px 9px;outline:0;font-size:10px}.form-grid input:focus,.form-grid textarea:focus{border-color:#62a0c5}.storage-modal{width:min(460px,100%)}.storage-body{display:grid;gap:13px;padding:20px}.storage-meter{height:8px;overflow:hidden;border-radius:99px;background:#e5eaf0}.storage-meter span{display:block;height:100%;border-radius:inherit;background:#17699a}.storage-body>strong{font-size:12px}.storage-body>p{margin:0;color:#6d7b8e;font-size:10px;line-height:1.6}.persist-state{display:flex;align-items:center;justify-content:space-between;gap:12px;padding-top:12px;border-top:1px solid #e2e7ec}.persist-state span{color:#a66a16;font-size:9px}.persist-state span.ok{color:#1d7748}.persist-state button{min-height:31px;border:1px solid #cbd6e1;border-radius:5px;background:#fff;font-size:9px;cursor:pointer}
  .ai-match-status{min-width:0;margin-right:auto;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#66768a;font-size:9px}
  @media(max-width:760px){:global(body){overflow:auto}.library-page{height:auto;min-height:100dvh;grid-template-rows:62px auto auto minmax(400px,auto) 32px}.topbar{padding:0 12px}.heading span{display:none}.toolbar{grid-template-columns:minmax(0,1fr) 84px}.toolbar select{grid-column:1/-1;grid-row:2}.view-switch{grid-column:2;grid-row:1}.tag-filter{padding-inline:12px}.workspace,.workspace.with-preview{display:block;overflow:visible}.shelf{padding:12px;overflow:visible}.book-grid{grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.book-card{padding:5px}.workspace.with-preview .shelf{display:none}.preview{min-height:calc(100dvh - 180px);border-left:0}.close-preview{display:block}.preview-cover{width:150px}.form-grid{grid-template-columns:1fr}.form-grid label.wide{grid-column:auto}.library-page>footer{position:sticky;bottom:0}.modal-backdrop{padding:10px}.modal{max-height:calc(100dvh - 20px)}}
</style>
