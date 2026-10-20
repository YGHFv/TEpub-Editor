<script lang="ts">
    import { goto } from "$app/navigation";
    import { base } from "$app/paths";
    import { page } from "$app/state";
    import { onMount, type Snippet } from "svelte";
    import { platform } from "$lib/platform";
    import {
        HOME_TAB,
        appRoutePath,
        closeTab,
        pushTab,
        subscribeTabs,
        tabIdFor,
        tabTooltipFor,
        type SecondaryTab,
    } from "$lib/secondaryTabs";
    import type { WindowChromeMode } from "$lib/windowChrome";

    /**
     * 窗口标题行。
     *
     * 桌面端窗口去掉了系统标题栏（decorations: false），这一行同时承担三件事：
     * 应用名、二级页面标签、窗口按钮。标签紧跟在软件名后面，右侧是可拖动的
     * 空白区 —— 用户要的就是「标签放在标题那一行」。
     *
     * macOS 走 titleBarStyle: "Overlay"：红黄绿按钮由系统画在左上角，
     * 所以左边留出 78px 安全区，右侧不再画一遍窗口按钮。
     *
     * 浏览器（chrome === "none"）里没有窗口边框，标题行整体不渲染，
     * 标签栏回到页面顶部（见下方 .title-bar.inline）。
     */
    let {
        chrome = "none",
        showTabs = true,
    }: {
        chrome?: WindowChromeMode;
        showTabs?: boolean;
    } = $props();

    // 这里用的是 runes 模式（组件里已有 $state/$derived），普通 `let` 不会
    // 触发重渲染 —— 订阅回调里换掉数组，界面却一直停在空列表
    let tabs = $state<SecondaryTab[]>([]);
    let mounted = $state(false);
    let maximized = $state(false);

    // $app/state 的 page 是响应式的：goto 之后路径自动变，不用监听事件
    const currentPath = $derived(appRoutePath(page.url.pathname));
    const currentSearch = $derived(page.url.search);
    // 当前页在标签里的 id：带查询串，因此同窗口能同时开两本书
    const currentId = $derived(tabIdFor(currentPath, currentSearch));
    const visibleTabs = $derived([HOME_TAB, ...(showTabs ? tabs : [])]);
    const activeId = $derived(
        currentPath === "/" || currentPath === "/toolbox"
            ? HOME_TAB.id
            : currentId,
    );

    $effect(() => {
        if (!mounted || !showTabs) return;
        pushTab(currentPath, currentSearch);
    });

    onMount(() => {
        mounted = true;
        if (showTabs) pushTab(appRoutePath(location.pathname), location.search);
        const unsubscribe = subscribeTabs((next) => (tabs = next));
        let stopResize: (() => void) | undefined;
        void platform
            .isCurrentWindowMaximized()
            .then((value) => (maximized = value))
            .catch(() => {});
        void platform
            .onCurrentWindowResized(async () => {
                try {
                    maximized = await platform.isCurrentWindowMaximized();
                } catch {
                    // 窗口销毁瞬间查询会失败：保持原状即可
                }
            })
            .then((stop) => {
                stopResize = stop;
            })
            .catch(() => {});
        return () => {
            unsubscribe();
            stopResize?.();
        };
    });

    function activate(tab: SecondaryTab) {
        if (tab.id === currentId) return;
        void goto(tab.href);
    }

    /** 标签上只写页面名，完整信息（含书名）放悬停提示 */
    function tooltipFor(tab: SecondaryTab) {
        const [, ...rest] = tab.id.split("?");
        return tabTooltipFor(tab.id.split("?")[0], rest.join("?"), tab.title);
    }

    function close(event: MouseEvent, tab: SecondaryTab) {
        event.stopPropagation();
        const index = tabs.findIndex((item) => item.id === tab.id);
        closeTab(tab.id);
        if (currentId !== tab.id) return;
        // 关掉的正是当前页：退到左边（没有就右边）那个标签，
        // 都没有才回工具箱 —— 一律回首页会把用户正在看的另一页也关掉似的
        const neighbour = tabs[index - 1] || tabs[index] || null;
        void goto(neighbour ? neighbour.href : `${base}/`);
    }

    function toggleMaximize() {
        void platform.toggleMaximizeCurrentWindow();
    }
</script>

{#if chrome === "none"}
    {#if showTabs}
        <nav class="title-bar inline" aria-label="已打开的页面">
            <div class="tab-strip">{@render tabStrip()}</div>
        </nav>
    {/if}
{:else}
    <header
        class="title-bar"
        class:mac-overlay={chrome === "mac-overlay"}
        role="presentation"
    >
        <!-- data-tauri-drag-region 由 Tauri 注入的脚本处理：
             按住拖动窗口，双击最大化（与系统标题栏一致）。
             只认事件目标元素自己，所以每个可拖区域都要单独标 -->
        <div class="app-name" data-tauri-drag-region>
            <img
                class="app-mark"
                src="{base}/favicon.png"
                alt=""
                aria-hidden="true"
                data-tauri-drag-region
            />
            <span data-tauri-drag-region>TEpub Editor</span>
        </div>
        {#if showTabs}
            <div class="tab-strip">{@render tabStrip()}</div>
        {/if}
        <div class="title-drag-area" data-tauri-drag-region></div>
        {#if chrome === "custom"}
            <div class="window-controls">
                <button
                    class="window-btn"
                    type="button"
                    aria-label="最小化"
                    title="最小化"
                    onclick={() => platform.minimizeCurrentWindow()}
                >
                    <svg viewBox="0 0 10 10" aria-hidden="true">
                        <path d="M1 5h8" />
                    </svg>
                </button>
                <button
                    class="window-btn"
                    type="button"
                    aria-label={maximized ? "还原" : "最大化"}
                    title={maximized ? "还原" : "最大化"}
                    onclick={toggleMaximize}
                >
                    {#if maximized}
                        <svg viewBox="0 0 10 10" aria-hidden="true">
                            <path d="M3 3V1h6v6H7" />
                            <rect x="1" y="3" width="6" height="6" />
                        </svg>
                    {:else}
                        <svg viewBox="0 0 10 10" aria-hidden="true">
                            <rect x="1" y="1" width="8" height="8" />
                        </svg>
                    {/if}
                </button>
                <button
                    class="window-btn close"
                    type="button"
                    aria-label="关闭"
                    title="关闭"
                    onclick={() => platform.closeCurrentWindow()}
                >
                    <svg viewBox="0 0 10 10" aria-hidden="true">
                        <path d="M1 1l8 8M9 1l-8 8" />
                    </svg>
                </button>
            </div>
        {/if}
    </header>
{/if}

{#snippet tabStrip()}
    {#each visibleTabs as tab (tab.id)}
        <div class="title-tab" class:active={activeId === tab.id}>
            <button
                class="title-tab-main"
                type="button"
                title={tooltipFor(tab)}
                aria-current={activeId === tab.id ? "page" : undefined}
                onclick={() => activate(tab)}
            >
                {tab.title}
            </button>
            {#if tab.id !== HOME_TAB.id}
                <button
                    class="title-tab-close"
                    type="button"
                    title="关闭标签"
                    aria-label={`关闭 ${tab.title}`}
                    onclick={(event) => close(event, tab)}>×</button
                >
            {/if}
        </div>
    {/each}
{/snippet}

<style>
    /* 标题行：软件名 | 标签 | 可拖拽空白 | 窗口按钮。
       高度固定 40px（app.css 的 .app-frame 用 flex 排布，页面只拿剩余高度） */
    .title-bar {
        display: flex;
        align-items: center;
        gap: 10px;
        height: 40px;
        padding: 0 0 0 12px;
        box-sizing: border-box;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface);
        /* 页面多为 fixed 满屏布局，标题行要压在上面 */
        position: relative;
        z-index: 70;
        flex: 0 0 auto;
        /* 拖动窗口时不要选中文字 */
        user-select: none;
    }

    /* macOS：红黄绿按钮浮在左上角，内容和窗口按钮都要让位 */
    .title-bar.mac-overlay {
        padding-left: 82px;
        padding-right: 12px;
    }

    .app-name {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        flex: 0 0 auto;
        color: var(--color-text);
        font-size: 13px;
        font-weight: 700;
        letter-spacing: 0.2px;
        white-space: nowrap;
    }

    .app-mark {
        width: 16px;
        height: 16px;
        border-radius: 4px;
        object-fit: contain;
    }

    .tab-strip {
        display: flex;
        align-items: center;
        gap: 4px;
        min-width: 0;
        flex: 0 1 auto;
        /* 标签再多也给可拖拽区留至少 1/3 宽度：否则开满标签后
           标题行除了标签就是按钮，窗口反而没地方抓 */
        max-width: 62%;
        /* 标签多了横向滚，不要撑破标题行 */
        overflow-x: auto;
        overflow-y: hidden;
        scrollbar-width: none;
        padding: 4px 2px;
    }

    .tab-strip::-webkit-scrollbar {
        height: 0;
    }

    /* 空白区吸收剩余宽度，鼠标拖着就是拖窗口。
       给一个下限：标签开满时它是唯一还能抓窗口的地方 */
    .title-drag-area {
        flex: 1 1 auto;
        align-self: stretch;
        min-width: 140px;
    }

    .title-tab {
        display: inline-flex;
        align-items: stretch;
        flex: 0 0 auto;
        max-width: 220px;
        height: 28px;
        border: 1px solid transparent;
        border-radius: var(--radius-sm);
        background: transparent;
        overflow: hidden;
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast);
    }

    .title-tab:hover {
        background: var(--color-hover);
    }

    .title-tab.active {
        border-color: var(--color-accent);
        background: var(--color-accent-soft);
    }

    .title-tab-main {
        min-width: 0;
        padding: 0 10px;
        border: 0;
        background: transparent;
        color: var(--color-text-soft);
        font: inherit;
        font-size: 13px;
        font-weight: 600;
        line-height: 1;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        cursor: pointer;
    }

    .title-tab.active .title-tab-main {
        color: var(--color-accent-deep);
    }

    .title-tab-close {
        flex: 0 0 auto;
        width: 22px;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--color-muted);
        font-size: 15px;
        line-height: 1;
        cursor: pointer;
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }

    .title-tab-close:hover {
        color: var(--color-danger);
        background: var(--color-danger-soft);
    }

    .window-controls {
        display: flex;
        align-items: stretch;
        align-self: stretch;
        flex: 0 0 auto;
    }

    .window-btn {
        width: 46px;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--color-text-soft);
        display: inline-flex;
        align-items: center;
        justify-content: center;
        cursor: default;
        transition: background var(--transition-fast);
    }

    .window-btn svg {
        width: 10px;
        height: 10px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1;
    }

    .window-btn:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .window-btn.close:hover {
        background: var(--color-danger);
        color: #fff;
    }

    /* 浏览器里没有窗口边框：退化成页面顶部的标签条 */
    .title-bar.inline {
        height: auto;
        min-height: 38px;
        padding: 5px 10px;
        gap: 4px;
    }

    .title-bar.inline .tab-strip {
        flex: 1 1 auto;
        padding: 0;
    }
</style>
