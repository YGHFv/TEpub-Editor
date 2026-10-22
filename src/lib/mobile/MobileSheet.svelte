<script lang="ts">
    import { tick, type Snippet } from "svelte";
    import Icon from "$lib/icons/Icon.svelte";

    /**
     * 手机底部抽屉。
     *
     * 只负责「打开/关闭」这一层行为，不判断当前是不是手机 —— 是否渲染由调用方
     * 决定（桌面调用方根本不会挂它，桌面布局因此不受影响）。
     *
     * 样式全部来自 src/mobile.css 的 .m-sheet-*（带 data-tepub-layout="phone"
     * 前缀的全局规则），这里只补几处布局微调，避免两套定义互相打架。
     */
    let {
        open = false,
        title = "",
        onclose = () => {},
        aboveOverlays = false,
        children,
        actions,
    }: {
        open?: boolean;
        title?: string;
        onclose?: () => void;
        /** 从设置等全屏弹窗里再打开抽屉时，抽屉必须高于这些弹窗。 */
        aboveOverlays?: boolean;
        children?: Snippet;
        actions?: Snippet;
    } = $props();

    let sheetEl = $state<HTMLDivElement | null>(null);
    let closeBtn = $state<HTMLButtonElement | null>(null);

    /**
     * 打开期间锁住背后的滚动，并在关闭时把焦点还给触发按钮。
     *
     * 滚动要锁两处：
     * 1. body。桌面浏览器里整页是 body 在滚。
     * 2. 手机外壳的 .app-frame-body / .ka-view。手机上一屏一列，body 本身
     *    不滚，滚的是这两层容器 —— 只锁 body 的话，在遮罩上拖手指，背后的
     *    长列表照样跟着滑（本组件最早就是这么写的，等于没锁）。容器名写在
     *    mobile.css 里，这里只加/去一个类名。
     *
     * 记住原值而不是关掉时写回 ""：别的弹层可能已经设过 overflow，
     * 直接清空会把它们的锁一起解掉。
     */
    let returnFocusTo: HTMLElement | null = null;

    $effect(() => {
        if (!open) return;
        const previous = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        document.documentElement.classList.add("m-sheet-open");
        // 调用方大多是「{#if open}<MobileSheet/>{/if}」，关闭时组件被销毁
        // 而不是收到 open=false，所以焦点必须在清理函数里还，不能等下面
        // 那个 $effect.pre 观察 open 变化（它永远看不到）。
        const active = document.activeElement;
        if (active instanceof HTMLElement) returnFocusTo = active;
        // 等抽屉真正挂上 DOM 再聚焦；否则 focus() 落在 null 上，读屏用户
        // 打开抽屉后焦点还停在触发按钮上，Esc 之外什么都操作不了
        void tick().then(() => closeBtn?.focus());
        return () => {
            document.body.style.overflow = previous;
            document.documentElement.classList.remove("m-sheet-open");
            const target = returnFocusTo;
            returnFocusTo = null;
            if (target?.isConnected) target.focus();
        };
    });

    /**
     * 兜底：调用方如果把 open 从 true 改成 false 而不是销毁组件，焦点先主动
     * 收掉（归还由上面的清理函数负责）。必须用 $effect.pre：普通 $effect 跑在
     * DOM 更新之后，那时 {#if open} 已经把抽屉整个移除，activeElement 早掉回
     * body，再判断「焦点是否还在抽屉内」永远是 false。
     */
    let wasOpen = false;
    $effect.pre(() => {
        if (wasOpen && !open) {
            const active = document.activeElement;
            if (active instanceof HTMLElement && sheetEl?.contains(active)) {
                active.blur();
            }
        }
        wasOpen = open;
    });

    // Esc 是移动端外接键盘/桌面调试时的主要退出方式；挂在 window 上而不是
    // 抽屉上，是因为焦点可能在 body 上（例如刚点完遮罩外区域）。
    //
    // 顺便做焦点陷阱：声明了 aria-modal="true" 就必须真的把 Tab 关在里面，
    // 否则读屏/键盘用户能一路 Tab 到背后页面，那个"模态"只是嘴上说说。
    // 遮罩是 position:fixed 的兄弟节点，背后内容不在抽屉里，所以只遍历本抽屉。
    $effect(() => {
        if (!open) return;
        const handleKeydown = (event: KeyboardEvent) => {
            if (event.key === "Escape") {
                onclose();
                return;
            }
            if (event.key !== "Tab" || !sheetEl) return;
            const focusables = Array.from(
                sheetEl.querySelectorAll<HTMLElement>(
                    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
                ),
            ).filter((el) => el.offsetParent !== null);
            if (!focusables.length) return;
            const first = focusables[0];
            const last = focusables[focusables.length - 1];
            const active = document.activeElement;
            if (event.shiftKey && (active === first || active === sheetEl)) {
                event.preventDefault();
                last.focus();
            } else if (!event.shiftKey && active === last) {
                event.preventDefault();
                first.focus();
            }
        };
        window.addEventListener("keydown", handleKeydown);
        return () => window.removeEventListener("keydown", handleKeydown);
    });
</script>

{#if open}
    <!-- 遮罩是抽屉的兄弟节点：抽屉内部的点击不需要冒泡判断，也不会因为点在
         抽屉空白处被误判成「点了遮罩」而关闭。
         tabindex="-1" 把它排除在 Tab 序列外：键盘用户靠 Esc 或抽屉里的关闭
         按钮退出，不该在这里多停一站 -->
    <button
        class="m-sheet-backdrop"
        class:above-overlays={aboveOverlays}
        type="button"
        tabindex="-1"
        aria-label="关闭"
        onclick={onclose}
    ></button>

    <div
        class="m-sheet"
        class:above-overlays={aboveOverlays}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        bind:this={sheetEl}
    >
        <div class="m-sheet-handle" aria-hidden="true"></div>

        <div class="m-sheet-head">
            <div class="m-sheet-title">{title}</div>
            <button
                class="m-icon-btn"
                type="button"
                aria-label="关闭"
                bind:this={closeBtn}
                onclick={onclose}
            >
                <Icon name="close" size={20} />
            </button>
        </div>

        <div class="m-sheet-body">
            {@render children?.()}
        </div>

        {#if actions}
            <div class="m-sheet-foot">
                {@render actions()}
            </div>
        {/if}
    </div>
{/if}

<style>
    /* mobile.css 里的 .m-* 都带 :root[data-tepub-layout="phone"] 前缀。全局 CSS
       只按 class 匹配、不看 Svelte 的作用域属性，所以元素写上 class 就会命中，
       这里不重复整套样式，只补一处排版：标题占满左侧，关闭按钮自然靠右。 */
    .m-sheet-head :global(.m-icon-btn) {
        margin-left: auto;
    }
</style>
