<script lang="ts">
    let {
        fileName,
        internalName,
        fontFamily,
        previewText = $bindable(),
        statsLoading,
        statsError,
        usedCharCount,
        missingGlyphCharCount,
        missingChars,
    }: {
        fileName: string;
        internalName: string;
        fontFamily: string;
        previewText: string;
        statsLoading: boolean;
        statsError: string;
        usedCharCount: number;
        missingGlyphCharCount: number;
        missingChars: string[];
    } = $props();
</script>

<div class="font-preview-container">
    <div class="font-preview-header">
        <div class="font-preview-title">
            {fileName}
            {#if internalName}
                <span class="font-internal-name">({internalName})</span>
            {/if}
        </div>
        <div class="font-preview-tip">
            &#20351;&#29992;&#23884;&#20837;&#23383;&#20307;&#23454;&#26102;&#39044;&#35272;
        </div>
    </div>
    <textarea
        class="font-preview-sample"
        style={'font-family: "' + fontFamily + '";'}
        bind:value={previewText}
        spellcheck="false"
        wrap="off"></textarea>
    <div class="font-stats">
        {#if statsLoading}
            <div class="font-stats-line">&#20998;&#26512;&#20013;...</div>
        {:else if statsError}
            <div class="font-stats-line font-stats-error">{statsError}</div>
        {:else}
            <div class="font-stats-line">
                &#24050;&#20351;&#29992;&#23383;&#31526;&#25968;&#65306;{usedCharCount}
            </div>
            <div class="font-stats-line">
                &#32570;&#22833;&#23383;&#24418;&#25968;&#65306;{missingGlyphCharCount}
            </div>
            {#if missingChars.length > 0}
                <div class="font-stats-line font-stats-missing-title">
                    &#32570;&#22833;&#23383;&#31526;&#65288;{missingChars.length}&#65289;&#65306;
                </div>
                <div class="font-missing-chars">{missingChars.join(" ")}</div>
            {/if}
        {/if}
    </div>
</div>

<style>
    .font-preview-container {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        overflow: auto;
        background: #fafbfc;
        padding: 24px;
        gap: 16px;
    }

    .font-preview-header {
        flex-shrink: 0;
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 16px;
        border-bottom: 1px solid #e6e9ef;
        padding-bottom: 12px;
    }

    .font-preview-title {
        color: #1f2d3d;
        font-size: 18px;
        font-weight: 700;
        line-height: 1.3;
    }

    .font-internal-name {
        color: #5f6b7a;
        font-size: 16px;
        font-weight: 600;
        margin-left: 8px;
    }

    .font-preview-tip {
        color: #7a8699;
        font-size: 13px;
        white-space: nowrap;
    }

    .font-preview-sample {
        flex: 0 1 auto;
        width: 100%;
        min-height: 240px;
        height: min(320px, 42vh);
        box-sizing: border-box;
        color: #13233a;
        background: #fff;
        border: 1px solid #e6e9ef;
        border-radius: 10px;
        padding: 20px;
        resize: vertical;
        outline: none;
        overflow: auto;
        white-space: pre;
        font-size: 46px;
        line-height: 1.45;
        letter-spacing: 0;
    }

    .font-preview-sample:focus {
        border-color: #4a90d9;
        box-shadow: 0 0 0 2px rgba(74, 144, 217, 0.16);
    }

    .font-stats {
        flex-shrink: 0;
        border-top: 1px solid #e6e9ef;
        padding-top: 14px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .font-stats-line {
        color: #3a4a5f;
        font-size: 14px;
        line-height: 1.5;
    }

    .font-stats-error {
        color: #c0392b;
    }

    .font-stats-missing-title {
        margin-top: 4px;
        font-weight: 600;
    }

    .font-missing-chars {
        color: #a73838;
        font-size: 15px;
        line-height: 1.6;
        word-break: break-all;
    }

    /* NOTE: 页面全局 ::-webkit-scrollbar 规则经 Svelte hash 作用域后不再穿透
       组件边界，此处复制以保持容器与样例 textarea 滚动条像素一致（aa171d2 先例）。 */
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
    /* NOTE: 页面原为 .image-preview-container, .font-preview-container 共享
       渐变背景一条规则；此处拆出 .font-preview-container 份，页面保留图片份。 */
    .font-preview-container {
        background: linear-gradient(
            180deg,
            rgba(247, 250, 253, 0.94),
            rgba(238, 244, 248, 0.94)
        );
    }

    .font-preview-container {
        padding: 26px;
        gap: 18px;
    }

    .font-preview-header {
        border-bottom-color: var(--color-border);
    }

    .font-preview-title {
        color: var(--color-text);
    }

    .font-internal-name,
    .font-preview-tip,
    .font-stats-line {
        color: var(--color-text-soft);
    }

    .font-preview-sample {
        background: rgba(255, 255, 255, 0.92);
        border-color: var(--color-border);
        border-radius: var(--radius-md);
        color: var(--color-text);
        box-shadow:
            inset 0 1px 0 rgba(255, 255, 255, 0.72),
            var(--shadow-xs);
    }

    .font-preview-sample:focus {
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
    }

    .font-stats {
        border-top-color: var(--color-border);
    }

    .font-stats-error,
    .font-missing-chars {
        color: var(--color-danger);
    }

    ::-webkit-scrollbar-track {
        background: rgba(226, 235, 244, 0.72);
    }

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(180deg, #bacbda, #93a8bb);
        border-radius: 999px;
    }
</style>
