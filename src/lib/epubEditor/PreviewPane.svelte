<script module lang="ts">
    export interface ValidationError {
        type: "tag" | "img";
        message: string;
        line: number;
    }
</script>

<script lang="ts">
    let {
        content,
        errors,
        hasSelectedFile,
    }: {
        content: string;
        errors: ValidationError[];
        hasSelectedFile: boolean;
    } = $props();

    let previewIframe: HTMLIFrameElement | null = $state(null);

    /** Expose the preview iframe's contentWindow for editor -> preview postMessage sync. */
    export function getContentWindow(): Window | null {
        return previewIframe?.contentWindow ?? null;
    }

    function handlePreviewIframeLoad() {
        const iframeDoc = previewIframe?.contentDocument;
        if (!iframeDoc || iframeDoc.documentElement.dataset.tepubFootnotesBound === "1") return;
        iframeDoc.documentElement.dataset.tepubFootnotesBound = "1";
        iframeDoc.addEventListener("click", (event) => {
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
            const isNoteref = trigger.getAttribute("role") === "doc-noteref"
                || trigger.classList.contains("duokan-footnote")
                || epubType.split(/\s+/).includes("noteref");
            if (!isNoteref) return;
            const href = trigger.getAttribute("href") || "";
            if (!href.startsWith("#")) return;
            const note = iframeDoc.getElementById(decodeURIComponent(href.slice(1)));
            if (!note) return;
            event.preventDefault();
            event.stopPropagation();
            iframeDoc.querySelector(".footnote-popup-backdrop")?.remove();
            const backdrop = iframeDoc.createElement("div");
            backdrop.className = "footnote-popup-backdrop";
            backdrop.style.cssText = "position:fixed;inset:0;z-index:9998;background:rgba(15,23,42,.72);display:flex;align-items:center;justify-content:center;padding:20px;box-sizing:border-box";
            const popup = iframeDoc.createElement("div");
            popup.className = "footnote-popup";
            popup.innerHTML = note.innerHTML;
            popup.style.cssText = "position:relative;width:min(90vw,720px);max-height:88vh;overflow:auto;background:#fff;padding:16px;box-sizing:border-box;border-radius:8px";
            backdrop.appendChild(popup);
            backdrop.addEventListener("click", (closeEvent) => {
                if (closeEvent.target === backdrop) backdrop.remove();
            });
            iframeDoc.body.appendChild(backdrop);
        }, true);
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
                } catch (err) {
                }
            };

            const preventContextMenu = (e: Event) => {
                e.preventDefault();
            };

            setTimeout(disableContextMenu, 100);
        }
    });
</script>

<div class="preview-container">
    {#if errors.length > 0}
        <div class="preview-error">
            <div class="error-header">
                <span class="error-icon">!</span>
                <span>&#39044;&#35272;&#36807;&#31243;&#20013;&#20986;&#29616;&#20102; {errors.length} &#20010;&#38169;&#35823;</span>
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
                onload={handlePreviewIframeLoad}
            ></iframe>
        </div>
    {:else}
        <div class="placeholder">
            {hasSelectedFile
                ? "少女祈祷中······"
                : "请从左侧选择一个文件"}
        </div>
    {/if}
</div>

<style>
    /* Preview Error (Absolute Position) */
    .preview-error {
        background: #fff3cd;
        border: 1px solid #ffc107;
        border-radius: 4px;
        margin: 8px;
        font-size: 12px;
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        z-index: 10;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        opacity: 0.95;
    }

    .preview-error .error-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 8px 12px;
        background: #ffc107;
        color: #856404;
        font-weight: 600;
        border-radius: 4px 4px 0 0;
    }

    .preview-error .error-content {
        margin: 0;
        padding: 8px 12px;
        color: #856404;
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
        color: #999;
        font-weight: 400;
        letter-spacing: 2px;
    }

    .preview-container {
        flex: 1; /* Grow vertically */
        width: 100%; /* Fill sidebar width */
        display: flex;
        flex-direction: column;
        overflow-y: hidden; /* Let iframe handle scroll, prevent double scrollbar */
        overflow-x: hidden; /* Prevent horizontal scrollbar */
        background: #f0f0f0; /* Darker background to distinguish phone frame */
        align-items: center; /* Center horizontally */
        justify-content: flex-start; /* Align to top (placeholder will center itself) */
        padding: 0; /* No padding */
    }

    .mobile-frame {
        width: 100%; /* 320px */
        height: 700px;
        max-height: 100%;
        background: #fff;
        border: 1px solid #ddd;
        box-sizing: border-box; /* Prevent border from adding width */
        box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1); /* Shadow for depth */
        overflow: hidden; /* Hide overflow outside the "phone screen" */
        position: relative;
        display: flex; /* Make it a flex container for iframe */
        flex-direction: column;
    }

    .mobile-frame::-webkit-scrollbar {
        display: none;
    }

    .preview-container iframe {
        width: 100%;
        height: 100%;
        border: none;
        background: #fff;
        margin: 0;
        padding: 0;
        display: block;
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        color: #999;
        font-size: 14px;
    }

    /* NOTE: 页面全局 ::-webkit-scrollbar 规则经 Svelte hash 作用域后不再穿透
       组件边界，此处复制以保持 .error-content 滚动条像素一致（dead06b 先例）。 */
    ::-webkit-scrollbar {
        width: 8px;
        height: 8px;
    }

    ::-webkit-scrollbar-track {
        background: #f1f1f1;
    }

    ::-webkit-scrollbar-thumb {
        background: #888;
        border-radius: 4px;
    }

    ::-webkit-scrollbar-thumb:hover {
        background: #555;
    }

    /* Modern UI overrides */
    .preview-container {
        background: linear-gradient(180deg, rgba(231, 240, 247, 0.88), rgba(239, 245, 249, 0.94));
        padding: 12px;
        box-sizing: border-box;
    }

    .mobile-frame {
        border: 1px solid var(--color-border);
        border-radius: 18px;
        box-shadow: var(--shadow-md);
    }

    .preview-container iframe {
        border-radius: 17px;
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
        color: #8a5a14;
        border-radius: var(--radius-md) var(--radius-md) 0 0;
    }

    .preview-error .error-content {
        color: #8a5a14;
        font-family: var(--font-code);
    }

    ::-webkit-scrollbar-track {
        background: rgba(226, 235, 244, 0.72);
    }

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(180deg, #bacbda, #93a8bb);
        border-radius: 999px;
    }
</style>
