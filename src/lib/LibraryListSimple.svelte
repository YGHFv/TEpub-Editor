<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let books: any[] = [];
  export let selectedBook: any = null;
  export let formatFileSize: (bytes: number) => string = (b) => b + " B";
  export let columns: string[] = ["title", "author", "fileSize", "fileType"];
  export let sortColumn = "addedAt";
  export let sortAsc = false;
  export let onSort: (col: string) => void = () => {};
  export let activeTagFilters: string[] = [];
  export let onTagClick: (tag: string) => void = () => {};
  export let showCover: boolean = false;
  export let coverCache: Map<string, string> = new Map();

  const COVER_COL_WIDTH = "44px";

  const dispatch = createEventDispatcher();

  const ALL_COLUMNS = [
    { id: "title", label: "书名" },
    { id: "subtitle", label: "副标题" },
    { id: "author", label: "作者" },
    { id: "filename", label: "文件名" },
    { id: "fileType", label: "格式" },
    { id: "fileSize", label: "大小" },
    { id: "tags", label: "标签" },
    { id: "addedAt", label: "添加日期" },
    { id: "createdAt", label: "创建日期" },
    { id: "modifiedAt", label: "修改日期" },
  ];

  const DEFAULT_WIDTHS: Record<string, string> = {
    title: "220px",     // 之前 1fr 在宽屏上撑得过大；改为固定 220px，需更长可拖拽
    subtitle: "120px",
    author: "110px",
    filename: "180px",
    fileType: "56px",
    fileSize: "72px",
    tags: "1fr",        // 让"标签"列承担弹性伸缩(多 chip 时自动扩展)
    addedAt: "96px",
    createdAt: "96px",
    modifiedAt: "96px",
  };

  // Per-column width overrides (pixels)
  let colWidths: Record<string, number> = {};

  function colWidth(col: string): string {
    if (colWidths[col]) return colWidths[col] + "px";
    return DEFAULT_WIDTHS[col] || "80px";
  }

  let headerMenuVisible = false;
  let headerMenuPos = { x: 0, y: 0 };

  // Column reorder drag state
  let dragSrcIndex: number | null = null;
  let dragOverIndex: number | null = null;
  let dragging = false;

  // Column resize state
  let resizeCol: string | null = null;
  let resizeStartX = 0;
  let resizeStartW = 0;

  // 标签单元格：按下并横向拖动可揭露被列宽截断的标签
  // 阈值以内的位移仍视为点击，触发筛选；超过阈值则吞掉后续 click 事件
  const TAG_DRAG_THRESHOLD = 4;

  function formatDate(ts: number | undefined): string {
    if (!ts) return "-";
    const d = new Date(ts * 1000);
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  function cellValue(book: any, col: string): string {
    switch (col) {
      case "title": return book.title;
      case "subtitle": return book.subtitle || "-";
      case "author": return book.author || "-";
      case "filename": return stripExt(book.filename) || "-";
      case "fileType": return book.fileType.toUpperCase();
      case "fileSize": return formatFileSize(book.fileSize);
      case "addedAt": return formatDate(book.addedAt);
      case "createdAt": return formatDate(book.createdAt);
      case "modifiedAt": return formatDate(book.modifiedAt);
      default: return "";
    }
  }

  // 默认隐藏文件名后缀（格式信息已由"格式"列体现）
  function stripExt(filename: string | undefined): string {
    if (!filename) return "";
    const idx = filename.lastIndexOf(".");
    return idx > 0 ? filename.slice(0, idx) : filename;
  }

  // 列对齐方式：数字右对齐、格式/日期居中
  const RIGHT_ALIGN_COLS = new Set(["fileSize"]);
  const CENTER_ALIGN_COLS = new Set(["fileType", "addedAt", "createdAt", "modifiedAt"]);

  function columnLabel(id: string): string {
    return ALL_COLUMNS.find(c => c.id === id)?.label || id;
  }

  function handleHeaderContextMenu(e: MouseEvent) {
    e.preventDefault();
    headerMenuPos = { x: e.clientX, y: e.clientY };
    headerMenuVisible = true;
  }

  function toggleColumn(colId: string) {
    let newCols: string[];
    if (columns.includes(colId)) {
      newCols = columns.filter(c => c !== colId);
    } else {
      newCols = [...columns, colId];
    }
    dispatch("columnChange", newCols);
  }

  function closeHeaderMenu() {
    headerMenuVisible = false;
  }

  // --- Column reorder drag ---
  function onHeaderMouseDown(e: MouseEvent, index: number) {
    if (e.button !== 0) return;
    dragSrcIndex = index;
    dragOverIndex = index;
    dragging = true;

    const onMove = (ev: MouseEvent) => {
      if (!dragging) return;
      const headers = document.querySelectorAll(".col-hdr");
      let targetIndex = dragSrcIndex;
      headers.forEach((h, i) => {
        const rect = h.getBoundingClientRect();
        if (ev.clientX >= rect.left && ev.clientX <= rect.right) {
          targetIndex = i;
        }
      });
      dragOverIndex = targetIndex;
    };

    const onUp = () => {
      if (dragSrcIndex !== null && dragOverIndex !== null && dragSrcIndex !== dragOverIndex) {
        const newCols = [...columns];
        const [moved] = newCols.splice(dragSrcIndex, 1);
        newCols.splice(dragOverIndex, 0, moved);
        dispatch("columnChange", newCols);
      }
      dragSrcIndex = null;
      dragOverIndex = null;
      dragging = false;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  // --- Column resize ---
  function onResizeMouseDown(e: MouseEvent, col: string) {
    e.preventDefault();
    e.stopPropagation();
    resizeCol = col;
    resizeStartX = e.clientX;
    const header = document.querySelector(`.col-hdr[data-col="${col}"]`) as HTMLElement;
    resizeStartW = header ? header.offsetWidth : 80;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (ev: MouseEvent) => {
      if (!resizeCol) return;
      const delta = ev.clientX - resizeStartX;
      const newW = Math.max(36, resizeStartW + delta);
      colWidths = { ...colWidths, [resizeCol]: newW };
    };

    const onUp = () => {
      resizeCol = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function onHeaderClick(col: string) {
    onSort(col);
  }

  function onResizeKeydown(event: KeyboardEvent, col: string) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const current = colWidths[col] || Number.parseInt(DEFAULT_WIDTHS[col] || "80", 10) || 80;
    colWidths = { ...colWidths, [col]: Math.max(36, current + (event.key === "ArrowRight" ? 10 : -10)) };
  }

  // --- 标签单元格：拖动揭露 + 点击筛选 ---
  // 鼠标进入时检测是否溢出并切换 grab 光标
  function onTagsCellEnter(e: MouseEvent) {
    const cell = e.currentTarget as HTMLElement;
    cell.classList.toggle("is-scrollable", cell.scrollWidth > cell.clientWidth + 1);
  }

  function onTagsCellMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    const cell = e.currentTarget as HTMLElement;
    // 内容未溢出则无需启用拖动滚动，让 click 走原有路径
    if (cell.scrollWidth <= cell.clientWidth + 1) return;

    const startX = e.clientX;
    const startScroll = cell.scrollLeft;
    let moved = false;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      if (!moved && Math.abs(dx) >= TAG_DRAG_THRESHOLD) {
        moved = true;
        cell.classList.add("is-dragging");
      }
      if (moved) {
        cell.scrollLeft = startScroll - dx;
        ev.preventDefault();
      }
    };

    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      cell.classList.remove("is-dragging");

      if (moved) {
        // 在 capture 阶段拦截一次紧随其后的 click，避免误触发标签筛选 / 行选中
        const swallow = (ce: MouseEvent) => {
          ce.stopPropagation();
          ce.preventDefault();
          window.removeEventListener("click", swallow, true);
        };
        window.addEventListener("click", swallow, { capture: true, once: true });
        // 兜底：若释放在非可点击区域使 click 不会触发，下一帧清理监听器
        setTimeout(() => window.removeEventListener("click", swallow, true), 0);
      }
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  // 直接内联 colWidths 引用，确保 Svelte 能正确追踪依赖（仅引用 colWidth 函数会丢失 colWidths 依赖）
  $: gridCols = (showCover ? `${COVER_COL_WIDTH} ` : "") + columns
    .map(c => (colWidths[c] ? `${colWidths[c]}px` : (DEFAULT_WIDTHS[c] || "80px")))
    .join(" ");
</script>

<div class="book-list-simple">
  <div
    class="list-header"
    role="group"
    aria-label="图书列表列标题"
    style="grid-template-columns: {gridCols}"
    on:contextmenu={handleHeaderContextMenu}
  >
    {#if showCover}<span class="col-hdr col-cover-hdr" aria-hidden="true"></span>{/if}
    {#each columns as col, i}
      <span
        class="col-hdr"
        role="button"
        aria-label={`按${columnLabel(col)}排序`}
        tabindex="0"
        data-col={col}
        class:sorted={sortColumn === col}
        class:drag-over={dragOverIndex === i && dragSrcIndex !== i}
        class:dragging={dragSrcIndex === i && dragging}
        class:col-right={RIGHT_ALIGN_COLS.has(col)}
        class:col-center={CENTER_ALIGN_COLS.has(col)}
        on:mousedown={(e) => onHeaderMouseDown(e, i)}
        on:click={() => onHeaderClick(col)}
        on:keydown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onHeaderClick(col); } }}
      >
        <span class="col-label">{columnLabel(col)}</span>
        {#if sortColumn === col}<span class="sort-arrow">{sortAsc ? "▲" : "▼"}</span>{/if}
        <button
          type="button"
          class="col-resize-handle"
          aria-label={`调整${columnLabel(col)}列宽`}
          on:mousedown={(e) => onResizeMouseDown(e, col)}
          on:click|stopPropagation
          on:keydown={(event) => onResizeKeydown(event, col)}
        ></button>
      </span>
    {/each}
  </div>

  {#each books as book (book.id)}
    <div
      class="book-row"
      role="button"
      tabindex="0"
      aria-label={`${book.title}${book.author ? `，${book.author}` : ""}`}
      class:selected={selectedBook?.id === book.id}
      class:has-cover={showCover}
      style="grid-template-columns: {gridCols}"
      on:click={() => dispatch("select", book)}
      on:keydown={(event) => {
        if (event.key === "Enter") dispatch("open", book);
        else if (event.key === " ") { event.preventDefault(); dispatch("select", book); }
      }}
      on:dblclick={() => dispatch("open", book)}
      on:contextmenu={(e) => dispatch("context", { event: e, book })}
      data-context-type="library-book"
    >
      {#if showCover}
        <span class="col-cell col-cover-cell">
          {#if coverCache.get(book.id)}
            <img src={coverCache.get(book.id)} alt={book.title} />
          {:else}
            <span class="col-cover-placeholder {book.fileType}">{book.title[0] || "?"}</span>
          {/if}
        </span>
      {/if}
      {#each columns as col}
        {#if col === "tags"}
          <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
          <span
            class="col-cell col-tags-cell"
            role="group"
            aria-label="图书标签"
            on:mouseenter={onTagsCellEnter}
            on:mousedown={onTagsCellMouseDown}
          >
            {#each (book.tags || []) as tag}
              <button
                type="button"
                class="col-tag"
                class:active={activeTagFilters.includes(tag)}
                title={activeTagFilters.includes(tag) ? `点击移除筛选: ${tag}` : `点击按"${tag}"筛选`}
                on:click|stopPropagation={() => onTagClick(tag)}
              >{tag}</button>
            {/each}
          </span>
        {:else}
          <span
            class="col-cell"
            class:col-primary={col === "title"}
            class:col-muted={col !== "title" && col !== "fileType" && col !== "filename"}
            class:col-accent={col === "fileType"}
            class:col-right={RIGHT_ALIGN_COLS.has(col)}
            class:col-center={CENTER_ALIGN_COLS.has(col)}
            title={cellValue(book, col)}
          >{cellValue(book, col)}</span>
        {/if}
      {/each}
    </div>
  {/each}
</div>

{#if headerMenuVisible}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="header-menu-overlay" role="presentation" on:click={closeHeaderMenu}>
    <div
      class="header-menu"
      role="dialog"
      aria-label="选择显示列"
      tabindex="-1"
      style="left: {headerMenuPos.x}px; top: {headerMenuPos.y}px;"
      on:click|stopPropagation
      on:contextmenu|stopPropagation
    >
      <div class="header-menu-title">选择列</div>
      {#each ALL_COLUMNS as colDef}
        <label class="header-menu-item">
          <input
            type="checkbox"
            checked={columns.includes(colDef.id)}
            disabled={colDef.id === "title" && columns.length === 1}
            on:change={() => toggleColumn(colDef.id)}
          />
          <span>{colDef.label}</span>
        </label>
      {/each}
    </div>
  </div>
{/if}

<style>
  .book-list-simple { display: flex; flex-direction: column; user-select: none; }

  .list-header {
    display: grid;
    gap: 8px;
    padding: 4px 12px;
    font-size: 10px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid var(--color-border);
    position: sticky;
    top: 0;
    background: var(--color-canvas);
    z-index: 2;
    cursor: default;
  }

  .col-hdr {
    position: relative;
    display: flex;
    align-items: center;
    white-space: nowrap;
    overflow: visible;
    padding: 2px 10px 2px 4px;
    border-radius: 3px;
    transition: background 0.15s, box-shadow 0.15s;
    cursor: pointer;
  }

  .col-hdr:hover { background: var(--color-hover); }

  .col-hdr.sorted {
    color: var(--color-accent);
    font-weight: 700;
  }

  .sort-arrow {
    font-size: 8px;
    margin-left: 2px;
    vertical-align: middle;
    flex-shrink: 0;
  }

  .col-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .col-hdr.drag-over {
    background: var(--color-accent-soft);
    box-shadow: inset 2px 0 0 var(--color-accent);
  }

  .col-hdr.dragging { opacity: 0.4; }

  /* Column resize handle */
  .col-resize-handle {
    position: absolute;
    right: -4px;
    top: 2px;
    bottom: 2px;
    width: 10px;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: col-resize;
    z-index: 3;
    border-radius: 3px;
    transition: background 0.12s;
  }

  .col-hdr:hover .col-resize-handle {
    background: var(--color-border);
  }

  .col-resize-handle:hover,
  .col-resize-handle:active {
    background: var(--color-accent);
  }

  .book-row {
    display: grid;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background var(--transition-fast);
    border: 1px solid transparent;
    font-size: 13px;
  }

  .book-row:hover { background: var(--color-hover); }

  .book-row.selected {
    background: var(--color-accent-soft);
    border-color: var(--color-accent);
  }

  .col-cell { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .col-primary { color: var(--color-text); font-weight: 600; }
  .col-muted { color: var(--color-muted); font-size: 12px; }
  .col-accent { color: var(--color-accent); font-weight: 700; font-size: 11px; }

  /* 封面列（list-cover 视图）：表头占位、行内渲染缩略图 */
  .col-cover-hdr { cursor: default; }
  .col-cover-hdr:hover { background: transparent; }
  .book-row.has-cover { align-items: center; padding-top: 4px; padding-bottom: 4px; }
  .col-cover-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 50px;
    border-radius: 3px;
    overflow: hidden;
    background: var(--color-surface-soft);
  }
  .col-cover-cell img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .col-cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 16px;
    font-weight: 700;
    color: var(--color-muted);
  }
  .col-cover-placeholder.epub {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
  }

  /* 标签列：内部多个小 chip，水平排列；列宽不足时允许指针拖动横向滚动揭露 */
  .col-tags-cell {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;            /* Firefox：隐藏滚动条 */
    -ms-overflow-style: none;         /* 旧版 IE/Edge */
    /* 边缘渐隐提示更多内容（无溢出时不可见） */
    -webkit-mask-image: linear-gradient(90deg, transparent 0, #000 8px, #000 calc(100% - 8px), transparent 100%);
            mask-image: linear-gradient(90deg, transparent 0, #000 8px, #000 calc(100% - 8px), transparent 100%);
  }
  .col-tags-cell::-webkit-scrollbar { display: none; } /* Chrome/Safari */

  /* 仅当存在溢出时（由 onTagsCellEnter 注入类）才显示可拖动光标。
     用 :global() 包裹动态类，避免 Svelte 因静态分析视其为未使用而裁剪。 */
  .col-tags-cell:global(.is-scrollable) { cursor: grab; }
  .col-tags-cell:global(.is-dragging) { cursor: grabbing; }

  .col-tag {
    flex-shrink: 0;
    padding: 1px 8px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.5;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text-soft);
    border-radius: 999px;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .col-tag:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    border-color: var(--color-accent);
  }

  /* 已激活筛选的标签：实色高亮，区分于普通可点击标签 */
  .col-tag.active {
    background: var(--color-accent);
    color: #fff;
    border-color: var(--color-accent);
  }

  /* 数字类列右对齐：表头作为 flex 容器使用 justify-content，单元格使用 text-align */
  .col-cell.col-right { text-align: right; }
  .col-hdr.col-right { justify-content: flex-end; padding-right: 14px; }
  .col-hdr.col-right .col-resize-handle { right: -4px; }

  /* 格式 / 日期等列居中对齐 */
  .col-cell.col-center { text-align: center; }
  .col-hdr.col-center { justify-content: center; padding-right: 14px; }
  .col-hdr.col-center .col-resize-handle { right: -4px; }

  .header-menu-overlay {
    position: fixed; top: 0; left: 0; right: 0; bottom: 0;
    z-index: 1000; background: transparent;
  }

  .header-menu {
    position: fixed;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-pop);
    padding: 4px; min-width: 160px; z-index: 1001;
  }

  .header-menu-title {
    font-size: 11px; font-weight: 700; color: var(--color-muted);
    padding: 6px 12px 8px; border-bottom: 1px solid var(--color-border);
    margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.5px;
  }

  .header-menu-item {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 12px; font-size: 13px; color: var(--color-text);
    cursor: pointer; border-radius: var(--radius-xs);
    transition: background var(--transition-fast);
  }

  .header-menu-item:hover { background: var(--color-hover); }
  .header-menu-item input[type="checkbox"] { accent-color: var(--color-accent); }
</style>
