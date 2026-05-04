<script lang="ts">
  /**
   * 阅读器目录的递归节点。负责：
   *   1. 显示一行（缩进按 level 计算）
   *   2. 有子节点时显示 ▾/▸ 折叠按钮
   *   3. 当前章节高亮 + 自动展开其祖先（即使用户之前手动折叠了，含当前章节
   *      的父节点会被强制展开，确保高亮项始终可见）
   *   4. 用 data-current 标识当前节点，父级 reader 可用 querySelector 自动滚动
   *
   * Svelte 4 兼容写法（项目其他组件也是 export let + $:），方便 svelte 5 兼容模式。
   */
  export interface TocNode {
    title: string;
    spineIdx: number;     // -1 表示无对应 spine 项（仅作分组标题）
    children: TocNode[];
  }

  export let node: TocNode;
  export let currentChapterIdx: number;
  export let level: number = 0;
  export let onSelect: (spineIdx: number) => void;

  // 用户主动折叠了吗？默认不折叠（即默认全展开）
  let userCollapsed = false;

  function containsCurrent(n: TocNode): boolean {
    if (n.spineIdx === currentChapterIdx && n.spineIdx >= 0) return true;
    for (const c of n.children) if (containsCurrent(c)) return true;
    return false;
  }

  $: hasChildren = node.children.length > 0;
  $: isCurrent = node.spineIdx === currentChapterIdx && node.spineIdx >= 0;
  $: hasCurrent = containsCurrent(node);
  // 子树中含当前章节时强制展开 —— 哪怕用户之前折叠过，也要让高亮项可见
  $: expanded = hasCurrent || !userCollapsed;

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    userCollapsed = !userCollapsed;
  }

  function handleClick() {
    if (node.spineIdx >= 0) onSelect(node.spineIdx);
  }
</script>

<div class="toc-row" class:current={isCurrent} data-current={isCurrent ? "1" : ""}>
  {#if hasChildren}
    <button
      type="button"
      class="toc-toggle"
      on:click={toggle}
      title={expanded ? "折叠" : "展开"}
    >{expanded ? "▾" : "▸"}</button>
  {:else}
    <span class="toc-toggle-spacer"></span>
  {/if}
  <button
    type="button"
    class="toc-label"
    style="padding-left: {level * 14 + 4}px"
    on:click={handleClick}
    disabled={node.spineIdx < 0}
    title={node.title}
  >{node.title}</button>
</div>

{#if hasChildren && expanded}
  {#each node.children as child}
    <svelte:self
      node={child}
      {currentChapterIdx}
      level={level + 1}
      {onSelect}
    />
  {/each}
{/if}

<style>
  .toc-row {
    display: flex;
    align-items: center;
    gap: 2px;
    border-radius: 4px;
    line-height: 1.4;
  }
  .toc-row.current {
    background: rgba(0, 0, 0, 0.06);
  }
  .toc-toggle,
  .toc-toggle-spacer {
    flex-shrink: 0;
    width: 18px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .toc-toggle {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--rd-text-soft, #888);
    cursor: pointer;
    font-size: 10px;
    border-radius: 4px;
  }
  .toc-toggle:hover {
    background: rgba(0, 0, 0, 0.06);
    color: var(--rd-text, inherit);
  }
  .toc-label {
    flex: 1;
    min-width: 0;
    text-align: left;
    background: transparent;
    border: 0;
    padding: 6px 6px;
    color: var(--rd-text, inherit);
    cursor: pointer;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border-radius: 4px;
  }
  .toc-label:hover {
    background: rgba(0, 0, 0, 0.04);
  }
  .toc-row.current .toc-label {
    color: var(--rd-accent, inherit);
    font-weight: 600;
  }
  .toc-label:disabled {
    cursor: default;
    opacity: 0.65;
  }
</style>
