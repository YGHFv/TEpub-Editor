<script lang="ts">
  import { onMount, onDestroy } from "svelte";

  let showMenu = false;
  let pos = { x: 0, y: 0 };
  let menuElement: HTMLDivElement;

  // Define menu items structure
  interface MenuItem {
    label: string;
    action: string;
    icon?: string;
    separator?: boolean;
    danger?: boolean;
  }

  let currentItems: MenuItem[] = [];
  let currentContext: any = null;

  async function handleAction(action: string) {
    showMenu = false;

    // Editor actions
    if (["copy", "cut", "paste", "select-all"].includes(action)) {
      try {
        const selectedText = window.getSelection()?.toString() || "";
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
            const activeEl = document.activeElement as HTMLElement;
            if (
              activeEl instanceof HTMLInputElement ||
              activeEl instanceof HTMLTextAreaElement
            ) {
              activeEl.select();
            } else {
              window.dispatchEvent(new CustomEvent("editor-select-all"));
            }
            break;
        }
      } catch (err) {
        console.error(err);
      }
      return;
    }

    // File/Folder/TOC actions
    // Dispatch event with context data
    window.dispatchEvent(
      new CustomEvent("context-menu-action", {
        detail: {
          action,
          context: currentContext,
        },
      }),
    );
  }

  function onContextMenu(e: MouseEvent) {
    // 1. Check for Context Data Attribute (File Tree / TOC)
    const target = e.target as HTMLElement;
    const contextNode = target.closest("[data-context-type]") as HTMLElement;

    // 2. Check for Editor
    const isInEditor =
      target.closest(".cm-content") || target.closest(".cm-editor");
    const isInput =
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.isContentEditable;
    const hasSelection = window.getSelection()?.toString().length > 0;

    if (contextNode) {
      e.preventDefault();
      const type = contextNode.dataset.contextType;
      currentContext = { ...contextNode.dataset }; // Copy all data attributes

      // Define menus based on type
      if (type === "folder") {
        const folderName = (currentContext.folderType || "").toLowerCase(); // defined in FileTreeItem as node.name
        // User requested: Text, Style, OEBPS only
        const allowedFolders = ["text", "style", "styles", "oebps"];

        if (allowedFolders.includes(folderName)) {
          currentItems = [
            { label: "新建文件", action: "new-file", icon: "📄" },
            { label: "导入文件", action: "import-file", icon: "📥" },
          ];
        } else {
          // For other folders, maybe just Import? Or nothing?
          // User said "remove New File". Let's keep "Import" or remove both?
          // "remove right-click New File" implies Import might stay or whole menu changes.
          // Let's safe-bet: allow Import, disallow New. Or just nothing if it's a structural folder?
          // Actually, usually you can import into Images/Fonts.
          // Let's show Import for others, but New only for allowed.
          currentItems = [
            { label: "导入文件", action: "import-file", icon: "📥" },
          ];
        }
      } else if (type === "file") {
        currentItems = [
          {
            label: "多选/取消多选 (Ctrl+Click)",
            action: "toggle-select",
            icon: "☑️",
          },
          { separator: true, label: "", action: "" },
        ];

        // 如果是 HTML/XHTML 文件，添加额外选项
        const path = currentContext.path || "";
        if (path.endsWith(".xhtml") || path.endsWith(".html")) {
          currentItems.push(
            { label: "另存为...", action: "save-as", icon: "💾" },
            { label: "添加副本", action: "duplicate", icon: "👯" },
            { label: "添加现有文件...", action: "import-sibling", icon: "📥" },
            { label: "添加空白HTML", action: "new-sibling-html", icon: "📄" },
            { label: "", action: "", separator: true },
          );
        }

        currentItems.push(
          { label: "重命名 (自动重构)", action: "rename", icon: "✏️" },
          { label: "删除", action: "delete", icon: "🗑️", danger: true },
        );
      } else if (type === "toc") {
        currentItems = [
          { label: "在文件树中选中", action: "select-in-tree", icon: "🔍" },
          {
            label: "选中当前卷所有文件",
            action: "select-children",
            icon: "📑",
          },
          // { label: "重命名", action: "rename-toc", icon: "✏️" },
        ];
      } else {
        return; // Unknown context
      }
    } else if (isInEditor || isInput || hasSelection) {
      if (!isInEditor && !isInput && !hasSelection) return;
      e.preventDefault();
      currentContext = { type: "editor" };
      currentItems = [
        { label: "剪切", action: "cut", icon: "✂️" },
        { label: "复制", action: "copy", icon: "📋" },
        { label: "粘贴", action: "paste", icon: "📄" },
        { separator: true, label: "", action: "" },
        { label: "全选", action: "select-all", icon: "✨" },
      ];
    } else {
      return; // Allow default or do nothing? User said "Always prevent default" in previous code?
      // Previous code: always preventDefault then check.
      // Let's stick to showing our menu or nothing.
      // If not matched, we hide menu.
      if (showMenu) showMenu = false;
      return;
    }

    // Position logic
    let x = e.clientX;
    let y = e.clientY;

    // Simple measurement (can be improved)
    const menuWidth = 160;
    const menuHeight = currentItems.length * 36 + 10;

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
    {#each currentItems as item}
      {#if item.separator}
        <div class="separator"></div>
      {:else}
        <div
          class="menu-item"
          class:danger={item.danger}
          on:click={() => handleAction(item.action)}
        >
          <span class="icon">{item.icon}</span>
          <span>{item.label}</span>
        </div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .context-menu {
    position: fixed;
    z-index: 999999;
    min-width: 160px;
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
    background: #f0f0f0;
  }

  .menu-item.danger {
    color: #d32f2f;
  }
  .menu-item.danger:hover {
    background: #ffebee;
  }

  .menu-item .icon {
    font-size: 14px;
    width: 18px;
    display: inline-block;
    text-align: center;
  }

  .separator {
    height: 1px;
    background: #eee;
    margin: 4px 0;
  }
</style>
