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
    import Icon from "$lib/icons/Icon.svelte";
    import { t } from "$lib/i18n.svelte";

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
            <span
                >{stats.volumes}{t("卷")} {stats.chapters}{t("章")}</span
            >
            <div class="header-btns">
                <button
                    class="icon-btn"
                    title={t("全部展开/折叠")}
                    onclick={onToggleAllVolumes}
                >
                    <Icon name="expand-vertical" size={15} />
                </button>
                <button
                    class="mini-btn {checkActive ? 'active' : ''}"
                    onpointerdown={onOpenProofCheck}
                    onclick={onOpenProofCheck}>{t("检查")}</button
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
                    <span class="arrow"
                        ><Icon
                            name={node.expanded
                                ? "chevron-down"
                                : "chevron-right"}
                            size={12}
                        /></span
                    >
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
        height: var(--control-height);
        min-width: 40px;
        border-radius: var(--radius-xs);
        border: 1px solid var(--color-border-strong);
        background: var(--color-surface);
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: var(--color-border);
        transform: translateY(0.5px) scale(0.985);
    }

    .sidebar {
        width: 280px;
        background: var(--color-surface-soft);
        border-right: 1px solid var(--color-border);
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
    }

    .sidebar-header-fixed {
        background: var(--color-border);
        border-bottom: 1px solid var(--color-border);
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
        border: 1px solid var(--color-border-strong);
        background: var(--color-surface);
        cursor: pointer;
        border-radius: var(--radius-xs);
    }

    .toc-list {
        flex: 1;
        overflow-y: auto;
    }
    .toc-item {
        padding: 12px;
        font-size: 14px;
        border-bottom: 1px solid var(--color-border);
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
        background: var(--color-surface-soft);
    }
    .toc-item.active {
        background: var(--color-accent-soft);
        color: var(--color-accent);
        border-left: 4px solid var(--color-accent);
        font-weight: bold;
    }
    /* 卷标吸顶 */
    .vol-title {
        background: var(--color-surface-soft);
        font-weight: bold;
        position: sticky;
        top: 0;
        z-index: 10;
        box-shadow: var(--shadow-xs);
    }
    .text-error {
        color: var(--color-danger);
        font-weight: bold;
    }
    .toc-count {
        color: var(--color-muted);
        font-size: 11px;
        margin-left: auto; /* Push to right */
    }
    .arrow {
        margin-right: 8px;
        color: var(--color-muted);
        width: 14px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
    .mini-btn {
        font-size: 11px;
        height: 26px;
        padding: 0 10px;
        border-radius: var(--radius-xs);
        border: 1px solid var(--color-border-strong);
        background: var(--color-surface);
    }
    .mini-btn.active {
        background: var(--color-accent);
        color: var(--color-accent-contrast);
    }

    /* Modern UI overrides */
    button {
        height: var(--control-height);
        min-width: 38px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: linear-gradient(
            180deg,
            var(--color-surface),
            var(--color-surface-soft)
        );
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
        background: var(--color-surface-raised);
        border-right: 1px solid var(--color-border);
        box-shadow: 10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .sidebar-header-fixed {
        background: linear-gradient(
            180deg,
            var(--color-surface),
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
        background: var(--color-surface-raised);
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
        color: var(--color-accent-contrast);
    }

    .toc-list {
        padding: 8px;
        background: linear-gradient(
            180deg,
            var(--color-surface-soft),
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
        background: var(--color-surface-raised);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
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
        background: var(--color-surface);
        box-shadow: var(--shadow-sm);
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
