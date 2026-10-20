<script lang="ts">
    /**
     * 单层保活容器：一个标签 = 一个常驻挂载的页面。
     *
     * 通过 context 把「我自己的 URL」「我是否可见」「怎么设置我的标题」
     * 交给页面，页面就不再依赖全局 `page.url` / `document.title`
     * （两者都会随别的标签变化）。
     */
    import { onMount, setContext, tick, type Component } from "svelte";
    import { titleForRoute } from "$lib/secondaryTabs";
    import { KEEP_ALIVE_CONTEXT_KEY } from "./viewContext";

    let {
        page: Page,
        url,
        active,
        onActivationChange,
        onTitleChange,
    }: {
        page: Component<any> | null;
        url: URL;
        active: boolean;
        onActivationChange: (listener: (active: boolean) => void) => () => void;
        onTitleChange: (title: string) => void;
    } = $props();

    // url / 回调对这一个视图是常量（视图 key 变了就会换组件实例），
    // 用 getter/闭包传出去，避免 Svelte 把"只在初始化时读一次"当成
    // 响应式遗漏来告警
    setContext(KEEP_ALIVE_CONTEXT_KEY, {
        get url() {
            return url;
        },
        isActive: () => active,
        onActivationChange: (listener: (active: boolean) => void) =>
            onActivationChange(listener),
        setTitle: (title: string) => {
            explicitTitle = true;
            onTitleChange(title);
        },
    });

    /**
     * 页面自己没登记标题时的兜底。
     *
     * 不能读 document.title：它可能已经被别的视图改写过（先开阅读器再开
     * 工具箱，工具箱读到的就是书名）。静默页面统一用路由表里的页面名，
     * 工具箱这类首页则用应用名。
     */
    let explicitTitle = false;
    onMount(() => {
        void tick().then(() => {
            if (explicitTitle) return;
            const routeTitle = titleForRoute(url.pathname);
            onTitleChange(routeTitle || "TEpub-Editor");
        });
    });
</script>

<div
    class="ka-view"
    data-ka-hidden={active ? undefined : "true"}
    data-ka-key={url.pathname + url.search}
    aria-hidden={active ? undefined : "true"}
    inert={!active}
>
    {#if Page}
        <Page />
    {:else}
        <div class="ka-view-loading">正在打开页面…</div>
    {/if}
</div>

<style>
    /* 铺满整个页面区：页面内部按 100% 高度排版时拿到的就是窗口的真实
       剩余高度，和没有保活时完全一致 */
    .ka-view {
        position: absolute;
        inset: 0;
        overflow: auto;
        overscroll-behavior: contain;
        display: block;
    }

    /* 隐藏只切 visibility：元素仍在布局树里，尺寸/列宽/滚动位置全部保留，
       切回来不需要重新测量。display:none 会让宽度瞬间变 0，各页的
       ResizeObserver 连环重排 —— 那才是「像重新加载」的来源 */
    .ka-view[data-ka-hidden="true"] {
        visibility: hidden;
        pointer-events: none;
    }

    .ka-view-loading {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: var(--color-muted);
        font-size: 13px;
    }
</style>
