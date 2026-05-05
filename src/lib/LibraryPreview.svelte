<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let book: any = null;
  export let coverCache: Map<string, string> = new Map();
  export let formatFileSize: (bytes: number) => string = (b) => b + " B";

  const dispatch = createEventDispatcher();

  function formatDate(ts: number | undefined): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    return d.toLocaleDateString("zh-CN");
  }

  // 把简介按换行拆成段落；trim 后前置两个全角空格作为统一缩进。
  // 用直接在文本里加缩进字符（U+3000）而非 CSS text-indent，
  // 避免某些 CSS 特异性 / 重置规则导致的"首段不缩进"问题。
  const INDENT = "　　"; // 两个全角空格
  function splitDescription(text: string | undefined): string[] {
    if (!text) return [];
    return text
      .split(/\r?\n+/)
      .map(p => p.trim())
      .filter(p => p.length > 0)
      .map(p => INDENT + p);
  }
</script>

<div class="preview-panel">
  {#if book}
    <div class="preview-cover">
      {#if coverCache.get(book.id)}
        <img src={coverCache.get(book.id)} alt={book.title} />
      {:else}
        <div class="preview-cover-placeholder {book.fileType}">
          <span class="preview-cover-letter">{book.title[0] || "?"}</span>
        </div>
      {/if}
    </div>
    <div class="preview-info">
      <h2 class="preview-title">{book.title}</h2>
      {#if book.subtitle}
        <div class="preview-subtitle">{book.subtitle}</div>
      {/if}
      {#if book.author}
        <div class="preview-author">作者：{book.author}</div>
      {/if}
      {#if book.tags && book.tags.length > 0}
        <div class="preview-tags">
          {#each book.tags as tag}
            <span class="preview-tag" title={tag}>{tag}</span>
          {/each}
        </div>
      {/if}
      <div class="preview-meta">
        <div class="meta-item">
          <span class="meta-label">文件名</span>
          <span class="meta-value">{book.filename || "-"}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">格式</span>
          <span class="meta-value">{book.fileType.toUpperCase()}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">大小</span>
          <span class="meta-value">{formatFileSize(book.fileSize)}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">添加日期</span>
          <span class="meta-value">{formatDate(book.addedAt)}</span>
        </div>
        {#if book.createdAt}
          <div class="meta-item">
            <span class="meta-label">创建日期</span>
            <span class="meta-value">{formatDate(book.createdAt)}</span>
          </div>
        {/if}
        {#if book.modifiedAt}
          <div class="meta-item">
            <span class="meta-label">修改日期</span>
            <span class="meta-value">{formatDate(book.modifiedAt)}</span>
          </div>
        {/if}
      </div>
      {#if book.description}
        <div class="preview-desc">
          {#each splitDescription(book.description) as para}
            <p>{para}</p>
          {/each}
        </div>
      {/if}
    </div>
    <div class="preview-actions">
      <button class="preview-btn primary" on:click={() => dispatch("open")}>阅读</button>
      <button class="preview-btn danger" on:click={() => dispatch("remove")}>移除</button>
    </div>
  {:else}
    <div class="preview-empty">
      <div class="preview-empty-icon">📖</div>
      <div class="preview-empty-text">选择一本书查看详情</div>
    </div>
  {/if}
</div>

<style>
  .preview-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    box-sizing: border-box;
    overflow-y: auto;
  }

  .preview-cover {
    width: 120px;
    height: 160px;
    margin: 0 auto 12px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    overflow: hidden;
    background: var(--color-surface);
    box-shadow: var(--shadow-sm);
  }

  .preview-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .preview-cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-surface);
  }

  .preview-cover-letter {
    font-size: 42px;
    font-weight: 700;
    color: var(--color-muted);
  }

  .preview-info {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .preview-title {
    font-size: 16px;
    font-weight: 700;
    color: var(--color-text);
    margin: 0 0 2px 0;
    line-height: 1.3;
  }

  .preview-subtitle {
    font-size: 13px;
    color: var(--color-text-soft);
    margin-bottom: 2px;
  }

  .preview-author {
    font-size: 13px;
    color: var(--color-accent);
    margin-bottom: 10px;
  }

  /* 标签胶囊：用于显示出版社等元数据，区别于普通 key/value */
  .preview-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0 0 12px 0;
  }

  .preview-tag {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    font-size: 11px;
    font-weight: 600;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .meta-item {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }

  .meta-label {
    color: var(--color-muted);
  }

  .meta-value {
    color: var(--color-text-soft);
    font-weight: 600;
  }

  .preview-desc {
    font-size: 12px;
    color: var(--color-text-soft);
    line-height: 1.5;
    /* 不再限制高度 / 不再单独滚动；交给外层 .preview-panel 整体滚 */
    margin-top: 4px;
  }

  /* 简介每段段间间距；缩进通过预置全角空格实现，不再依赖 text-indent */
  .preview-desc p {
    margin: 0;
  }

  .preview-desc p + p {
    margin-top: 4px;
  }

  .preview-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .preview-btn {
    flex: 1;
    padding: 10px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-text);
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    transition: background var(--transition-fast);
    text-align: center;
  }

  .preview-btn:hover {
    background: var(--color-hover);
  }

  .preview-btn.primary {
    background: var(--gradient-accent);
    color: #fff;
    border: none;
  }

  .preview-btn.primary:hover {
    opacity: 0.9;
  }

  .preview-btn.danger {
    color: var(--color-danger);
    flex: 0.5;
  }

  .preview-btn.danger:hover {
    background: var(--color-danger-soft);
  }

  .preview-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-muted);
    gap: 8px;
  }

  .preview-empty-icon {
    font-size: 48px;
    opacity: 0.4;
  }

  .preview-empty-text {
    font-size: 13px;
  }
</style>
