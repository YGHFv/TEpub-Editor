<script lang="ts">
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
        <button class="btn-primary" onclick={onSelectFile}>📂</button>
        <button
            class={isModified ? "btn-save-modified" : "btn-save-default"}
            onclick={onSaveFile}>💾</button
        >
        <button class="btn-secondary" onclick={onUndo}>↩️</button>
        <button class="btn-secondary" onclick={onRedo}>↪️</button>
        <button class="btn-secondary" onclick={onToggleSidebar}>📖</button>
        <button class="btn-secondary" onclick={onOpenEpubModal}>📚</button>
    </div>
    <div class="toolbar-tail">
        <button
            class="btn-secondary"
            title="查找与替换 (Ctrl+F)"
            onclick={onOpenSearch}>🔍</button
        >
        <button
            class="btn-secondary proof-tool-btn"
            title="校对"
            aria-label="校对"
            onclick={onToggleProof}
        >
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M5 4.5h10.2a2.3 2.3 0 0 1 2.3 2.3v3.45" />
                <path d="M5 8.5h8.5" />
                <path d="M5 12.5h6.2" />
                <path d="M5 16.5h4.1" />
                <path d="m14.2 16.7 2.05 2.05L20.8 14.2" />
                <path d="M4.5 3.5h11.2a3.8 3.8 0 0 1 3.8 3.8v2.2" />
            </svg>
        </button>
        <button class="btn-secondary" title="偏好设置" onclick={onOpenSettings}
            >⚙️</button
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
        font-size: 18px;
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

    .proof-tool-btn svg {
        width: 20px;
        height: 20px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1.9;
        stroke-linecap: round;
        stroke-linejoin: round;
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
