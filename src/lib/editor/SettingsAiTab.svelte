<script lang="ts">
    import { localizeAiProviderName, t } from "$lib/i18n.svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import type { AiProviderConfig } from "./SettingsApiTab.svelte";

    let {
        message,
        providers,
        proofProviderId,
        approvalProviderId,
        maxChapterChars = $bindable(),
        responseTimeoutSec = $bindable(),
        autoApprove = $bindable(),
        extraPrompt = $bindable(),
        onProofProviderChange,
        onApprovalProviderChange,
    }: {
        message: string;
        providers: AiProviderConfig[];
        proofProviderId: string;
        approvalProviderId: string;
        maxChapterChars: number;
        responseTimeoutSec: number;
        autoApprove: boolean;
        extraPrompt: string;
        onProofProviderChange: (providerId: string) => void;
        onApprovalProviderChange: (providerId: string) => void;
    } = $props();
</script>

<div class="ai-settings-form">
    {#if message}
        <div class="font-settings-status">{t(message)}</div>
    {/if}
    <div class="set-row">
        <label for="aiProofProvider">{t("校对 API:")}</label>
        <CustomSelect
            id="aiProofProvider"
            value={proofProviderId}
            options={providers.map((provider) => ({
                value: provider.id,
                label: localizeAiProviderName(provider.name) || provider.model,
            }))}
            on:change={(event) => onProofProviderChange(event.detail)}
        />
    </div>
    <div class="set-row">
        <label for="aiApprovalProvider">{t("审批 API:")}</label>
        <CustomSelect
            id="aiApprovalProvider"
            value={approvalProviderId}
            options={providers.map((provider) => ({
                value: provider.id,
                label: localizeAiProviderName(provider.name) || provider.model,
            }))}
            on:change={(event) => onApprovalProviderChange(event.detail)}
        />
    </div>
    <div class="set-row">
        <label for="aiMaxChars">{t("单章上限:")}</label>
        <input
            id="aiMaxChars"
            type="number"
            min="1000"
            step="1000"
            bind:value={maxChapterChars}
        />
    </div>
    <div class="set-row">
        <label for="aiTimeoutSec">{t("最长响应时间:")}</label>
        <input
            id="aiTimeoutSec"
            type="number"
            min="30"
            max="1800"
            step="30"
            bind:value={responseTimeoutSec}
        />
    </div>
    <label class="set-row ai-toggle-row" for="aiAutoApprove">
        <span>{t("自动审批:")}</span>
        <input id="aiAutoApprove" type="checkbox" bind:checked={autoApprove} />
        <small
            >{t("AI 审核非自动建议，通过则自动应用，存疑保留人工审核。")}</small
        >
    </label>
    <div class="set-row ai-textarea-row">
        <label for="aiExtraPrompt">{t("额外要求:")}</label>
        <textarea
            id="aiExtraPrompt"
            rows="4"
            bind:value={extraPrompt}
            placeholder={t("例如：保留作者口癖，不做风格润色。")}></textarea>
    </div>
</div>

<style>
    .font-settings-status {
        padding: 8px 10px;
        border-radius: var(--radius-sm);
        background: var(--color-surface-soft);
        color: var(--color-text-soft);
        font-size: 12px;
        line-height: 1.5;
    }

    .ai-settings-form {
        display: flex;
        flex-direction: column;
        gap: 12px;
        max-width: 100%;
        padding-top: 4px;
    }

    .ai-settings-form .set-row {
        margin-bottom: 0;
    }

    .ai-settings-form .set-row label {
        width: 110px;
        flex-shrink: 0;
        font-weight: bold;
        color: var(--color-text-soft);
        font-size: 13px;
        line-height: 1.4;
    }

    .ai-settings-form .set-row {
        align-items: center;
        justify-content: flex-start;
        gap: 10px;
    }

    .ai-settings-form .set-row input:not([type="checkbox"]),
    .ai-settings-form .set-row textarea {
        flex: 1 1 auto;
        min-width: 0;
        min-height: var(--control-height);
        box-sizing: border-box;
        font-size: 13px;
    }

    .ai-settings-form .ai-textarea-row {
        align-items: flex-start;
    }

    .ai-settings-form .ai-textarea-row label {
        padding-top: 8px;
    }

    .ai-settings-form .set-row textarea {
        flex: 1;
        width: 100%;
        min-height: 78px;
        resize: vertical;
        line-height: 1.55;
    }

    .ai-settings-form .ai-toggle-row {
        display: grid;
        grid-template-columns: 110px 24px minmax(0, 1fr);
        align-items: center;
        gap: 10px;
    }

    .ai-settings-form .ai-toggle-row input {
        width: 16px;
        height: 16px;
        min-height: 0;
        accent-color: var(--color-teal);
    }

    .ai-settings-form .ai-toggle-row small {
        color: var(--color-muted);
        font-size: 11px;
        line-height: 1.45;
    }

    /* Modern UI overrides */
    .set-row label {
        color: var(--color-text-soft);
    }

    .set-row input,
    .set-row textarea {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast),
            background var(--transition-fast);
    }

    .set-row input:focus,
    .set-row textarea:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: var(--color-surface);
    }

    .set-row :global(.custom-select) {
        flex: 1 1 auto;
        min-width: 0;
    }
</style>
