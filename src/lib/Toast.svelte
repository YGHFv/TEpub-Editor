<script lang="ts">
    import { toasts, dismissToast } from "./toast";
    import { fly } from "svelte/transition";
    import { t } from "$lib/i18n.svelte";
</script>

{#if $toasts.length > 0}
    <div class="toast-container">
        {#each $toasts as toast (toast.id)}
            <div
                class="toast toast-{toast.type}"
                role={toast.type === "error" ? "alert" : "status"}
                transition:fly={{ y: -12, duration: 160 }}
            >
                <span class="toast-message">{toast.message}</span>
                <button
                    class="toast-close"
                aria-label={t("关闭提示")}
                    on:click={() => dismissToast(toast.id)}>×</button
                >
            </div>
        {/each}
    </div>
{/if}

<style>
    .toast-container {
        position: fixed;
        top: 16px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 10000;
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-width: min(480px, calc(100vw - 32px));
        pointer-events: none;
    }

    .toast {
        display: flex;
        align-items: flex-start;
        gap: 8px;
        padding: 10px 12px;
        border-radius: var(--radius-md, 8px);
        background: var(--color-surface-raised, var(--color-text));
        color: var(--color-text, var(--color-border));
        box-shadow: var(--shadow-pop, 0 4px 16px rgba(0, 0, 0, 0.35));
        font-family: var(--font-ui, sans-serif);
        font-size: 13px;
        line-height: 1.5;
        border-left: 3px solid var(--color-border, var(--color-text-soft));
        pointer-events: auto;
    }

    .toast-success {
        border-left-color: var(--color-success);
    }

    .toast-error {
        border-left-color: var(--color-danger);
    }

    .toast-message {
        flex: 1;
        white-space: pre-wrap;
        word-break: break-word;
    }

    .toast-close {
        flex-shrink: 0;
        border: none;
        background: transparent;
        color: var(--color-text-soft, var(--color-muted));
        font-size: 16px;
        line-height: 1;
        cursor: pointer;
        padding: 0 2px;
    }

    /* 关闭按钮就是一个 16px 的 ×，桌面用鼠标还行，手指点不中。
       手机形态下撑到触控尺寸（负 margin 抵消视觉上的留白，不改变外观）。 */
    :global(:root[data-tepub-layout="phone"]) .toast-close {
        min-width: var(--tap-target);
        min-height: var(--tap-target);
        margin: -10px -8px -10px 0;
        font-size: 20px;
    }

    /* 手机上提示条被压成一条窄柱：容器用 left:50% + translateX(-50%) 居中，
       而绝对定位元素只写了 left、没写 right 时，收缩到合适的可用宽度是
       "包含块宽度 − left"——390px 的屏幕里只剩 195px，一条「浏览器版暂不支持
       load_library（HTTP 404）」被折成五行，还和 × 抢宽度。
       手机上改用左右各留 8px 的定宽定位，整行文字有 374px 可用。

       同时从顶部挪到底部：手机顶上是页头 / 书库工具栏，提示条一出现就把
       「添加图书」「打开文件」「搜索框」整条盖住，而这条提示本身不重要。
       底部导航是正常流里的一格（--m-nav-h），提示条压在它上方即可；
       /mobile 子树没有导航，多出来的 60px 空白并不遮挡任何东西。

       桌面窗口不命中属性选择器，顶部居中与 480px 上限保持原样。 */
    :global(:root[data-tepub-layout="phone"]) .toast-container {
        top: auto;
        left: 8px;
        right: 8px;
        bottom: calc(var(--safe-bottom) + var(--m-nav-h) + 8px);
        transform: none;
        max-width: none;
    }

    /* 底部抽屉开着时改回顶部。
       抽屉最后一行正好排到底部导航上沿，而提示条就压在那一条上：
       390×844 实测抽屉里的「电脑版」行 732–784 全被 toast 盖住，错误提示
       存活 7 秒，这期间点它没有任何反应。MobileSheet 打开时会给 <html>
       挂 .m-sheet-open（见 src/lib/mobile/MobileSheet.svelte），据此换边。 */
    :global(:root[data-tepub-layout="phone"].m-sheet-open) .toast-container {
        /* 二级页正文的 --safe-top 是 0（根布局已画返回行），但 toast 是
           fixed 到窗口的，仍要让开真实状态栏。 */
        top: calc(var(--safe-top-native, var(--safe-top)) + 8px);
        bottom: auto;
    }

    .toast-close:hover {
        color: var(--color-text, var(--color-border));
    }
</style>
