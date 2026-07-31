<script module lang="ts">
    export interface EpubFileNode {
        name: string;
        path: string;
        file_type: string;
        size?: number;
        title?: string;
        resolution?: string;
        children?: EpubFileNode[];
    }
</script>

<script lang="ts">
    import FileTreeItem from "$lib/FileTreeItem.svelte";
    import { getFileIcon } from "./fileIcons";

    let {
        fileTree,
        expandedFolders,
        selectedFile,
        multiSelectedFiles,
        isSaving = false,
        isDirty = false,
        onToggleFolder,
        onSelectFile,
        onSaveAll,
        onRefresh,
    }: {
        fileTree: EpubFileNode[];
        expandedFolders: Set<string>;
        selectedFile: EpubFileNode | null;
        multiSelectedFiles: Set<string>;
        isSaving?: boolean;
        isDirty?: boolean;
        onToggleFolder: (path: string) => void;
        onSelectFile: (
            file: EpubFileNode,
            event?: MouseEvent | KeyboardEvent,
        ) => void;
        onSaveAll: () => void;
        onRefresh: () => void;
    } = $props();

    function getFileDescription(file: EpubFileNode): string {
        if (file.file_type === "html" && file.title) {
            return file.title;
        }

        const fileName = file.name.toLowerCase();
        if (fileName === "container.xml") return "容器配置";
        if (fileName === "content.opf") return "元数据";
        if (fileName.includes("toc") || fileName.includes("ncx"))
            return "目录结构";
        if (file.file_type === "css") return "样式表";
        if (file.file_type === "font") {
            return "字体 " + ((file.size || 0) / 1024).toFixed(1) + "KB";
        }
        if (file.file_type === "image") {
            const sizeStr = file.size
                ? Math.round(file.size / 1024) + "KB"
                : "";
            if (file.resolution) {
                return file.resolution + (sizeStr ? " " + sizeStr : "");
            }
            return sizeStr ? "图片 " + sizeStr : "图片资源";
        }
        return file.path;
    }
</script>

<aside class="file-tree">
    <div class="tree-header">
        <h3>&#25991;&#20214;&#32467;&#26500;</h3>
        <div class="header-actions">
            <button
                class="icon-btn"
                onclick={onSaveAll}
                disabled={isSaving || !isDirty}
                title="&#20445;&#23384;&#25152;&#26377;&#26356;&#25913; (Ctrl+S)"
                aria-label="&#20445;&#23384;&#25152;&#26377;&#26356;&#25913;"
                class:dirty={isDirty}
            >
                {isSaving ? "保存中..." : "\u{1F4BE}"}
            </button>
            <button
                class="icon-btn"
                onclick={onRefresh}
                title="&#21047;&#26032; EPUB"
                aria-label="&#21047;&#26032; EPUB"
            >
                &#128260;
            </button>
        </div>
    </div>
    <div class="tree-content">
        {#each fileTree as node (node.path)}
            <FileTreeItem
                {node}
                {expandedFolders}
                {selectedFile}
                {multiSelectedFiles}
                toggleFolder={onToggleFolder}
                selectFile={onSelectFile}
                {getFileIcon}
                {getFileDescription}
            />
        {/each}
    </div>
</aside>

<style>
    .file-tree {
        width: 240px;
        background: #fff;
        border-right: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
    }

    .tree-header {
        height: 40px; /* Matched with tabs-bar */
        padding: 0 16px;
        border-bottom: 1px solid #eee;
        background: #fafafa;
        display: flex;
        align-items: center;
        justify-content: space-between;
        box-sizing: border-box;
    }

    .tree-header h3 {
        margin: 0;
        font-size: 16px;
        color: #333;
        flex: 1;
    }

    .header-actions {
        display: flex;
        gap: 8px;
        flex-shrink: 0;
    }

    .icon-btn {
        background: transparent;
        border: none;
        cursor: pointer;
        padding: 4px;
        border-radius: 4px;
        font-size: 16px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.2s;
    }

    .icon-btn:hover:not(:disabled) {
        background: #e0e0e0;
    }

    .icon-btn:disabled {
        opacity: 0.3;
        cursor: default;
    }

    .icon-btn.dirty {
        color: #ff9800;
        filter: drop-shadow(0 0 2px rgba(255, 152, 0, 0.5));
    }

    .tree-content {
        flex: 1;
        overflow-y: auto;
        padding: 8px;
    }

    /* Modern UI overrides */
    .file-tree {
        width: 260px;
        background: rgba(255, 255, 255, 0.82);
        border-right: 1px solid var(--color-border);
        box-shadow: 10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .tree-header {
        background: rgba(255, 255, 255, 0.78);
        border-bottom: 1px solid var(--color-border);
        backdrop-filter: blur(18px) saturate(1.12);
        height: 52px;
        padding: 0 14px;
        justify-content: space-between;
    }

    .tree-header h3 {
        color: var(--color-text);
        font-size: 18px;
        font-weight: 800;
        letter-spacing: 0.01em;
    }

    .header-actions {
        gap: 8px;
    }

    .header-actions .icon-btn {
        width: 34px;
        min-width: 34px;
        height: 34px;
        padding: 0;
        white-space: nowrap;
        font-size: 16px;
        font-weight: 600;
        line-height: 1;
    }

    .icon-btn {
        width: 32px;
        height: 32px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: rgba(255, 255, 255, 0.82);
        color: var(--color-text-soft);
        box-shadow: var(--shadow-xs);
        transition:
            transform var(--transition-fast),
            border-color var(--transition-fast),
            background var(--transition-fast),
            color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .icon-btn:hover:not(:disabled) {
        background: var(--color-hover);
        border-color: var(--color-border-strong);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    .icon-btn:active:not(:disabled) {
        transform: translateY(1px) scale(0.98);
    }

    .icon-btn.dirty {
        color: var(--color-warning);
        background: var(--color-warning-soft);
        border-color: rgba(233, 155, 47, 0.38);
        filter: none;
    }

    .tree-content {
        padding: 10px;
        background: linear-gradient(
            180deg,
            rgba(255, 255, 255, 0.44),
            rgba(246, 249, 252, 0.64)
        );
    }
</style>
