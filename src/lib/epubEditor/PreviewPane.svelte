<script module lang="ts">
    export interface ValidationError {
        type: "tag" | "img";
        message: string;
        line: number;
    }
</script>

<script lang="ts">
    import {
        describePreviewClick,
        revealTextInPreview,
        type PreviewClickPayload,
        type RevealMode,
    } from "./previewSync";
    import { measureEpubContentHeight } from "./contentHeight";
    import { applyPaperTheme } from "$lib/epubPaperTheme";

    let {
        content,
        errors,
        hasSelectedFile,
        onPreviewClick,
        showScrollbar = false,
        darkPaper = false,
    }: {
        content: string;
        errors: ValidationError[];
        hasSelectedFile: boolean;
        /** 在预览里点了某段文字：交给页面去定位编辑器 */
        onPreviewClick?: (payload: PreviewClickPayload) => void;
        /**
         * 是否显示预览滚动条，由「设置 → 通用」控制。
         * 默认隐藏：滚动条会占掉右侧 8px 的槽位，书页就不在预览列正中
         */
        showScrollbar?: boolean;
        /**
         * 书页是否按深色主题渲染。由页面按 uiTheme 传入，
         * 组件自己不读设置 —— 这样切主题时父级重算一次就够了。
         */
        darkPaper?: boolean;
    } = $props();

    let previewIframe: HTMLIFrameElement | null = $state(null);
    let previewFrameHeight = $state(0);

    /** 暴露预览 iframe 的 window，供需要直接操作预览文档的调用方使用 */
    export function getContentWindow(): Window | null {
        return previewIframe?.contentWindow ?? null;
    }

    /**
     * 编辑器里点了某一行 / 选中了一段文字时，把预览滚到对应位置。
     * 不能再往 iframe 里 postMessage：iframe 已被撑到正文高度，
     * 它自己没有可滚动区域，要滚的是包住它的手机框（.mobile-frame）。
     */
    export function revealText(text: string, mode: RevealMode = "center") {
        return revealTextInPreview(
            previewIframe?.parentElement ?? null,
            previewIframe,
            text,
            { mode },
        );
    }

    /**
     * 让 iframe 长到正文高度，滚动交给外面的手机框（.mobile-frame）。
     *
     * 手机框本身是固定尺寸的（宽 320、最高 700），不再跟着正文一起变长：
     * 之前 iframe 一路撑高、整列一起滚，长章节时"手机"会变成一条没有
     * 底边的长纸带，早就不是手机的样子了。现在框保持手机大小，正文在框
     * 里滚，框始终保有圆角和上下两条边。
     */
    function measureFrameHeight() {
        const frame = previewIframe;
        const innerBody = frame?.contentDocument?.body;
        if (!innerBody) return;
        const contentHeight = measureEpubContentHeight(innerBody);
        if (contentHeight <= 0) return;
        // 短章节也铺满手机屏，避免书页下面露出底色
        const shell = frame.parentElement;
        // 框不可见时量不到可用高度，保留上一次结果，
        // 避免切回预览页签时高度先塌再长、滚动条闪一下
        if (shell && shell.clientHeight <= 0) return;
        // clientHeight 本身就是"去掉边框、含内边距"的高度，不用再扣边框
        const availableHeight = shell ? Math.floor(shell.clientHeight) : 0;
        const next = Math.max(contentHeight, availableHeight);
        // 值没变就别写：$state 赋值虽然不会重排，但省掉一次无谓的调度
        if (next !== previewFrameHeight) previewFrameHeight = next;
    }

    // 图片解码、字体回流都会改变高度，分几个时间点补测
    function scheduleMeasure() {
        measureFrameHeight();
        requestAnimationFrame(measureFrameHeight);
        setTimeout(measureFrameHeight, 80);
        setTimeout(measureFrameHeight, 320);
        setTimeout(measureFrameHeight, 900);
    }

    /**
     * 正文内部的尺寸一变就重量高度。
     * 只靠几个固定 setTimeout 会漏掉"晚到的图片把内容撑高"：
     * iframe 停在旧高度上，内部就长出滚动条，而且再也不会消失。
     */
    let contentObserver: ResizeObserver | null = null;
    function watchInnerContent() {
        contentObserver?.disconnect();
        const innerDoc = previewIframe?.contentDocument;
        const innerBody = innerDoc?.body;
        if (!innerBody || typeof ResizeObserver === "undefined") return;
        contentObserver = new ResizeObserver(() => measureFrameHeight());
        contentObserver.observe(innerBody);
    }

    function trackContainerWidth(node: HTMLElement) {
        let lastWidth = node.clientWidth;
        let lastHeight = node.clientHeight;
        const observer = new ResizeObserver(() => {
            const width = node.clientWidth;
            const height = node.clientHeight;
            if (width <= 0) return;
            // 宽高任一变化都要重量：宽度决定正文折行（内容高度），
            // 高度决定手机框的可视高度——窗口拉高拉矮时框是跟着长的，
            // 不重量的话 iframe 会停在旧高度，正文上下多出空白/被裁
            if (
                Math.abs(width - lastWidth) < 2 &&
                Math.abs(height - lastHeight) < 2
            ) {
                return;
            }
            lastWidth = width;
            lastHeight = height;
            scheduleMeasure();
        });
        observer.observe(node);
        return {
            destroy: () => {
                observer.disconnect();
                contentObserver?.disconnect();
                contentObserver = null;
                unbindPreviewClick?.();
                unbindPreviewClick = null;
            },
        };
    }

    /**
     * 预览文档里的滚动条一律隐藏，滚动只发生在外层容器。
     *
     * 这不只是观感问题：iframe 处于过渡高度时会出现内部滚动条，
     * 滚动条把正文宽度挤窄约 15px，文字重新折行后内容反而变高，
     * 量出来的高度永远追不上，内部滚动条就"闪出来不消失"。
     */
    const PREVIEW_CHROME_STYLE = `
        html, body { scrollbar-width: none !important; -ms-overflow-style: none !important; }
        ::-webkit-scrollbar { width: 0 !important; height: 0 !important; display: none !important; }
    `;

    function installPreviewChrome(innerDoc: Document) {
        if (innerDoc.getElementById("tepub-preview-chrome")) return;
        const style = innerDoc.createElement("style");
        style.id = "tepub-preview-chrome";
        style.textContent = PREVIEW_CHROME_STYLE;
        (innerDoc.head ?? innerDoc.documentElement).appendChild(style);
    }

    function bindFootnotePopups(iframeDoc: Document) {
        if (iframeDoc.documentElement.dataset.tepubFootnotesBound === "1") {
            return;
        }
        iframeDoc.documentElement.dataset.tepubFootnotesBound = "1";
        iframeDoc.addEventListener(
            "click",
            (event) => {
                const clicked = event.target as Element | null;
                const trigger = clicked?.closest<HTMLAnchorElement>("a");
                if (!trigger) return;
                if (trigger.classList.contains("duokan-footnote-back")) {
                    event.preventDefault();
                    event.stopPropagation();
                    trigger.closest(".footnote-popup-backdrop")?.remove();
                    return;
                }
                const epubType = trigger.getAttribute("epub:type") || "";
                const isNoteref =
                    trigger.getAttribute("role") === "doc-noteref" ||
                    trigger.classList.contains("duokan-footnote") ||
                    epubType.split(/\s+/).includes("noteref");
                if (!isNoteref) return;
                const href = trigger.getAttribute("href") || "";
                if (!href.startsWith("#")) return;
                const note = iframeDoc.getElementById(
                    decodeURIComponent(href.slice(1)),
                );
                if (!note) return;
                event.preventDefault();
                event.stopPropagation();
                iframeDoc.querySelector(".footnote-popup-backdrop")?.remove();
                const backdrop = iframeDoc.createElement("div");
                backdrop.className = "footnote-popup-backdrop";
                backdrop.style.cssText =
                    "position:fixed;inset:0;z-index:9998;background:rgba(15,23,42,.72);display:flex;align-items:center;justify-content:center;padding:20px;box-sizing:border-box";
                const popup = iframeDoc.createElement("div");
                popup.className = "footnote-popup";
                popup.innerHTML = note.innerHTML;
                popup.style.cssText =
                    "position:relative;width:min(90vw,720px);max-height:88vh;overflow:auto;background:#fff;padding:16px;box-sizing:border-box;border-radius:8px";
                backdrop.appendChild(popup);
                backdrop.addEventListener("click", (closeEvent) => {
                    if (closeEvent.target === backdrop) backdrop.remove();
                });
                iframeDoc.body.appendChild(backdrop);
            },
            true,
        );
    }

    // 预览点击 -> 编辑器定位。每次换章节都要重新绑到新文档上
    let unbindPreviewClick: (() => void) | null = null;
    function bindPreviewClickHandler() {
        unbindPreviewClick?.();
        unbindPreviewClick = null;
        const iframeDoc = previewIframe?.contentDocument;
        if (!iframeDoc || !onPreviewClick) return;
        const handler = (event: Event) => {
            const payload = describePreviewClick(
                event.target as Element | null,
            );
            if (payload) onPreviewClick(payload);
        };
        iframeDoc.addEventListener("click", handler, true);
        unbindPreviewClick = () =>
            iframeDoc.removeEventListener("click", handler, true);
    }

    function handlePreviewIframeLoad() {
        scheduleMeasure();
        watchInnerContent();
        const iframeDoc = previewIframe?.contentDocument;
        if (!iframeDoc) return;
        installPreviewChrome(iframeDoc);
        applyPaperTheme(iframeDoc, darkPaper);
        bindFootnotePopups(iframeDoc);
        bindPreviewClickHandler();
    }

    // 主题在预览已经渲染完之后才切换时，重新给这篇文档上色
    $effect(() => {
        const dark = darkPaper;
        const iframeDoc = previewIframe?.contentDocument;
        if (iframeDoc) applyPaperTheme(iframeDoc, dark);
    });

    $effect(() => {
        if (previewIframe && content) {
            const disableContextMenu = () => {
                try {
                    const iframeDoc = previewIframe?.contentDocument;
                    if (iframeDoc) {
                        iframeDoc.removeEventListener(
                            "contextmenu",
                            preventContextMenu,
                        );
                        iframeDoc.addEventListener(
                            "contextmenu",
                            preventContextMenu,
                        );
                    }
                } catch (err) {}
            };

            const preventContextMenu = (e: Event) => {
                e.preventDefault();
            };

            setTimeout(disableContextMenu, 100);
        }
    });
</script>

<div
    class="preview-container"
    class:dark-paper={darkPaper}
    use:trackContainerWidth
>
    {#if errors.length > 0}
        <div class="preview-error">
            <div class="error-header">
                <span class="error-icon">!</span>
                <span
                    >&#39044;&#35272;&#36807;&#31243;&#20013;&#20986;&#29616;&#20102;
                    {errors.length} &#20010;&#38169;&#35823;</span
                >
            </div>
            <div class="error-content">
                {#each errors as err}
                    <div class="error-item">{err.message}</div>
                {/each}
            </div>
        </div>
    {/if}

    {#if content}
        <div class="mobile-frame" class:hide-scrollbar={!showScrollbar}>
            <iframe
                bind:this={previewIframe}
                title="preview"
                srcdoc={content}
                sandbox="allow-same-origin allow-scripts"
                style:height={previewFrameHeight
                    ? `${previewFrameHeight}px`
                    : null}
                onload={handlePreviewIframeLoad}
            ></iframe>
        </div>
    {:else}
        <div class="placeholder">
            {hasSelectedFile ? "少女祈祷中······" : "请从左侧选择一个文件"}
        </div>
    {/if}
</div>

<style>
    /* Preview Error (Absolute Position) */
    .preview-error {
        background: var(--color-warning-soft);
        border: 1px solid var(--color-warning);
        border-radius: var(--radius-xs);
        margin: 8px;
        font-size: 12px;
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        z-index: 10;
        box-shadow: var(--shadow-sm);
        opacity: 0.95;
    }

    .preview-error .error-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 8px 12px;
        background: var(--color-warning);
        color: var(--color-warning);
        font-weight: 600;
        border-radius: 4px 4px 0 0;
    }

    .preview-error .error-content {
        margin: 0;
        padding: 8px 12px;
        color: var(--color-warning);
        white-space: pre-wrap;
        max-height: 100px;
        overflow-y: auto;
        font-family: "Consolas", monospace;
    }

    /* Ensure container is relative and takes full height for centering */
    .preview-container {
        position: relative;
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    /* NOTE: 页面 .placeholder 两条规则同时被编辑区占位符使用，无法整体下沉；
       此处按原级联顺序逐条复制（后一条 font-size: 14px 覆盖前一条 18px）。 */
    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        font-size: 18px;
        color: var(--color-muted);
        font-weight: 400;
        letter-spacing: 2px;
    }

    .preview-container {
        flex: 1; /* Grow vertically */
        width: 100%; /* Fill sidebar width */
        display: flex;
        flex-direction: column;
        /* 滚动交给手机框本身（.mobile-frame）：框保持手机尺寸，
           正文在框里滚，手机不会因为章节变长而拉成一条长纸带 */
        overflow: hidden;
        overscroll-behavior: contain;
        background: var(
            --color-hover
        ); /* Darker background to distinguish phone frame */
        align-items: center; /* Center horizontally */
        /* 顶部对齐：手机框高度到顶后，多出来的空间留在下方，
           和"从上往下看这一章"的阅读起点一致；垂直居中会让框悬在
           一大片灰底中间，看着像飘着 */
        justify-content: flex-start;
        padding: 0; /* No padding */
    }

    /* 隐藏滚动条（默认）。滚动条会占掉框内右侧 8px，正文就偏左、
       不在手机框里居中，所以隐藏时必须一并改成 auto */
    .mobile-frame.hide-scrollbar {
        scrollbar-width: none;
        -ms-overflow-style: none;
        scrollbar-gutter: auto;
    }

    .mobile-frame.hide-scrollbar::-webkit-scrollbar {
        width: 0;
        height: 0;
        display: none;
    }

    .mobile-frame {
        /* 设计宽度 320px（历史值：`Standard Android/iPhone Width`）。
           写 100% 时框宽 = 预览列宽 − 左右内边距，会随窗口在 244~316 之间
           浮动，早就不在设计值上了；min() 让窄窗口退化、宽窗口定死 320 */
        width: min(100%, 320px);
        /* 高度也定死上限：撑满预览列，但最多 700px（20:9 手机比例，
           320 宽对应的设计高度）。章节再长也不再往下长，正文在框内滚动。
           用 height:100% 而不是 flex-basis —— 框高只由预览列决定，
           不再反过来依赖 iframe 的量测结果，不会形成"框高→可用高→
           iframe 高→框高"的自持回路 */
        height: 100%;
        max-height: 700px;
        /* 框自己滚动：圆角和上下两条边任何章节都能看见，
           滚动条（开启时）画在手机屏内侧，而不是跑到预览列边缘 */
        overflow-y: auto;
        overflow-x: hidden;
        overscroll-behavior: contain;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        /* 深色书页时外框同色（见 .dark-paper 规则），这里只留兜底 */
        box-sizing: border-box; /* Prevent border from adding width */
        box-shadow: var(--shadow-sm); /* Shadow for depth */
        position: relative;
        display: flex; /* Make it a flex container for iframe */
        flex-direction: column;
        flex-shrink: 0;
    }

    .preview-container iframe {
        width: 100%;
        /* 高度由 JS 实测正文高度写入 inline style；
           量测完成前先铺满手机框，避免首帧只有默认高度、随后再长高 */
        height: 100%;
        border: none;
        background: var(--color-surface);
        margin: 0;
        padding: 0;
        display: block;
        flex-shrink: 0;
    }

    /* 深色书页：外框和 iframe 垫底都换成纸色，否则书页四周会露一圈白。
       !important 是必需的：app.css 里有一条全局
       `:root[data-theme="dark"] ... .mobile-frame { background: surface !important }`
       （给其它预览用），不带 !important 压不住它，手机框会刷回 surface，
       书页外圈就多出一圈深灰，看着像没对齐 */
    :root[data-theme="dark"] .preview-container.dark-paper,
    :root[data-theme="dark"] .preview-container.dark-paper .mobile-frame,
    :root[data-theme="dark"] .preview-container.dark-paper iframe {
        background: var(--color-canvas, #191e26) !important;
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        color: var(--color-muted);
        font-size: 14px;
    }

    /* NOTE: 页面全局 ::-webkit-scrollbar 规则经 Svelte hash 作用域后不再穿透
       组件边界，此处复制以保持 .error-content 滚动条像素一致（dead06b 先例）。 */
    ::-webkit-scrollbar {
        width: 8px;
        height: 8px;
    }

    ::-webkit-scrollbar-track {
        background: var(--color-surface-soft);
    }

    ::-webkit-scrollbar-thumb {
        background: var(--color-muted);
        border-radius: var(--radius-xs);
    }

    ::-webkit-scrollbar-thumb:hover {
        background: var(--color-text-soft);
    }

    /* Modern UI overrides */
    /* 预览槽的底色走 token：写死浅色时深色主题下会在书页两侧
       露出一圈发白的板，看起来像没切主题 */
    .preview-container {
        background: linear-gradient(
            180deg,
            var(--color-surface-soft),
            var(--color-hover)
        );
        padding: 10px;
        box-sizing: border-box;
    }

    .mobile-frame {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-md);
    }

    .preview-container iframe {
        border-radius: var(--radius-lg);
    }

    .placeholder {
        color: var(--color-muted);
        letter-spacing: 0.03em;
    }

    .preview-error {
        background: var(--color-warning-soft);
        border-color: rgba(233, 155, 47, 0.38);
        border-radius: var(--radius-md);
        box-shadow: var(--shadow-sm);
    }

    .preview-error .error-header {
        background: rgba(233, 155, 47, 0.18);
        color: var(--color-warning);
        border-radius: var(--radius-md) var(--radius-md) 0 0;
    }

    .preview-error .error-content {
        color: var(--color-warning);
        font-family: var(--font-code);
    }

    ::-webkit-scrollbar-track {
        background: var(--color-surface-soft);
    }

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(
            180deg,
            var(--color-border-strong),
            var(--color-border)
        );
        border-radius: 999px;
    }
</style>
