<script lang="ts">
    import { onMount, type Snippet } from "svelte";
    import { applyTheme, loadAppSettings } from "$lib/appSettings";
    import { applyClientProfile } from "$lib/clientProfile";
    import { platform } from "$lib/platform";
    import { listenForLaunchFiles } from "$lib/launchFiles";
    import { usesMultiWindow } from "$lib/secondaryTabs";
    import {
        currentWindowLabel,
        detectWindowChrome,
        isUtilityWindowLabel,
        type WindowChromeMode,
    } from "$lib/windowChrome";
    import SecondaryTabs from "$lib/SecondaryTabs.svelte";
    import KeepAliveViews from "$lib/keepAlive/KeepAliveViews.svelte";
    import { base } from "$app/paths";

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

    onMount(() => {
        applyTheme(loadAppSettings().uiTheme);
        const cleanupClientProfile = applyClientProfile();
        const refreshTheme = () => applyTheme(loadAppSettings().uiTheme);
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
        window.addEventListener("tepub-settings-updated", refreshTabs);
        // 跨窗口：设置写进 localStorage 后另一个窗口收到 storage 事件
        window.addEventListener("storage", refreshTheme);
        window.addEventListener("storage", refreshTabs);
        return () => {
            disposed = true;
            stopLaunchListener?.();
            cleanupClientProfile();
            window.removeEventListener("tepub-settings-updated", refreshTheme);
            window.removeEventListener("tepub-settings-updated", refreshTabs);
            window.removeEventListener("storage", refreshTheme);
            window.removeEventListener("storage", refreshTabs);
        };
    });
</script>

<!-- 固定框架：标题行 + 页面各占一格。用 flex 而不是让页面自己分高度，
     页面内部的 100vh 才能在标题行存在时正确地变成"剩余高度"（见 app.css） -->
<div class="app-frame">
    {#if chrome !== "none" || showTabs}
        <SecondaryTabs {chrome} {showTabs} />
    {/if}
    <div class="app-frame-body">
        {#if keepAlive}
            <KeepAliveViews fallback={children} />
        {:else}
            {@render children?.()}
        {/if}
    </div>
</div>
