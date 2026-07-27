<script lang="ts" module>
    export interface ProofLogInfo {
        fileName: string;
        path: string;
        timestamp: number;
        size: number;
    }
</script>

<script lang="ts">
    let {
        logs,
        message,
        selectedPath,
        selectedContent,
        onRefresh,
        onOpenLog,
    }: {
        logs: ProofLogInfo[];
        message: string;
        selectedPath: string;
        selectedContent: string;
        onRefresh: () => void;
        onOpenLog: (log: ProofLogInfo) => void;
    } = $props();
</script>

<div class="proof-log-settings">
    <div class="proof-log-toolbar">
        <span>{message || `共 ${logs.length} 个日志文件`}</span>
        <button class="mini-action" type="button" onclick={onRefresh}>刷新</button>
    </div>
    <div class="proof-log-browser">
        <div class="proof-log-list">
            {#each logs as log}
                <button
                    type="button"
                    class:active={selectedPath === log.path}
                    onclick={() => onOpenLog(log)}
                >
                    <span class="proof-log-name">{log.fileName}</span>
                    <span class="proof-log-meta">
                        {new Date(log.timestamp * 1000).toLocaleString()} · {(log.size / 1024).toFixed(1)}KB
                    </span>
                </button>
            {:else}
                <div class="empty-msg">暂无校对日志</div>
            {/each}
        </div>
        <textarea
            class="proof-log-content"
            readonly
            value={selectedContent || "选择左侧日志后查看完整内容"}
            data-native-context-menu="true"
            oncontextmenu={(event) => event.stopPropagation()}
        ></textarea>
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

    .proof-log-settings {
        display: flex;
        flex-direction: column;
        gap: 10px;
        min-height: 0;
        height: 100%;
    }

    .proof-log-toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        color: #52606d;
        font-size: 12px;
    }

    .proof-log-browser {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(180px, 0.9fr) minmax(0, 1.35fr);
        gap: 10px;
    }

    .proof-log-list {
        min-height: 0;
        overflow: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-right: 2px;
    }

    .proof-log-list button {
        width: 100%;
        min-height: 54px;
        display: grid;
        gap: 4px;
        padding: 9px 10px;
        border: 1px solid #e0e6ed;
        border-radius: 8px;
        background: #fff;
        text-align: left;
    }

    .proof-log-list button.active {
        border-color: #1677b8;
        background: #eef8ff;
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.1);
    }

    .proof-log-name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #1b2d3f;
        font-size: 12px;
        font-weight: 800;
    }

    .proof-log-meta {
        color: #748295;
        font-size: 11px;
    }

    .proof-log-content {
        width: 100%;
        min-width: 0;
        min-height: 0;
        resize: none;
        padding: 10px;
        border: 1px solid #d6e0eb;
        border-radius: 8px;
        background: #fbfcfe;
        color: #334155;
        font-family: Consolas, "Microsoft YaHei", monospace;
        font-size: 11px;
        line-height: 1.55;
        box-sizing: border-box;
    }

    @media (max-width: 760px) {
        .proof-log-browser {
            grid-template-columns: 1fr;
        }
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
