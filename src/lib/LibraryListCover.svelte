<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let books: any[] = [];
  export let coverCache: Map<string, string> = new Map();
  export let selectedBook: any = null;
  export let formatFileSize: (bytes: number) => string = (b) => b + " B";

  const dispatch = createEventDispatcher();
</script>

<div class="book-list-cover">
  {#each books as book (book.id)}
    <div
      class="book-row"
      class:selected={selectedBook?.id === book.id}
      on:click={() => dispatch("select", book)}
      on:dblclick={() => dispatch("open", book)}
      on:contextmenu={(e) => dispatch("context", { event: e, book })}
      data-context-type="library-book"
    >
      <div class="row-cover">
        {#if coverCache.get(book.id)}
          <img src={coverCache.get(book.id)} alt={book.title} />
        {:else}
          <div class="row-cover-placeholder {book.fileType}">
            {book.title[0] || "?"}
          </div>
        {/if}
      </div>
      <div class="row-info">
        <div class="row-title" title={book.title}>{book.title}</div>
        <div class="row-meta">
          {#if book.author}<span class="row-author">{book.author}</span>{/if}
          <span class="row-size">{formatFileSize(book.fileSize)}</span>
          <span class="row-type">{book.fileType.toUpperCase()}</span>
        </div>
      </div>
    </div>
  {/each}
</div>

<style>
  .book-list-cover {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .book-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--transition-fast);
    border: 1px solid transparent;
  }

  .book-row:hover {
    background: var(--color-hover);
  }

  .book-row.selected {
    background: var(--color-accent-soft);
    border-color: var(--color-accent);
  }

  .row-cover {
    width: 40px;
    height: 56px;
    flex-shrink: 0;
    border-radius: 3px;
    overflow: hidden;
    background: var(--color-surface-soft);
  }

  .row-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .row-cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    font-weight: 700;
    color: var(--color-muted);
  }

  .row-cover-placeholder.epub {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
  }

  .row-info {
    flex: 1;
    min-width: 0;
  }

  .row-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row-meta {
    display: flex;
    gap: 12px;
    margin-top: 3px;
    font-size: 11px;
    color: var(--color-muted);
  }

  .row-type {
    font-weight: 700;
    color: var(--color-accent);
  }
</style>
