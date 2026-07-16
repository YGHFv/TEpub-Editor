<script lang="ts">
  import { createEventDispatcher, tick } from "svelte";

  type CustomSelectOption = {
    value: string;
    label: string;
    meta?: string;
    disabled?: boolean;
  };

  export let value = "";
  export let options: CustomSelectOption[] = [];
  export let placeholder = "请选择";
  export let disabled = false;
  export let className = "";
  export let id = "";
  export let ariaLabel = "";

  const dispatch = createEventDispatcher<{ change: string }>();
  let open = false;
  let menuElement: HTMLDivElement | null = null;
  let activeIndex = -1;

  $: selectedOption = options.find((option) => option.value === value);
  $: displayLabel = selectedOption?.label || placeholder;
  $: if (open) activeIndex = Math.max(0, options.findIndex((option) => option.value === value && !option.disabled));

  function enabledIndexes() {
    return options.map((option, index) => option.disabled ? -1 : index).filter((index) => index >= 0);
  }

  async function focusOption(index: number) {
    activeIndex = index;
    await tick();
    menuElement?.querySelectorAll<HTMLButtonElement>("[role='option']")[index]?.focus();
  }

  async function showMenu(preferred: "selected" | "first" | "last" = "selected") {
    if (disabled || options.length === 0) return;
    open = true;
    const indexes = enabledIndexes();
    if (!indexes.length) return;
    const selectedIndex = options.findIndex((option) => option.value === value && !option.disabled);
    const index = preferred === "first" ? indexes[0] : preferred === "last" ? indexes[indexes.length - 1] : selectedIndex >= 0 ? selectedIndex : indexes[0];
    await focusOption(index);
  }

  function toggle() {
    if (disabled) return;
    if (open) open = false;
    else void showMenu();
  }

  function choose(option: CustomSelectOption) {
    if (disabled || option.disabled) return;
    value = option.value;
    open = false;
    dispatch("change", option.value);
  }

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") open = false;
  }

  function onTriggerKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Home" || event.key === "End") {
      event.preventDefault();
      void showMenu(event.key === "ArrowUp" || event.key === "End" ? "last" : "first");
    }
  }

  function onOptionKeydown(event: KeyboardEvent, index: number) {
    const indexes = enabledIndexes();
    const position = indexes.indexOf(index);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const offset = event.key === "ArrowDown" ? 1 : -1;
      void focusOption(indexes[(position + offset + indexes.length) % indexes.length]);
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      void focusOption(event.key === "Home" ? indexes[0] : indexes[indexes.length - 1]);
    }
  }
</script>

<svelte:window on:click={() => (open = false)} on:keydown={closeOnEscape} />

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class={["custom-select", className].filter(Boolean).join(" ")}
  class:open
  class:disabled
  on:click|stopPropagation
>
  <button
    class="custom-select-trigger"
    {id}
    type="button"
    disabled={disabled}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label={ariaLabel || undefined}
    on:click={toggle}
    on:keydown={onTriggerKeydown}
  >
    <span>{displayLabel}</span>
    <span class="select-caret" aria-hidden="true"></span>
  </button>
  {#if open}
    <div class="custom-select-menu" role="listbox" bind:this={menuElement} aria-label={ariaLabel || undefined}>
      {#each options as option, index}
        <button
          type="button"
          role="option"
          aria-selected={value === option.value}
          class:active={value === option.value}
          disabled={option.disabled}
          on:click={() => choose(option)}
          on:keydown={(event) => onOptionKeydown(event, index)}
        >
          <span class="option-label"><span>{option.label}</span>{#if value === option.value}<b aria-hidden="true">✓</b>{/if}</span>
          {#if option.meta}
            <small>{option.meta}</small>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .custom-select {
    position: relative;
    min-width: 0;
    width: 100%;
  }

  .custom-select-trigger {
    box-sizing: border-box;
    width: 100%;
    min-height: var(--control-height, 34px);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 12px;
    border: 1px solid var(--color-border, #cbd5e1);
    border-radius: var(--radius-sm, 6px);
    background: color-mix(in srgb, var(--color-surface, #ffffff) 94%, var(--color-accent-quiet, #e8f2f8));
    color: var(--color-text, #172033);
    box-shadow: var(--shadow-xs, none);
    text-align: left;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }

  .custom-select-trigger span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .custom-select.open .custom-select-trigger,
  .custom-select-trigger:focus-visible {
    outline: none;
    border-color: var(--color-accent, #1677b8);
    box-shadow: var(--focus-ring, 0 0 0 2px rgba(22, 119, 184, 0.18)), var(--shadow-xs, none);
  }

  .custom-select.disabled .custom-select-trigger {
    cursor: not-allowed;
    opacity: 0.55;
  }

  .select-caret {
    flex: 0 0 auto;
    width: 0;
    height: 0;
    border-left: 6px solid transparent;
    border-right: 6px solid transparent;
    border-top: 7px solid var(--color-text-soft, #334155);
  }

  .custom-select-menu {
    position: absolute;
    z-index: 100;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    box-sizing: border-box;
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
    border: 1px solid var(--color-border-strong, #cbd5e1);
    border-radius: var(--radius-sm, 6px);
    background: var(--color-surface, #ffffff);
    box-shadow: var(--shadow-pop, 0 8px 24px rgba(15, 23, 42, 0.16));
  }

  .custom-select-menu button {
    width: 100%;
    min-height: 34px;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-xs, 4px);
    background: transparent;
    color: var(--color-text, #172033);
    text-align: left;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    cursor: pointer;
  }

  .custom-select-menu button span,
  .custom-select-menu button small {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .custom-select-menu .option-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .custom-select-menu .option-label span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .custom-select-menu .option-label b {
    flex: 0 0 auto;
    font-size: 12px;
  }

  .custom-select-menu button small {
    margin-top: 2px;
    color: var(--color-muted, #64748b);
    font-size: 11px;
    font-weight: 600;
  }

  .custom-select-menu button:hover,
  .custom-select-menu button:focus-visible {
    outline: none;
    background: var(--color-hover, #f1f5f9);
    color: var(--color-accent-deep, #155e96);
  }

  .custom-select-menu button.active {
    background: var(--color-accent, #1677b8);
    color: var(--color-accent-contrast, #fff);
  }

  .custom-select-menu button.active small {
    color: color-mix(in srgb, var(--color-accent-contrast, #fff) 82%, transparent);
  }

  .custom-select-menu button:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }
</style>
