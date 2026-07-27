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
        </button
        >
        <button class="btn-secondary" title="偏好设置" onclick={onOpenSettings}
            >⚙️</button
        >
    </div>
</header>

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

    .toolbar {
        padding-top: env(safe-area-inset-top);
        background: #f3f3f3;
        height: 44px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-left: 10px;
        padding-right: 10px;
        border-bottom: 1px solid #ddd;
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
        background: #0066b8;
        color: #fff;
        border: none;
    }
    .btn-save-modified {
        background: #d32f2f;
        color: #fff;
        border: none;
        animation: pulse 2s infinite;
    }
    @keyframes pulse {
        0% {
            opacity: 1;
        }
        50% {
            opacity: 0.7;
        }
        100% {
            opacity: 1;
        }
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

    .toolbar {
        height: 52px;
        padding: env(safe-area-inset-top) 14px 0;
        background: rgba(255, 255, 255, 0.78);
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
        color: #fff;
        box-shadow: 0 10px 22px rgba(22, 119, 184, 0.2);
    }

    .btn-secondary,
    .btn-save-default {
        background: rgba(255, 255, 255, 0.84);
        color: var(--color-text-soft);
    }

    .btn-save-modified {
        background: var(--gradient-danger);
        border-color: transparent;
        color: #fff;
        animation: pulse 2s infinite;
    }
</style>
