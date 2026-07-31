<script lang="ts" module>
    export type ImportedFontInfo = {
        family: string;
        css_value: string;
        file_name: string;
        path: string;
    };
</script>

<script lang="ts">
    let {
        fonts,
        message,
        isImporting,
        renamingFileName,
        deletingFileName,
        onImport,
        onRename,
        onDelete,
    }: {
        fonts: ImportedFontInfo[];
        message: string;
        isImporting: boolean;
        renamingFileName: string;
        deletingFileName: string;
        onImport: () => void;
        onRename: (font: ImportedFontInfo) => void;
        onDelete: (font: ImportedFontInfo) => void;
    } = $props();
</script>

<div class="font-settings-panel">
    <div class="font-settings-head">
        <div>
            <div class="font-settings-title">外部字体</div>
            <div class="font-settings-note">
                导入后自动复制到当前书库目录下的“字体”文件夹，并可直接在块编辑里选择使用。
            </div>
        </div>
        <button class="grid-btn blue" disabled={isImporting} onclick={onImport}>
            {isImporting ? "导入中..." : "导入字体"}
        </button>
    </div>
    {#if message}
        <div class="font-settings-status">{message}</div>
    {/if}
    <div class="font-settings-list">
        {#each fonts as font}
            <div class="font-settings-item">
                <div class="font-settings-item-top">
                    <div class="font-settings-meta">
                        <strong>{font.family}</strong>
                        <span>{font.file_name}</span>
                    </div>
                    <div class="font-settings-actions">
                        <button
                            class="mini-action"
                            type="button"
                            disabled={renamingFileName === font.file_name ||
                                deletingFileName === font.file_name}
                            onclick={() => onRename(font)}
                            >{renamingFileName === font.file_name
                                ? "重命名中..."
                                : "重命名"}</button
                        >
                        <button
                            class="mini-action"
                            type="button"
                            disabled={deletingFileName === font.file_name ||
                                renamingFileName === font.file_name}
                            onclick={() => onDelete(font)}
                            >{deletingFileName === font.file_name
                                ? "删除中..."
                                : "删除"}</button
                        >
                    </div>
                </div>
                <code>{font.css_value}</code>
            </div>
        {:else}
            <div class="empty-msg">还没有导入外部字体</div>
        {/each}
    </div>
</div>

<style>
    /* 基础按钮样式（与页面同源，保证拆出后视觉一致） */
    button {
        height: 34px;
        min-width: 40px;
        border-radius: 6px;
        border: 1px solid #ccc;
        background: #fff;
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: #eee;
        transform: scale(0.96);
    }

    .mini-action {
        height: 34px;
        min-width: 0;
        padding: 0 12px;
        border: 1px solid #d7e0ea;
        border-radius: 10px;
        background: #fff;
        color: #526071;
        font-size: 12px;
        font-weight: 700;
        box-shadow: 0 4px 12px rgba(15, 23, 42, 0.06);
    }

    .font-settings-panel {
        display: grid;
        gap: 12px;
        min-height: 0;
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
        color: #253547;
    }

    .font-settings-note {
        margin-top: 4px;
        font-size: 12px;
        line-height: 1.5;
        color: #708093;
    }

    .font-settings-status {
        padding: 8px 10px;
        border-radius: 8px;
        background: #f5f9fc;
        color: #486277;
        font-size: 12px;
        line-height: 1.5;
    }

    .font-settings-list {
        display: grid;
        gap: 8px;
        max-height: 320px;
        overflow: auto;
        padding-right: 2px;
    }

    .font-settings-item {
        display: grid;
        gap: 6px;
        padding: 10px 12px;
        border: 1px solid #e0e6ed;
        border-radius: 8px;
        background: #fff;
    }

    .font-settings-item-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 12px;
    }

    .font-settings-meta {
        display: grid;
        gap: 2px;
    }

    .font-settings-meta strong {
        font-size: 13px;
        color: #1b2d3f;
    }

    .font-settings-meta span {
        font-size: 11px;
        color: #7b8794;
    }

    .font-settings-item code {
        font-size: 11px;
        color: #466176;
        white-space: normal;
        word-break: break-word;
    }

    .font-settings-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 6px;
        flex-shrink: 0;
    }

    /* Modern UI overrides */
    button {
        height: var(--control-height);
        min-width: 38px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
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
</style>
