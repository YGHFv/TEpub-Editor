<script lang="ts">
    export let node: any;
    export let expandedFolders: Set<string>;
    export let selectedFile: any;
    export let multiSelectedFiles: Set<string>;
    export let toggleFolder: (path: string) => void;
    export let selectFile: (
        file: any,
        event?: MouseEvent | KeyboardEvent,
    ) => void;
    export let getFileIcon: (type: string) => string;
    export let getFileDescription: (node: any) => string;

    $: isExpanded = expandedFolders.has(node.path);
    $: isSelected =
        selectedFile?.path === node.path || multiSelectedFiles.has(node.path);
</script>

{#if node.file_type === "folder"}
    <div
        class="tree-node folder-node"
        class:subpath={node.path.includes("/")}
        data-context-type="folder"
        data-path={node.path}
        data-folder-type={node.name}
    >
        <div
            class="node-label"
            on:click={() => toggleFolder(node.path)}
            on:keydown={(e) => e.key === "Enter" && toggleFolder(node.path)}
            role="button"
            tabindex="0"
        >
            <span class="expand-icon">
                {isExpanded ? "▼" : "▶"}
            </span>
            <span class="icon">{getFileIcon(node.file_type)}</span>
            <span class="name">{node.name}</span>
        </div>
        {#if node.children && isExpanded}
            <div class="children">
                {#each node.children as child (child.path)}
                    <svelte:self
                        node={child}
                        {expandedFolders}
                        {selectedFile}
                        {multiSelectedFiles}
                        {toggleFolder}
                        {selectFile}
                        {getFileIcon}
                        {getFileDescription}
                    />
                {/each}
            </div>
        {/if}
    </div>
{:else}
    <div
        class="tree-node file-node"
        data-path={node.path}
        data-context-type="file"
        class:selected={isSelected}
        on:click={(e) => selectFile(node, e)}
        on:keydown={(e) => e.key === "Enter" && selectFile(node, e)}
        role="button"
        tabindex="0"
    >
        <span class="icon">{getFileIcon(node.file_type)}</span>
        <div class="file-info">
            <span class="name">{node.name}</span>
            <span class="description">{getFileDescription(node)}</span>
        </div>
    </div>
{/if}

<style>
    .tree-node {
        margin: 2px 0;
    }

    .folder-node {
        margin-bottom: 4px;
    }

    .node-label {
        display: flex;
        align-items: center;
        padding: 6px 8px;
        font-weight: 600;
        color: #555;
        background: #f0f0f0;
        border-radius: 4px;
        cursor: pointer;
        user-select: none;
        font-size: 14px;
    }

    .node-label:hover {
        background: #e8e8e8;
    }

    .expand-icon {
        margin-right: 4px;
        font-size: 10px;
        color: #666;
        width: 14px;
        display: inline-block;
        text-align: center;
    }

    /* 子文件夹样式 */
    .children {
        margin-left: 14px;
        border-left: 1px solid #eee;
        padding-left: 4px;
    }

    .folder-node.subpath > .node-label {
        background: transparent;
        font-weight: 500;
        font-size: 13px;
        padding: 4px 8px;
    }

    .folder-node.subpath > .node-label:hover {
        background: #f5f5f5;
    }

    .file-node {
        display: flex;
        align-items: center;
        padding: 4px 8px 4px 18px;
        cursor: pointer;
        border-radius: 4px;
        transition: background 0.1s;
    }

    .file-node:hover {
        background: #f5f5f5;
    }

    .file-node.selected {
        background: #e3f2fd;
        color: #1976d2;
    }

    .icon {
        margin-right: 8px;
        font-size: 16px;
        flex-shrink: 0;
    }

    .file-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 0px;
        min-width: 0;
    }

    .name {
        font-size: 13px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .description {
        font-size: 11px;
        color: #999;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
</style>
