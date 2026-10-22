<script lang="ts">
    import Icon from "$lib/icons/Icon.svelte";
    import { t } from "$lib/i18n.svelte";

    let {
        isModified,
        onSelectFile,
        onSaveFile,
        onUndo,
        onRedo,
        onToggleSidebar,
        onOpenEpubModal,
        onOpenSearch,
        onToggleProof,
        onOpenSettings,
    }: {
        isModified: boolean;
        onSelectFile: () => void;
        onSaveFile: () => void;
        onUndo: () => void;
        onRedo: () => void;
        onToggleSidebar: () => void;
        onOpenEpubModal: () => void;
        onOpenSearch: () => void;
        onToggleProof: () => void;
        onOpenSettings: () => void;
    } = $props();
</script>

<header class="toolbar">
    <div class="btn-group">
        <button
            class="btn-primary"
            title={t("打开文件")}
            aria-label={t("打开文件")}
            onclick={onSelectFile}
        >
            <Icon name="folder-open" />
        </button>
        <button
            class={isModified ? "btn-save-modified" : "btn-save-default"}
            title={t("保存 (Ctrl+S)")}
            aria-label={t("保存")}
            onclick={onSaveFile}><Icon name="save" /></button
        >
        <button
            class="btn-secondary"
            title={t("撤销 (Ctrl+Z)")}
            aria-label={t("撤销")}
            onclick={onUndo}><Icon name="undo" /></button
        >
        <button
            class="btn-secondary"
            title={t("重做 (Ctrl+Y)")}
            aria-label={t("重做")}
            onclick={onRedo}><Icon name="redo" /></button
        >
        <button
            class="btn-secondary"
            title={t("章节侧栏")}
            aria-label={t("章节侧栏")}
            onclick={onToggleSidebar}><Icon name="book-open" /></button
        >
        <button
            class="btn-secondary"
            title={t("制作 EPUB")}
            aria-label={t("制作 EPUB")}
            onclick={onOpenEpubModal}><Icon name="books" /></button
        >
    </div>
    <div class="toolbar-tail">
        <button
            class="btn-secondary"
            title={t("查找与替换 (Ctrl+F)")}
            aria-label={t("查找与替换")}
            onclick={onOpenSearch}><Icon name="search" /></button
        >
        <button
            class="btn-secondary proof-tool-btn"
            title={t("校对")}
            aria-label={t("校对")}
            onclick={onToggleProof}
        >
            <Icon name="proof" />
        </button>
        <button
            class="btn-secondary"
            title={t("偏好设置")}
            aria-label={t("偏好设置")}
            onclick={onOpenSettings}><Icon name="settings" /></button
        >
    </div>
</header>

<style>
    /* 基础按钮样式（与页面同源，保证拆出后视觉一致） */
    button {
        height: var(--control-height);
        min-width: 40px;
        border-radius: var(--radius-xs);
        border: 1px solid var(--color-border-strong);
        background: var(--color-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active:not(:disabled) {
        background: var(--color-active);
        transform: translateY(0.5px);
    }

    .toolbar {
        padding-top: env(safe-area-inset-top);
        background: var(--color-surface-soft);
        height: 44px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-left: 10px;
        padding-right: 10px;
        border-bottom: 1px solid var(--color-border);
        z-index: 100;
    }
    .btn-group {
        display: flex;
        gap: 6px;
    }
    .toolbar-tail {
        display: flex;
        gap: 6px;
        margin-left: auto;
    }
    .btn-primary {
        background: var(--gradient-accent);
        color: var(--color-accent-contrast);
        border-color: transparent;
    }
    .btn-save-modified {
        background: var(--color-danger);
        color: var(--color-accent-contrast);
        border-color: transparent;
    }

    .proof-tool-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 0;
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

    .toolbar {
        height: 52px;
        padding: env(safe-area-inset-top) 14px 0;
        background: var(--color-surface-raised);
        border-bottom: 1px solid var(--color-border);
        box-shadow: var(--shadow-xs);
        backdrop-filter: blur(18px) saturate(1.15);
    }

    .btn-group {
        gap: 8px;
    }

    .btn-primary {
        background: var(--gradient-accent);
        border-color: transparent;
        color: var(--color-accent-contrast);
        box-shadow: none;
    }

    .btn-secondary,
    .btn-save-default {
        background: var(--color-surface-raised);
        color: var(--color-text-soft);
    }

    .btn-save-modified {
        background: var(--gradient-danger);
        border-color: transparent;
        color: var(--color-surface);
    }
</style>
