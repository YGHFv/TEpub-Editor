<script lang="ts">
  import type { Snippet } from "svelte";

  export type SettingsTab = {
    id: string;
    label: string;
  };

  let {
    title,
    tabs = [],
    activeTab,
    onTabChange,
    onClose,
    closeLabel = "关闭",
    actionLabel = "",
    onAction = undefined,
    actionDisabled = false,
    actionClass = "",
    shellClass = "",
    contentClass = "",
    footerClass = "",
    children,
    footer,
  }: {
    title: string;
    tabs: SettingsTab[];
    activeTab: string;
    onTabChange: (tabId: string) => void;
    onClose: () => void;
    closeLabel?: string;
    actionLabel?: string;
    onAction?: (() => void) | undefined;
    actionDisabled?: boolean;
    actionClass?: string;
    shellClass?: string;
    contentClass?: string;
    footerClass?: string;
    children?: Snippet;
    footer?: Snippet;
  } = $props();

  const mergeClasses = (...values: Array<string | false | null | undefined>) =>
    values.filter(Boolean).join(" ");
</script>

<div
  class={mergeClasses("settings-shell", shellClass)}
  role="dialog"
  aria-modal="true"
  tabindex="-1"
  onclick={(event) => event.stopPropagation()}
  onkeydown={(event) => {
    if (event.key === "Escape") onClose();
  }}
>
  <div class="settings-shell-header">
    <h3>{title}</h3>
    <button
      class="settings-shell-close"
      type="button"
      title={closeLabel}
      aria-label={closeLabel}
      onclick={onClose}
    >
      ×
    </button>
  </div>

  <div class="settings-shell-tabs">
    {#each tabs as tab}
      <button
        type="button"
        class={mergeClasses("tab-btn", activeTab === tab.id && "active")}
        aria-pressed={activeTab === tab.id}
        onclick={() => onTabChange(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <div class={mergeClasses("settings-shell-body", contentClass)}>
    {@render children?.()}
  </div>

  <div class={mergeClasses("settings-shell-footer", footerClass)}>
    {#if actionLabel && onAction}
      <button
        type="button"
        class={mergeClasses("settings-shell-action", "settings-shell-action-primary", actionClass)}
        disabled={actionDisabled}
        onclick={onAction}
      >
        {actionLabel}
      </button>
    {/if}
    {@render footer?.()}
  </div>
</div>
