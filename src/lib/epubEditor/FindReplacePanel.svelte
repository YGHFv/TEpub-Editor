<script lang="ts">
    import CustomSelect from "$lib/CustomSelect.svelte";
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
</script>

<div class="find-replace-panel">
    <div class="fr-row">
        <span class="fr-label">&#26597;&#25214;:</span>
        <div class="fr-input-wrapper">
            <input
                id="epub-search-input-fr"
                type="text"
                class="fr-input"
                bind:value={findPattern}
                onkeydown={(e) => e.key === "Enter" && onFindNext()}
                onclick={() => {
                    showFindHistory = !showFindHistory;
                    showReplaceHistory = false;
                }}
                title="&#26174;&#31034;&#26597;&#25214;&#21382;&#21490;"
            />
            {#if showFindHistory && findHistory.length > 0}
                <div class="fr-history-dropdown">
                    {#each findHistory as item, i}
                        <div class="fr-history-item">
                            <span
                                class="fr-history-text"
                                onclick={() => selectFindHistory(item)}
                                onkeydown={(e) => e.key === "Enter" && selectFindHistory(item)}
                                role="button"
                                tabindex="0"
                                >{item}</span
                            >
                            <button
                                class="fr-history-del"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    onRemoveFindHistory(i);
                                }}
                                title="&#21024;&#38500;"
                                >&times;</button
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
        <div class="fr-actions fr-find-actions">
            <button class="fr-btn fr-btn-text" onclick={onCountMatches} title="&#35745;&#25968;"
                >&#35745;&#25968;</button
            >
            <button class="fr-btn fr-btn-text" onclick={onFindPrev} title="&#19978;&#19968;&#20010;"
                >&#19978;&#19968;&#20010;</button
            >
            <button class="fr-btn fr-btn-text" onclick={onFindNext} title="&#19979;&#19968;&#20010;"
                >&#19979;&#19968;&#20010;</button
            >
        </div>
    </div>
    <div class="fr-row">
        <span class="fr-label">&#26367;&#25442;:</span>
        <div class="fr-input-wrapper">
            <input
                type="text"
                class="fr-input"
                bind:value={replacePattern}
                onfocus={() => {
                    showFindHistory = false;
                    showReplaceHistory = false;
                }}
            />
            <button
                class="fr-dropdown-btn"
                onclick={() => {
                    showReplaceHistory = !showReplaceHistory;
                    showFindHistory = false;
                }}
                title="&#26174;&#31034;&#26367;&#25442;&#21382;&#21490;"
                >&#9662;</button
            >
            {#if showReplaceHistory && replaceHistory.length > 0}
                <div class="fr-history-dropdown">
                    {#each replaceHistory as item, i}
                        <div class="fr-history-item">
                            <span
                                class="fr-history-text"
                                onclick={() => selectReplaceHistory(item)}
                                onkeydown={(e) => e.key === "Enter" && selectReplaceHistory(item)}
                                role="button"
                                tabindex="0"
                                >{item}</span
                            >
                            <button
                                class="fr-history-del"
                                onclick={(e) => {
                                    e.stopPropagation();
                                    onRemoveReplaceHistory(i);
                                }}
                                title="&#21024;&#38500;"
                                >&times;</button
                            >
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
        <div class="fr-actions">
            <button class="fr-btn fr-btn-text" onclick={onReplace} title="&#26367;&#25442;"
                >&#26367;&#25442;</button
            >
            <button
                class="fr-btn fr-btn-wide fr-danger"
                onclick={onReplaceAll}
                title="&#20840;&#37096;&#26367;&#25442;"
                >&#20840;&#37096;&#26367;&#25442;</button
            >
        </div>
    </div>
    <div class="fr-row fr-options">
        <span class="fr-label">&#25628;&#32034;&#36873;&#39033;:</span>
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
            on:change={(event) => (searchDirection = event.detail as SearchDirection)}
        />
        <label class="fr-checkbox">
            <input type="checkbox" bind:checked={wrapAround} />&#24490;&#29615;
        </label>
        <label class="fr-checkbox">
            <input type="checkbox" bind:checked={textOnly} />&#32431;&#25991;&#26412;
        </label>
        <span class="fr-message">{searchMessage}</span>
    </div>
</div>

<style>
    .find-replace-panel {
        background: #f0f0f0;
        border-top: 1px solid #ccc;
        padding: 6px 10px;
        font-size: 13px;
        flex-shrink: 0;
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
        color: #555;
        min-width: 40px;
        text-align: right;
    }

    .fr-input {
        flex: 1;
        padding: 6px 10px;
        border: 1px solid #ccc;
        border-radius: 4px;
        font-size: 13px;
        font-family: "Consolas", "Monaco", monospace;
        background: #fff;
    }

    .fr-input:focus {
        outline: none;
        border-color: #2196f3;
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-btn {
        padding: 4px 8px;
        border: 1px solid #bbb;
        border-radius: 4px;
        background: linear-gradient(to bottom, #fff, #e8e8e8);
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
        background: linear-gradient(to bottom, #f5f5f5, #ddd);
        border-color: #999;
    }

    .fr-btn:active {
        background: #ddd;
    }

    .fr-danger {
        background: linear-gradient(to bottom, #fff0e6, #ffddcc);
        border-color: #e65c00;
    }

    .fr-danger:hover {
        background: linear-gradient(to bottom, #ffe6d9, #ffcc99);
    }

    .fr-options {
        font-size: 12px;
    }

    .fr-find-actions {
        width: 102px;
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
        border-color: #ccc;
        border-radius: 4px;
        background: #fff;
        color: #333;
        box-shadow: none;
        font-size: 12px;
        font-weight: 600;
    }

    .fr-options :global(.fr-custom-select.open .custom-select-trigger),
    .fr-options :global(.fr-custom-select .custom-select-trigger:focus-visible) {
        border-color: #2196f3;
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu) {
        top: auto;
        bottom: calc(100% + 4px);
        border-color: #ccc;
        border-radius: 4px;
        background: #fff;
        box-shadow: 0 4px 12px rgba(15, 23, 42, 0.14);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu button) {
        min-height: 28px;
        padding: 5px 8px;
        border-radius: 3px;
        font-size: 12px;
        font-weight: 600;
    }

    .fr-checkbox {
        display: flex;
        align-items: center;
        gap: 4px;
        cursor: pointer;
        color: #555;
    }

    .fr-checkbox input {
        margin: 0;
        cursor: pointer;
    }

    .fr-message {
        margin-left: auto;
        color: #2196f3;
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
        border: 1px solid #ccc;
        border-left: none;
        border-top-right-radius: 4px;
        border-bottom-right-radius: 4px;
        background: linear-gradient(to bottom, #fff, #e8e8e8);
        cursor: pointer;
        font-size: 10px;
        color: #666;
    }

    .fr-dropdown-btn:hover {
        background: linear-gradient(to bottom, #f5f5f5, #ddd);
    }

    .fr-history-dropdown {
        position: absolute;
        bottom: 100%;
        left: 0;
        right: 0;
        margin-bottom: 2px;
        background: #fff;
        border: 1px solid #ccc;
        border-radius: 4px;
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
        z-index: 200;
        max-height: 400px;
        overflow-y: auto;
    }

    .fr-history-item {
        display: flex;
        align-items: center;
        padding: 6px 10px;
        border-bottom: 1px solid #eee;
    }

    .fr-history-item:last-child {
        border-bottom: none;
    }

    .fr-history-item:hover {
        background: #f5f5f5;
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
        color: #999;
        padding: 2px 6px;
        border-radius: 3px;
        margin-left: 8px;
    }

    .fr-history-del:hover {
        background: #ffdddd;
        color: #d32f2f;
    }

    /* Modern UI overrides */
    .find-replace-panel {
        background: rgba(255, 255, 255, 0.86);
        border-top: 1px solid var(--color-border);
        padding: 9px 12px;
        box-shadow: 0 -10px 26px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(14px);
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
        background: rgba(255, 255, 255, 0.92);
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
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
    }

    .fr-btn {
        border-color: var(--color-border);
        border-radius: var(--radius-xs);
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
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
        background: #ffe7eb;
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
