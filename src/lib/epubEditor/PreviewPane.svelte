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

    let {
        content,
        errors,
        hasSelectedFile,
        onPreviewClick,
        showScrollbar = false,
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
    } = $props();

    let previewIframe: HTMLIFrameElement | null = $state(null);
    let previewContainer: HTMLDivElement | null = $state(null);
    let previewFrameHeight = $state(0);

    /** 暴露预览 iframe 的 window，供需要直接操作预览文档的调用方使用 */
    export function getContentWindow(): Window | null {
        return previewIframe?.contentWindow ?? null;
    }

    /**
     * 编辑器里点了某一行 / 选中了一段文字时，把预览滚到对应位置。
     * 不能再往 iframe 里 postMessage：iframe 已被撑到正文高度，
     * 它自己没有可滚动区域，要滚的是外层 .preview-container。
     */
    export function revealText(text: string, mode: RevealMode = "center") {
        return revealTextInPreview(previewContainer, previewIframe, text, {
            mode,
        });
    }

    /**
     * 让 iframe 长到正文高度、由外层 .preview-container 统一滚动。
     * 否则 iframe 被固定成容器高度，滚动条会画在白色书页框内部。
     */
    function measureFrameHeight() {
        const frame = previewIframe;
        const innerBody = frame?.contentDocument?.body;
        if (!innerBody) return;
        // scrollHeight 不含 body 自己的外边距。书的 CSS 常给 body 上下留 8px，
        // 漏掉这 16px，iframe 就比内容矮一截，正文能滚、章节末尾被裁掉
        const bodyStyle = getComputedStyle(innerBody);
        const verticalMargin =
            (parseFloat(bodyStyle.marginTop) || 0) +
            (parseFloat(bodyStyle.marginBottom) || 0);
        const contentHeight =
            Math.max(innerBody.scrollHeight || 0, innerBody.offsetHeight || 0) +
            verticalMargin;
        if (contentHeight <= 0) return;
        // 短章节也占满可视区域，避免书页框下面露出灰底
        const host = frame.closest<HTMLElement>(".preview-container");
        // 容器不可见时量不到可用高度，保留上一次结果，
        // 避免切回预览页签时高度先塌再长、滚动条闪一下
        if (host && host.clientHeight <= 0) return;
        let availableHeight = 0;
        if (host) {
            const hostStyle = getComputedStyle(host);
            const shell = frame.parentElement;
            const shellStyle = shell ? getComputedStyle(shell) : null;
            availableHeight = Math.floor(
                host.clientHeight -
                    parseFloat(hostStyle.paddingTop) -
                    parseFloat(hostStyle.paddingBottom) -
                    parseFloat(shellStyle?.borderTopWidth || "0") -
                    parseFloat(shellStyle?.borderBottomWidth || "0"),
            );
        }
        previewFrameHeight = Math.max(contentHeight, availableHeight);
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
        const observer = new ResizeObserver(() => {
            // 把可视高度写到 CSS 变量，供 iframe 在实测完成前先占满一屏，
            // 避免首帧只有默认高度、随后再长高导致的滚动条闪动
            const style = getComputedStyle(node);
            const inner =
                node.clientHeight -
                parseFloat(style.paddingTop) -
                parseFloat(style.paddingBottom);
            if (inner > 0) node.style.setProperty("--preview-h", `${inner}px`);

            const width = node.clientWidth;
            if (width <= 0) return;
            if (Math.abs(width - lastWidth) < 2) return;
            lastWidth = width;
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
        bindFootnotePopups(iframeDoc);
        bindPreviewClickHandler();
    }

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
    class:hide-scrollbar={!showScrollbar}
    bind:this={previewContainer}
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
        <div class="mobile-frame">
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
        /* 滚动交给预览容器本身，滚动条贴在这一栏的右边缘；
           让 iframe 自己滚会把滚动条画在白色书页框里面 */
        overflow-y: auto;
        overflow-x: hidden; /* Prevent horizontal scrollbar */
        overscroll-behavior: contain;
        scrollbar-gutter: stable;
        background: var(
            --color-hover
        ); /* Darker background to distinguish phone frame */
        align-items: center; /* Center horizontally */
        justify-content: flex-start; /* Align to top (placeholder will center itself) */
        padding: 0; /* No padding */
    }

    /* 隐藏滚动条（默认）。
       scrollbar-gutter: stable 会永久预留 8px 槽位，书页因此偏左、
       不在预览列正中，所以隐藏时必须一并改成 auto */
    .preview-container.hide-scrollbar {
        scrollbar-width: none;
        -ms-overflow-style: none;
        scrollbar-gutter: auto;
    }

    .preview-container.hide-scrollbar::-webkit-scrollbar {
        width: 0;
        height: 0;
        display: none;
    }

    .mobile-frame {
        width: 100%; /* 320px */
        /* 高度跟随 iframe 实测的正文高度，由外层容器滚动 */
        height: auto;
        max-height: none;
        background: var(--color-surface);
        border: 1px solid var(--color-border);
        box-sizing: border-box; /* Prevent border from adding width */
        box-shadow: var(--shadow-sm); /* Shadow for depth */
        overflow: hidden; /* Hide overflow outside the "phone screen" */
        position: relative;
        display: flex; /* Make it a flex container for iframe */
        flex-direction: column;
        flex-shrink: 0;
    }

    .mobile-frame::-webkit-scrollbar {
        display: none;
    }

    .preview-container iframe {
        width: 100%;
        /* 高度由 JS 实测正文高度写入 inline style；
           测量完成前先铺满预览区可视高度（--preview-h 由 trackContainerWidth 写入） */
        height: var(--preview-h, 100%);
        border: none;
        background: var(--color-surface);
        margin: 0;
        padding: 0;
        display: block;
        flex-shrink: 0;
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
    .preview-container {
        background: linear-gradient(
            180deg,
            rgba(231, 240, 247, 0.88),
            rgba(239, 245, 249, 0.94)
        );
        padding: 12px;
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
        background: rgba(226, 235, 244, 0.72);
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
