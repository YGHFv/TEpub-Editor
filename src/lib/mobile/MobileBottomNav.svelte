<script lang="ts">
    import { onMount } from "svelte";
    import { page } from "$app/state";
    import { base } from "$app/paths";
    import { goto } from "$app/navigation";
    import { platform } from "$lib/platform";
    import { setWebClientViewOverride } from "$lib/clientProfile";
    import {
        appRoutePath,
        isHomeRoute,
        subscribeTabs,
        type SecondaryTab,
    } from "$lib/secondaryTabs";
    import { APP_VERSION } from "$lib/appVersion";
    import Icon from "$lib/icons/Icon.svelte";
    import MobileSheet from "./MobileSheet.svelte";
    import { t } from "$lib/i18n.svelte";

    /**
     * 手机底部导航。
     *
     * 槽位是固定的三段（工具箱 / 书库 / 更多），书库只在原生壳里出现：
     * 浏览器版没有文件系统权限，点进去必然是个死页，宁可不给入口。
     *
     * 是否渲染整个导航由调用方决定（桌面根本不挂它），这里不判断形态。
     */
    let moreOpen = $state(false);
    let tabs = $state<SecondaryTab[]>([]);

    // $app/state 的 page 是响应式的，goto 之后这里自动更新
    const currentPath = $derived(appRoutePath(page.url.pathname));
    const toolboxActive = $derived(isHomeRoute(page.url.pathname));
    const libraryActive = $derived(
        currentPath === "/library" || currentPath.startsWith("/library/"),
    );

    onMount(() => subscribeTabs((next) => (tabs = next)));

    function navigate(href: string) {
        moreOpen = false;
        void goto(href);
    }

    /**
     * 浏览器里切回电脑版：写覆盖值后整页重载。
     * 不用 goto 或 store 热切换 —— 布局分支在首帧就定了（app.html 的内联脚本
     * 读的是 localStorage），只有重载才能保证桌面 CSS 从第一帧就生效。
     */
    function switchToDesktop() {
        setWebClientViewOverride("desktop");
        location.reload();
    }
</script>

<nav class="m-nav" aria-label={t("主导航")}>
    <button
        class="m-nav-item"
        class:is-active={toolboxActive}
        type="button"
        aria-current={toolboxActive ? "page" : undefined}
        onclick={() => navigate(base + "/")}
    >
        <Icon name="grid" size={20} />
        <span>{t("工具箱")}</span>
    </button>

    {#if !platform.isWeb}
        <button
            class="m-nav-item"
            class:is-active={libraryActive}
            type="button"
            aria-current={libraryActive ? "page" : undefined}
            onclick={() => navigate(base + "/library")}
        >
            <Icon name="books" size={20} />
            <span>{t("书库")}</span>
        </button>
    {/if}

    <button
        class="m-nav-item"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={moreOpen}
        onclick={() => (moreOpen = true)}
    >
        <Icon name="settings" size={20} />
        <span>{t("更多")}</span>
    </button>
</nav>

<!-- 不用 bind:open：MobileSheet 的 API 是受控的（open + onclose），
     回写状态由这里做，避免子组件多一个可写 prop -->
<MobileSheet
    open={moreOpen}
    title={t("更多")}
    onclose={() => (moreOpen = false)}
>
    <!-- 已打开的二级页面：标签为空时整段不渲染，避免出现一个空标题 -->
    {#if tabs.length}
        {#each tabs as tab (tab.id)}
            <button
                class="m-list-row"
                type="button"
                onclick={() => navigate(tab.href)}
            >
                <Icon name="file" size={20} />
                <span>{t(tab.title)}</span>
            </button>
        {/each}
    {/if}

    {#if platform.isWeb}
        <button class="m-list-row" type="button" onclick={switchToDesktop}>
            <Icon name="monitor" size={20} />
            <span>{t("电脑版")}</span>
        </button>
    {:else}
        <button
            class="m-list-row"
            type="button"
            onclick={() => navigate(base + "/mobile")}
        >
            <Icon name="folder" size={20} />
            <span>{t("移动端工具")}</span>
        </button>
    {/if}

    <div class="m-list-row">
        <Icon name="settings" size={20} />
        <span>{t("关于")}</span>
        <span class="m-about-version">TEpub Editor {APP_VERSION}</span>
    </div>
</MobileSheet>

<style>
    /* 导航文字可能被窄屏挤成两行，这里只保证图标与文字不换行溢出 */
    .m-nav-item span {
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* 标签名可能很长（书名进标题的情况），一行省略而不是把行撑出横向滚动 */
    .m-list-row span {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    /* 「关于」行右侧的版本号：.m-list-row 的 gap 已经排开，这里只把它推到
       最右边并压暗，不参与点击 */
    .m-about-version {
        margin-left: auto;
        color: var(--color-muted);
        font-size: 11px;
        flex: 0 0 auto;
    }
</style>
