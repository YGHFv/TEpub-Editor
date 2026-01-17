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
          if (
            activeEl instanceof HTMLInputElement ||
            activeEl instanceof HTMLTextAreaElement
          ) {
            activeEl.select();
          } else {
            // 派发自定义事件让父组件处理
            window.dispatchEvent(new CustomEvent("editor-select-all"));
          }
          break;
      }
    } catch (err) {
      console.error(err);
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();

    // 检查是否在编辑器内（CodeMirror）
    const target = e.target as HTMLElement;
    const isInEditor =
      target.closest(".cm-content") || target.closest(".cm-editor");

    if (!isInEditor) {
      return; // 不在编辑器内，不显示菜单
    }

    let x = e.clientX;
    let y = e.clientY;
    const menuWidth = 120;
    const menuHeight = 130;
    if (x + menuWidth > window.innerWidth) x -= menuWidth;
    if (y + menuHeight > window.innerHeight) y -= menuHeight;
    pos = { x, y };
    showMenu = true;
  }

  function onGlobalClick() {
    if (showMenu) showMenu = false;
  }
  function onScroll() {
    if (showMenu) showMenu = false;
  }

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
    <div class="menu-item" on:click={() => handleAction("cut")}>
      <span class="icon">✂️</span>
      <span>剪切</span>
    </div>
    <div class="menu-item" on:click={() => handleAction("copy")}>
      <span class="icon">📋</span>
      <span>复制</span>
    </div>
    <div class="menu-item" on:click={() => handleAction("paste")}>
      <span class="icon">📄</span>
      <span>粘贴</span>
    </div>
    <div class="separator"></div>
    <div class="menu-item" on:click={() => handleAction("select-all")}>
      <span class="icon">✨</span>
      <span>全选</span>
    </div>
  </div>
{/if}

<style>
  .context-menu {
    position: fixed;
    z-index: 999999;
    min-width: 140px;
    background: #ffffff;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    padding: 6px 0;
    font-family: system-ui, sans-serif;
    font-size: 13px;
    color: #333;
    cursor: default;
    user-select: none;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .menu-item:hover {
    background: linear-gradient(90deg, #0066b8 0%, #0077cc 100%);
    color: white;
  }

  .menu-item .icon {
    font-size: 14px;
    width: 18px;
    display: inline-block;
  }

  .separator {
    height: 1px;
    background: linear-gradient(
      90deg,
      transparent 0%,
      #e0e0e0 50%,
      transparent 100%
    );
    margin: 4px 8px;
  }
</style>
