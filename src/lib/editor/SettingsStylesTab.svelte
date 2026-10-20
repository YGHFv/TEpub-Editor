<script lang="ts" module>
    export type StyleTemplateInfo = {
        id: string;
        name: string;
        file_name: string;
        path: string;
        is_builtin: boolean;
    };
    export type CssPropertyOption = { label: string; value: string };
    export type CssPropertyItem = {
        label: string;
        name: string;
        value: string;
        options?: CssPropertyOption[];
        color?: boolean;
        hiddenInBlockEditor?: boolean;
    };
    export type StyleBlock = {
        id: string;
        title: string;
        selector: string;
        note: string;
        accent: string;
        properties: CssPropertyItem[];
        hiddenInBlockEditor?: boolean;
    };
</script>

<script lang="ts">
    import CustomSelect from "$lib/CustomSelect.svelte";

    let {
        message,
        templates,
        selectedTemplateId,
        currentTemplateName,
        isImporting,
        isSaving,
        showSourceEditor,
        sourceDraft,
        blocks,
        activeBlockId = $bindable(),
        fontFamilyOptions,
        parseColor,
        onImport,
        onToggleSourceEditor,
        onReset,
        onSave,
        onSelectTemplate,
        onRestoreBuiltin,
        onSourceInput,
        onUpdateValue,
        onUpdateColor,
    }: {
        message: string;
        templates: StyleTemplateInfo[];
        selectedTemplateId: string;
        currentTemplateName: string;
        isImporting: boolean;
        isSaving: boolean;
        showSourceEditor: boolean;
        sourceDraft: string;
        blocks: StyleBlock[];
        activeBlockId: string;
        fontFamilyOptions: CssPropertyOption[];
        parseColor: (value: string) => { hex: string; swatch: string };
        onImport: () => void;
        onToggleSourceEditor: () => void;
        onReset: () => void;
        onSave: () => void;
        onSelectTemplate: (template: StyleTemplateInfo) => void;
        onRestoreBuiltin: () => void;
        onSourceInput: (value: string) => void;
        onUpdateValue: (
            blockId: string,
            propName: string,
            value: string,
        ) => void;
        onUpdateColor: (
            blockId: string,
            propName: string,
            value: string,
        ) => void;
    } = $props();

    const visibleBlocks = $derived(
        blocks
            .filter((block) => !block.hiddenInBlockEditor)
            .map((block) => ({
                ...block,
                properties: block.properties.filter(
                    (prop) => !prop.hiddenInBlockEditor,
                ),
            })),
    );
    const activeBlock = $derived(
        visibleBlocks.find((block) => block.id === activeBlockId) ||
            visibleBlocks[0],
    );
    $effect(() => {
        if (
            visibleBlocks.length > 0 &&
            !visibleBlocks.some((block) => block.id === activeBlockId)
        ) {
            activeBlockId = visibleBlocks[0].id;
        }
    });
</script>

<div class="font-settings-panel style-settings-panel">
    <div class="font-settings-head">
        <div>
            <div class="font-settings-title">样式模板</div>
            <div class="font-settings-note">
                选择默认 CSS 模板，并直接编辑当前 EPUB 输出样式。
            </div>
        </div>
        <div class="style-settings-actions">
            <button
                class="mini-action style-toolbar-btn style-toolbar-btn-primary"
                disabled={isImporting}
                onclick={onImport}
            >
                {isImporting ? "导入中..." : "导入 CSS"}
            </button>
            <button
                class="mini-action style-toolbar-btn"
                onclick={onToggleSourceEditor}
                >{showSourceEditor ? "块编辑" : "源码"}</button
            >
            <button class="mini-action style-toolbar-btn" onclick={onReset}
                >重置</button
            >
            <button
                class="mini-action primary style-toolbar-btn"
                disabled={isSaving}
                onclick={onSave}>{isSaving ? "保存中..." : "保存模板"}</button
            >
        </div>
    </div>
    {#if message}
        <div class="font-settings-status">{message}</div>
    {/if}
    <div class="style-template-list">
        {#each templates as template}
            <label
                class="style-template-item {selectedTemplateId === template.id
                    ? 'active'
                    : ''}"
            >
                <div class="style-template-main">
                    <input
                        type="radio"
                        name="editor-style-template"
                        checked={selectedTemplateId === template.id}
                        onchange={() => onSelectTemplate(template)}
                    />
                    <div class="style-template-meta">
                        <strong>{template.name}</strong>
                        <span
                            >{template.is_builtin
                                ? "内置模板（始终置顶）"
                                : template.file_name}</span
                        >
                    </div>
                </div>
                {#if template.is_builtin}
                    <button
                        class="mini-action"
                        type="button"
                        onclick={(event) => {
                            event.stopPropagation();
                            onRestoreBuiltin();
                        }}>恢复默认样式</button
                    >
                {/if}
            </label>
        {/each}
    </div>
    <div class="style-settings-editor">
        <div class="style-panel-header in-settings">
            <div>
                <div class="style-panel-title">样式编辑</div>
                <div class="style-panel-subtitle">
                    当前模板：{currentTemplateName}
                </div>
            </div>
        </div>
        {#if showSourceEditor}
            <div class="style-source-editor">
                <textarea
                    spellcheck="false"
                    value={sourceDraft}
                    oninput={(event) =>
                        onSourceInput(event.currentTarget.value)}></textarea>
            </div>
        {:else}
            <div class="style-block-editor-layout">
                <nav class="style-block-nav" aria-label="样式块">
                    {#each visibleBlocks as block}
                        <button
                            type="button"
                            class:active={activeBlockId === block.id}
                            style={`--block-accent:${block.accent}`}
                            onclick={() => (activeBlockId = block.id)}
                        >
                            <strong>{block.title}</strong>
                            <span>{block.note}</span>
                        </button>
                    {/each}
                </nav>
                <div class="style-block-detail">
                    {#if activeBlock}
                        <section
                            class="style-block-card"
                            style={`--block-accent:${activeBlock.accent}`}
                        >
                            <header class="style-block-head">
                                <div>
                                    <strong>{activeBlock.title}</strong>
                                    <span>{activeBlock.note}</span>
                                </div>
                                <code>
                                    {#each activeBlock.selector
                                        .split(",")
                                        .map((item) => item.trim())
                                        .filter(Boolean) as selector}
                                        <span>{selector}</span>
                                    {/each}
                                </code>
                            </header>
                            <div class="style-prop-list">
                                {#each activeBlock.properties as prop}
                                    <label class="style-prop-row">
                                        <span class="prop-title">
                                            <span class="prop-label"
                                                >{prop.label}</span
                                            >
                                            <span class="prop-name"
                                                >{prop.name}</span
                                            >
                                        </span>
                                        {#if prop.options}
                                            <CustomSelect
                                                value={prop.value}
                                                options={(prop.name ===
                                                "font-family"
                                                    ? fontFamilyOptions
                                                    : prop.options
                                                ).map((option) => ({
                                                    value: option.value,
                                                    label: option.label,
                                                }))}
                                                ariaLabel={prop.label}
                                                on:change={(event) =>
                                                    onUpdateValue(
                                                        activeBlock.id,
                                                        prop.name,
                                                        event.detail,
                                                    )}
                                            />
                                        {:else if prop.color}
                                            {@const parsedColor = parseColor(
                                                prop.value,
                                            )}
                                            <span class="color-value-control">
                                                <input
                                                    class="color-swatch"
                                                    type="color"
                                                    value={parsedColor.swatch}
                                                    title={prop.value}
                                                    oninput={(event) =>
                                                        onUpdateColor(
                                                            activeBlock.id,
                                                            prop.name,
                                                            event.currentTarget
                                                                .value,
                                                        )}
                                                />
                                                <input
                                                    class="color-hex-input"
                                                    value={parsedColor.hex}
                                                    placeholder="#RRGGBB / #RRGGBBAA"
                                                    oninput={(event) =>
                                                        onUpdateColor(
                                                            activeBlock.id,
                                                            prop.name,
                                                            event.currentTarget
                                                                .value,
                                                        )}
                                                />
                                            </span>
                                        {:else}
                                            <input
                                                value={prop.value}
                                                oninput={(event) =>
                                                    onUpdateValue(
                                                        activeBlock.id,
                                                        prop.name,
                                                        event.currentTarget
                                                            .value,
                                                    )}
                                            />
                                        {/if}
                                    </label>
                                {/each}
                            </div>
                        </section>
                    {/if}
                </div>
            </div>
        {/if}
    </div>
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

    .style-panel-header {
        min-height: 58px;
        padding: 8px 10px;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-raised);
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        box-sizing: border-box;
    }

    .style-panel-title {
        color: var(--color-text);
        font-size: 16px;
        font-weight: 900;
        line-height: 1.3;
    }

    .style-panel-subtitle {
        color: var(--color-muted);
        font-size: 11px;
        line-height: 1.4;
        margin-top: 2px;
    }

    .mini-action {
        height: var(--control-height);
        min-width: 0;
        padding: 0 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        background: var(--color-surface);
        color: var(--color-text-soft);
        font-size: 12px;
        font-weight: 700;
        box-shadow: var(--shadow-xs);
    }

    .mini-action.primary {
        border-color: var(--color-accent);
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
    }

    .style-block-editor-layout {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-columns: 178px minmax(0, 1fr);
        gap: 0;
        overflow: hidden;
    }

    .style-block-nav {
        min-height: 0;
        overflow-y: auto;
        padding: 12px;
        border-right: 1px solid var(--color-border);
        /* 用 token 而不是固定的近白色，深色主题下才不会整条侧栏发白 */
        background: var(--color-surface-soft);
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .style-block-nav button {
        width: 100%;
        height: auto;
        min-width: 0;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 3px;
        padding: 10px 11px;
        border: 1px solid transparent;
        border-radius: var(--radius-sm);
        background: transparent;
        color: var(--color-text-soft);
        text-align: left;
        box-shadow: none;
        transform: none;
    }

    .style-block-nav button.active {
        /* 描边同样跟着块强调色走，但压到文字色里，避免深色主题下固定蓝边突兀 */
        border-color: color-mix(
            in srgb,
            var(--block-accent) 34%,
            var(--color-border)
        );
        background: var(--color-surface);
        /* 强调色由各块自带（部分是很暗的棕/绿），和当前主题文字色混合后
           浅色下足够深、深色下足够亮，两种主题都能读清 */
        color: color-mix(in srgb, var(--block-accent) 78%, var(--color-text));
        box-shadow: var(--shadow-sm);
    }

    .style-block-nav strong {
        width: 100%;
        font-size: 13px;
        line-height: 1.3;
        text-align: left;
    }

    .style-block-nav span {
        width: 100%;
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--color-muted);
        font-size: 10px;
        line-height: 1.25;
        text-align: left;
    }

    .style-block-detail {
        min-width: 0;
        min-height: 0;
        overflow-y: auto;
        padding: 14px;
    }

    .style-block-card {
        overflow: hidden;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        /* 原先写死的浅紫白，深色主题下会变成一块亮色卡片 */
        background: var(--color-surface);
        box-shadow: var(--shadow-xs);
        min-height: max-content;
    }

    .style-block-head {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        padding: 14px 16px;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-raised);
    }

    .style-block-head strong {
        display: block;
        /* 同 nav.active：块强调色混入主题文字色，保证深色主题下暗色 accent 不至于看不清 */
        color: color-mix(in srgb, var(--block-accent) 78%, var(--color-text));
        font-family: Georgia, "Times New Roman", serif;
        font-size: 0.98rem;
        letter-spacing: 0;
        line-height: 1.25;
    }

    .style-block-head span {
        display: block;
        margin-top: 2px;
        color: var(--color-muted);
        font-size: 10px;
        line-height: 1.35;
    }

    .style-block-head code {
        padding: 5px 8px;
        border-radius: var(--radius-md);
        /* 原值 #a6781d 正是第一个块的 accent，说明底色本意是「强调色的淡染」，
           因此按 accent 混合透明色，而不是固定成某个 token 的色调 */
        background: color-mix(in srgb, var(--block-accent) 12%, transparent);
        color: color-mix(in srgb, var(--block-accent) 78%, var(--color-text));
        font-family: Consolas, monospace;
        font-size: 10px;
        display: grid;
        gap: 4px;
        line-height: 1.45;
        text-align: right;
        max-width: 180px;
    }

    .style-block-head code span {
        display: block;
        margin-top: 0;
        white-space: normal;
        word-break: break-all;
    }

    .style-prop-list {
        display: grid;
        padding: 10px 16px 16px;
    }

    .style-prop-row {
        display: grid;
        grid-template-columns: minmax(112px, 150px) minmax(0, 1fr);
        align-items: center;
        gap: 14px;
        min-height: 50px;
        border-bottom: 1px solid var(--color-border);
    }

    .style-prop-row:last-child {
        border-bottom: 0;
    }

    .prop-title {
        display: grid;
        gap: 2px;
        min-width: 0;
    }

    .prop-label {
        color: var(--color-muted);
        font-size: 10px;
        white-space: nowrap;
        line-height: 1.2;
    }

    .prop-name {
        color: var(--color-text);
        font-family: Georgia, "Times New Roman", serif;
        font-size: 0.92rem;
        font-weight: 800;
        letter-spacing: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        line-height: 1.2;
    }

    .style-prop-row input {
        width: 100%;
        min-width: 0;
        max-width: 100%;
        box-sizing: border-box;
        border: 1px solid transparent;
        border-radius: var(--radius-xs);
        background: var(--color-surface-raised);
        color: var(--color-text);
        font: inherit;
        font-size: 0.8rem;
        padding: 7px 10px;
    }

    .style-prop-row input:focus {
        outline: none;
        border-color: var(--color-accent);
        /* 固定 alpha 的蓝环在深色主题下偏亮，统一走主题 focus-ring */
        box-shadow: var(--focus-ring);
        background: var(--color-surface);
    }

    .color-value-control {
        display: grid;
        grid-template-columns: 28px minmax(0, 1fr);
        grid-template-areas: "swatch text";
        align-items: center;
        gap: 6px;
        min-width: 0;
        width: 100%;
        max-width: 100%;
        box-sizing: border-box;
    }

    .style-prop-row input.color-swatch {
        grid-area: swatch;
        width: 28px;
        height: 28px;
        padding: 2px;
        border-color: var(--color-border);
        background: var(--color-surface);
        cursor: pointer;
    }

    .style-prop-row input.color-hex-input {
        grid-area: text;
        font-family: Consolas, "SFMono-Regular", monospace;
        text-transform: uppercase;
    }

    .style-source-editor {
        flex: 1;
        min-height: 0;
        padding: 14px;
        display: flex;
    }

    .font-settings-panel {
        display: grid;
        gap: 12px;
        min-height: 0;
    }

    .style-template-list {
        display: grid;
        gap: 10px;
    }

    .style-template-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 10px 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
    }

    .style-template-item.active {
        border-color: var(--color-accent);
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.1);
    }

    .style-template-main {
        display: flex;
        align-items: center;
        gap: 10px;
        min-width: 0;
        flex: 1;
    }

    .style-template-main input[type="radio"] {
        width: auto;
        flex-shrink: 0;
    }

    .style-template-meta {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .style-template-meta strong {
        font-size: 13px;
        color: var(--color-text);
    }

    .style-template-meta span {
        font-size: 11px;
        color: var(--color-muted);
        word-break: break-all;
    }

    .font-settings-head {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 12px;
    }

    .font-settings-title {
        font-size: 14px;
        font-weight: 800;
        color: var(--color-text);
    }

    .font-settings-note {
        margin-top: 4px;
        font-size: 12px;
        line-height: 1.5;
        color: var(--color-muted);
    }

    .font-settings-status {
        padding: 8px 10px;
        border-radius: var(--radius-sm);
        background: var(--color-surface-soft);
        color: var(--color-text-soft);
        font-size: 12px;
        line-height: 1.5;
    }

    .style-settings-panel {
        gap: 16px;
    }

    .style-settings-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 8px;
    }

    .style-toolbar-btn {
        min-width: 88px;
        justify-content: center;
    }

    .style-toolbar-btn-primary {
        border-color: var(--color-accent);
        background: linear-gradient(
            135deg,
            var(--color-accent) 0%,
            var(--color-teal) 100%
        );
        color: var(--color-surface);
        box-shadow: none;
    }

    .style-toolbar-btn-primary:disabled {
        opacity: 0.72;
        box-shadow: none;
    }

    .style-settings-editor {
        min-height: 360px;
        overflow: hidden;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        background: var(--color-surface-soft);
        display: flex;
        flex-direction: column;
    }

    .style-panel-header.in-settings {
        min-height: 50px;
    }

    .style-source-editor textarea {
        flex: 1;
        width: 100%;
        min-height: 0;
        resize: none;
        box-sizing: border-box;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        font-family: Consolas, "SFMono-Regular", monospace;
        font-size: 12px;
        line-height: 1.55;
        padding: 12px;
        outline: none;
        box-shadow: inset 0 1px 3px rgba(23, 36, 52, 0.05);
    }

    .style-source-editor textarea:focus {
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
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

    .style-prop-row :global(.custom-select) {
        flex: 1 1 auto;
        min-width: 0;
    }

    .style-prop-row :global(.custom-select-trigger) {
        min-height: var(--control-height);
        background: var(--color-surface-raised);
        font-size: 0.8rem;
        font-weight: 600;
    }
</style>
