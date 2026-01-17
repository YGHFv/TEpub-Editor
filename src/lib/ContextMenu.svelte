<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  let showMenu = false;
  let pos = { x: 0, y: 0 };
  let menuElement: HTMLDivElement;

  async function handleAction(action: string) {
    showMenu = false;
    const activeEl = document.activeElement as HTMLElement;
    const selection = window.getSelection();
    const selectedText = selection?.toString() || "";

    try {
      switch (action) {
        case "copy":
          if (selectedText) await navigator.clipboard.writeText(selectedText);
          break; 
        case "cut":
          if (selectedText) {
            await navigator.clipboard.writeText(selectedText);
            document.execCommand("cut");
          }
          break;
        case "paste":
          const text = await navigator.clipboard.readText();
          document.execCommand("insertText", false, text);
          break;
        case "select-all":
          if (activeEl instanceof HTMLInputElement || activeEl instanceof HTMLTextAreaElement) {
            activeEl.select();
          } else {
            document.execCommand("selectAll");
          }
          break;
      }
    } catch (err) { console.error(err); }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    let x = e.clientX;
    let y = e.clientY;
    const menuWidth = 120; 
    const menuHeight = 130; 
    if (x + menuWidth > window.innerWidth) x -= menuWidth;
    if (y + menuHeight > window.innerHeight) y -= menuHeight;
    pos = { x, y };
    showMenu = true;
  }

  function onGlobalClick() { if (showMenu) showMenu = false; }
  function onScroll() { if (showMenu) showMenu = false; }

  onMount(() => {
    window.addEventListener("contextmenu", onContextMenu);
    window.addEventListener("click", onGlobalClick);
    window.addEventListener("scroll", onScroll, true);
  });

  onDestroy(() => {
    if (typeof window !== "undefined") {
        window.removeEventListener("contextmenu", onContextMenu);
        window.removeEventListener("click", onGlobalClick);
        window.removeEventListener("scroll", onScroll, true);
    }
  });
</script>

{#if showMenu}
  <div 
    class="context-menu" 
    bind:this={menuElement}
    style="top: {pos.y}px; left: {pos.x}px;"
    on:click|stopPropagation
  >
    <div class="menu-item" on:click={() => handleAction('cut')}>剪切</div>
    <div class="menu-item" on:click={() => handleAction('copy')}>复制</div>
    <div class="menu-item" on:click={() => handleAction('paste')}>粘贴</div>
    <div class="separator"></div>
    <div class="menu-item" on:click={() => handleAction('select-all')}>全选</div>
  </div>
{/if}

<style>
  .context-menu {
    position: fixed;
    z-index: 999999;
    width: 100px; /* 更窄，更像原生 */
    background: #ffffff;
    border: 1px solid #d0d0d0;
    box-shadow: 2px 2px 5px rgba(0, 0, 0, 0.1);
    padding: 4px 0;
    font-family: 'Segoe UI', sans-serif;
    font-size: 12px; /* 字体稍微改小 */
    color: #333;
    cursor: default;
    user-select: none;
  }

  .menu-item {
    padding: 6px 12px; /* 调整内边距 */
    cursor: pointer;
    transition: background 0.1s;
  }

  .menu-item:hover {
    background-color: #e8e8e8; /* 原生风格通常是浅灰或深蓝，这里用浅灰更自然 */
    color: #000;
  }

  .separator {
    height: 1px;
    background-color: #e0e0e0;
    margin: 3px 0;
  }
</style>