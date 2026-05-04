<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let books: any[] = [];
  export let coverCache: Map<string, string> = new Map();
  export let selectedBook: any = null;
  export let showTitle: boolean = true;
  export let showAuthor: boolean = true;

  const dispatch = createEventDispatcher();
</script>

<div class="book-grid">
  {#each books as book (book.id)}
    <div
      class="book-card"
      class:selected={selectedBook?.id === book.id}
      on:click={() => dispatch("select", book)}
      on:dblclick={() => dispatch("open", book)}
      on:contextmenu={(e) => dispatch("context", { event: e, book })}
      data-context-type="library-book"
    >
      <div class="book-cover">
        {#if coverCache.get(book.id)}
          <img src={coverCache.get(book.id)} alt={book.title} />
        {:else}
          <div class="cover-placeholder {book.fileType}">
            <span class="cover-title">{book.title[0] || "?"}</span>
          </div>
        {/if}
      </div>
      {#if showTitle || showAuthor}
        <div class="book-info">
          {#if showTitle}
            <div class="book-title" title={book.title}>{book.title}</div>
          {/if}
          {#if showAuthor && book.author}
            <div class="book-author" title={book.author}>{book.author}</div>
          {/if}
        </div>
      {/if}
      <span class="book-type-badge">{book.fileType.toUpperCase()}</span>
    </div>
  {/each}
</div>

<style>
  .book-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 14px;
    padding: 4px;
  }

  .book-card {
    background: var(--color-surface);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-xs);
    overflow: hidden;
    cursor: pointer;
    transition: box-shadow var(--transition-normal), transform var(--transition-fast);
    position: relative;
    border: 2px solid transparent;
  }

  .book-card:hover {
    box-shadow: var(--shadow-sm);
    transform: translateY(-2px);
  }

  .book-card.selected {
    border-color: var(--color-accent);
    box-shadow: var(--shadow-md);
  }

  .book-cover {
    width: 100%;
    aspect-ratio: 3/4;
    background: var(--color-surface-soft);
    overflow: hidden;
  }

  .book-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .cover-placeholder {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 48px;
    color: var(--color-muted);
  }

  .cover-placeholder.epub {
    background: linear-gradient(135deg, var(--color-accent-soft), var(--color-accent-quiet));
    color: var(--color-accent-deep);
  }

  .cover-placeholder.txt {
    background: linear-gradient(135deg, var(--color-surface-soft), var(--color-border));
    color: var(--color-text-soft);
  }

  .cover-title {
    font-size: 48px;
    font-weight: 700;
  }

  .book-info {
    padding: 8px 10px;
  }

  .book-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .book-author {
    font-size: 11px;
    color: var(--color-muted);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .book-type-badge {
    position: absolute;
    top: 6px;
    right: 6px;
    background: rgba(0, 0, 0, 0.60);
    color: #fff;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    font-weight: 700;
    letter-spacing: 0.5px;
  }
</style>
