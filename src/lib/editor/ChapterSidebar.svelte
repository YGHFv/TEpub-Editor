<script module lang="ts">
    export interface TocNode {
        id: string;
        title: string;
        line_number: number;
        type: "Volume" | "Chapter" | "Meta";
        word_count: number;
        children: TocNode[];
        expanded: boolean;
        parentId?: string;
        level?: number;
    }
</script>

<script lang="ts">
    let {
        stats,
        tocTree,
        activeChapterId,
        invalidSequenceIds,
        checkActive,
        onToggleAllVolumes,
        onOpenProofCheck,
        onToggleVolume,
        onChapterClick,
    }: {
        stats: { volumes: number; chapters: number };
        tocTree: TocNode[];
        activeChapterId: string;
        invalidSequenceIds: Set<string>;
        checkActive: boolean;
        onToggleAllVolumes: () => void;
        onOpenProofCheck: () => void;
        onToggleVolume: (node: TocNode) => void;
        onChapterClick: (id: string, line: number) => void;
    } = $props();
</script>

<aside class="sidebar">
    <!-- 头部固定，不再随列表滚动 -->
    <div class="sidebar-header-fixed">
        <div class="sidebar-header-row">
            <span>{stats.volumes}卷 {stats.chapters}章</span>
            <div class="header-btns">
                <button
                    class="icon-btn"
                    title="全部展开/折叠"
                    onclick={onToggleAllVolumes}>⇅</button
                >
                <button
                    class="mini-btn {checkActive ? 'active' : ''}"
                    onpointerdown={onOpenProofCheck}
                    onclick={onOpenProofCheck}>检查</button
                >
            </div>
        </div>
    </div>

    <div class="toc-list">
        {#each tocTree as node (node.id)}
            <div
                role="button"
                tabindex="0"
                id={`toc-${node.id}`}
                class="toc-item {node.type === 'Volume'
                    ? 'vol-title'
                    : ''} {activeChapterId === node.id ? 'active' : ''}"
                onclick={() =>
                    node.type === "Volume"
                        ? onToggleVolume(node)
                        : onChapterClick(node.id, node.line_number)}
                onkeydown={() => {}}
            >
                {#if node.type === "Volume"}
                    <span class="arrow">{node.expanded ? "▼" : "▶"}</span>
                {/if}
                <span
                    class="toc-title {invalidSequenceIds.has(node.id)
                        ? 'text-error'
                        : ''}">{node.title}</span
                >
                <span class="toc-count"
                    >{node.type === "Volume"
                        ? node.children.length
                        : node.word_count}</span
                >
            </div>

            {#if node.expanded}
                {#each node.children as child (child.id)}
                    <div
                        role="button"
                        tabindex="0"
                        id={`toc-${child.id}`}
                        class="toc-item indent {activeChapterId === child.id
                            ? 'active'
                            : ''}"
                        onclick={() =>
                            onChapterClick(child.id, child.line_number)}
                        onkeydown={() => {}}
                    >
                        <span
                            class="toc-title {invalidSequenceIds.has(child.id)
                                ? 'text-error'
                                : ''}">{child.title}</span
                        >
                        <span class="toc-count">{child.word_count}</span>
                    </div>
                {/each}
            {/if}
        {/each}
    </div>
</aside>

<style>
    /* 基础按钮样式（与页面同源，保证拆出后视觉一致） */
    button {
        height: 34px;
        min-width: 40px;
        border-radius: 6px;
        border: 1px solid #ccc;
        background: #fff;
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: #eee;
        transform: scale(0.96);
    }

    .sidebar {
        width: 280px;
        background: #f8f8f8;
        border-right: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
    }

    .sidebar-header-fixed {
        background: #eee;
        border-bottom: 1px solid #ddd;
        flex-shrink: 0;
        z-index: 20;
    }
    .sidebar-header-row {
        padding: 10px;
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        font-weight: bold;
        align-items: center;
    }
    .header-btns {
        display: flex;
        gap: 5px;
    }
    .icon-btn {
        width: 26px;
        height: 26px;
        padding: 0;
        font-size: 14px;
        border: 1px solid #ccc;
        background: #fff;
        cursor: pointer;
        border-radius: 4px;
    }

    .toc-list {
        flex: 1;
        overflow-y: auto;
    }
    .toc-item {
        padding: 12px;
        font-size: 14px;
        border-bottom: 1px solid #eee;
        display: flex;
        /* justify-content: space-between; Removed to fix centering issue */
        align-items: center;
        cursor: pointer;
        cursor: pointer;
        position: relative; /* Fix z-index stacking */
        z-index: 1;
    }
    .indent {
        padding-left: 28px;
        background: #fafafa;
    }
    .toc-item.active {
        background: #d4e8fa;
        color: #0066b8;
        border-left: 4px solid #0066b8;
        font-weight: bold;
    }
    /* 卷标吸顶 */
    .vol-title {
        background: #eaeaea;
        font-weight: bold;
        position: sticky;
        top: 0;
        z-index: 10;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    }
    .text-error {
        color: #d32f2f;
        font-weight: bold;
    }
    .toc-count {
        color: #999;
        font-size: 11px;
        margin-left: auto; /* Push to right */
    }
    .arrow {
        font-size: 10px;
        margin-right: 8px;
        color: #888;
        width: 12px;
        display: inline-block;
    }
    .mini-btn {
        font-size: 11px;
        height: 26px;
        padding: 0 10px;
        border-radius: 4px;
        border: 1px solid #ccc;
        background: #fff;
    }
    .mini-btn.active {
        background: #0066b8;
        color: #fff;
    }

    /* Modern UI overrides */
    button {
        height: var(--control-height);
        min-width: 38px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
        color: var(--color-text-soft);
        box-shadow: var(--shadow-xs);
        cursor: pointer;
        transition:
            transform var(--transition-fast),
            border-color var(--transition-fast),
            background var(--transition-fast),
            box-shadow var(--transition-fast),
            color var(--transition-fast);
    }

    button:hover:not(:disabled) {
        border-color: var(--color-border-strong);
        background: var(--color-hover);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    button:active:not(:disabled) {
        background: var(--color-active);
        transform: translateY(1px) scale(0.98);
        box-shadow: var(--shadow-xs);
    }

    button:disabled {
        opacity: 0.52;
        box-shadow: none;
    }

    .sidebar {
        width: 292px;
        background: rgba(255, 255, 255, 0.82);
        border-right: 1px solid var(--color-border);
        box-shadow: 10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .sidebar-header-fixed {
        background: linear-gradient(
            180deg,
            rgba(255, 255, 255, 0.92),
            rgba(246, 249, 252, 0.92)
        );
        border-bottom: 1px solid var(--color-border);
    }

    .sidebar-header-row {
        padding: 12px;
        color: var(--color-text-soft);
        letter-spacing: 0.02em;
    }

    .icon-btn,
    .mini-btn {
        border: 1px solid var(--color-border);
        background: rgba(255, 255, 255, 0.86);
        color: var(--color-text-soft);
        border-radius: var(--radius-xs);
        box-shadow: none;
    }

    .icon-btn {
        width: 30px;
        height: 30px;
        min-width: 30px;
    }

    .mini-btn {
        height: 28px;
        border-radius: 999px;
        padding: 0 12px;
    }

    .mini-btn.active {
        background: var(--gradient-accent);
        border-color: transparent;
        color: #fff;
    }

    .toc-list {
        padding: 8px;
        background: linear-gradient(
            180deg,
            rgba(255, 255, 255, 0.42),
            rgba(246, 249, 252, 0.64)
        );
    }

    .toc-item {
        margin: 2px 0;
        padding: 10px 12px;
        border-bottom: 0;
        border-left: 3px solid transparent;
        border-radius: var(--radius-sm);
        color: var(--color-text-soft);
        transition:
            background var(--transition-fast),
            color var(--transition-fast),
            border-color var(--transition-fast),
            transform var(--transition-fast);
    }

    .toc-item:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .indent {
        background: transparent;
    }

    .toc-item.active {
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
        border-left-color: var(--color-accent);
        box-shadow: inset 0 0 0 1px rgba(22, 119, 184, 0.12);
        font-weight: 700;
    }

    .vol-title {
        background: rgba(255, 255, 255, 0.88);
        color: var(--color-text);
        box-shadow: 0 8px 18px rgba(23, 36, 52, 0.06);
        backdrop-filter: blur(14px);
    }

    .toc-count,
    .arrow {
        color: var(--color-muted);
    }

    /* Keep TXT volume folding and sticky headers precise. */
    .toc-list {
        position: relative;
        padding: 0;
        scroll-padding-top: 42px;
    }

    .toc-item {
        margin: 2px 8px;
    }

    .vol-title {
        position: sticky;
        top: 0;
        z-index: 30;
        margin: 0;
        border-radius: 0;
        border-bottom: 1px solid var(--color-border);
        background: rgba(255, 255, 255, 0.96);
        box-shadow: 0 6px 14px rgba(23, 36, 52, 0.08);
    }

    .vol-title:hover {
        background: rgba(246, 249, 252, 0.98);
    }

    .vol-title.active {
        border-left-color: var(--color-accent);
    }

    .indent {
        margin-left: 14px;
        padding-left: 24px;
    }
</style>
