<script lang="ts">
    import { onMount, type Snippet } from "svelte";
    import { afterNavigate, goto } from "$app/navigation";
    import { applyTheme, loadAppSettings } from "$lib/appSettings";
    import { initializeI18n, t } from "$lib/i18n.svelte";
    import { applyClientProfile, phoneDevice } from "$lib/clientProfile";
    import { platform } from "$lib/platform";
    import { listenForLaunchFiles } from "$lib/launchFiles";
    import {
        appRoutePath,
        fileLabel,
        isHomeRoute,
        pushTab,
        tabIdFor,
        titleForRoute,
        trimTabs,
        usesMultiWindow,
    } from "$lib/secondaryTabs";
    import { canEvictKeepAlive } from "$lib/keepAlive/dirtyGuard";
    import {
        currentWindowLabel,
        detectWindowChrome,
        isUtilityWindowLabel,
        type WindowChromeMode,
    } from "$lib/windowChrome";
    import SecondaryTabs from "$lib/SecondaryTabs.svelte";
    import MobileBottomNav from "$lib/mobile/MobileBottomNav.svelte";
    import MobilePageHeader from "$lib/mobile/MobilePageHeader.svelte";
    import { installViewportBridge } from "$lib/mobile/viewport";
    import {
        phoneBackTarget,
        recordPhoneNavigation,
        routeHref,
    } from "$lib/mobile/phoneHistory";
    import KeepAliveViews from "$lib/keepAlive/KeepAliveViews.svelte";
    import { base } from "$app/paths";
    import { hasRouteComponent } from "$lib/keepAlive/routeComponents";
    import { page } from "$app/state";

    let { children }: { children?: Snippet } = $props();

    // 首帧就把标题行算出来：等 onMount 再置会先少 40px，整页往下跳一跳。
    // 工具型小窗（查找替换 / EPUB 元数据）不是主界面：它们带系统标题栏，
    // 再画一条自绘标题行 + 标签栏会把 250px 高的窗口挤得只剩一条缝，
    // 点标签还会把面板导航走，主窗口的搜索桥接直接断掉。所以按窗口 label 关掉。
    const utilityWindow = isUtilityWindowLabel(currentWindowLabel());
    let chrome = $state<WindowChromeMode>(
        utilityWindow ? "none" : detectWindowChrome(),
    );
    let multiWindow = $state(usesMultiWindow());

    // 标签栏常驻：回到工具箱时也保留，第一个「工具箱」标签固定在最前面，
    // 用户始终有个明确的"主页"入口可点。只有多窗口模式下每个窗口只有一个
    // 页面，标签没有意义才隐藏。
    const showTabs = $derived(!multiWindow && !utilityWindow);

    /**
     * 同窗口模式把页面交给 keep-alive 宿主：打开过的标签全部保持挂载，
     * 切标签只切可见性，不再销毁/重建（详见 lib/keepAlive/KeepAliveViews）。
     *
     * 多窗口模式每个窗口只有一个页面，没有"切回去"这回事，走普通路由渲染；
     * 工具型小窗（查找替换）必须走普通渲染 —— 它靠 Esc 关闭、靠事件桥接
     * 主窗口，不能被当成标签缓存起来。
     */
    const keepAlive = $derived(!multiWindow && !utilityWindow);

    // 外壳切换只看"是不是真机手机"（$phoneDevice），不看 $clientLayout：
    // 桌面浏览器窗口拖窄时 layout 也会变 phone（页面 CSS 沿用旧 web-mobile
    // 语义继续响应），但那是一台电脑，收掉它的顶部标签条换底部导航就是倒退。
    const phone = $derived($phoneDevice);

    /**
     * mobile 这一棵页面自带入口列表和返回链接，属于"全屏沉浸"的自成一体导航；
     * 外面再叠一条底部导航就是双份，整棵子树都不挂。
     * pathname 带着部署 base（web 版可能挂在子路径），要先剥掉再比。
     */
    function isMobileTreeRoute(pathname: string) {
        const prefix = (base || "").replace(/\/$/, "");
        let local = pathname || "/";
        // 只剥完整的一段前缀："/tepubx" 不能被当成 "/tepub" 下的路径
        if (prefix && (local === prefix || local.startsWith(`${prefix}/`))) {
            local = local.slice(prefix.length) || "/";
        }
        return local === "/mobile" || local.startsWith("/mobile/");
    }

    // page 来自 $app/state，本身已是 rune 响应式对象，直接读属性即可；
    // 写成 $page 会被当成 store 订阅，而它没有 subscribe，运行时会报错。
    const showPhoneNav = $derived(
        phone &&
            !utilityWindow &&
            !isMobileTreeRoute(page.url.pathname) &&
            (isHomeRoute(page.url.pathname) ||
                appRoutePath(page.url.pathname) === "/library"),
    );

    const phoneSecondaryRoute = $derived(
        phone &&
            !utilityWindow &&
            !isMobileTreeRoute(page.url.pathname) &&
            !showPhoneNav,
    );
    const phoneSecondaryTitle = $derived(
        t(titleForRoute(page.url.pathname) || "TEpub-Editor"),
    );
    const phoneSecondarySubtitle = $derived(fileLabel(page.url.search) || "");

    // 只记录应用内导航。Android WebView 冷启动时 history.length 往往已经
    // 大于 1，直接 history.back() 可能退到空白页或应用外，再回来时保活视图
    // 和当前路由会短暂叠在一起。这里改为记录 pathname + search + hash。
    let phoneHistory: string[] = [];
    let pendingPhoneBack = "";

    afterNavigate(({ to, type }) => {
        if (!to) return;
        const href = routeHref(to.url);
        const primaryRoute =
            isHomeRoute(to.url.pathname) ||
            appRoutePath(to.url.pathname) === "/library";
        const customBack = pendingPhoneBack !== "" && href === pendingPhoneBack;
        if (pendingPhoneBack && !customBack) pendingPhoneBack = "";
        phoneHistory = recordPhoneNavigation(
            phoneHistory,
            href,
            customBack ? "popstate" : type,
            primaryRoute,
        );
    });

    function goBackFromPhoneSecondary() {
        const target =
            phoneBackTarget(phoneHistory, routeHref(page.url)) ?? base + "/";
        pendingPhoneBack = target;
        void goto(target);
    }

    /**
     * 手机上最多保留几个二级页面。
     *
     * 桌面标签由用户手动关，条数天然有界；手机外壳不渲染标签条，访问过的
     * 页面只进不出 —— 而 KeepAliveViews 恰好是「标签栏里开着的都活着」，
     * 于是每逛一页就多挂一个活组件，一本解压后的书就是几十 MB。
     * 留 3 个够「工具箱 ↔ 编辑器 ↔ 文件树」来回切，再多收益很小。
     */
    const PHONE_MAX_TABS = 3;

    /**
     * 手机也要登记"访问过的页面"。
     *
     * 桌面靠 SecondaryTabs 里的 pushTab 登记，手机不渲染标签条 —— 所以这里
     * 必须自己登记，否则 KeepAliveViews 的 openIds 恒为空，prune() 只留当前页
     * + 首页：在 TXT 编辑器里改了一半再回工具箱，那一页被直接销毁，改动没了，
     * 而且「更多」里也看不到它。
     *
     * 光登记等于只进不出（一本解压后的书就是几十 MB），所以下面同时按
     * PHONE_MAX_TABS 裁剪 —— 手机端的内存上限由 trimTabs 这一个机制负责，
     * KeepAliveViews 里没有第二个上限。
     */
    /**
     * web 端几个「只做跳转」的入口，登记了就是一条死标签。
     *
     * /mobile 和 /mobile/make 在浏览器里一挂载就 window.location.replace 到
     * 别的地址（移动端工具页在浏览器里没有意义，制作页有独立的 toolbox 版）。
     * 但 layout 的 effect 先于页面 onMount 跑，标签已经写进 sessionStorage ——
     * 用户从「更多」里点它，会触发一次整文档重载，所有保活页（包括被
     * dirtyGuard 保护、带未保存改动的编辑器）一起没了，守卫根本拦不住。
     */
    function isWebRedirectOnly(path: string) {
        if (!platform.isWeb) return false;
        return path === "/mobile" || path === "/mobile/make";
    }

    $effect(() => {
        if (!phone) return;
        const path = appRoutePath(page.url.pathname);
        if (!hasRouteComponent(path)) return;
        if (isWebRedirectOnly(path)) return;
        // promote：重新访问的页面挪到末尾。手机没有标签栏，顺序纯粹是
        // trimTabs 的"最近使用"依据，必须按访问顺序而不是首次登记顺序。
        pushTab(path, page.url.search, undefined, undefined, {
            promote: true,
        });
        // 手机没有桌面标签栏，用户回到工具箱后无法主动关掉旧工具页。
        // 如果继续保活，下一次从首页打开同一个工具会直接恢复上次的文件
        // 和滚动位置，看起来像“缓存很久没清”。回首页时清掉所有没有
        // 未保存改动的二级页，让下一次进入工具是全新会话；dirtyGuard
        // 仍然优先，带未保存内容的页面不会被静默销毁。
        if (isHomeRoute(page.url.pathname)) {
            trimTabs(0, { canEvict: canEvictKeepAlive });
            return;
        }
        // 当前页当然不能裁；其余页面里带未暂存修改的，由页面自己登记
        // 的 guard 否决（见 lib/keepAlive/dirtyGuard）。没有这层保护，
        // 用户从编辑器逛两页回来，改动已经无声消失。
        trimTabs(PHONE_MAX_TABS, {
            keep: [tabIdFor(path, page.url.search)],
            canEvict: canEvictKeepAlive,
        });
    });

    // 软键盘高度：只在手机形态下桥接（桌面没有软键盘，浏览器缩放也不该算）。
    // 写在 layout 而不是各页面，是因为底部抽屉、动作条分散在多个页面，
    // 每页各自监听 visualViewport 迟早会漂移。
    $effect(() => {
        if (!phone) return;
        const cleanupViewport = installViewportBridge();
        return cleanupViewport;
    });

    // 手机二级页的统一样式开关。桌面端永远没有这个属性，所有相关 CSS
    // 都只命中手机；离开二级页时立即清掉，避免影响首页/书库。
    $effect(() => {
        if (typeof document === "undefined") return;
        const root = document.documentElement;
        if (phoneSecondaryRoute) root.dataset.tepubPhoneSecondary = "true";
        else delete root.dataset.tepubPhoneSecondary;
        return () => {
            delete root.dataset.tepubPhoneSecondary;
        };
    });

    onMount(() => {
        const initialSettings = loadAppSettings();
        applyTheme(initialSettings.uiTheme);
        initializeI18n(initialSettings.uiLanguage);
        const cleanupClientProfile = applyClientProfile();
        const refreshTheme = () => applyTheme(loadAppSettings().uiTheme);
        const refreshLanguage = () =>
            initializeI18n(loadAppSettings().uiLanguage);
        const refreshTabs = () => (multiWindow = usesMultiWindow());
        let disposed = false;
        let stopLaunchListener: (() => void) | undefined;
        void listenForLaunchFiles(platform, base)
            .then((cleanup) => {
                if (disposed) cleanup();
                else stopLaunchListener = cleanup;
            })
            .catch((error) => console.error("启动文件监听失败:", error));
        window.addEventListener("tepub-settings-updated", refreshTheme);
        window.addEventListener("tepub-settings-updated", refreshLanguage);
        window.addEventListener("tepub-settings-updated", refreshTabs);
        // 跨窗口：设置写进 localStorage 后另一个窗口收到 storage 事件
        window.addEventListener("storage", refreshTheme);
        window.addEventListener("storage", refreshLanguage);
        window.addEventListener("storage", refreshTabs);
        return () => {
            disposed = true;
            stopLaunchListener?.();
            cleanupClientProfile();
            window.removeEventListener("tepub-settings-updated", refreshTheme);
            window.removeEventListener(
                "tepub-settings-updated",
                refreshLanguage,
            );
            window.removeEventListener("tepub-settings-updated", refreshTabs);
            window.removeEventListener("storage", refreshTheme);
            window.removeEventListener("storage", refreshLanguage);
            window.removeEventListener("storage", refreshTabs);
        };
    });
</script>

{#snippet frameBody()}
    <!-- 手机/桌面两条分支共用这份 body，以后改保活逻辑只需改一处；
         两边渲染出的 DOM 完全一致，桌面观感不受影响。 -->
    <div class="app-frame-body">
        {#if keepAlive}
            <KeepAliveViews fallback={children} />
        {:else}
            {@render children?.()}
        {/if}
    </div>
{/snippet}

<!-- 固定框架：标题行 + 页面各占一格。用 flex 而不是让页面自己分高度，
     页面内部的 100vh 才能在标题行存在时正确地变成"剩余高度"（见 app.css） -->
<div class="app-frame">
    {#if phone}
        <!-- 手机：没有标签条，页面占满，底部导航作为正常流里的最后一格。
             标签集由上面那个 $effect 自己登记并按 PHONE_MAX_TABS 裁剪
             （桌面靠 SecondaryTabs 内部调 pushTab，两条路径互不影响）。 -->
        {#if phoneSecondaryRoute}
            <MobilePageHeader
                title={phoneSecondaryTitle}
                subtitle={phoneSecondarySubtitle}
                onback={goBackFromPhoneSecondary}
            />
        {/if}
        {@render frameBody()}
        {#if showPhoneNav}
            <MobileBottomNav />
        {/if}
    {:else}
        {#if chrome !== "none" || showTabs}
            <SecondaryTabs {chrome} {showTabs} />
        {/if}
        {@render frameBody()}
    {/if}
</div>
