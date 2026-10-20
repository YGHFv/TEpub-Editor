<script lang="ts">
    /**
     * 同窗口模式下所有打开过的标签的宿主。
     *
     * SvelteKit 没有 keep-alive：`goto()` 是 SPA 导航，但被路由到的页面
     * 组件会整个销毁再重建。于是每次切标签，TXT 编辑器丢光标和滚动位置、
     * EPUB 编辑器丢打开的书和标签页、阅读器要从头解析整本书 —— 用户看到
     * 的就是「切回去像重新加载，进度没了」。
     *
     * 这里不再让路由决定"渲染哪个页面"，而是把打开过的标签**全部挂着**，
     * 只把非当前标签的容器隐藏（visibility:hidden）。隐藏容器仍占满
     * 页面区的真实尺寸，因此阅读器分页、编辑器高度在切回来时都不用重算。
     *
     * 页面组件按需动态 import（routeComponents.ts），保留路由级代码分割。
     */
    import { untrack } from "svelte";
    import { page } from "$app/state";
    import {
        HOME_TAB,
        appRoutePath,
        isHomeRoute,
        subscribeTabs,
        tabIdFor,
    } from "$lib/secondaryTabs";
    import KeepAliveView from "./KeepAliveView.svelte";
    import {
        cachedRouteComponent,
        hasRouteComponent,
        loadRouteComponent,
    } from "./routeComponents";
    import type { Component, Snippet } from "svelte";

    let { fallback }: { fallback?: Snippet } = $props();

    interface ViewEntry {
        key: string;
        path: string;
        url: URL;
        component: Component<any> | null;
    }

    /**
     * 视图 key 与标签 id 同构（路径 + 查询串），关标签时才能对得上。
     * 首页统一成 HOME_TAB.id：`/` 和 `/toolbox` 是同一个工具箱，
     * 不能因为路径写法不同就挂两份。
     */
    function viewKeyFor(url: URL) {
        if (isHomeRoute(url.pathname)) return HOME_TAB.id;
        return tabIdFor(appRoutePath(url.pathname), url.search);
    }

    const activeKey = $derived(viewKeyFor(page.url));
    /** 当前路径有没有被保活宿主接管（未登记的路径交给 SvelteKit 路由） */
    const knownRoute = $derived(
        hasRouteComponent(appRoutePath(page.url.pathname)),
    );
    let views = $state<ViewEntry[]>([]);
    /** 标签栏里还开着的页面 id；空集合表示"标签栏还没通知过" */
    let openIds = $state<Set<string>>(new Set());

    function attachComponent(key: string, path: string) {
        void loadRouteComponent(path).then((component) => {
            if (!component) return;
            views = views.map((view) =>
                view.key === key ? { ...view, component } : view,
            );
        });
    }

    function ensureView(key: string, path: string, url: URL) {
        if (!hasRouteComponent(path)) return;
        if (views.some((view) => view.key === key)) return;
        const component = cachedRouteComponent(path);
        views = [...views, { key, path, url, component }];
        if (!component) attachComponent(key, path);
    }

    /**
     * 卸载"既不是当前视图、也不在标签栏里"的页面。
     *
     * 关标签时顺序是：closeTab() 先把 id 从标签栏摘掉，再 goto。
     * 所以必须在**标签变化**和**地址变化**两个时机都裁一次 —— 只看标签的话，
     * 关闭当前标签后页面会以"当前视图"的身份留到下一次切换。
     */
    function prune(current: string) {
        views = views.filter(
            (view) =>
                view.key === current ||
                view.key === HOME_TAB.id ||
                openIds.has(view.key),
        );
    }

    // 进入一个标签：没有就新建一层。用 untrack 读 views —— 这个 effect
    // 只该跟着地址变化跑，不该因为 views 自己变了再跑一次
    $effect(() => {
        const url = new URL(page.url);
        const key = viewKeyFor(url);
        const path = appRoutePath(url.pathname);
        untrack(() => {
            ensureView(key, path, url);
            prune(key);
        });
    });

    // 首帧就把当前页挂上，避免空一帧
    {
        const url = new URL(page.url);
        ensureView(viewKeyFor(url), appRoutePath(url.pathname), url);
    }

    // 标签栏变化（新开 / 关闭）时同步 openIds 并裁剪
    $effect(() => {
        return subscribeTabs((tabs) => {
            untrack(() => {
                openIds = new Set(tabs.map((tab) => tab.id));
                prune(activeKey);
            });
        });
    });

    // ---- 激活状态广播：隐藏的页面据此关掉自己的 document 级副作用 ----
    type ActivationListener = (active: boolean) => void;
    const activationListeners = new Map<string, Set<ActivationListener>>();

    function onActivationChangeFor(key: string) {
        return (listener: ActivationListener) => {
            let set = activationListeners.get(key);
            if (!set) {
                set = new Set();
                activationListeners.set(key, set);
            }
            set.add(listener);
            return () => {
                set!.delete(listener);
                if (set!.size === 0) activationListeners.delete(key);
            };
        };
    }

    let lastActiveKey = "";
    $effect(() => {
        const key = activeKey;
        if (key === lastActiveKey) return;
        lastActiveKey = key;
        untrack(() => {
            for (const [viewKey, listeners] of activationListeners) {
                const active = viewKey === key;
                for (const listener of listeners) listener(active);
            }
            applyTitle(key);
        });
    });

    // ---- 窗口标题：由当前可见视图决定 ----
    //
    // 页面各自 `<svelte:head><title>` 在保活下会互相覆盖（后挂载的赢），
    // 所以隐藏视图的标题不再由页面直接写，改成登记到宿主，切回时再落盘。
    const viewTitles = new Map<string, string>();

    function applyTitle(key: string) {
        if (typeof document === "undefined") return;
        const title = viewTitles.get(key);
        // 没登记过（比如工具箱）就保持文档原有标题
        if (title) document.title = title;
    }

    function onTitleChangeFor(key: string) {
        return (title: string) => {
            if (!title) return;
            viewTitles.set(key, title);
            if (key === activeKey) document.title = title;
        };
    }
</script>

{#if knownRoute}
    {#each views as view (view.key)}
        <KeepAliveView
            page={view.component}
            url={view.url}
            active={activeKey === view.key}
            onActivationChange={onActivationChangeFor(view.key)}
            onTitleChange={onTitleChangeFor(view.key)}
        />
    {/each}
{:else if fallback}
    <!-- 未登记的路径（例如手工输入的地址）仍交给 SvelteKit 自己的路由渲染，
         不要变成一片空白 -->
    {@render fallback()}
{/if}
