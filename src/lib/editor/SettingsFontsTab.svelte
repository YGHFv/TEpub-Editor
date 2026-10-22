<script lang="ts" module>
    export type ImportedFontInfo = {
        family: string;
        css_value: string;
        file_name: string;
        path: string;
    };
</script>

<script lang="ts">
    import { t } from "$lib/i18n.svelte";

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
    <div class="font-settings-title">{t("外部字体")}</div>
            <div class="font-settings-note">
        {t("导入后自动复制到当前书库目录下的“字体”文件夹，并可直接在块编辑里选择使用。")}
            </div>
        </div>
        <button class="grid-btn blue" disabled={isImporting} onclick={onImport}>
        {t(isImporting ? "导入中..." : "导入字体")}
        </button>
    </div>
    {#if message}
        <div class="font-settings-status">{t(message)}</div>
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
            ? t("重命名中...")
            : t("重命名")}</button
                        >
                        <button
                            class="mini-action"
                            type="button"
                            disabled={deletingFileName === font.file_name ||
                                renamingFileName === font.file_name}
                            onclick={() => onDelete(font)}
                            >{deletingFileName === font.file_name
            ? t("删除中...")
            : t("删除")}</button
                        >
                    </div>
                </div>
                <code>{font.css_value}</code>
            </div>
        {:else}
    <div class="empty-msg">{t("还没有导入外部字体")}</div>
        {/each}
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
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
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
        color: var(--color-text);
    }

    .font-settings-meta span {
        font-size: 11px;
        color: var(--color-muted);
    }

    .font-settings-item code {
        font-size: 11px;
        color: var(--color-text-soft);
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
</style>
