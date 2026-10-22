<script lang="ts">
    import CustomSelect from "$lib/CustomSelect.svelte";
    import { t } from "$lib/i18n.svelte";
    import type { SearchDirection, SearchScope } from "$lib/findReplace";

    let {
        findPattern = $bindable(),
        replacePattern = $bindable(),
        isRegex = $bindable(),
        searchScope = $bindable(),
        searchDirection = $bindable(),
        wrapAround = $bindable(),
        textOnly = $bindable(),
        findHistory,
        replaceHistory,
        searchMessage,
        onFindNext,
        onFindPrev,
        onCountMatches,
        onReplace,
        onReplaceAll,
        onRemoveFindHistory,
        onRemoveReplaceHistory,
    }: {
        findPattern: string;
        replacePattern: string;
        isRegex: boolean;
        searchScope: SearchScope;
        searchDirection: SearchDirection;
        wrapAround: boolean;
        textOnly: boolean;
        findHistory: string[];
        replaceHistory: string[];
        searchMessage: string;
        onFindNext: () => void;
        onFindPrev: () => void;
        onCountMatches: () => void;
        onReplace: () => void;
        onReplaceAll: () => void;
        onRemoveFindHistory: (index: number) => void;
        onRemoveReplaceHistory: (index: number) => void;
    } = $props();

    const FIND_MODE_OPTIONS = [
        { value: "text", label: "普通文本" },
        { value: "regex", label: "正则表达式" },
    ];

    const SEARCH_SCOPE_OPTIONS = [
        { value: "current", label: "当前文件" },
        { value: "open", label: "已打开" },
        { value: "html", label: "HTML文件" },
        { value: "selected", label: "选中文件" },
        { value: "all", label: "全部文件" },
    ];

    const SEARCH_DIRECTION_OPTIONS = [
        { value: "down", label: "向下" },
        { value: "up", label: "向上" },
    ];

    let showFindHistory = $state(false);
    let showReplaceHistory = $state(false);

    function selectFindHistory(text: string) {
        findPattern = text;
        showFindHistory = false;
    }

    function selectReplaceHistory(text: string) {
        replacePattern = text;
        showReplaceHistory = false;
    }

    // 点输入框本身不再切换历史面板，否则和右侧 ▾ 按钮的语义重复
    function closeHistory() {
        showFindHistory = false;
        showReplaceHistory = false;
    }
</script>

<div class="find-replace-panel">
    <div class="fr-row">
        <span class="fr-label">查找:</span>
        <div class="fr-input-wrapper">
            <input
                id="epub-search-input-fr"
                type="text"
                class="fr-input"
                bind:value={findPattern}
                onkeydown={(e) => e.key === "Enter" && onFindNext()}
                onfocus={closeHistory}
                onclick={closeHistory}
                autocomplete="off"
            />
            <button
                class="fr-dropdown-btn"
                onclick={() => {
                    showFindHistory = !showFindHistory;
                    showReplaceHistory = false;
                }}
                title="显示查找历史"
                >&#9662;</button
            >
            {#if showFindHistory && findHistory.length > 0}
                <div class="fr-history-dropdown">
                    {#each findHistory as item, i}
                        <div class="fr-history-item">
                            <span
                                class="fr-history-text"
                                onclick={() => selectFindHistory(item)}
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    selectFindHistory(item)}
                                role="button"
                                tabindex="0">{item}</span
                            >
                            <button
                                class="fr-history-del"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    onRemoveFindHistory(i);
                                }}
                                title="删除">&times;</button
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
        <div class="fr-actions fr-find-actions">
            <button
                class="fr-btn fr-btn-text"
                onclick={onCountMatches}
                title="计数">计数</button
            >
            <button
                class="fr-btn fr-btn-text"
                onclick={onFindPrev}
                title="上一个"
                >上一个</button
            >
            <button
                class="fr-btn fr-btn-text"
                onclick={onFindNext}
                title="下一个"
                >下一个</button
            >
        </div>
    </div>
    <div class="fr-row">
        <span class="fr-label">替换:</span>
        <div class="fr-input-wrapper">
            <input
                type="text"
                class="fr-input"
                bind:value={replacePattern}
                onfocus={closeHistory}
                onclick={closeHistory}
                autocomplete="off"
            />
            <button
                class="fr-dropdown-btn"
                onclick={() => {
                    showReplaceHistory = !showReplaceHistory;
                    showFindHistory = false;
                }}
                title="显示替换历史"
                >&#9662;</button
            >
            {#if showReplaceHistory && replaceHistory.length > 0}
                <div class="fr-history-dropdown">
                    {#each replaceHistory as item, i}
                        <div class="fr-history-item">
                            <span
                                class="fr-history-text"
                                onclick={() => selectReplaceHistory(item)}
                                onkeydown={(e) =>
                                    e.key === "Enter" &&
                                    selectReplaceHistory(item)}
                                role="button"
                                tabindex="0">{item}</span
                            >
                            <button
                                class="fr-history-del"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    onRemoveReplaceHistory(i);
                                }}
                                title="删除">&times;</button
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
        <div class="fr-actions">
            <button
                class="fr-btn fr-btn-text"
                onclick={onReplace}
                title="替换">替换</button
            >
            <button
                class="fr-btn fr-btn-wide fr-danger"
                onclick={onReplaceAll}
                title="全部替换"
                >全部替换</button
            >
        </div>
    </div>
    <div class="fr-row fr-options">
        <span class="fr-label">搜索选项:</span>
        <CustomSelect
            className="fr-custom-select fr-select-sm"
            value={isRegex ? "regex" : "text"}
            options={FIND_MODE_OPTIONS}
            on:change={(event) => (isRegex = event.detail === "regex")}
        />
        <CustomSelect
            className="fr-custom-select"
            value={searchScope}
            options={SEARCH_SCOPE_OPTIONS}
            on:change={(event) => (searchScope = event.detail as SearchScope)}
        />
        <CustomSelect
            className="fr-custom-select fr-select-xs"
            value={searchDirection}
            options={SEARCH_DIRECTION_OPTIONS}
            on:change={(event) =>
                (searchDirection = event.detail as SearchDirection)}
        />
        <label class="fr-checkbox">
            <input type="checkbox" bind:checked={wrapAround} />循环
        </label>
        <label class="fr-checkbox">
            <input
                type="checkbox"
                bind:checked={textOnly}
            />纯文本
        </label>
        <span class="fr-message">{t(searchMessage)}</span>
    </div>
</div>

<style>
    .find-replace-panel {
        background: var(--color-hover);
        border-top: 1px solid var(--color-border-strong);
        padding: 6px 10px;
        font-size: 13px;
        flex-shrink: 0;
        box-sizing: border-box;
        width: 100%;
        overflow: visible;
    }

    .fr-row {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-bottom: 6px;
        overflow: visible;
    }

    .fr-actions {
        display: flex;
        gap: 4px;
        width: 154px;
        justify-content: space-between;
        flex-shrink: 0;
    }

    .fr-row:last-child {
        margin-bottom: 0;
    }

    .fr-label {
        font-weight: 600;
        color: var(--color-text-soft);
        min-width: 40px;
        text-align: right;
    }

    .fr-input {
        flex: 1;
        padding: 6px 10px;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        font-size: 13px;
        font-family: "Consolas", "Monaco", monospace;
        background: var(--color-surface);
    }

    .fr-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-btn {
        padding: 4px 8px;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        background: linear-gradient(
            to bottom,
            var(--color-surface),
            var(--color-border)
        );
        cursor: pointer;
        font-size: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        white-space: nowrap;
    }

    .fr-btn-text {
        padding: 4px 10px;
    }

    .fr-btn:hover {
        background: linear-gradient(
            to bottom,
            var(--color-surface-soft),
            var(--color-border)
        );
        border-color: var(--color-muted);
    }

    .fr-btn:active {
        background: var(--color-border);
    }

    .fr-danger {
        background: linear-gradient(
            to bottom,
            var(--color-warning-soft),
            #ffddcc
        );
        border-color: var(--color-warning);
    }

    .fr-danger:hover {
        background: linear-gradient(to bottom, #ffe6d9, #ffcc99);
    }

    .fr-options {
        font-size: 12px;
    }

    .fr-find-actions {
        width: auto;
        justify-content: flex-start;
    }

    .fr-options :global(.fr-custom-select) {
        flex: 0 0 104px;
        width: 104px;
    }

    .fr-options :global(.fr-custom-select.fr-select-sm) {
        flex-basis: 96px;
        width: 96px;
    }

    .fr-options :global(.fr-custom-select.fr-select-xs) {
        flex-basis: 70px;
        width: 70px;
    }

    .fr-options :global(.fr-custom-select .custom-select-trigger) {
        height: 30px;
        min-height: 30px;
        padding: 0 8px;
        border-color: var(--color-border-strong);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text);
        box-shadow: none;
        font-size: 12px;
        font-weight: 600;
    }

    .fr-options :global(.fr-custom-select.open .custom-select-trigger),
    .fr-options
        :global(.fr-custom-select .custom-select-trigger:focus-visible) {
        border-color: var(--color-accent);
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu) {
        top: auto;
        bottom: calc(100% + 4px);
        border-color: var(--color-border-strong);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        box-shadow: var(--shadow-md);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu button) {
        min-height: 28px;
        padding: 5px 8px;
        border-radius: var(--radius-xs);
        font-size: 12px;
        font-weight: 600;
    }

    .fr-checkbox {
        display: flex;
        align-items: center;
        gap: 4px;
        cursor: pointer;
        color: var(--color-text-soft);
    }

    .fr-checkbox input {
        margin: 0;
        cursor: pointer;
    }

    .fr-message {
        margin-left: auto;
        color: var(--color-accent);
        font-weight: 500;
        min-width: 80px;
        text-align: right;
    }

    .fr-input-wrapper {
        flex: 1;
        display: flex;
        position: relative;
    }

    .fr-input-wrapper .fr-input {
        flex: 1;
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
        border-right: none;
    }

    .fr-dropdown-btn {
        padding: 6px 8px;
        border: 1px solid var(--color-border-strong);
        border-left: none;
        border-top-right-radius: 4px;
        border-bottom-right-radius: 4px;
        background: linear-gradient(
            to bottom,
            var(--color-surface),
            var(--color-border)
        );
        cursor: pointer;
        font-size: 10px;
        color: var(--color-muted);
    }

    .fr-dropdown-btn:hover {
        background: linear-gradient(
            to bottom,
            var(--color-surface-soft),
            var(--color-border)
        );
    }

    .fr-history-dropdown {
        position: absolute;
        bottom: 100%;
        left: 0;
        right: 0;
        margin-bottom: 2px;
        background: var(--color-surface);
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        box-shadow: var(--shadow-sm);
        z-index: 200;
        max-height: 400px;
        overflow-y: auto;
    }

    .fr-history-item {
        display: flex;
        align-items: center;
        padding: 6px 10px;
        border-bottom: 1px solid var(--color-border);
    }

    .fr-history-item:last-child {
        border-bottom: none;
    }

    .fr-history-item:hover {
        background: var(--color-surface-soft);
    }

    .fr-history-text {
        flex: 1;
        cursor: pointer;
        font-family: "Consolas", "Monaco", monospace;
        font-size: 12px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .fr-history-del {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 12px;
        color: var(--color-muted);
        padding: 2px 6px;
        border-radius: var(--radius-xs);
        margin-left: 8px;
    }

    .fr-history-del:hover {
        background: #ffdddd;
        color: var(--color-danger);
    }

    /* Modern UI overrides */
    .find-replace-panel {
        background: var(--color-surface);
        border-top: 1px solid var(--color-border-strong);
        padding: 9px 12px;
        box-shadow: 0 -8px 20px
            color-mix(in srgb, var(--color-border) 42%, transparent);
        backdrop-filter: none;
    }

    .fr-row {
        gap: 8px;
    }

    .fr-label,
    .fr-checkbox {
        color: var(--color-text-soft);
    }

    .fr-input,
    .fr-dropdown-btn {
        border-color: var(--color-border);
        background: var(--color-surface);
        color: var(--color-text);
        border-radius: var(--radius-xs);
    }

    .fr-input {
        font-family: var(--font-code);
    }

    .fr-input:focus {
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
    }

    .fr-input-wrapper .fr-input {
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
    }

    .fr-dropdown-btn {
        border-left: none;
        border-top-left-radius: 0;
        border-bottom-left-radius: 0;
        background: var(--color-surface);
    }

    .fr-btn {
        border-color: var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text-soft);
        box-shadow: var(--shadow-xs);
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast),
            color var(--transition-fast),
            transform var(--transition-fast);
    }

    .fr-btn:hover {
        background: var(--color-hover);
        border-color: var(--color-border-strong);
        color: var(--color-text);
    }

    .fr-btn:active {
        background: var(--color-active);
        transform: translateY(1px);
    }

    .fr-danger {
        background: var(--color-danger-soft);
        border-color: rgba(215, 68, 82, 0.3);
        color: var(--color-danger);
    }

    .fr-danger:hover {
        background: var(--color-danger-soft);
        border-color: rgba(215, 68, 82, 0.42);
    }

    .fr-message {
        color: var(--color-accent-deep);
    }

    .fr-history-dropdown {
        border-color: var(--color-border);
        border-radius: var(--radius-sm);
        box-shadow: var(--shadow-md);
    }

    .fr-history-item {
        border-bottom-color: var(--color-border);
    }

    .fr-history-item:hover {
        background: var(--color-hover);
    }

    .fr-history-text {
        font-family: var(--font-code);
    }

    .fr-history-del:hover {
        background: var(--color-danger-soft);
        color: var(--color-danger);
    }
</style>
