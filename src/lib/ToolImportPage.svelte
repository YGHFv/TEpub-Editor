<script module lang="ts">
  export type ToolImportFeature = {
    title: string;
    detail: string;
  };
</script>

<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { validateBrowserFiles } from "$lib/webFileWorkflow";

  export let mark = "FILE";
  export let kicker = "WEB TOOL";
  export let title = "选择文件";
  export let description = "";
  export let privacy = "文件仅在当前设备中处理。";
  export let outputLabel = "支持格式";
  export let outputValue = "";
  export let features: ToolImportFeature[] = [];
  export let prompt = "选择或拖入文件";
  export let hint = "";
  export let actionLabel = "选择文件";
  export let accept = "";
  export let multiple = false;
  export let busy = false;
  export let errorText = "";

  const dispatch = createEventDispatcher<{
    select: void;
    files: File[];
  }>();

  let dragActive = false;
  let internalError = "";

  function choose() {
    if (!busy) dispatch("select");
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragActive = false;
    if (busy) return;
    const files = Array.from(event.dataTransfer?.files || []);
    const acceptParts = accept.split(",").map((item) => item.trim()).filter(Boolean);
    const extensions = acceptParts.filter((item) => item.startsWith(".")).map((item) => item.slice(1));
    const mimeTypes = acceptParts.filter((item) => item.includes("/") && !item.endsWith("/*"));
    const validation = validateBrowserFiles(files, { extensions, mimeTypes, multiple });
    internalError = validation.message;
    if (validation.accepted.length) dispatch("files", validation.accepted);
  }
</script>

<div class="tool-import-page">
  <section class="tool-import-intro">
    <div class="tool-import-mark">{mark}</div>
    <div class="tool-import-copy">
      <span class="tool-import-kicker">{kicker}</span>
      <h1>{title}</h1>
      <p>{description}</p>
      <small>{privacy}</small>
    </div>
    <div class="tool-import-output">
      <span>{outputLabel}</span>
      <strong>{outputValue}</strong>
    </div>
  </section>

  <button
    class:drag-active={dragActive}
    class="tool-import-drop"
    type="button"
    disabled={busy}
    aria-label={actionLabel}
    on:click={choose}
    on:dragenter={(event) => { event.preventDefault(); dragActive = true; }}
    on:dragover={(event) => event.preventDefault()}
    on:dragleave={() => { dragActive = false; }}
    on:drop={handleDrop}
  >
    {#if busy}
      <span class="tool-import-spinner" aria-hidden="true"></span>
    {:else}
      <span class="tool-import-drop-icon" aria-hidden="true">+</span>
    {/if}
    <strong>{busy ? "正在读取文件" : prompt}</strong>
    {#if hint}<span class="tool-import-hint">{hint}</span>{/if}
    <span class="tool-import-action">{busy ? "处理中…" : actionLabel}</span>
  </button>

  {#if features.length}
    <section class="tool-import-features" aria-label="功能说明">
      {#each features as feature, index}
        <article>
          <b>{String(index + 1).padStart(2, "0")}</b>
          <strong>{feature.title}</strong>
          <span>{feature.detail}</span>
        </article>
      {/each}
    </section>
  {/if}

  {#if errorText || internalError}
    <p class="tool-import-error" role="alert">{errorText || internalError}</p>
  {/if}
</div>

<style>
  .tool-import-page { box-sizing: border-box; width: min(var(--web-tool-content-width), calc(100% - 36px)); min-height: 100%; margin: 0 auto; display: grid; align-content: start; gap: var(--web-tool-gap); padding: 26px 0 40px; overflow: auto; }
  .tool-import-intro { display: grid; grid-template-columns: 70px minmax(0, 1fr) auto; align-items: center; gap: 18px; padding: 22px; border: 1px solid var(--web-tool-border); border-radius: var(--web-tool-radius-md); background: var(--web-tool-surface); box-shadow: 0 8px 25px rgba(42,62,84,.05); }
  .tool-import-mark { width: 64px; height: 64px; display: grid; place-items: center; border-radius: 14px; background: var(--web-tool-accent-soft); color: var(--web-tool-accent); font-size: 12px; font-weight: 900; letter-spacing: .04em; }
  .tool-import-kicker { color: #7b899d; font-size: 9px; font-weight: 800; letter-spacing: .15em; }
  h1 { margin: 4px 0 7px; color: var(--web-tool-text); font-size: 22px; line-height: 1.3; }
  p { margin: 0; }
  .tool-import-copy p { color: var(--web-tool-text-soft); font-size: 13px; line-height: 1.55; }
  .tool-import-copy small { display: block; margin-top: 5px; color: #8491a2; font-size: 11px; }
  .tool-import-output { min-width: 132px; display: grid; gap: 5px; padding-left: 20px; border-left: 1px solid #e0e6ed; }
  .tool-import-output span { color: #8491a2; font-size: 10px; }
  .tool-import-output strong { color: #31516b; font: 11px ui-monospace, Consolas, monospace; }
  .tool-import-features { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .tool-import-features article { min-width: 0; display: grid; grid-template-columns: 36px 1fr; gap: 2px 10px; padding: 14px 18px; border: 1px solid var(--web-tool-border); border-radius: 9px; background: var(--web-tool-surface); }
  .tool-import-features b { grid-row: 1 / 3; color: var(--web-tool-accent); font-size: 11px; }
  .tool-import-features strong { color: var(--web-tool-text); font-size: 13px; }
  .tool-import-features span { color: var(--web-tool-muted); font-size: 11px; line-height: 1.45; }
  .tool-import-drop { width: 100%; min-height: 290px; height: auto; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; padding: 18px; border: 1px dashed var(--web-tool-border-strong); border-radius: var(--web-tool-radius-md); background: var(--web-tool-surface-translucent); color: var(--web-tool-text); cursor: pointer; transition: .18s ease; }
  .tool-import-drop:hover:not(:disabled), .tool-import-drop.drag-active { border-color: var(--web-tool-accent); background: var(--web-tool-surface); box-shadow: 0 10px 32px rgba(42,74,103,.1); }
  .tool-import-drop:disabled { cursor: wait; opacity: .7; }
  .tool-import-drop-icon { width: 48px; height: 48px; display: grid; place-items: center; border-radius: 50%; background: var(--web-tool-accent-soft); color: var(--web-tool-accent); font-size: 28px; font-weight: 300; }
  .tool-import-drop > strong { font-size: 18px; }
  .tool-import-hint { color: #728197; font-size: 12px; }
  .tool-import-action { margin-top: 8px; padding: 8px 17px; border-radius: 6px; background: var(--web-tool-accent); color: #fff; font-size: 11px; font-weight: 800; }
  .tool-import-error { padding: 10px 14px; border: 1px solid var(--web-tool-error-border); border-radius: var(--web-tool-radius-sm); background: var(--web-tool-error-bg); color: var(--web-tool-error); font-size: 13px; }
  .tool-import-spinner { width: 28px; height: 28px; border: 3px solid #d7e2ec; border-top-color: #17699a; border-radius: 50%; animation: spin .75s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 760px) {
    .tool-import-page { width: calc(100% - 24px); padding: 14px 0 28px; }
    .tool-import-intro { grid-template-columns: 52px minmax(0, 1fr); padding: 16px; gap: 12px; }
    .tool-import-mark { width: 50px; height: 50px; }
    h1 { font-size: 18px; }
    .tool-import-output { grid-column: 1 / -1; padding: 10px 0 0; border-top: 1px solid #e0e6ed; border-left: 0; }
    .tool-import-features { grid-template-columns: 1fr; }
    .tool-import-drop { min-height: 220px; }
    .tool-import-hint { max-width: 280px; text-align: center; line-height: 1.5; }
  }
</style>
