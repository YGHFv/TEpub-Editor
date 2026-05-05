<script lang="ts">
  // 标签编辑器：三级固定分类（男频/女频/出版 → 子类 → 主角）+ 自由自定义标签
  // 用法：
  //   <TagsEditor bind:tags={metaForm.tags} suggestions={librarySuggestions} />
  // suggestions：从外部库收集来的"自定义标签"候选列表，用于建议补全；可选

  export let tags: string[] = [];
  export let suggestions: string[] = [];

  // ---- 标签分类体系 ----
  const TAG_L1: readonly string[] = ["男频", "女频", "出版"];
  const TAG_L2: Record<string, string[]> = {
    "男频": ["玄幻", "奇幻", "武侠", "仙侠", "都市", "现实", "历史", "军事", "游戏", "体育", "科幻", "悬疑", "灵异", "无限流", "轻小说"],
    "女频": ["古代言情", "现代言情", "玄幻言情", "悬疑推理", "浪漫青春", "仙侠奇缘", "科幻空间", "游戏竞技", "现实生活", "轻小说"],
    "出版": ["文学", "小说", "散文诗歌", "历史", "传记", "哲学", "社科", "心理", "经管", "法律", "军事", "科技", "艺术", "教育", "励志", "健康", "旅游", "漫画", "童书", "工具书"],
  };
  const TAG_L3: Record<string, string[]> = {
    "男频": ["单女主", "多女主", "无女主"],
  };
  const TAXONOMY_SET: Set<string> = new Set([
    ...TAG_L1,
    ...Object.values(TAG_L2).flat(),
    ...Object.values(TAG_L3).flat(),
  ]);

  // ---- 内部状态 ----
  let tagInput = "";
  let tagPanelOpen = false;

  // ---- 操作 ----
  function commitTag() {
    const value = tagInput.trim();
    if (!value) return;
    const parts = value.split(/[,，]/).map(s => s.trim()).filter(Boolean);
    let next = [...tags];
    for (const p of parts) {
      if (!next.includes(p)) next.push(p);
    }
    tags = next;
    tagInput = "";
  }

  function removeTagByName(name: string) {
    tags = tags.filter(t => t !== name);
  }

  function onTagKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === "," || e.key === "，") {
      e.preventDefault();
      commitTag();
    } else if (e.key === "Backspace" && !tagInput && tags.length > 0) {
      e.preventDefault();
      tags = tags.slice(0, -1);
    } else if (e.key === "Escape") {
      tagPanelOpen = false;
      (e.currentTarget as HTMLElement).blur();
    }
  }

  // ---- 派生 ----
  $: currentL1 = tags.find(t => TAG_L1.includes(t)) || "";
  $: currentL2List = currentL1 ? (TAG_L2[currentL1] || []) : [];
  $: currentL2 = tags.find(t => currentL2List.includes(t)) || "";
  $: currentL3List = currentL1 && TAG_L3[currentL1] ? TAG_L3[currentL1] : [];
  $: currentL3 = tags.find(t => currentL3List.includes(t)) || "";
  $: customTagsOnBook = tags.filter(t => !TAXONOMY_SET.has(t));
  $: orderedTagsForDisplay = [
    ...(currentL1 ? [currentL1] : []),
    ...(currentL2 ? [currentL2] : []),
    ...(currentL3 ? [currentL3] : []),
    ...customTagsOnBook,
  ];

  $: tiersComplete = (() => {
    if (!currentL1) return false;
    if (currentL1 === "男频") return !!(currentL2 && currentL3);
    return !!currentL2;
  })();

  $: tagPanelPhase = (tiersComplete || tagInput.trim()) ? "custom" : "tiers";

  // 候选建议：外部 suggestions 减去已加上的标签 + 排序
  $: librarySuggestions = (() => {
    const seen = new Set<string>();
    for (const t of suggestions) {
      if (!t) continue;
      if (TAXONOMY_SET.has(t)) continue;
      seen.add(t);
    }
    for (const t of tags) seen.delete(t);
    return Array.from(seen).sort((a, b) => a.localeCompare(b, "zh"));
  })();

  $: filteredSuggestions = (() => {
    const q = tagInput.trim().toLowerCase();
    if (!q) return librarySuggestions;
    return librarySuggestions.filter(t => t.toLowerCase().includes(q));
  })();

  function selectL1(value: string) {
    let next = tags.filter(t => !TAG_L1.includes(t));
    const newL2List = TAG_L2[value] || [];
    const newL3List = TAG_L3[value] || [];
    next = next.filter(t => {
      if (!TAXONOMY_SET.has(t)) return true;
      if (TAG_L1.includes(t)) return false;
      return newL2List.includes(t) || newL3List.includes(t);
    });
    if (currentL1 !== value) {
      next = [value, ...next];
    }
    tags = next;
  }

  function selectL2(value: string) {
    let next = tags.filter(t => !currentL2List.includes(t));
    if (currentL2 !== value) next = [...next, value];
    tags = next;
  }

  function selectL3(value: string) {
    let next = tags.filter(t => !currentL3List.includes(t));
    if (currentL3 !== value) next = [...next, value];
    tags = next;
  }

  function addSuggestion(value: string) {
    if (!tags.includes(value)) {
      tags = [...tags, value];
    }
  }

  function onTagsEditorFocusOut(e: FocusEvent) {
    const ct = e.currentTarget as HTMLElement;
    const next = e.relatedTarget as Node | null;
    if (!next || !ct.contains(next)) {
      setTimeout(() => {
        if (!document.activeElement || !ct.contains(document.activeElement)) {
          commitTag();
          tagPanelOpen = false;
        }
      }, 0);
    }
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="tags-editor"
  class:open={tagPanelOpen}
  on:focusout={onTagsEditorFocusOut}
>
  <div class="tags-chip-row">
    {#each orderedTagsForDisplay as tag (tag)}
      <span class="tag-chip" class:tag-chip-tier={TAXONOMY_SET.has(tag)}>
        <span class="tag-chip-text">{tag}</span>
        <button
          type="button"
          class="tag-chip-remove"
          on:click={() => removeTagByName(tag)}
          title="移除标签"
        >×</button>
      </span>
    {/each}
    <input
      class="tag-input"
      type="text"
      bind:value={tagInput}
      placeholder={tags.length === 0 ? "点击此处选择/添加标签…" : "+ 标签"}
      on:focus={() => tagPanelOpen = true}
      on:click={() => tagPanelOpen = true}
      on:keydown={onTagKeydown}
    />
  </div>

  {#if tagPanelOpen}
    <div class="tag-panel">
      {#if tagPanelPhase === "tiers"}
        <div class="tier-row">
          <span class="tier-label">类型</span>
          <div class="tier-options">
            {#each TAG_L1 as opt}
              <button
                type="button"
                class="tier-opt"
                class:active={currentL1 === opt}
                on:click={() => selectL1(opt)}
              >{opt}</button>
            {/each}
          </div>
        </div>

        {#if currentL1 && currentL2List.length > 0}
          <div class="tier-row">
            <span class="tier-label">分类</span>
            <div class="tier-options">
              {#each currentL2List as opt}
                <button
                  type="button"
                  class="tier-opt"
                  class:active={currentL2 === opt}
                  on:click={() => selectL2(opt)}
                >{opt}</button>
              {/each}
            </div>
          </div>
        {/if}

        {#if currentL3List.length > 0}
          <div class="tier-row">
            <span class="tier-label">主角</span>
            <div class="tier-options">
              {#each currentL3List as opt}
                <button
                  type="button"
                  class="tier-opt"
                  class:active={currentL3 === opt}
                  on:click={() => selectL3(opt)}
                >{opt}</button>
              {/each}
            </div>
          </div>
        {/if}

        <div class="tag-panel-hint">
          选完前三级后,可在此添加自定义标签
        </div>
      {:else}
        {#if filteredSuggestions.length > 0}
          <div class="tier-row">
            <span class="tier-label">建议</span>
            <div class="tier-options tier-suggestions">
              {#each filteredSuggestions.slice(0, 60) as t}
                <button
                  type="button"
                  class="tier-opt suggest"
                  on:click={() => addSuggestion(t)}
                >+ {t}</button>
              {/each}
              {#if filteredSuggestions.length > 60}
                <span class="tier-more">…还有 {filteredSuggestions.length - 60}</span>
              {/if}
            </div>
          </div>
        {:else if tagInput.trim()}
          <div class="tag-panel-empty">
            没有匹配 "<b>{tagInput.trim()}</b>" 的标签 — 回车将其作为新标签添加
          </div>
        {:else}
          <div class="tag-panel-empty">
            输入文字搜索建议标签,或按回车直接添加新标签
          </div>
        {/if}

        <div class="tag-panel-hint">
          回车 / 逗号 添加;退格 删除最后一个;Esc 关闭
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tags-editor {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
  }

  .tags-editor:focus-within,
  .tags-editor.open {
    border-color: var(--color-accent);
    box-shadow: var(--focus-ring);
  }

  .tags-chip-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 4px 6px;
  }

  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px 2px 10px;
    border-radius: 999px;
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.4;
    max-width: 100%;
  }

  .tag-chip.tag-chip-tier {
    background: var(--color-accent);
    color: #fff;
  }

  .tag-chip-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag-chip-remove {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border: none;
    background: transparent;
    color: inherit;
    border-radius: 50%;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    opacity: 0.6;
    transition: background var(--transition-fast), opacity var(--transition-fast);
  }

  .tag-chip-remove:hover {
    background: rgba(0, 0, 0, 0.15);
    opacity: 1;
  }

  .tag-input {
    flex: 1;
    min-width: 80px;
    border: none !important;
    outline: none;
    background: transparent !important;
    color: var(--color-text);
    font-size: 12px !important;
    padding: 4px 6px !important;
  }

  .tag-input:focus {
    box-shadow: none !important;
  }

  .tag-panel {
    margin-top: 4px;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--color-surface);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-pop);
    /* 不再用 absolute 浮层：避免被外层 modal 的 overflow:hidden 裁切。
       内嵌后会撑开 .tags-editor 的高度，外层 set-row 自然让位。 */
    max-height: 280px;
    overflow-y: auto;
    animation: tagPanelIn 0.12s ease-out;
  }

  @keyframes tagPanelIn {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .tier-row {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    flex-wrap: nowrap;
  }

  .tier-label {
    flex-shrink: 0;
    width: 32px;
    margin-top: 4px;
    font-size: 10px;
    font-weight: 700;
    color: var(--color-muted);
    text-transform: uppercase;
    letter-spacing: 0.4px;
    text-align: right;
  }

  .tier-options {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .tier-options.tier-suggestions {
    max-height: 96px;
    overflow-y: auto;
  }

  .tier-opt {
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text-soft);
    padding: 3px 10px;
    border-radius: 999px;
    font-size: 11px;
    cursor: pointer;
    line-height: 1.6;
    transition: background var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .tier-opt:hover {
    background: var(--color-hover);
    color: var(--color-text);
    border-color: var(--color-text-soft);
  }

  .tier-opt.active {
    background: var(--color-accent);
    color: #fff;
    border-color: var(--color-accent);
  }

  .tier-opt.suggest {
    color: var(--color-text-soft);
    border-style: dashed;
  }

  .tier-opt.suggest:hover {
    background: var(--color-accent-soft);
    color: var(--color-accent-deep);
    border-color: var(--color-accent);
    border-style: solid;
  }

  .tier-more {
    align-self: center;
    font-size: 10px;
    color: var(--color-muted);
    padding: 0 6px;
  }

  .tag-panel-hint {
    font-size: 10px;
    color: var(--color-muted);
    border-top: 1px dashed var(--color-border);
    padding-top: 6px;
    text-align: center;
  }

  .tag-panel-empty {
    padding: 12px 4px;
    font-size: 11px;
    color: var(--color-muted);
    text-align: center;
    line-height: 1.5;
  }

  .tag-panel-empty b {
    color: var(--color-text-soft);
  }
</style>
