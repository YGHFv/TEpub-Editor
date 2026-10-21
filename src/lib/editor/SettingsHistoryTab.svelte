<script lang="ts" module>
    export interface HistoryMeta {
        filename: string;
        path: string;
        timestamp: number;
        size: number;
    }
</script>

<script lang="ts">
    let {
        history,
        onRestore,
    }: {
        history: HistoryMeta[];
        onRestore: (snapshot: HistoryMeta) => void;
    } = $props();
</script>

<div class="history-settings-card">
    {#each history as h}
        <button class="hist-item" onclick={() => onRestore(h)}>
            <span class="hist-time"
                >{new Date(h.timestamp * 1000).toLocaleString()}</span
            >
            <span class="hist-size">{(h.size / 1024).toFixed(1)}KB</span>
        </button>
    {:else}
        <div class="empty-msg">暂无历史快照</div>
    {/each}
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

    .history-settings-card {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .hist-item {
        width: 100%;
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        column-gap: 24px;
        text-align: left;
    }

    .hist-time {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .hist-size {
        min-width: 72px;
        justify-self: end;
        text-align: right;
        color: var(--color-muted);
        font-family: monospace;
        font-size: 12px;
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
