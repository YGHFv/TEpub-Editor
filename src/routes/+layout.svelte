<script lang="ts">
    import { onMount, type Snippet } from "svelte";
    import { applyTheme, loadAppSettings } from "$lib/appSettings";
    import { applyClientProfile } from "$lib/clientProfile";
    import { platform } from "$lib/platform";
    import { listenForLaunchFiles } from "$lib/launchFiles";
    import { usesMultiWindow } from "$lib/secondaryTabs";
    import {
        detectWindowChrome,
        type WindowChromeMode,
    } from "$lib/windowChrome";
    import SecondaryTabs from "$lib/SecondaryTabs.svelte";
    import { base } from "$app/paths";

    let { children }: { children?: Snippet } = $props();

    // 首帧就把标题行算出来：等 onMount 再置会先少 40px，整页往下跳一跳
    let chrome = $state<WindowChromeMode>(detectWindowChrome());
    let multiWindow = $state(usesMultiWindow());

    // 标签栏常驻：回到工具箱时也保留，第一个「工具箱」标签固定在最前面，
    // 用户始终有个明确的"主页"入口可点。只有多窗口模式下每个窗口只有一个
    // 页面，标签没有意义才隐藏。
    const showTabs = $derived(!multiWindow);

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
        {@render children?.()}
    </div>
</div>
