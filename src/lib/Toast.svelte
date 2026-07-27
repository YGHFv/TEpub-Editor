<script lang="ts">
    import { toasts, dismissToast } from "./toast";
    import { fly } from "svelte/transition";
</script>

{#if $toasts.length > 0}
    <div class="toast-container">
        {#each $toasts as toast (toast.id)}
            <div
                class="toast toast-{toast.type}"
                role={toast.type === "error" ? "alert" : "status"}
                transition:fly={{ y: -12, duration: 160 }}
            >
                <span class="toast-message">{toast.message}</span>
                <button
                    class="toast-close"
                    aria-label="关闭提示"
                    on:click={() => dismissToast(toast.id)}>×</button
                >
            </div>
        {/each}
    </div>
{/if}

<style>
    .toast-container {
        position: fixed;
        top: 16px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 10000;
        display: flex;
        flex-direction: column;
        gap: 8px;
        max-width: min(480px, calc(100vw - 32px));
        pointer-events: none;
    }

    .toast {
        display: flex;
        align-items: flex-start;
        gap: 8px;
        padding: 10px 12px;
        border-radius: var(--radius-md, 8px);
        background: var(--color-surface-raised, #2b2b2b);
        color: var(--color-text, #eee);
        box-shadow: var(--shadow-pop, 0 4px 16px rgba(0, 0, 0, 0.35));
        font-family: var(--font-ui, sans-serif);
        font-size: 13px;
        line-height: 1.5;
        border-left: 3px solid var(--color-border, #555);
        pointer-events: auto;
    }

    .toast-success {
        border-left-color: #34a853;
    }

    .toast-error {
        border-left-color: #ea4335;
    }

    .toast-message {
        flex: 1;
        white-space: pre-wrap;
        word-break: break-word;
    }

    .toast-close {
        flex-shrink: 0;
        border: none;
        background: transparent;
        color: var(--color-text-soft, #aaa);
        font-size: 16px;
        line-height: 1;
        cursor: pointer;
        padding: 0 2px;
    }

    .toast-close:hover {
        color: var(--color-text, #eee);
    }
</style>
