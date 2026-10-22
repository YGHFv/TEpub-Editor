<script lang="ts" module>
    export interface AiProviderConfig {
        id: string;
        name: string;
        kind?: "text" | "image";
        baseUrl: string;
        apiKey: string;
        model: string;
        temperature: number;
    }
</script>

<script lang="ts">
    import { localizeAiProviderName, t } from "$lib/i18n.svelte";

    let {
        message,
        providers,
        editorOpen,
        editorMode,
        editorDraft,
        onAdd,
        onEdit,
        onRemove,
        onSave,
        onCancel,
        onUpdateDraft,
    }: {
        message: string;
        providers: AiProviderConfig[];
        editorOpen: boolean;
        editorMode: "new" | "edit";
        editorDraft: AiProviderConfig | null;
        onAdd: () => void;
        onEdit: (provider: AiProviderConfig) => void;
        onRemove: (providerId: string) => void;
        onSave: () => void;
        onCancel: () => void;
        onUpdateDraft: (
            field: keyof AiProviderConfig,
            value: string | number,
        ) => void;
    } = $props();
</script>

<div class="ai-settings-form">
    {#if message}
        <div class="font-settings-status">{t(message)}</div>
    {/if}
    <div class="font-settings-head ai-provider-head">
        <div>
            <div class="font-settings-title">{t("API 配置")}</div>
            <div class="font-settings-note">
                {t("可添加多个 OpenAI 兼容 API，分别供校对和自动审批选择。")}
            </div>
        </div>
        <div class="style-settings-actions">
            <button class="mini-action" type="button" onclick={onAdd}
                >{t("新增")}</button
            >
        </div>
    </div>

    {#if providers.length === 0 && !editorOpen}
        <div class="api-empty">
            {t("暂无 API 配置，点击“新增”添加文字模型。")}
        </div>
    {:else}
        <div class="api-list">
            {#each providers as provider}
                <div class="api-item">
                    <div class="api-item-main">
                        <div class="api-item-title">
                            <strong
                                >{localizeAiProviderName(provider.name) ||
                                    provider.model}</strong
                            >
                            <span>{t("文字模型")}</span>
                        </div>
                        <div class="api-item-meta">
                            <span>{provider.model || t("未填写模型")}</span>
                            <span
                                >{provider.baseUrl ||
                                    t("未填写 API 地址")}</span
                            >
                            <span
                                >{provider.apiKey
                                    ? t("已保存 Key")
                                    : t("未填写 Key")}</span
                            >
                        </div>
                    </div>
                    <div class="api-item-actions">
                        <button
                            class="mini-action"
                            type="button"
                            onclick={() => onEdit(provider)}>{t("编辑")}</button
                        >
                        <button
                            class="mini-action danger"
                            type="button"
                            onclick={() => onRemove(provider.id)}
                            >{t("删除")}</button
                        >
                    </div>
                </div>
            {/each}
        </div>
    {/if}

    {#if editorOpen && editorDraft}
        <div class="api-editor">
            <div class="api-editor-head">
                <strong
                    >{t(editorMode === "new" ? "新增 API" : "编辑 API")}</strong
                >
                <div class="api-editor-actions">
                    <button
                        class="mini-action primary"
                        type="button"
                        onclick={onSave}>{t("保存")}</button
                    >
                    <button class="mini-action" type="button" onclick={onCancel}
                        >{t("取消")}</button
                    >
                </div>
            </div>
            <div class="set-row">
                <label for="aiProviderName">{t("名称")}:</label>
                <input
                    id="aiProviderName"
                    type="text"
                    value={editorDraft.name || ""}
                    oninput={(e) =>
                        onUpdateDraft("name", e.currentTarget.value)}
                />
            </div>
            <div class="set-row">
                <label for="aiProviderBaseUrl">{t("API 地址")}:</label>
                <input
                    id="aiProviderBaseUrl"
                    type="text"
                    value={editorDraft.baseUrl || ""}
                    oninput={(e) =>
                        onUpdateDraft("baseUrl", e.currentTarget.value)}
                    placeholder="https://api.openai.com/v1"
                />
            </div>
            <div class="set-row">
                <label for="aiProviderKey">API Key:</label>
                <input
                    id="aiProviderKey"
                    type="password"
                    value={editorDraft.apiKey || ""}
                    oninput={(e) =>
                        onUpdateDraft("apiKey", e.currentTarget.value)}
                    placeholder="sk-..."
                />
            </div>
            <div class="set-row">
                <label for="aiProviderModel">{t("模型")}:</label>
                <input
                    id="aiProviderModel"
                    type="text"
                    value={editorDraft.model || ""}
                    oninput={(e) =>
                        onUpdateDraft("model", e.currentTarget.value)}
                    placeholder="deepseek-chat / gpt-4o-mini"
                />
            </div>
            <div class="set-row">
                <label for="aiProviderTemperature">{t("温度")}:</label>
                <input
                    id="aiProviderTemperature"
                    type="number"
                    min="0"
                    max="1"
                    step="0.1"
                    value={editorDraft.temperature ?? 0.1}
                    oninput={(e) =>
                        onUpdateDraft(
                            "temperature",
                            Number(e.currentTarget.value),
                        )}
                />
            </div>
        </div>
    {/if}
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

    .mini-action.primary {
        border-color: var(--color-accent);
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
    }

    .mini-action.danger {
        border-color: color-mix(
            in srgb,
            var(--color-danger-soft) 88%,
            var(--color-danger)
        );
        color: var(--color-danger);
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

    .style-settings-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 8px;
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

    .ai-provider-head {
        margin-bottom: 4px;
    }

    .api-empty,
    .api-item,
    .api-editor {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
    }

    .api-empty {
        padding: 10px 12px;
        color: var(--color-muted);
        font-size: 13px;
        line-height: 1.5;
    }

    .api-list {
        display: grid;
        gap: 10px;
    }

    .api-item {
        min-width: 0;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 12px;
    }

    .api-item-main {
        min-width: 0;
        display: grid;
        gap: 6px;
    }

    .api-item-title,
    .api-item-actions,
    .api-editor-head,
    .api-editor-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .api-item-title strong {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 14px;
        color: var(--color-text);
    }

    .api-item-title span {
        flex: 0 0 auto;
        padding: 2px 7px;
        border-radius: 999px;
        background: var(--color-accent-quiet);
        color: var(--color-accent-deep);
        font-size: 11px;
        font-weight: 800;
    }

    .api-item-meta {
        min-width: 0;
        display: flex;
        flex-wrap: wrap;
        gap: 6px 12px;
        color: var(--color-muted);
        font-size: 12px;
        line-height: 1.4;
    }

    .api-item-meta span {
        max-width: 100%;
        overflow-wrap: anywhere;
    }

    .api-editor {
        margin-top: 2px;
        padding: 12px;
        display: grid;
        gap: 12px;
        background: var(--color-surface-soft);
    }

    .api-editor-head {
        justify-content: space-between;
    }

    .api-editor-head strong {
        color: var(--color-text);
        font-size: 14px;
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

    .ai-settings-form .set-row input:not([type="checkbox"]) {
        flex: 1 1 auto;
        min-width: 0;
        min-height: var(--control-height);
        box-sizing: border-box;
        font-size: 13px;
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

    .set-row label {
        color: var(--color-text-soft);
    }

    .set-row input {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast),
            background var(--transition-fast);
    }

    .set-row input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: var(--color-surface);
    }
</style>
