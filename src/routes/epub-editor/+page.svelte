<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { page } from "$app/stores";

    interface EpubFileNode {
        name: string;
        path: string;
        file_type: string;
        size?: number;
        title?: string;
        children?: EpubFileNode[];
    }

    let epubPath = "";
    let fileTree: EpubFileNode[] = [];
    let selectedFile: EpubFileNode | null = null;
    let fileContent = "";
    let isLoading = true;
    let error = "";
    let expandedFolders: Set<string> = new Set();

    function toggleFolder(path: string) {
        if (expandedFolders.has(path)) {
            expandedFolders.delete(path);
        } else {
            expandedFolders.add(path);
        }
        expandedFolders = expandedFolders; // trigger reactivity
    }

    onMount(async () => {
        // 从 URL 参数获取 EPUB 路径
        epubPath = $page.url.searchParams.get("file") || "";

        if (!epubPath) {
            error = "未指定 EPUB 文件路径";
            isLoading = false;
            return;
        }

        try {
            // 调用后端解压 EPUB
            fileTree = await invoke<EpubFileNode[]>("extract_epub", {
                epubPath: epubPath,
            });
            isLoading = false;
        } catch (e) {
            error = `加载失败: ${e}`;
            isLoading = false;
        }
    });

    async function selectFile(file: EpubFileNode) {
        if (file.file_type === "folder") return;

        selectedFile = file;
        try {
            fileContent = await invoke<string>("read_epub_file_content", {
                epubPath: epubPath,
                filePath: file.path,
            });
        } catch (e) {
            fileContent = `读取失败: ${e}`;
        }
    }

    function getFileIcon(type: string): string {
        switch (type) {
            case "folder":
                return "📁";
            case "html":
                return "📄";
            case "css":
                return "🎨";
            case "xml":
                return "⚙️";
            case "image":
                return "🖼️";
            case "font":
                return "🔤";
            default:
                return "📎";
        }
    }

    function getFileDescription(file: EpubFileNode): string {
        // HTML 文件显示章节标题
        if (file.file_type === "html" && file.title) {
            return file.title;
        }

        // 特殊文件的描述
        const fileName = file.name.toLowerCase();
        if (fileName === "container.xml") return "容器文件";
        if (fileName === "content.opf") return "元数据";
        if (fileName.includes("toc") || fileName.includes("ncx"))
            return "目录结构";
        if (file.file_type === "css") return "样式表";
        if (file.file_type === "font")
            return `字体 ${(file.size! / 1024).toFixed(1)}KB`;
        if (file.file_type === "image") {
            return `封面 ${file.size ? `${Math.round(file.size / 1024)}KB` : ""}`;
        }

        // 默认返回文件类型
        return "";
    }

    function highlightHTML(code: string): string {
        // 先转义HTML
        let result = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");

        const placeholders: string[] = [];
        let placeholderIndex = 0;

        // 高亮注释
        result = result.replace(/(&lt;!--[\s\S]*?--&gt;)/g, (match) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                '<span class="comment">' + match + "</span>";
            return placeholder;
        });

        // 高亮标签名
        result = result.replace(/(&lt;\/?)(\w+)/g, (match, p1, p2) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                p1 + '<span class="tag">' + p2 + "</span>";
            return placeholder;
        });

        // 高亮属性名
        result = result.replace(/(\s)([\w-]+)(=)/g, (match, p1, p2, p3) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                p1 + '<span class="attr">' + p2 + "</span>" + p3;
            return placeholder;
        });

        // 高亮字符串
        result = result.replace(/="([^"]*)"/g, (match, p1) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                '="<span class="string">' + p1 + '</span>"';
            return placeholder;
        });

        // 替换所有占位符
        placeholders.forEach((value, index) => {
            result = result.replace(`___PH${index}___`, value);
        });

        return result;
    }

    function highlightCSS(code: string): string {
        // 先转义HTML
        let result = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");

        const placeholders: string[] = [];
        let placeholderIndex = 0;

        // 高亮注释
        result = result.replace(/(\/\*[\s\S]*?\*\/)/g, (match) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                '<span class="comment">' + match + "</span>";
            return placeholder;
        });

        // 高亮属性名
        result = result.replace(/([\w-]+)(\s*)(:)/g, (match, p1, p2, p3) => {
            const placeholder = `___PH${placeholderIndex}___`;
            placeholders[placeholderIndex++] =
                '<span class="property">' + p1 + "</span>" + p2 + p3;
            return placeholder;
        });

        // 替换所有占位符
        placeholders.forEach((value, index) => {
            result = result.replace(`___PH${index}___`, value);
        });

        return result;
    }
</script>

<div class="epub-editor">
    {#if isLoading}
        <div class="loading">加载中...</div>
    {:else if error}
        <div class="error">{error}</div>
    {:else}
        <!-- 左侧：文件树 -->
        <aside class="file-tree">
            <div class="tree-header">
                <h3>📚 EPUB 文件结构</h3>
            </div>
            <div class="tree-content">
                {#each fileTree as node}
                    <div class="tree-node folder-node">
                        <div
                            class="node-label"
                            on:click={() => toggleFolder(node.path)}
                            on:keydown={(e) =>
                                e.key === "Enter" && toggleFolder(node.path)}
                            role="button"
                            tabindex="0"
                        >
                            <span class="expand-icon">
                                {expandedFolders.has(node.path) ? "▼" : "▶"}
                            </span>
                            <span class="icon"
                                >{getFileIcon(node.file_type)}</span
                            >
                            <span class="name">{node.name}</span>
                        </div>
                        {#if node.children && expandedFolders.has(node.path)}
                            <div class="children">
                                {#each node.children as child}
                                    {#if child.file_type === "folder"}
                                        <!-- 嵌套文件夹 -->
                                        <div
                                            class="tree-node folder-node subfolder"
                                        >
                                            <div
                                                class="node-label"
                                                on:click={() =>
                                                    toggleFolder(child.path)}
                                                on:keydown={(e) =>
                                                    e.key === "Enter" &&
                                                    toggleFolder(child.path)}
                                                role="button"
                                                tabindex="0"
                                            >
                                                <span class="expand-icon">
                                                    {expandedFolders.has(
                                                        child.path,
                                                    )
                                                        ? "▼"
                                                        : "▶"}
                                                </span>
                                                <span class="icon"
                                                    >{getFileIcon(
                                                        child.file_type,
                                                    )}</span
                                                >
                                                <span class="name"
                                                    >{child.name}</span
                                                >
                                            </div>
                                            {#if child.children && expandedFolders.has(child.path)}
                                                <div class="children">
                                                    {#each child.children as subChild}
                                                        <div
                                                            class="tree-node file-node"
                                                            class:selected={selectedFile?.path ===
                                                                subChild.path}
                                                            on:click={() =>
                                                                selectFile(
                                                                    subChild,
                                                                )}
                                                            on:keydown={(e) =>
                                                                e.key ===
                                                                    "Enter" &&
                                                                selectFile(
                                                                    subChild,
                                                                )}
                                                            role="button"
                                                            tabindex="0"
                                                        >
                                                            <span class="icon"
                                                                >{getFileIcon(
                                                                    subChild.file_type,
                                                                )}</span
                                                            >
                                                            <div
                                                                class="file-info"
                                                            >
                                                                <span
                                                                    class="name"
                                                                >
                                                                    {subChild.name}
                                                                </span>
                                                                <span
                                                                    class="description"
                                                                >
                                                                    {getFileDescription(
                                                                        subChild,
                                                                    )}
                                                                </span>
                                                            </div>
                                                        </div>
                                                    {/each}
                                                </div>
                                            {/if}
                                        </div>
                                    {:else}
                                        <!-- 文件 -->
                                        <div
                                            class="tree-node file-node"
                                            class:selected={selectedFile?.path ===
                                                child.path}
                                            on:click={() => selectFile(child)}
                                            on:keydown={(e) =>
                                                e.key === "Enter" &&
                                                selectFile(child)}
                                            role="button"
                                            tabindex="0"
                                        >
                                            <span class="icon"
                                                >{getFileIcon(
                                                    child.file_type,
                                                )}</span
                                            >
                                            <div class="file-info">
                                                <span class="name">
                                                    {child.name}
                                                </span>
                                                <span class="description">
                                                    {getFileDescription(child)}
                                                </span>
                                            </div>
                                        </div>
                                    {/if}
                                {/each}
                            </div>
                        {/if}
                    </div>
                {/each}
            </div>
        </aside>

        <!-- 中间：编辑器 -->
        <main class="editor-pane">
            {#if selectedFile}
                <div class="editor-header">
                    <span class="file-name">{selectedFile.name}</span>
                    <span class="file-path">{selectedFile.path}</span>
                </div>
                <div class="editor-content">
                    {#if selectedFile.file_type === "html" || selectedFile.file_type === "xml"}
                        <pre class="code-block language-html"><code
                                >{@html highlightHTML(fileContent)}</code
                            ></pre>
                    {:else if selectedFile.file_type === "css"}
                        <pre class="code-block language-css"><code
                                >{@html highlightCSS(fileContent)}</code
                            ></pre>
                    {:else}
                        <pre class="code-block">{fileContent}</pre>
                    {/if}
                </div>
            {:else}
                <div class="placeholder">点击左侧文件以查看内容</div>
            {/if}
        </main>

        <!-- 右侧：预览 -->
        <aside class="preview-pane">
            {#if selectedFile?.file_type === "html"}
                <div class="preview-header">
                    <h4>📖 预览</h4>
                </div>
                <div class="preview-content">
                    {@html fileContent}
                </div>
            {:else}
                <div class="placeholder">
                    {selectedFile
                        ? "仅支持预览 HTML 文件"
                        : "选择 HTML 文件以预览"}
                </div>
            {/if}
        </aside>
    {/if}
</div>

<style>
    .epub-editor {
        display: flex;
        height: 100vh;
        background: #f5f5f5;
    }

    .loading,
    .error {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        font-size: 18px;
    }

    .error {
        color: #d32f2f;
    }

    /* 文件树 */
    .file-tree {
        width: 300px;
        background: #fff;
        border-right: 1px solid #ddd;
        display: flex;
        flex-direction: column;
    }

    .tree-header {
        padding: 16px;
        border-bottom: 1px solid #eee;
        background: #fafafa;
    }

    .tree-header h3 {
        margin: 0;
        font-size: 16px;
        color: #333;
    }

    .tree-content {
        flex: 1;
        overflow-y: auto;
        padding: 8px;
    }

    .tree-node {
        margin: 4px 0;
    }

    .folder-node {
        margin-bottom: 12px;
    }

    .node-label {
        display: flex;
        align-items: center;
        padding: 8px;
        font-weight: 600;
        color: #555;
        background: #f0f0f0;
        border-radius: 4px;
        cursor: pointer;
        user-select: none;
    }

    .node-label:hover {
        background: #e8e8e8;
    }

    .expand-icon {
        margin-right: 4px;
        font-size: 12px;
        color: #666;
        width: 16px;
        display: inline-block;
    }

    .subfolder {
        margin-left: 16px;
    }

    .subfolder .node-label {
        background: #f8f8f8;
        font-weight: 500;
        font-size: 13px;
    }

    .file-node {
        display: flex;
        align-items: center;
        padding: 8px 8px 8px 24px;
        cursor: pointer;
        border-radius: 4px;
        transition: background 0.2s;
    }

    .file-node:hover {
        background: #f5f5f5;
    }

    .file-node.selected {
        background: #e3f2fd;
        border-left: 3px solid #2196f3;
    }

    .icon {
        margin-right: 8px;
        font-size: 18px;
    }

    .file-info {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
    }

    .name {
        font-size: 14px;
        color: #333;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .description {
        font-size: 12px;
        color: #999;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .children {
        margin-top: 4px;
    }

    /* 编辑器 */
    .editor-pane {
        flex: 1;
        display: flex;
        flex-direction: column;
        background: #fff;
        border-right: 1px solid #ddd;
    }

    .editor-header {
        padding: 12px 16px;
        background: #fafafa;
        border-bottom: 1px solid #eee;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }

    .file-name {
        font-weight: 600;
        color: #333;
    }

    .file-path {
        font-size: 12px;
        color: #999;
    }

    .editor-content {
        flex: 1;
        overflow: auto;
        padding: 16px;
        background: #fff;
    }

    .code-block {
        margin: 0;
        font-family: "Consolas", "Monaco", "Courier New", monospace;
        font-size: 14px;
        line-height: 1.6;
        white-space: pre-wrap;
        word-wrap: break-word;
        color: #000;
    }

    .code-block code {
        display: block;
    }

    /* 语法高亮颜色 - 浅色主题 */
    .code-block :global(.tag) {
        color: #0000ff;
        font-weight: 600;
    }

    .code-block :global(.attr) {
        color: #ff0000;
    }

    .code-block :global(.string) {
        color: #0451a5;
    }

    .code-block :global(.comment) {
        color: #008000;
        font-style: italic;
    }

    .code-block :global(.property) {
        color: #ff0000;
    }

    .code-block :global(.value) {
        color: #0451a5;
    }

    .code-block :global(.selector) {
        color: #800000;
        font-weight: 600;
    }

    .code-block :global(.keyword) {
        color: #af00db;
    }

    .editor-content pre {
        margin: 0;
        font-family: "Consolas", "Monaco", monospace;
        font-size: 14px;
        line-height: 1.6;
        white-space: pre-wrap;
        word-wrap: break-word;
    }

    /* 预览 */
    .preview-pane {
        width: 400px;
        background: #fff;
        display: flex;
        flex-direction: column;
    }

    .preview-header {
        padding: 12px 16px;
        background: #fafafa;
        border-bottom: 1px solid #eee;
    }

    .preview-header h4 {
        margin: 0;
        font-size: 14px;
        color: #333;
    }

    .preview-content {
        flex: 1;
        overflow: auto;
        padding: 16px;
        background: #fefefe;
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        color: #999;
        font-size: 14px;
    }

    /* 滚动条美化 */
    ::-webkit-scrollbar {
        width: 8px;
        height: 8px;
    }

    ::-webkit-scrollbar-track {
        background: #f1f1f1;
    }

    ::-webkit-scrollbar-thumb {
        background: #888;
        border-radius: 4px;
    }

    ::-webkit-scrollbar-thumb:hover {
        background: #555;
    }
</style>
