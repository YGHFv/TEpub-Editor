<script lang="ts">
    import type { EpubFileNode } from "./FileTreePanel.svelte";
    import { getFileIcon } from "./fileIcons";

    let {
        openTabs,
        activeTabIndex,
        modifiedFiles,
        onSelectTab,
        onCloseTab,
    }: {
        openTabs: EpubFileNode[];
        activeTabIndex: number;
        modifiedFiles: Set<string>;
        onSelectTab: (index: number) => void;
        onCloseTab: (event: Event, index: number) => void;
    } = $props();

    let tabsBarDiv: HTMLElement | null = $state(null);

    /** Scroll the tab at `index` into view (smooth, centered horizontally). */
    export function scrollTabIntoView(index: number) {
        if (tabsBarDiv && tabsBarDiv.children[index]) {
            const tabElement = tabsBarDiv.children[index] as HTMLElement;
            tabElement.scrollIntoView({
                behavior: "smooth",
                block: "nearest",
                inline: "center",
            });
        }
    }

    /** Jump the scroll container to the far right (newly appended tab). */
    export function scrollToEnd() {
        if (tabsBarDiv) {
            tabsBarDiv.scrollLeft = tabsBarDiv.scrollWidth;
        }
    }
</script>

{#if openTabs.length > 0}
    <div class="tabs-bar" bind:this={tabsBarDiv}>
        {#each openTabs as tab, index}
            <div
                class="editor-tab"
                class:active={index === activeTabIndex}
                onclick={() => onSelectTab(index)}
                onkeydown={(e) => e.key === "Enter" && onSelectTab(index)}
                role="button"
                tabindex="0"
            >
                <span class="tab-icon">{getFileIcon(tab.file_type)}</span>
                <span class="tab-name" title={tab.name}
                    >{tab.name}{#if modifiedFiles.has(tab.path)}
                        <span class="modified-indicator">*</span>
                    {/if}</span
                >
                <button
                    class="tab-close"
                    onclick={(e) => onCloseTab(e, index)}
                    aria-label={"关闭标签页"}
                >
                    &times;
                </button>
            </div>
        {/each}
    </div>
{/if}

<style>
    .tabs-bar {
        display: flex;
        background: #f3f3f3;
        border-bottom: 1px solid #ddd;
        overflow-x: auto;
        overflow-y: hidden;
        gap: 0;
        flex-shrink: 0;
        max-height: 40px;
        width: 100%;
        box-sizing: border-box;
    }

    .editor-tab {
        display: flex;
        align-items: center;
        padding: 8px 12px;
        min-width: 120px;
        max-width: 200px;
        border-right: 1px solid #ddd;
        cursor: pointer;
        background: #e8e8e8;
        transition: background 0.2s;
        user-select: none;
        flex-shrink: 0;
    }

    .editor-tab:hover {
        background: #d8d8d8;
    }

    .editor-tab.active {
        background: #fff;
        border-bottom: 2px solid #2196f3;
        position: relative;
    }

    .tab-icon {
        font-size: 14px;
        margin-right: 6px;
        flex-shrink: 0;
    }

    .tab-name {
        flex: 1;
        font-size: 13px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #333;
    }

    .tab-close {
        margin-left: 6px;
        width: 20px;
        height: 20px;
        border: none;
        background: transparent;
        color: #999;
        font-size: 16px;
        font-family:
            "Segoe UI Symbol", "Segoe UI", "Microsoft YaHei UI", sans-serif;
        font-weight: 400;
        line-height: 1;
        cursor: pointer;
        border-radius: 3px;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        padding: 0;
    }

    .tab-close:hover {
        background: #ccc;
        color: #333;
    }

    /* Mod Indicator */
    .modified-indicator {
        color: #ff9800;
        margin-left: 4px;
        font-weight: bold;
    }

    /* Modern UI overrides */
    .tabs-bar {
        background: rgba(255, 255, 255, 0.78);
        border-bottom: 1px solid var(--color-border);
        backdrop-filter: blur(18px) saturate(1.12);
    }

    .tabs-bar {
        min-height: 46px;
        max-height: 46px;
        gap: 4px;
        padding: 6px 8px 0;
    }

    .editor-tab {
        min-width: 132px;
        border: 1px solid transparent;
        border-bottom: 0;
        border-radius: 13px 13px 0 0;
        background: rgba(255, 255, 255, 0.48);
        color: var(--color-text-soft);
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast),
            color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .editor-tab:hover {
        background: var(--color-hover);
        border-color: var(--color-border);
    }

    .editor-tab.active {
        background: var(--color-surface);
        border-color: var(--color-border);
        border-bottom: 2px solid var(--color-accent);
        box-shadow: 0 -8px 18px rgba(23, 36, 52, 0.05);
    }

    .tab-name {
        color: inherit;
        font-weight: 600;
    }

    .tab-close {
        color: var(--color-muted);
        border-radius: 999px;
        transition:
            background var(--transition-fast),
            color var(--transition-fast),
            transform var(--transition-fast);
    }

    .tab-close:hover {
        background: var(--color-danger-soft);
        color: var(--color-danger);
        transform: scale(1.02);
    }

    .modified-indicator {
        color: var(--color-warning);
    }
</style>
