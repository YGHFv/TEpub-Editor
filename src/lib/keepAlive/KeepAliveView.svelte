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
        visibility: hidden !important;
        opacity: 0 !important;
        pointer-events: none;
        /* 只有隐藏页需要独立层叠上下文。激活页不能 isolation/z-index：那会把
           页面里的设置弹窗、抽屉统一关在 z-index:1 里，外层手机底栏（z-index:60）
           反而压在所有页内模态层上面，出现“弹窗底部被底栏挡一截、底栏还能点”。 */
        isolation: isolate;
        /* visibility 对少数自行声明 visibility:visible 的浮层不可靠，且固定定位
           的后代可能跳出普通裁剪。paint containment 会在这一层建立独立绘制
           边界，把隐藏页里的遮罩、动画和固定浮层一起裁掉，但元素本身仍保持
           inset:0 的真实尺寸，不会触发各页 ResizeObserver 把宽度算成 0。 */
        contain: layout paint style;
        z-index: 0;
    }

    .ka-view:not([data-ka-hidden="true"]) {
        z-index: auto;
        isolation: auto;
    }

    .ka-view[data-ka-hidden="true"] :global(*) {
        animation: none !important;
        transition: none !important;
    }

    /* visibility:hidden 挡不住 backdrop-filter：隐藏视图里的弹窗（书库首次
       启动的目录选择框就是）依然会把自己的模糊层合成到当前页面上 ——
       真机实测切到工具箱后整页发灰、左上角还残留一块模糊的按钮。
       隐藏时只关掉后代的后备滤镜，布局和滚动位置都不受影响。 */
    .ka-view[data-ka-hidden="true"] :global(*) {
        backdrop-filter: none !important;
        -webkit-backdrop-filter: none !important;
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
