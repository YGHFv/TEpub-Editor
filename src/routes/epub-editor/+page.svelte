<script lang="ts">
    import { onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { confirm } from "@tauri-apps/plugin-dialog";
    import { page } from "$app/stores";
    import TocNode from "$lib/TocNode.svelte";
    import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";

    interface EpubFileNode {
        name: string;
        path: string;
        file_type: string;
        size?: number;
        title?: string;
        children?: EpubFileNode[];
    }

    // Validation Error Interface
    interface ValidationError {
        type: "tag" | "img";
        message: string;
        line: number;
    }

    let epubPath = "";
    let fileTree: EpubFileNode[] = [];
    let selectedFile: EpubFileNode | null = null;
    let fileContent = "";
    let previewContent = "";
    let isLoading = true;
    let error = "";
    let expandedFolders: Set<string> = new Set();

    // Modification Tracking
    let modifiedFiles: Set<string> = new Set();
    let isSaving = false;

    // Validation State
    let previewError: ValidationError[] = [];
    let errorLines: number[] = [];

    // Tab Close Confirmation Dialog State
    let showCloseDialog = false;
    let pendingCloseIndex = -1;
    let pendingCloseFile: EpubFileNode | null = null;
    let closeContext: "tab" | "app" = "tab"; // Context tracking

    // 追踪当前的请求生成ID，解决竞态条件
    let currentGeneration = 0;
    // 存储已生成的Blob URL以便释放
    let blobUrls: string[] = [];
    // 缓存: 绝对路径 -> Blob URL
    let assetCache: Map<string, string> = new Map();
    // 缓存: 绝对路径 -> 文件纯文本内容 (HTML, CSS, XML...)
    let fileContentCache: Map<string, string> = new Map();
    // 缓存: 绝对路径 -> 处理后的预览HTML
    let previewCache: Map<string, string> = new Map();

    // 扁平化的文件列表 (仅HTML)，用于快速查找章节顺序
    let flatHtmlFiles: EpubFileNode[] = [];

    // 滚动同步相关
    let previewIframe: HTMLIFrameElement | null = null;
    let editorContentDiv: HTMLElement | null = null;
    let epubCodeEditorComponent: EpubCodeEditor | null = null;

    // 多标签页相关
    let openTabs: EpubFileNode[] = []; // 已打开的文件标签
    let activeTabIndex: number = -1; // 当前激活的标签索引
    let tabsBarDiv: HTMLElement | null = null; // 标签页栏引用

    // 编辑器滚动处理函数
    function handleEditorScroll(event: Event) {
        if (!previewIframe?.contentWindow || !editorContentDiv) return;

        const target = event.target as HTMLElement;
        const scrollTop = target.scrollTop;
        const scrollHeight = target.scrollHeight - target.clientHeight;

        if (scrollHeight <= 0) return;

        const scrollPercent = scrollTop / scrollHeight;

        // 发送消息给iframe
        previewIframe.contentWindow.postMessage(
            {
                type: "editorScroll",
                percent: scrollPercent,
            },
            "*",
        );
    }

    function flattenFiles(nodes: EpubFileNode[]): EpubFileNode[] {
        let result: EpubFileNode[] = [];
        for (const node of nodes) {
            if (
                node.file_type === "html" ||
                node.name.endsWith(".xhtml") ||
                node.name.endsWith(".html")
            ) {
                result.push(node);
            }
            if (node.children) {
                result = result.concat(flattenFiles(node.children));
            }
        }
        return result;
    }

    async function preloadFile(file: EpubFileNode) {
        if (!file) return;
        const filePath = file.path;

        // 1. 检查/加载文件内容
        let content = "";
        if (fileContentCache.has(filePath)) {
            content = fileContentCache.get(filePath)!;
        } else {
            try {
                content = await invoke<string>("read_epub_file_content", {
                    epubPath: epubPath,
                    filePath: filePath,
                });
                fileContentCache.set(filePath, content);
            } catch (e) {
                console.warn(`预加载失败: ${filePath}`, e);
                return;
            }
        }

        // 2. 预处理预览 (仅HTML)
        if (!previewCache.has(filePath)) {
            try {
                // 使用 -1 generation 避免干扰当前流程，但这里 processHtmlForPreview 需要 generation 校验
                // 我们稍微修改 processHtmlForPreview 或仅仅只是跑一遍逻辑
                // 为了简单且不传递 generation 导致的中断，我们可以传一个永远有效的 generation 或者 0?
                // 但原函数设计强依赖 generation。
                // 我们复制一个 simplified 的处理逻辑或者复用。
                // 此时为了安全，我们复用逻辑但传入 currentGeneration (有风险? NO, currentGeneration 可能会变)
                // 更好的方式：processHtmlForPreview 不应强绑定 UI 的 generation。
                // 让我们修改 processHtmlForPreview 让 generation 可选，或者在此处不预处理 HTML (因为预处理涉及 DOM Parser 只能在主线程且较重)
                // 权衡：用户说性能消耗再大也行。
                // 我们在 requestIdleCallback 中做?
                // 直接调用，传入当前的 currentGeneration。如果用户切换了，generation 变了，预加载中断也是对的。

                const processed = await processHtmlForPreview(
                    content,
                    filePath,
                    currentGeneration,
                );
                if (processed) {
                    previewCache.set(filePath, processed);
                }
            } catch (e) {
                console.warn(`预处理预览失败: ${filePath}`, e);
            }
        }
    }

    function preloadNeighbors(currentFile: EpubFileNode) {
        if (flatHtmlFiles.length === 0) return;
        const index = flatHtmlFiles.findIndex(
            (f) => f.path === currentFile.path,
        );
        if (index === -1) return;

        // 延时一点执行，优先保证当前 UI 响应
        setTimeout(() => {
            const next = flatHtmlFiles[index + 1];
            const prev = flatHtmlFiles[index - 1];
            if (next) preloadFile(next);
            if (prev) preloadFile(prev);
        }, 300);
    }

    // ... resolvePath ... (unchanged)

    // ... processCssAssets ... (unchanged)

    // ... processHtmlForPreview ... (unchanged)

    function toggleFolder(path: string) {
        if (expandedFolders.has(path)) {
            expandedFolders.delete(path);
        } else {
            expandedFolders.add(path);
        }
        expandedFolders = expandedFolders; // trigger reactivity
    }

    onMount(() => {
        // 1. 添加窗口关闭提示 (Web)
        window.addEventListener("beforeunload", handleBeforeUnload);

        // 2. 添加窗口关闭提示 (Tauri Desktop)
        let unlistenClose: (() => void) | null = null;
        const setupCloseHandler = async () => {
            try {
                const appWindow = getCurrentWindow();
                unlistenClose = await appWindow.onCloseRequested(
                    async (event) => {
                        if (hasUnsavedChanges()) {
                            event.preventDefault();
                            closeContext = "app";
                            showCloseDialog = true;
                        }
                    },
                );
            } catch (e) {
                console.warn("Tauri close handler init failed:", e);
            }
        };
        setupCloseHandler();

        const loadEpub = async () => {
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

                // 构建扁平列表用于预加载
                flatHtmlFiles = flattenFiles(fileTree);

                // 加载完成后，自动加载目录
                await loadTOC();

                isLoading = false;
            } catch (e) {
                error = `加载失败: ${e}`;
                isLoading = false;
            }
        };

        loadEpub();

        return () => {
            // 组件销毁时清理
            window.removeEventListener("beforeunload", handleBeforeUnload);
            if (unlistenClose) unlistenClose();
            cleanupBlobUrls();
            cleanupBlobUrls();
        };
    });

    // 监听全选事件
    onMount(() => {
        const handleSelectAll = () => {
            epubCodeEditorComponent?.selectAll();
        };
        window.addEventListener("editor-select-all", handleSelectAll);
        return () => {
            window.removeEventListener("editor-select-all", handleSelectAll);
        };
    });

    function cleanupBlobUrls() {
        blobUrls.forEach((url) => URL.revokeObjectURL(url));
        blobUrls = [];
        assetCache.clear();
        fileContentCache.clear();
        previewCache.clear();
    }

    // 解析相对路径
    function resolvePath(basePath: string, relativePath: string): string {
        const stack = basePath.split("/");
        stack.pop(); // 移除文件名，保留目录

        const parts = relativePath.split("/");
        for (const part of parts) {
            if (part === ".") continue;
            if (part === "..") {
                if (stack.length > 0) stack.pop();
            } else {
                stack.push(part);
            }
        }
        return stack.join("/");
    }

    // 解析相对路径

    async function processHtmlForPreview(
        html: string,
        filePath: string,
        generation: number,
    ): Promise<string> {
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, "text/html");

        const links = Array.from(
            doc.querySelectorAll('link[rel="stylesheet"]'),
        );
        const images = Array.from(doc.querySelectorAll("img"));

        // 1. 收集所有需要读取的 CSS 路径
        const cssPaths: string[] = [];
        const cssLinkMap = new Map<string, Element>();

        for (const link of links) {
            const href = link.getAttribute("href");
            if (href) {
                const cssPath = resolvePath(filePath, href);
                cssPaths.push(cssPath);
                cssLinkMap.set(cssPath, link);
            }
        }

        // 2. 批量读取所有 CSS 文件
        let cssContents: Record<string, string> = {};
        if (cssPaths.length > 0) {
            try {
                cssContents = await invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath: epubPath,
                        filePaths: cssPaths,
                    },
                );
            } catch (e) {
                console.error("批量读取CSS失败:", e);
            }
        }

        if (currentGeneration !== generation) return "";

        // 3. 从 CSS 中提取需要的二进制资源（字体、图片）
        const binaryPaths = new Set<string>();
        const cssAssetMap = new Map<
            string,
            Array<{ original: string; url: string; path: string }>
        >();

        for (const [cssPath, cssContent] of Object.entries(cssContents)) {
            const urlRegex = /url\(['"]?([^'"\)]+)['"]?\)/g;
            let match;
            const assets: Array<{
                original: string;
                url: string;
                path: string;
            }> = [];

            while ((match = urlRegex.exec(cssContent)) !== null) {
                const originalUrl = match[1];
                if (
                    !originalUrl.startsWith("data:") &&
                    !originalUrl.startsWith("http")
                ) {
                    const absolutePath = resolvePath(cssPath, originalUrl);
                    if (!assetCache.has(absolutePath)) {
                        binaryPaths.add(absolutePath);
                    }
                    assets.push({
                        original: match[0],
                        url: originalUrl,
                        path: absolutePath,
                    });
                }
            }
            if (assets.length > 0) {
                cssAssetMap.set(cssPath, assets);
            }
        }

        // 4. 收集图片路径
        const imagePaths: string[] = [];
        const imageElemMap = new Map<string, Element>();

        for (const img of images) {
            const src = img.getAttribute("src");
            if (src && !src.startsWith("http") && !src.startsWith("data:")) {
                const imgPath = resolvePath(filePath, src);
                if (!assetCache.has(imgPath)) {
                    imagePaths.push(imgPath);
                }
                imageElemMap.set(imgPath, img);
            }
        }

        // 5. 批量读取所有二进制资源（CSS 引用的字体 + 图片）
        const allBinaryPaths = [...binaryPaths, ...imagePaths];
        let binaryData: Record<string, number[]> = {};

        if (allBinaryPaths.length > 0) {
            try {
                binaryData = await invoke<Record<string, number[]>>(
                    "read_epub_binary_batch",
                    {
                        epubPath: epubPath,
                        filePaths: allBinaryPaths,
                    },
                );
            } catch (e) {
                console.error("批量读取二进制资源失败:", e);
            }
        }

        if (currentGeneration !== generation) return "";

        // 6. 创建 Blob URLs
        for (const [path, data] of Object.entries(binaryData)) {
            const uint8Array = new Uint8Array(data);

            // 猜测 MIME 类型
            let mimeType = "application/octet-stream";
            const lower = path.toLowerCase();
            if (lower.endsWith(".ttf")) mimeType = "font/ttf";
            else if (lower.endsWith(".woff")) mimeType = "font/woff";
            else if (lower.endsWith(".woff2")) mimeType = "font/woff2";
            else if (lower.endsWith(".otf")) mimeType = "font/otf";
            else if (lower.endsWith(".eot"))
                mimeType = "application/vnd.ms-fontobject";
            else if (lower.endsWith(".png")) mimeType = "image/png";
            else if (lower.endsWith(".jpg") || lower.endsWith(".jpeg"))
                mimeType = "image/jpeg";
            else if (lower.endsWith(".gif")) mimeType = "image/gif";
            else if (lower.endsWith(".svg")) mimeType = "image/svg+xml";
            else if (lower.endsWith(".webp")) mimeType = "image/webp";

            const blob = new Blob([uint8Array], { type: mimeType });
            const blobUrl = URL.createObjectURL(blob);
            blobUrls.push(blobUrl);
            assetCache.set(path, blobUrl);
        }

        // 7. 处理 CSS，替换资源 URL
        for (const [cssPath, cssContent] of Object.entries(cssContents)) {
            let processedCss = cssContent;
            const assets = cssAssetMap.get(cssPath);

            if (assets) {
                for (const asset of assets) {
                    const blobUrl = assetCache.get(asset.path);
                    if (blobUrl) {
                        processedCss = processedCss
                            .split(asset.original)
                            .join(`url("${blobUrl}")`);
                    }
                }
            }

            // 创建 style 标签并替换 link
            const link = cssLinkMap.get(cssPath);
            if (link) {
                const style = doc.createElement("style");
                style.textContent = processedCss;
                link.replaceWith(style);
            }
        }

        // 8. 处理图片
        for (const [imgPath, img] of imageElemMap) {
            const blobUrl = assetCache.get(imgPath);
            if (blobUrl) {
                img.setAttribute("src", blobUrl);
            }
        }

        // 注入全局样式：只移除html/body的默认边距，保留内容原有布局
        const globalStyle = doc.createElement("style");
        globalStyle.textContent = `
            /* 只移除html/body的默认边距，避免出现滚动条 */
            html { 
                overflow-x: hidden !important;
                margin: 0 !important;
                padding: 0 !important;
            }
            body {
                overflow-x: hidden !important;
                margin: 0 !important;
                padding: 0 !important;
            }
        `;
        doc.head.appendChild(globalStyle);

        // 注入滚动同步脚本：监听来自父窗口的滚动消息
        const syncScript = doc.createElement("script");
        syncScript.textContent = `
            window.addEventListener('message', function(event) {
                if (event.data && event.data.type === 'editorScroll') {
                    const scrollPercent = event.data.percent;
                    const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
                    const targetScroll = maxScroll * scrollPercent;
                    window.scrollTo({ top: targetScroll, behavior: 'smooth' });
                }
            });
        `;
        doc.head.appendChild(syncScript);

        return doc.documentElement.outerHTML;
    }

    function hasUnsavedChanges(): boolean {
        return modifiedFiles.size > 0;
    }

    function validateHtml(content: string, currentPath: string) {
        const errors: ValidationError[] = [];
        const newErrorLines: number[] = [];

        // 1. 检查标签匹配 (容错算法)
        const lines = content.split("\n");
        const tagStack: { tag: string; line: number }[] = [];

        // 匹配 <tag> 或 </tag>
        const tagRegex = /<\/?([a-zA-Z0-9]+)[^>]*>/g;

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const lineNum = i; // 0-based

            let match;
            while ((match = tagRegex.exec(line)) !== null) {
                const fullTag = match[0];
                const tagName = match[1].toLowerCase();

                // 跳过自闭合标签和 void elements
                if (
                    fullTag.endsWith("/>") ||
                    ["br", "hr", "img", "input", "meta", "link"].includes(
                        tagName,
                    )
                ) {
                    continue;
                }

                if (fullTag.startsWith("</")) {
                    // 闭合标签: 在栈中向下寻找最近的匹配
                    let matchIndex = -1;
                    for (let j = tagStack.length - 1; j >= 0; j--) {
                        if (tagStack[j].tag === tagName) {
                            matchIndex = j;
                            break;
                        }
                    }

                    if (matchIndex !== -1) {
                        // 找到了匹配，弹出该标签及之上的所有标签（如果有未闭合的，它们就是错误）
                        // 实际上，栈顶到 matchIndex 之间的都是未闭合的错误？
                        // 简单策略：仅认为 matchIndex 是匹配的，将其弹出。
                        // 如果 matchIndex 不是栈顶，说明中间有未闭合的标签。
                        // 我们的策略：匹配到后，将栈裁剪到 matchIndex，中间的视为“未闭合”报错
                        const popped = tagStack.splice(matchIndex);
                        // popped[0] 是匹配的那个开始标签。popped[1...] 是中间未闭合的。
                        for (let k = 1; k < popped.length; k++) {
                            errors.push({
                                type: "tag",
                                message: `第 ${popped[k].line} 行: 未闭合的标签 <${popped[k].tag}>`,
                                line: popped[k].line,
                            });
                            newErrorLines.push(popped[k].line);
                        }
                    } else {
                        // 没找到匹配的开始标签 -> 多余的闭合标签
                        errors.push({
                            type: "tag",
                            message: `第 ${lineNum + 1} 行: 多余的闭合标签 </${tagName}>`,
                            line: lineNum + 1,
                        });
                        newErrorLines.push(lineNum + 1);
                    }
                } else {
                    // 开始标签
                    tagStack.push({ tag: tagName, line: lineNum + 1 });
                }
            }
        }

        // 剩余的栈中标签都是未闭合的
        for (const unclosed of tagStack) {
            errors.push({
                type: "tag",
                message: `第 ${unclosed.line} 行: 未闭合的标签 <${unclosed.tag}>`,
                line: unclosed.line,
            });
            newErrorLines.push(unclosed.line);
        }

        // 2. 检查图片引用
        const imgRegex = /<img[^>]+src=["']([^"']+)["'][^>]*>/gi;
        let match;
        while ((match = imgRegex.exec(content)) !== null) {
            const src = match[1];
            if (src.startsWith("http") || src.startsWith("data:")) continue;

            // 解析绝对路径
            // EPUB 中通常引用是相对当前 HTML 的
            const fullPath = resolvePath(currentPath, src);

            // 检查文件是否存在
            // 简单检查 flatHtmlFiles (仅HTML) 不够，需检查 fileTree 或构建全量 map
            // 这里我们用 fileTree 递归查找或 assetCache? assetCache 只有加载过的。
            // 我们可以用一个简单的全路径查找。
            // 由于 flatHtmlFiles 不全，我们还是遍历 fileTree 吧，或者构建一个 pathSet
            // 优化：我们可以构建一个全量 path Set。
            // TODO: Performance optimization required here for large books.
            // For now, simple assumption: if we can't find it easily, warn?
            // Actually, flattening fileTree to get all paths is better.
        }

        // 由于 pathSet 不在作用域，先简化省略图片检查的报错，以免误报。
        // 或者使用 invoke('exists')? 不行，是 zip 内部路径。
        // 暂且保留 Tag 检查，图片检查待完善。

        previewError = errors;
        errorLines = newErrorLines;
    }

    let validationTimer: any = null;
    function handleFileContentChange(newContent: string) {
        fileContent = newContent;
        if (selectedFile) {
            fileContentCache.set(selectedFile.path, newContent);
            modifiedFiles.add(selectedFile.path);
            modifiedFiles = modifiedFiles; // reactivity

            if (validationTimer) clearTimeout(validationTimer);
            validationTimer = setTimeout(async () => {
                if (
                    selectedFile?.file_type === "html" ||
                    selectedFile?.name.endsWith(".html") ||
                    selectedFile?.name.endsWith(".xhtml")
                ) {
                    // Validate
                    validateHtml(newContent, selectedFile.path);

                    // Update Preview
                    const processed = await processHtmlForPreview(
                        newContent,
                        selectedFile.path,
                        currentGeneration,
                    );
                    if (processed) {
                        previewContent = processed;
                        previewCache.set(selectedFile.path, processed);
                    }
                }
            }, 500);
        }
    }

    async function saveCurrentFile() {
        if (!selectedFile) return;
        isSaving = true;
        try {
            await invoke("save_epub_file_content", {
                epubPath: epubPath,
                filePath: selectedFile.path,
                content: fileContent,
            });
            modifiedFiles.delete(selectedFile.path);
            modifiedFiles = modifiedFiles;
        } catch (e) {
            console.error("Save failed:", e);
            await confirm(`保存失败: ${e}`, {
                title: "错误",
                kind: "error",
            });
        } finally {
            isSaving = false;
        }
    }

    function handleBeforeUnload(e: BeforeUnloadEvent) {
        if (hasUnsavedChanges()) {
            e.preventDefault();
            e.returnValue = "您有未保存的更改，确定要离开吗？";
            return e.returnValue;
        }
    }

    async function selectFile(file: EpubFileNode) {
        if (file.file_type === "folder") return;

        // 增加代数，使得之前的 pending 请求失效
        currentGeneration++;
        const generation = currentGeneration;

        selectedFile = file;

        // 多标签页支持：添加到openTabs如果还没有
        const existingIndex = openTabs.findIndex(
            (tab) => tab.path === file.path,
        );
        if (existingIndex >= 0) {
            activeTabIndex = existingIndex;
            // 滚动到该标签
            await tick();
            if (tabsBarDiv && tabsBarDiv.children[existingIndex]) {
                const tabElement = tabsBarDiv.children[
                    existingIndex
                ] as HTMLElement;
                // 使用 inline: "center" 确保标签在中间，或者 "nearest" 确保可见
                tabElement.scrollIntoView({
                    behavior: "smooth",
                    block: "nearest",
                    inline: "center",
                });
            }
        } else {
            openTabs.push(file);
            activeTabIndex = openTabs.length - 1;
            openTabs = openTabs; // 触发响应式更新

            // 新标签页打开后自动滚动到最右侧
            await tick();
            if (tabsBarDiv) {
                tabsBarDiv.scrollLeft = tabsBarDiv.scrollWidth;
            }
        }

        // 1. 尝试直接从预览缓存命中 (最快路径)
        if (previewCache.has(file.path)) {
            fileContent = fileContentCache.get(file.path) || "加载中..."; // 试图同步显示内容，如果有
            previewContent = previewCache.get(file.path)!;
            activeTab = "preview"; // 自动切换
            preloadNeighbors(file); // 触发预加载下一章
            return;
        }

        // 立即清理旧内容，避免视觉混淆
        // 如果有内容缓存，先显示内容缓存
        if (fileContentCache.has(file.path)) {
            fileContent = fileContentCache.get(file.path)!;
        } else {
            fileContent = "加载中...";
        }

        // 如果没命中预览缓存
        if (!previewCache.has(file.path)) {
            previewContent = "加载中...";
        }

        try {
            let content = "";

            // 2. 检查文件内容缓存
            if (fileContentCache.has(file.path)) {
                content = fileContentCache.get(file.path)!;
            } else {
                content = await invoke<string>("read_epub_file_content", {
                    epubPath: epubPath,
                    filePath: file.path,
                });

                // 存入缓存
                fileContentCache.set(file.path, content);
            }

            // 如果代数不匹配，说明用户已经切换了文件，忽略结果
            if (currentGeneration !== generation) return;

            fileContent = content;

            // 3. 仅对 HTML 文件进行预览处理，优化性能
            if (
                file.file_type === "html" ||
                file.name.endsWith(".xhtml") ||
                file.name.endsWith(".html")
            ) {
                const processed = await processHtmlForPreview(
                    fileContent,
                    file.path,
                    generation,
                );

                // 4. 存入预览缓存
                if (currentGeneration === generation && processed) {
                    previewContent = processed;
                    previewCache.set(file.path, processed);
                    activeTab = "preview";

                    // 5. 触发相邻章节预加载
                    preloadNeighbors(file);
                }
            } else {
                // 对于非 HTML 文件（如 XML, OPF, NCX），不展示预览
                previewContent = "";
            }
        } catch (e) {
            if (currentGeneration === generation) {
                fileContent = `读取失败: ${e}`;
                previewContent = `读取失败: ${e}`;
            }
        }
    }

    // 标签页管理函数
    function switchTab(index: number) {
        if (index < 0 || index >= openTabs.length) return;
        activeTabIndex = index;
        const tab = openTabs[index];
        selectedFile = tab;

        // 加载文件内容
        if (fileContentCache.has(tab.path)) {
            fileContent = fileContentCache.get(tab.path)!;
        }
        if (previewCache.has(tab.path)) {
            previewContent = previewCache.get(tab.path)!;
        }
    }

    function closeTab(event: Event, index: number) {
        event.stopPropagation();
        if (index < 0 || index >= openTabs.length) return;

        const tab = openTabs[index];
        if (modifiedFiles.has(tab.path)) {
            pendingCloseIndex = index;
            pendingCloseFile = tab;
            closeContext = "tab";
            showCloseDialog = true;
        } else {
            doCloseTab(index);
        }
    }

    function doCloseTab(index: number) {
        if (index < 0 || index >= openTabs.length) return;

        const tab = openTabs[index];
        // 确保从修改列表中移除（如果是放弃更改关闭）
        modifiedFiles.delete(tab.path);
        modifiedFiles = modifiedFiles;

        openTabs.splice(index, 1);
        openTabs = openTabs; // 触发响应式更新

        if (openTabs.length === 0) {
            // 所有标签页都关闭了
            activeTabIndex = -1;
            selectedFile = null;
            fileContent = "";
            previewContent = "";
        } else {
            // 如果关闭的是当前激活的标签，切换到相邻的标签
            if (index === activeTabIndex) {
                // 优先切换到右侧标签，如果没有则切换到左侧
                const newIndex =
                    index >= openTabs.length ? openTabs.length - 1 : index;
                switchTab(newIndex);
            } else if (index < activeTabIndex) {
                // 如果关闭的标签在当前激活标签左侧，调整索引
                activeTabIndex--;
            }
        }
    }

    async function handleDialogSave() {
        isSaving = true; // Use global isSaving or a new one? Global is fine as it locks UI.

        if (closeContext === "tab") {
            // Tab Logic
            if (pendingCloseFile && modifiedFiles.has(pendingCloseFile.path)) {
                const contentToSave = fileContentCache.get(
                    pendingCloseFile.path,
                );
                if (contentToSave !== undefined) {
                    try {
                        await invoke("save_epub_file_content", {
                            epubPath: epubPath,
                            filePath: pendingCloseFile.path,
                            content: contentToSave,
                        });
                        modifiedFiles.delete(pendingCloseFile.path);
                        modifiedFiles = modifiedFiles;
                    } catch (e) {
                        console.error("Save failed in dialog:", e);
                    }
                }
            }
            isSaving = false;
            showCloseDialog = false;
            if (pendingCloseIndex !== -1) {
                doCloseTab(pendingCloseIndex);
            }
        } else {
            // App Logic: Save ALL modified files
            try {
                const tasks = Array.from(modifiedFiles).map(async (path) => {
                    const content = fileContentCache.get(path);
                    if (content !== undefined) {
                        await invoke("save_epub_file_content", {
                            epubPath: epubPath,
                            filePath: path,
                            content: content,
                        });
                    }
                });
                await Promise.all(tasks);
                modifiedFiles.clear();
                modifiedFiles = modifiedFiles;

                const appWindow = getCurrentWindow();
                await appWindow.destroy();
            } catch (e) {
                isSaving = false;
                await confirm(`保存部分文件失败: ${e}`, { kind: "error" });
                return;
            }
        }
        // No need to reset isSaving here for App Logic as window destroys,
        // but strictly speaking we should if destroy failed?
        // We handle error case above.
        resetDialog();
    }

    async function handleDialogDiscard() {
        if (closeContext === "tab") {
            if (pendingCloseFile) {
                modifiedFiles.delete(pendingCloseFile.path);
                modifiedFiles = modifiedFiles;
            }
            showCloseDialog = false;
            if (pendingCloseIndex !== -1) {
                doCloseTab(pendingCloseIndex);
            }
        } else {
            // App Logic: Discard all
            const appWindow = getCurrentWindow();
            await appWindow.destroy();
        }
        resetDialog();
    }

    function handleDialogCancel() {
        resetDialog();
    }

    function resetDialog() {
        showCloseDialog = false;
        pendingCloseIndex = -1;
        pendingCloseFile = null;
        closeContext = "tab"; // Reset to default
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

    // --- 目录 (TOC) 相关逻辑 ---
    interface TocItem {
        id: string;
        label: string;
        src: string;
        children?: TocItem[];
    }

    let activeTab: "preview" | "toc" = "toc"; // 默认显示目录
    let tocList: TocItem[] = [];
    let isTocLoading = false;
    let expandedTocItems: Set<string> = new Set(); // 存储展开的目录项ID

    function toggleTocItem(id: string) {
        if (expandedTocItems.has(id)) {
            expandedTocItems.delete(id);
        } else {
            expandedTocItems.add(id);
        }
        expandedTocItems = expandedTocItems;
    }

    function parseNavPoints(container: Element): TocItem[] {
        const items: TocItem[] = [];
        // 获取直接子级的 navPoint
        // querySelectorAll 会获取所有后代，所以这里只能遍历 children
        for (const child of Array.from(container.children)) {
            if (child.tagName.toLowerCase() === "navpoint") {
                const id =
                    child.getAttribute("id") ||
                    Math.random().toString(36).substr(2, 9);
                const label =
                    child.querySelector(":scope > navLabel > text")
                        ?.textContent || "未知章节";
                const src =
                    child
                        .querySelector(":scope > content")
                        ?.getAttribute("src") || "";

                const item: TocItem = {
                    id,
                    label,
                    src,
                    children: [],
                };

                // 递归查找子项
                item.children = parseNavPoints(child);
                if (item.children.length === 0) delete item.children;

                items.push(item);
            }
        }
        return items;
    }

    function sortFileTree(nodes: EpubFileNode[], tocPaths: string[]) {
        // 1. Root Level Priority
        const rootPriority = ["oebps", "meta-inf"];

        // 2. OEBPS Children Priority
        const oebpsFilePriority = ["content.opf", "toc.ncx"];
        const oebpsFolderPriority = ["text", "styles", "fonts", "images"];

        // Helper to get sorting weight
        const getWeight = (node: EpubFileNode, parentName: string) => {
            const name = node.name.toLowerCase();

            // Auto-expand logic
            if (
                name === "oebps" ||
                (parentName === "oebps" && name === "text")
            ) {
                expandedFolders.add(node.path);
            }

            // Root Level Sorting
            if (!parentName) {
                const idx = rootPriority.indexOf(name);
                return idx !== -1 ? idx : 100;
            }

            // OEBPS Level Sorting
            if (parentName === "oebps") {
                if (node.file_type !== "folder") {
                    const idx = oebpsFilePriority.indexOf(name);
                    return idx !== -1 ? idx : 200; // Files without specific priority
                } else {
                    const idx = oebpsFolderPriority.indexOf(name);
                    return idx !== -1 ? 300 + idx : 400; // Folders
                }
            }

            // Text Folder Sorting (based on TOC)
            if (parentName === "text") {
                const idx = tocPaths.indexOf(node.path);
                return idx !== -1 ? idx : 9999;
            }

            return 0; // Default
        };

        const sortRecursive = (
            list: EpubFileNode[],
            parentName: string = "",
        ) => {
            list.sort((a, b) => {
                const wA = getWeight(a, parentName);
                const wB = getWeight(b, parentName);
                if (wA !== wB) return wA - wB;
                return a.name.localeCompare(b.name, undefined, {
                    numeric: true,
                });
            });

            list.forEach((node) => {
                if (node.children) {
                    sortRecursive(node.children, node.name.toLowerCase());
                }
            });
        };

        sortRecursive(nodes);
        expandedFolders = expandedFolders; // Trigger reactivity
    }

    async function loadTOC() {
        if (tocList.length > 0) return; // 已经加载过
        isTocLoading = true;

        // 1. 在文件树中查找 .ncx 文件
        function findNcx(nodes: EpubFileNode[]): EpubFileNode | null {
            for (const node of nodes) {
                if (node.file_type === "folder" && node.children) {
                    const found = findNcx(node.children);
                    if (found) return found;
                } else if (node.name.toLowerCase().endsWith(".ncx")) {
                    return node;
                }
            }
            return null;
        }

        const ncxNode = findNcx(fileTree);

        if (!ncxNode) {
            console.warn("未找到 .ncx 文件");
            isTocLoading = false;
            return;
        }

        try {
            // 2. 读取 ncx 内容
            const ncxContent = await invoke<string>("read_epub_file_content", {
                epubPath: epubPath,
                filePath: ncxNode.path,
            });

            // 3. 解析 XML
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(ncxContent, "text/xml");
            const navMap = xmlDoc.querySelector("navMap");

            if (navMap) {
                tocList = parseNavPoints(navMap);

                // 收集所有 TOC 引用的文件路径，用于排序
                tocNcxPath = ncxNode.path;
                const tocPaths: string[] = [];
                const collectPaths = (items: TocItem[]) => {
                    for (const item of items) {
                        // 解析为绝对路径 (去除锚点)
                        const [relativePath] = item.src.split("#");
                        if (relativePath) {
                            const fullPath = resolvePath(
                                tocNcxPath,
                                relativePath,
                            );
                            if (!tocPaths.includes(fullPath)) {
                                tocPaths.push(fullPath);
                            }
                        }
                        if (item.children) collectPaths(item.children);
                    }
                };
                collectPaths(tocList);

                // 执行排序
                sortFileTree(fileTree, tocPaths);
                fileTree = fileTree; // 触发更新
            }

            // 存储 ncx 文件的路径，用于后续解析相对路径
            tocNcxPath = ncxNode.path;
        } catch (e) {
            console.error("加载目录失败", e);
        } finally {
            isTocLoading = false;
        }
    }

    let tocNcxPath = ""; // ncx 文件的完整路径

    // 处理目录点击
    function handleTocClick(src: string) {
        // src 可能是 "Text/chapter1.xhtml" 或 "chapter1.xhtml#point"
        let [relativePath, anchor] = src.split("#");

        // 解析出绝对路径
        const targetPath = resolvePath(tocNcxPath, relativePath);

        // 展开文件所在的所有父文件夹
        function expandParentFolders(path: string) {
            const parts = path.split("/");
            let currentPath = "";
            for (let i = 0; i < parts.length - 1; i++) {
                currentPath += (i > 0 ? "/" : "") + parts[i];
                expandedFolders.add(currentPath);
            }
            expandedFolders = expandedFolders; // 触发响应式更新
        }

        // 在 fileTree 中查找对应节点并选中
        function findAndSelect(nodes: EpubFileNode[]): boolean {
            for (const node of nodes) {
                if (node.path === targetPath) {
                    selectFile(node);

                    // 滚动到文件节点
                    setTimeout(() => {
                        const fileElement = document.querySelector(
                            `.file-node[data-path="${targetPath}"]`,
                        );
                        if (fileElement) {
                            fileElement.scrollIntoView({
                                behavior: "smooth",
                                block: "center",
                            });
                        }
                    }, 100);

                    return true;
                }
                if (node.children) {
                    if (findAndSelect(node.children)) return true;
                }
            }
            return false;
        }

        expandParentFolders(targetPath);
        findAndSelect(fileTree);
        // 注释掉自动切换，保持在目录页
        // if (found) {
        //      activeTab = "preview";
        // }
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

    function isEditable(type: string): boolean {
        return ["html", "css", "xml", "opf", "ncx"].includes(type);
    }

    function getFileLanguage(type: string): "html" | "css" | "xml" {
        if (type === "css") return "css";
        if (type === "xml" || type === "opf" || type === "ncx") return "xml";
        return "html";
    }

    // 添加行号
    function addLineNumbers(highlighted: string): string {
        const lines = highlighted.split("\n");
        return lines
            .map((line, i) => {
                const lineNum = i + 1;
                // 使用 div 而不是 span，避免换行符导致的额外间距
                return `<div class="line-with-number"><span class="line-number">${lineNum}</span><span class="line-content">${line || " "}</span></div>`;
            })
            .join(""); // 不加换行符，因为 div 本身会换行
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
                <h3>文件结构</h3>
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
                                                            data-path={subChild.path}
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
                                            data-path={child.path}
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
            {#if openTabs.length > 0}
                <!-- 标签页栏 -->
                <div class="tabs-bar" bind:this={tabsBarDiv}>
                    {#each openTabs as tab, index}
                        <div
                            class="editor-tab"
                            class:active={index === activeTabIndex}
                            on:click={() => switchTab(index)}
                            on:keydown={(e) =>
                                e.key === "Enter" && switchTab(index)}
                            role="button"
                            tabindex="0"
                        >
                            <span class="tab-icon"
                                >{getFileIcon(tab.file_type)}</span
                            >
                            <span class="tab-name" title={tab.name}
                                >{tab.name}{#if modifiedFiles.has(tab.path)}
                                    <span class="modified-indicator">*</span>
                                {/if}</span
                            >
                            <button
                                class="tab-close"
                                on:click={(e) => closeTab(e, index)}
                                aria-label="关闭标签页"
                            >
                                ×
                            </button>
                        </div>
                    {/each}
                </div>
            {/if}

            {#if selectedFile}
                <!-- Editor Header Removed -->
                <div class="editor-content" bind:this={editorContentDiv}>
                    {#if isEditable(selectedFile.file_type)}
                        <EpubCodeEditor
                            bind:this={epubCodeEditorComponent}
                            doc={fileContent}
                            language={getFileLanguage(selectedFile.file_type)}
                            onChange={handleFileContentChange}
                            onSave={saveCurrentFile}
                        />
                    {:else}
                        <pre class="code-block">{@html addLineNumbers(
                                fileContent
                                    .replace(/</g, "&lt;")
                                    .replace(/>/g, "&gt;"),
                            )}</pre>
                    {/if}
                </div>
            {:else}
                <div class="placeholder">点击左侧文件以查看内容</div>
            {/if}
        </main>

        <!-- 右侧：预览/目录 -->
        <aside class="preview-pane">
            <div class="preview-header">
                <div class="tabs">
                    <button
                        class="tab"
                        class:active={activeTab === "preview"}
                        on:click={() => (activeTab = "preview")}
                    >
                        预览
                    </button>
                    <button
                        class="tab"
                        class:active={activeTab === "toc"}
                        on:click={() => {
                            activeTab = "toc";
                            loadTOC();
                        }}
                    >
                        目录
                    </button>
                </div>
            </div>

            {#if activeTab === "preview"}
                <div class="preview-container">
                    {#if previewError.length > 0}
                        <div class="preview-error">
                            <div class="error-header">
                                <span class="error-icon">⚠️</span>
                                <span>发现 {previewError.length} 个问题</span>
                            </div>
                            <div class="error-content">
                                {#each previewError as err}
                                    <div class="error-item">
                                        {err.message}
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/if}

                    {#if selectedFile?.file_type === "html" || selectedFile?.name.endsWith(".xhtml") || selectedFile?.name.endsWith(".html")}
                        <div class="mobile-frame">
                            <iframe
                                bind:this={previewIframe}
                                title="preview"
                                srcdoc={previewContent}
                                sandbox="allow-same-origin allow-scripts"
                            ></iframe>
                        </div>
                    {:else}
                        <div class="placeholder">
                            {selectedFile
                                ? "选择 HTML 文件以预览"
                                : "请从左侧选择一个文件"}
                        </div>
                    {/if}
                </div>
            {:else}
                <div class="toc-container">
                    {#if isTocLoading}
                        <div class="loading">加载目录...</div>
                    {:else if tocList.length === 0}
                        <div class="empty">暂无目录或未找到 toc.ncx</div>
                    {:else}
                        <div class="toc-list">
                            {#each tocList as item}
                                <TocNode {item} onSelect={handleTocClick} />
                            {/each}
                        </div>
                    {/if}
                </div>
            {/if}
        </aside>
    {/if}
</div>

<!-- Context Menu -->
<ContextMenu />

{#if showCloseDialog}
    <div class="dialog-overlay">
        <div class="dialog">
            <div class="dialog-header">未保存的更改</div>
            <div class="dialog-content">
                {#if closeContext === "tab"}
                    文件 "{pendingCloseFile?.name}" 有未保存的更改，是否保存？
                {:else}
                    您有 {modifiedFiles.size} 个文件包含未保存的更改，是否保存所有并退出？
                {/if}
            </div>
            <div class="dialog-actions">
                <button
                    class="btn primary"
                    on:click={handleDialogSave}
                    disabled={isSaving}
                >
                    {isSaving ? "保存中..." : "保存"}
                </button>
                <button
                    class="btn danger"
                    on:click={handleDialogDiscard}
                    disabled={isSaving}>不保存</button
                >
                <button
                    class="btn secondary"
                    on:click={handleDialogCancel}
                    disabled={isSaving}>取消</button
                >
            </div>
        </div>
    </div>
{/if}

<style>
    /* Dialog Styles */
    .dialog-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 1000;
    }

    .dialog {
        background: white;
        padding: 20px;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        min-width: 300px;
    }

    .dialog-header {
        font-size: 18px;
        font-weight: bold;
        margin-bottom: 15px;
    }

    .dialog-content {
        margin-bottom: 20px;
        color: #333;
    }

    .dialog-actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
    }

    .btn {
        padding: 8px 16px;
        border-radius: 4px;
        border: none;
        cursor: pointer;
        font-weight: 500;
    }

    .btn.primary {
        background: #2196f3;
        color: white;
    }

    .btn.danger {
        background: #f44336;
        color: white;
    }

    .btn.secondary {
        background: #e0e0e0;
        color: #333;
    }

    /* Mod Indicator */
    .modified-indicator {
        color: #ff9800;
        margin-left: 4px;
        font-weight: bold;
    }

    /* Preview Error (Absolute Position) */
    .preview-error {
        background: #fff3cd;
        border: 1px solid #ffc107;
        border-radius: 4px;
        margin: 8px;
        font-size: 12px;
        position: absolute;
        top: 0;
        left: 0;
        right: 0;
        z-index: 10;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        opacity: 0.95;
    }

    .preview-error .error-header {
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 8px 12px;
        background: #ffc107;
        color: #856404;
        font-weight: 600;
        border-radius: 4px 4px 0 0;
    }

    .preview-error .error-content {
        margin: 0;
        padding: 8px 12px;
        color: #856404;
        white-space: pre-wrap;
        max-height: 100px;
        overflow-y: auto;
        font-family: "Consolas", monospace;
    }

    /* Ensure container is relative */
    .preview-container {
        position: relative;
    }
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

    /* 全局重置，防止出现额外的滚动条 */
    :global(body) {
        margin: 0;
        padding: 0;
        overflow: hidden;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
            Oxygen, Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;
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
        height: 40px; /* Matched with tabs-bar */
        padding: 0 16px;
        border-bottom: 1px solid #eee;
        background: #fafafa;
        display: flex;
        align-items: center;
        justify-content: center; /* Center the title */
        box-sizing: border-box;
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
        min-width: 0; /* 关键：允许 flex 子项收缩，从而触发内部滚动 */
    }

    /* 标签页栏 */
    .tabs-bar {
        display: flex;
        background: #f3f3f3;
        border-bottom: 1px solid #ddd;
        overflow-x: auto;
        overflow-y: hidden;
        gap: 0;
        flex-shrink: 0;
        max-height: 40px;
        width: 100%; /* 确保不超出父元素 */
        box-sizing: border-box; /* 包含边框在宽度内 */
    }

    .editor-tab {
        display: flex;
        align-items: center;
        padding: 8px 12px;
        min-width: 120px;
        max-width: 200px;
        border-right: 1px solid #ddd;
        cursor: pointer;
        background: #e8e8e8;
        transition: background 0.2s;
        user-select: none;
        flex-shrink: 0; /* 防止标签被压缩，允许横向滚动 */
    }

    .editor-tab:hover {
        background: #d8d8d8;
    }

    .editor-tab.active {
        background: #fff;
        border-bottom: 2px solid #2196f3;
        position: relative;
    }

    .tab-icon {
        font-size: 14px;
        margin-right: 6px;
        flex-shrink: 0;
    }

    .tab-name {
        flex: 1;
        font-size: 13px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #333;
    }

    .tab-close {
        margin-left: 6px;
        width: 18px;
        height: 18px;
        border: none;
        background: transparent;
        color: #999;
        font-size: 18px;
        line-height: 1;
        cursor: pointer;
        border-radius: 3px;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        padding: 0;
    }

    .tab-close:hover {
        background: #ccc;
        color: #333;
    }

    /* 行号相关 */
    .code-block :global(.line-with-number) {
        display: flex; /* 改用 flex 而不是 grid */
        line-height: 1.5;
        margin: 0;
        padding: 0;
    }

    .code-block :global(.line-number) {
        color: #858585;
        text-align: right;
        padding-right: 12px;
        user-select: none;
        border-right: 1px solid #e0e0e0;
        min-width: 40px;
        background: #f8f8f8;
        flex-shrink: 0;
    }

    .code-block :global(.line-content) {
        padding-left: 8px;
        flex: 1;
        white-space: pre-wrap; /* 保留空白但允许换行 */
    }

    .editor-content {
        flex: 1;
        overflow-y: hidden; /* Let CodeMirror handle scroll */
        position: relative;
        padding: 0; /* Remove padding to fix black border */
        /* background removal handled by CodeMirror theme */
    }

    .code-block {
        margin: 0;
        padding: 0;
        font-family: "Consolas", "Monaco", monospace;
        font-size: 14px;
        line-height: 1.5;
        white-space: pre; /* crucial for code formatting */
        counter-reset: line;
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
        width: 360px; /* 标准安卓手机CSS宽度 */
        background: #fff;
        display: flex;
        flex-direction: column;
        border-left: 1px solid #ddd;
    }

    .preview-header {
        height: 40px; /* Matched with tabs-bar */
        background: #fafafa;
        border-bottom: 1px solid #eee;
        display: flex;
        align-items: center;
        box-sizing: border-box;
    }

    .tabs {
        display: flex;
        height: 100%;
        width: 100%;
    }

    .tab {
        flex: 1;
        border: none;
        background: transparent;
        font-size: 16px; /* Matched with tree-header h3 */
        color: #666;
        cursor: pointer;
        border-bottom: 2px solid transparent;
        transition: all 0.2s;
        font-weight: bold; /* Matched with tree-header h3 */
    }

    .tab:hover {
        background: #f0f0f0;
        color: #333;
    }

    .tab.active {
        color: #2196f3;
        border-bottom: 2px solid #2196f3;
        background: #fff;
    }

    .preview-container {
        flex: 1;
        overflow: hidden;
        background: #fff;
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
    }

    .mobile-frame {
        width: 360px; /* 标准安卓手机CSS宽度 */
        height: 100%;
        max-height: 812px; /* iPhone X height approx, or just limit it */
        background: #fff;
        box-shadow:
            0 4px 6px -1px rgba(0, 0, 0, 0.1),
            0 2px 4px -1px rgba(0, 0, 0, 0.06);
        border: 1px solid #d1d5db;
        border-radius: 8px; /* 添加圆角模拟手机外观 */
        overflow: hidden; /* 确保内容不超出边框 */
        display: flex;
        flex-direction: column;
    }

    .preview-container iframe {
        width: 100%;
        height: 100%;
        border: none;
        background: #fff;
        /* 关键：确保 iframe 内容无边距 */
        margin: 0;
        padding: 0;
        display: block;
    }

    .toc-container {
        flex: 1;
        overflow-y: auto;
        background: #fff;
        padding: 0;
    }

    .toc-list {
        margin: 0;
        padding: 0;
    }

    .toc-container .empty,
    .toc-container .loading {
        padding: 20px;
        text-align: center;
        color: #999;
        font-size: 14px;
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
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
