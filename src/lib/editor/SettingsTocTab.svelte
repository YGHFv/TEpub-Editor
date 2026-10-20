<script lang="ts" module>
    export interface CustomRegexRule {
        level: number;
        pattern: string;
    }
</script>

<script lang="ts">
    import CustomSelect from "$lib/CustomSelect.svelte";

    let {
        rules,
        presetPatterns,
        prefixLevel = $bindable(),
        prefixText = $bindable(),
        onAddPrefix,
        onUpdateRule,
        onRemoveRule,
        onResetRules,
        onAddRule,
    }: {
        rules: CustomRegexRule[];
        presetPatterns: string[];
        prefixLevel: number;
        prefixText: string;
        onAddPrefix: (level: number, prefix: string) => void;
        onUpdateRule: (index: number, patch: Partial<CustomRegexRule>) => void;
        onRemoveRule: (index: number) => void;
        onResetRules: () => void;
        onAddRule: () => void;
    } = $props();
</script>

<div class="rules-header">正则表达式</div>
<div class="rule-helper-row">
    <CustomSelect
        className="rule-type rule-type-helper"
        value={String(prefixLevel)}
        options={[1, 2, 3, 4, 5].map((level) => ({
            value: String(level),
            label: `层级 ${level}`,
        }))}
        ariaLabel="前缀层级"
        on:change={(event) => (prefixLevel = Number(event.detail))}
    />
    <input
        class="rule-prefix-input"
        bind:value={prefixText}
        placeholder="前缀识别，例如：番外、后日谈"
    />
    <button
        type="button"
        class="rule-btn add"
        onclick={() => {
            const prefix = prefixText.trim();
            if (!prefix) return;
            onAddPrefix(prefixLevel, prefix);
            prefixText = "";
            prefixLevel = 3;
        }}>＋ 前缀</button
    >
</div>
<div class="rules-list">
    {#each rules as rule, idx}
        <div class="rule-item" style="gap: 8px; align-items: center;">
            <CustomSelect
                className="rule-type"
                value={String(rule.level)}
                options={[1, 2, 3, 4, 5].map((level) => ({
                    value: String(level),
                    label: `层级 ${level}`,
                }))}
                ariaLabel="目录层级"
                on:change={(event) =>
                    onUpdateRule(idx, { level: Number(event.detail) })}
            />

            <div class="rule-input-group">
                <input
                    class="rule-input"
                    type="text"
                    value={rule.pattern}
                    oninput={(event) =>
                        onUpdateRule(idx, {
                            pattern: event.currentTarget.value,
                        })}
                    placeholder="输入正则表达式"
                />
                <CustomSelect
                    className="rule-preset-select"
                    value=""
                    placeholder="预设"
                    options={presetPatterns.map((pattern) => ({
                        value: pattern,
                        label: pattern,
                    }))}
                    ariaLabel="选择预设正则"
                    on:change={(event) => {
                        if (event.detail)
                            onUpdateRule(idx, { pattern: event.detail });
                    }}
                />
            </div>

            <button class="rule-btn remove" onclick={() => onRemoveRule(idx)}
                >－</button
            >
        </div>
    {/each}
</div>
<div
    style="display: flex; justify-content: space-between; align-items: center; margin-top: 10px;"
>
    <button
        class="grid-btn"
        style="padding: 4px 10px; font-size: 13px;"
        onclick={onResetRules}>↺ 还原正则</button
    >
    <button
        class="grid-btn"
        style="padding: 4px 10px; font-size: 13px; color: #0066b8; border-color: #0066b8;"
        onclick={onAddRule}>＋ 新增正则</button
    >
</div>

<style>
    /* 基础按钮样式（与页面同源，保证拆出后视觉一致） */
    button {
        height: var(--control-height);
        min-width: 40px;
        border-radius: var(--radius-xs);
        border: 1px solid var(--color-border-strong);
        background: var(--color-surface);
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: var(--color-border);
        transform: translateY(0.5px) scale(0.985);
    }

    .rules-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .rule-item {
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .rule-input-group {
        display: flex;
        flex: 1;
        align-items: center;
        position: relative;
    }
    .rule-input {
        flex: 1;
        height: 32px;
        padding: 0 10px;
        border: 1px solid var(--color-border-strong);
        border-right: none;
        border-radius: 6px 0 0 6px;
        font-size: 13px;
        background: var(--color-surface);
        min-width: 0;
        z-index: 1;
    }
    .rule-btn {
        width: 32px;
        height: 32px;
        min-width: 32px;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: 0.1s;
    }
    .rule-btn:hover {
        background: var(--color-danger-soft);
        color: var(--color-danger);
        border-color: color-mix(in srgb, var(--color-danger-soft) 92%, var(--color-danger));
    }
    .rule-btn.add:hover {
        background: var(--color-accent-quiet);
        color: var(--color-accent);
        border-color: var(--color-accent-soft);
    }
    .rules-header {
        font-size: 13px;
        font-weight: bold;
        color: var(--color-muted);
        margin-bottom: 5px;
    }
    .rule-helper-row {
        display: flex;
        gap: 8px;
        align-items: center;
        margin-bottom: 8px;
    }
    .rule-prefix-input {
        flex: 1;
        min-width: 0;
        height: 32px;
        padding: 0 10px;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        font-size: 13px;
        background: var(--color-surface);
    }

    /* Modern UI overrides */
    button {
        height: var(--control-height);
        min-width: 38px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: linear-gradient(180deg, var(--color-surface), var(--color-surface-soft));
        color: var(--color-text-soft);
        box-shadow: var(--shadow-xs);
        cursor: pointer;
        transition:
            transform var(--transition-fast),
            border-color var(--transition-fast),
            background var(--transition-fast),
            box-shadow var(--transition-fast),
            color var(--transition-fast);
    }

    button:hover:not(:disabled) {
        border-color: var(--color-border-strong);
        background: var(--color-hover);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    button:active:not(:disabled) {
        background: var(--color-active);
        transform: translateY(1px) scale(0.98);
        box-shadow: var(--shadow-xs);
    }

    button:disabled {
        opacity: 0.52;
        box-shadow: none;
    }

    .rule-btn {
        border: 1px solid var(--color-border);
        background: var(--color-surface-raised);
        color: var(--color-text-soft);
        border-radius: var(--radius-xs);
        box-shadow: none;
    }

    .rules-header {
        color: var(--color-text-soft);
    }

    .rule-input {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast),
            background var(--transition-fast);
    }

    .rule-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: var(--color-surface);
    }
    .rule-prefix-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: var(--color-surface);
    }

    .rule-btn:hover {
        background: var(--color-danger-soft);
        border-color: rgba(215, 68, 82, 0.3);
        color: var(--color-danger);
    }

    .rule-item :global(.rule-type),
    .rule-helper-row :global(.rule-type-helper) {
        width: 112px;
        flex: 0 0 112px;
    }

    .rule-item :global(.rule-type .custom-select-trigger),
    .rule-helper-row :global(.rule-type-helper .custom-select-trigger) {
        min-height: 32px;
        padding-block: 5px;
        font-weight: 600;
    }

    .rule-input-group :global(.rule-preset-select) {
        width: 112px;
        flex: 0 0 112px;
    }

    .rule-input-group :global(.rule-preset-select .custom-select-trigger) {
        min-height: 32px;
        border-radius: 0 6px 6px 0;
        font-size: 12px;
        font-weight: 600;
    }
</style>
