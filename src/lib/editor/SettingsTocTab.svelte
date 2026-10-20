<script lang="ts" module>
    export interface CustomRegexRule {
        level: number;
        pattern: string;
    }
</script>

<script lang="ts">
    import { onDestroy } from "svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";

    let {
        rules,
        presetPatterns = [],
        presetOptions = undefined,
        prefixLevel = $bindable(),
        prefixText = $bindable(),
        onAddPrefix,
        onUpdateRule,
        onRemoveRule,
        onResetRules,
        onAddRule,
    }: {
        rules: CustomRegexRule[];
        presetPatterns?: string[];
        presetOptions?: { label: string; value: string }[] | undefined;
        prefixLevel: number;
        prefixText: string;
        onAddPrefix: (level: number, prefix: string) => void;
        onUpdateRule: (index: number, patch: Partial<CustomRegexRule>) => void;
        onRemoveRule: (index: number) => void;
        onResetRules: () => void;
        onAddRule: () => void;
    } = $props();

    // 预设下拉的 label 直接显示正则原文时，112px 宽的触发区里只看得到 "^\s*第[…"，
    // 完全分不清是哪条规则。这里按 pattern 里的关键字反推中文名；
    // 判断顺序必须严格：卷/终章/番外/元信息里都可能含「章」「回」，先判具体再判章节。
    function inferPresetLabel(pattern: string) {
        if (pattern.includes("卷")) return "第X卷 / 卷X";
        if (pattern.includes("终章")) return "终章 标题";
        if (pattern.includes("番外") || pattern.includes("后日谈"))
            return "番外 / 后日谈";
        if (
            pattern.includes("完本感言") ||
            pattern.includes("内容简介") ||
            pattern.includes("本书相关")
        )
            return "内容简介 / 本书相关 / 完本感言";
        // 简介/楔子那条里含 "序(?:章|言)?"，所以要排在「章」之前
        if (
            pattern.includes("楔子") ||
            pattern.includes("前言") ||
            pattern.includes("后记") ||
            pattern.includes("尾声") ||
            pattern.includes("简介")
        )
            return "简介 / 前言 / 楔子 / 后记 / 尾声";
        if (pattern.includes("序列")) return "序列 数字";
        if (pattern.includes("回")) return "第X回";
        if (pattern.includes("节")) return "第X节";
        // 「第\d+章」和「第[一二三..]章」是两条独立预设，用数字/汉字区分开
        if (pattern.includes("章"))
            return pattern.includes("\\d") ? "第X章（阿拉伯数字）" : "第X章";
        if (/chapter/i.test(pattern)) return "Chapter N";
        if (/\d/.test(pattern)) return "纯数字标题";
        // 关键字都没命中时退回截断原文，至少不比原来的整串正则更差
        return pattern.length > 30 ? `${pattern.slice(0, 30)}…` : pattern;
    }

    // 保留 presetPatterns 是为了不改调用方（editor/+page.svelte 目前只传它），
    // presetOptions 一旦提供就优先使用，便于以后从调用侧直接给可读名。
    const resolvedPresetOptions = $derived(
        presetOptions && presetOptions.length > 0
            ? presetOptions
            : presetPatterns.map((pattern) => ({
                  value: pattern,
                  label: inferPresetLabel(pattern),
              })),
    );

    let prefixHint = $state("");
    let prefixRejected = $state(false);
    let hintTimer: ReturnType<typeof setTimeout> | undefined;

    // 前缀为空时原本直接 return，用户点「＋ 前缀」毫无反馈，会以为按钮坏了。
    // 这里给 2 秒的可见提示 + 按钮标红，比 alert 打断小。
    function rejectEmptyPrefix() {
        prefixHint = "请先填写前缀，例如：番外";
        prefixRejected = true;
        if (hintTimer) clearTimeout(hintTimer);
        hintTimer = setTimeout(() => {
            prefixHint = "";
            prefixRejected = false;
        }, 2000);
    }

    function handleAddPrefix() {
        const prefix = prefixText.trim();
        if (!prefix) {
            rejectEmptyPrefix();
            return;
        }
        // 中文前缀无需额外编码：调用方会用 escapeRegExp 转义后再拼成
        // "番外[：:、] 标题" 这类正则，这里只负责把原文传下去。
        onAddPrefix(prefixLevel, prefix);
        prefixText = "";
        prefixLevel = 3;
    }

    // 组件提前关闭时清掉定时器，避免回调落到已卸载的组件上
    onDestroy(() => {
        if (hintTimer) clearTimeout(hintTimer);
    });
</script>

<div class="rules-header">正则表达式</div>
<div class="rule-helper">
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
            class:invalid={prefixRejected}
            onclick={handleAddPrefix}>＋ 前缀</button
        >
    </div>
    {#if prefixHint}
        <div class="rule-helper-hint" role="status" aria-live="polite">
            {prefixHint}
        </div>
    {/if}
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
                    options={resolvedPresetOptions}
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
        /* 见文件末尾菜单定位的说明：留一点余量，避免末行下拉紧贴下方按钮 */
        padding-bottom: 4px;
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
        border-color: color-mix(
            in srgb,
            var(--color-danger-soft) 92%,
            var(--color-danger)
        );
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
    .rule-helper {
        display: flex;
        flex-direction: column;
        gap: 4px;
        margin-bottom: 8px;
    }
    .rule-helper-row {
        display: flex;
        gap: 8px;
        align-items: center;
    }
    /* 点「＋ 前缀」但输入为空时给一次短提示，按钮同步标红，
       否则用户得不到任何反馈，会以为按钮没生效 */
    .rule-helper-hint {
        align-self: flex-end;
        color: var(--color-danger);
        font-size: 12px;
        line-height: 1.4;
    }
    .rule-btn.add.invalid {
        border-color: var(--color-danger);
        background: var(--color-danger-soft);
        color: var(--color-danger);
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
        background: linear-gradient(
            180deg,
            var(--color-surface),
            var(--color-surface-soft)
        );
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

    /* 触发区只有 112px，而菜单默认 left:0/right:0 跟着触发区同宽，
       正则原文在 112px 里只剩 "^\s*第[…" 一截，等于没法选。
       所以给菜单一个能放下可读名的宽度。对齐方式刻意用 right:0 而不是 left:0：
       右侧还有「－」按钮，若从 left 向右长 320px 会直接压住它并冲出
       .settings-shell-body 右边框；改为贴着 trigger 右缘向左展开，
       掉落区域就落在输入框上，属于下拉菜单的正常行为。 */
    .rule-input-group :global(.rule-preset-select .custom-select-menu) {
        left: auto;
        right: 0;
        min-width: 320px;
        max-width: min(70vw, 460px);
    }

    /* 选项单行省略：CustomSelect 自身已对 label 做了 ellipsis，
       这里只补 min-width:0 防止 flex 子项撑开宽度。悬停看全文需要的 title
       属性在 CustomSelect 内部，本次改动范围不含该组件，故未加。 */
    .rule-input-group
        :global(.rule-preset-select .custom-select-menu .option-label span) {
        min-width: 0;
        white-space: nowrap;
        text-overflow: ellipsis;
        overflow: hidden;
    }

    /* 纵向裁剪：app.css 里 .settings-shell-body（约 535 行）是 overflow:auto 而非
       hidden，菜单的包含块 .custom-select(position:relative) 就在这个滚动容器内，
       所以向下溢出只会让面板多出可滚动高度，不会被硬裁掉。
       因此既不用给 .rules-list 放开 overflow（它不是滚动容器，改了也没用），
       也不用把菜单改成 position:fixed（会丢掉随行定位）。列表尾部留的 4px
       见 .rules-list 的注释，这里不再重复加 margin。 */

    .rule-input-group :global(.rule-preset-select .custom-select-trigger) {
        min-height: 32px;
        border-radius: 0 6px 6px 0;
        font-size: 12px;
        font-weight: 600;
    }
</style>
