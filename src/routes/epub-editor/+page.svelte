<script lang="ts">
    import { onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { emit } from "@tauri-apps/api/event"; // Added emit
    import { confirm, open, save } from "@tauri-apps/plugin-dialog";
    import { page } from "$app/stores";
    import TocNode from "$lib/TocNode.svelte";
    import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";
    import FileTreeItem from "$lib/FileTreeItem.svelte";

    // Setup Close Listener
    onMount(async () => {
        const win = getCurrentWindow();
        // NOTE: Do NOT emit restore-main-window here. It should only be emitted
        // after the user confirms closing (or if there are no unsaved changes).
        // The actual restoration is handled by the confirmation dialog logic.
        const unlisten = await win.listen(
            "tauri://close-requested",
            async (event) => {
                // Prevent default close, show confirmation if needed
                // The 'beforeunload' or custom logic should handle this
                console.log("EPUB window close requested");
                // Do NOT emit restore-main-window here as it's too early
            },
        );

        // 监听来自 Preview Iframe 的消息
        const handleMessage = (event: MessageEvent) => {
            if (event.data && event.data.type === "previewClick") {
                // 优先使用文本定位
                if (event.data.text) {
                    if (event.data.context) {
                        // 使用上下文感知同步 (Context-Aware Sync)
                        epubCodeEditorComponent?.selectTextWithContext(
                            event.data.text,
                            event.data.context,
                        );
                    } else {
                        // Fallback to strict text matching
                        epubCodeEditorComponent?.selectText(event.data.text);
                    }
                } else if (event.data.percent !== undefined) {
                    // Fallback to percent
                    epubCodeEditorComponent?.scrollToRatio(event.data.percent);
                }
            }
            // NOTE: Removed 'previewSelection' handling to prevent sync loop.
            // Selecting text in preview should NOT sync back to editor and then to preview again.
        };
        window.addEventListener("message", handleMessage);

        return () => {
            unlisten();
            window.removeEventListener("message", handleMessage);
        };
    });

    // Editor -> Preview Sync
    function handleEditorClick(line: number) {
        const iframeWindow = previewIframe?.contentWindow;
        if (!iframeWindow) return;

        const view = epubCodeEditorComponent?.getView();
        if (!view) return;

        // 获取该行文本
        const lineContent = view.state.doc.line(line).text;
        // 移除标签，只保留文本，用于模糊搜索
        const cleanText = lineContent
            .replace(/<[^>]+>/g, " ") // 标签变空格
            .replace(/\s+/g, " ") // 多空格变单空格
            .trim();

        if (cleanText.length < 2) return; // 太短不搜索

        iframeWindow.postMessage(
            {
                type: "editorClickText",
                text: cleanText,
            },
            "*",
        );
    }

    let editorSelectionTimeout: any;
    function handleEditorSelection(text: string) {
        if (!previewIframe?.contentWindow) return;

        if (editorSelectionTimeout) clearTimeout(editorSelectionTimeout);
        editorSelectionTimeout = setTimeout(() => {
            // 移除 HTML 标签，并标准化空白字符 (换行符转空格)，只同步纯文本
            const cleanText = text
                .replace(/<[^>]+>/g, "")
                .replace(/\s+/g, " ")
                .trim();
            // Prevent flashing on short selections
            if (!cleanText || cleanText.length < 2) return;

            previewIframe.contentWindow.postMessage(
                {
                    type: "editorSelection",
                    text: cleanText,
                },
                "*",
            );
        }, 300);
    }

    interface EpubFileNode {
        name: string;
        path: string;
        file_type: string;
        size?: number;
        title?: string;
        resolution?: string;
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
    let isProjectDirty = false;
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

    // 查找替换状态
    let currentImageSrc: string | null = null;

    function isImageFile(name: string) {
        return /\.(jpg|jpeg|png|gif|webp|svg|bmp)$/i.test(name);
    }

    // 自定义输入对话框状态 (替代 JavaScript prompt，因为在 Tauri 中 prompt 不工作)
    let showPrompt = false;
    let promptTitle = "";
    let promptValue = "";
    let promptOptions = false; // 是否显示额外选项 (如：自动更新链接)
    let promptCheckValue = true; // 额外选项的勾选状态
    let isPromptBusy = false; // 是否正在处理中 (如：更新链接)
    let promptResolve:
        | ((value: { value: string; confirmRefactor: boolean } | null) => void)
        | null = null;

    // Confirm Dialog State
    let showConfirm = false;
    let confirmTitle = "";
    let confirmMessage = "";
    let confirmResolve: ((result: boolean) => void) | null = null;

    function showPromptDialog(
        title: string,
        defaultValue: string = "",
        options: boolean = false,
    ): Promise<{ value: string; confirmRefactor: boolean } | null> {
        console.log("[DEBUG] showPromptDialog called:", title, defaultValue);
        return new Promise((resolve) => {
            promptTitle = title;
            promptValue = defaultValue;
            promptOptions = options;
            promptCheckValue = true;
            isPromptBusy = false;
            promptResolve = resolve;
            showPrompt = true;
            console.log("[DEBUG] showPrompt set to true");
        });
    }

    function handlePromptConfirm() {
        if (isPromptBusy) return;
        console.log("[DEBUG] handlePromptConfirm called, value:", promptValue);
        // 如果是复杂操作（重命名并更新链接），由调用方负责关闭对话框并设置 isPromptBusy
        if (promptResolve) {
            promptResolve({
                value: promptValue,
                confirmRefactor: promptCheckValue,
            });
            // 注意：我们不立即关闭 showPrompt = false;
            // 直到 handleContextMenuAction 完成处理或发现它是简单操作
        }
    }

    function handlePromptCancel() {
        if (isPromptBusy) return;
        showPrompt = false;
        if (promptResolve) {
            promptResolve(null);
            promptResolve = null;
        }
    }

    function showConfirmDialog(
        title: string,
        message: string,
    ): Promise<boolean> {
        return new Promise((resolve) => {
            confirmTitle = title;
            confirmMessage = message;
            showConfirm = true;
            confirmResolve = resolve;
        });
    }

    function handleConfirmConfirm() {
        if (confirmResolve) confirmResolve(true);
        showConfirm = false;
        confirmResolve = null;
    }

    function handleConfirmCancel() {
        if (confirmResolve) confirmResolve(false);
        showConfirm = false;
        confirmResolve = null;
    }

    let showFindReplace = false;
    let findPattern = "";
    let replacePattern = "";
    let isRegex = false;
    let searchScope: "all" | "html" | "selected" | "open" | "current" =
        "current";
    let searchDirection: "down" | "up" = "down";
    let wrapAround = true;
    let textOnly = false;
    let searchMessage = "";
    let currentMatchInfo: {
        filePath: string;
        from: number;
        to: number;
    } | null = null;

    // 查找替换历史
    let findHistory: string[] = [];
    let replaceHistory: string[] = [];
    let showFindHistory = false;
    let showReplaceHistory = false;

    // Multi-select
    let multiSelectedFiles = new Set<string>();
    const MAX_HISTORY = 10;

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

    function getAllFiles(nodes: EpubFileNode[]): EpubFileNode[] {
        let result: EpubFileNode[] = [];
        for (const node of nodes) {
            if (node.file_type !== "folder") {
                result.push(node);
            }
            if (node.children) {
                result = result.concat(getAllFiles(node.children));
            }
        }
        return result;
    }

    const loadEpub = async () => {
        epubPath = $page.url.searchParams.get("file") || "";
        if (!epubPath) {
            error = "未指定 EPUB 文件路径";
            isLoading = false;
            return;
        }

        try {
            // 清理旧缓存
            blobUrls.forEach((url) => URL.revokeObjectURL(url));
            blobUrls = [];
            assetCache.clear();
            fileContentCache.clear();
            previewCache.clear();

            fileTree = await invoke<EpubFileNode[]>("extract_epub", {
                epubPath: epubPath,
            });
            isProjectDirty = false;
            modifiedFiles.clear();
            modifiedFiles = modifiedFiles;

            // 排序在 loadTOC 中基于 TOC 顺序执行
            flatHtmlFiles = flattenFiles(fileTree);
            await loadSpine(); // Load Spine for sorting
            await loadTOC();

            // 自动展开核心文件夹 (OEBPS, Text)
            const autoExpand = (nodes: EpubFileNode[]) => {
                nodes.forEach((node) => {
                    const name = node.name.toLowerCase();
                    if (name === "oebps") {
                        expandedFolders.add(node.path);
                        if (node.children) {
                            node.children.forEach((child) => {
                                if (child.name.toLowerCase() === "text") {
                                    expandedFolders.add(child.path);
                                }
                            });
                        }
                    }
                    if (node.children) autoExpand(node.children);
                });
            };
            autoExpand(fileTree);
            expandedFolders = expandedFolders;

            // 立即显示 UI（目录先加载完毕）
            isLoading = false;

            // Background: Pre-cache all CSS files (non-blocking)
            // 在后台预加载 CSS，不阻塞用户操作
            precacheCssAssets(fileTree).then(() => {
                console.log("CSS pre-caching completed");
                // After CSS is cached, auto-open first chapter
                if (activeTabIndex === -1 && !selectedFile) {
                    const firstHtml = flatHtmlFiles.find(
                        (f) =>
                            f.file_type === "html" ||
                            f.name.endsWith(".xhtml") ||
                            f.name.endsWith(".html"),
                    );
                    if (firstHtml) {
                        selectFile(firstHtml);
                    } else if (flatHtmlFiles.length > 0) {
                        selectFile(flatHtmlFiles[0]);
                    }
                }
            });
        } catch (e) {
            error = `加载失败: ${e}`;
            isLoading = false;
        }
    };

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

        // 延时执行，优先保证当前 UI 响应
        // 预加载更多章节以提升后续导航速度
        setTimeout(() => {
            // Pre-cache next 3 chapters
            for (let i = 1; i <= 3; i++) {
                const next = flatHtmlFiles[index + i];
                if (next) preloadFile(next);
            }
            // Pre-cache previous 1 chapter
            const prev = flatHtmlFiles[index - 1];
            if (prev) preloadFile(prev);
        }, 300);
    }

    // ... resolvePath ... (unchanged)

    // Pre-cache all CSS files and their referenced resources (fonts, images)
    async function precacheCssAssets(nodes: EpubFileNode[]) {
        // 1. Collect all CSS files
        const allFiles = getAllFiles(nodes);
        const cssFiles = allFiles.filter(
            (f) => f.name.endsWith(".css") || f.file_type === "css",
        );

        if (cssFiles.length === 0) return;

        const cssPaths = cssFiles.map((f) => f.path);

        // 2. Batch read all CSS files
        let cssContents: Record<string, string> = {};
        try {
            cssContents = await invoke<Record<string, string>>(
                "read_epub_files_batch",
                {
                    epubPath: epubPath,
                    filePaths: cssPaths,
                },
            );
        } catch (e) {
            console.error("Pre-cache CSS: batch read failed", e);
            return;
        }

        // 3. Extract all url() references from CSS
        const binaryPaths = new Set<string>();

        for (const [cssPath, cssContent] of Object.entries(cssContents)) {
            const urlRegex = /url\(['"]?([^'")\s]+)['"]?\)/g;
            let match;

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
                }
            }
        }

        // 4. Batch read all referenced binary files (fonts, images)
        if (binaryPaths.size === 0) return;

        let binaryData: Record<string, number[]> = {};
        try {
            binaryData = await invoke<Record<string, number[]>>(
                "read_epub_binary_batch",
                {
                    epubPath: epubPath,
                    filePaths: [...binaryPaths],
                },
            );
        } catch (e) {
            console.error("Pre-cache CSS: binary read failed", e);
            return;
        }

        // 5. Create Blob URLs and cache them
        for (const [path, data] of Object.entries(binaryData)) {
            if (assetCache.has(path)) continue;

            const uint8Array = new Uint8Array(data);
            let mimeType = "application/octet-stream";
            const lower = path.toLowerCase();

            if (lower.endsWith(".woff2")) mimeType = "font/woff2";
            else if (lower.endsWith(".woff")) mimeType = "font/woff";
            else if (lower.endsWith(".ttf")) mimeType = "font/ttf";
            else if (lower.endsWith(".otf")) mimeType = "font/otf";
            else if (lower.endsWith(".eot"))
                mimeType = "application/vnd.ms-fontobject";
            else if (lower.endsWith(".png")) mimeType = "image/png";
            else if (lower.endsWith(".jpg") || lower.endsWith(".jpeg"))
                mimeType = "image/jpeg";
            else if (lower.endsWith(".gif")) mimeType = "image/gif";
            else if (lower.endsWith(".webp")) mimeType = "image/webp";
            else if (lower.endsWith(".svg")) mimeType = "image/svg+xml";

            const blob = new Blob([uint8Array], { type: mimeType });
            const url = URL.createObjectURL(blob);
            assetCache.set(path, url);
            blobUrls.push(url);
        }

        console.log(
            `Pre-cached ${Object.keys(cssContents).length} CSS files and ${Object.keys(binaryData).length} binary assets`,
        );
    }

    // ... processCssAssets ... (unchanged)

    // ... processHtmlForPreview ... (unchanged)

    function toggleFolder(path: string) {
        if (expandedFolders.has(path)) {
            expandedFolders.delete(path);
        } else {
            expandedFolders.add(path);
        }
        expandedFolders = expandedFolders;
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

        // Calling loadEpub (now top-level)
        loadEpub();

        return () => {
            // 组件销毁时清理
            window.removeEventListener("beforeunload", handleBeforeUnload);
            if (unlistenClose) unlistenClose();
            cleanupBlobUrls();
        };
    });

    // 监听全选及 context-menu
    onMount(() => {
        const handleContextMenuAction = async (e: CustomEvent) => {
            const { action, context } = e.detail;
            console.log(
                "[DEBUG] context-menu-action received:",
                action,
                context,
            );
            try {
                if (action === "toggle-select") {
                    if (context.path) {
                        if (multiSelectedFiles.has(context.path)) {
                            multiSelectedFiles.delete(context.path);
                        } else {
                            multiSelectedFiles.add(context.path);
                        }
                        multiSelectedFiles = multiSelectedFiles;
                    }
                } else if (action === "save-as") {
                    try {
                        const res = await invoke<Record<string, string>>(
                            "read_epub_files_batch",
                            {
                                epubPath,
                                filePaths: [context.path],
                            },
                        );
                        const content = res[context.path];
                        if (!content) throw new Error("无法读取文件内容");
                        const savedPath = await save({
                            defaultPath: context.path.split("/").pop(),
                            filters: [
                                {
                                    name: "Text",
                                    extensions: ["xhtml", "html", "txt"],
                                },
                            ],
                        });
                        if (savedPath) {
                            await invoke("save_text_file", {
                                path: savedPath,
                                content,
                            });
                            alert("导出成功!");
                        }
                    } catch (e) {
                        alert("导出失败: " + e);
                    }
                } else if (action === "duplicate") {
                    try {
                        const oldPath = context.path;
                        const res = await invoke<Record<string, string>>(
                            "read_epub_files_batch",
                            {
                                epubPath,
                                filePaths: [oldPath],
                            },
                        );
                        const content = res[oldPath];
                        if (!content) throw new Error("无法读取源文件内容");
                        // Simple duplicate naming: foo.xhtml -> foo_copy.xhtml
                        const parts = oldPath.split(".");
                        const ext = parts.pop();
                        const base = parts.join(".");
                        const newPath = `${base}_copy.${ext}`;

                        await invoke("add_epub_file", {
                            epubPath,
                            filePath: newPath,
                            content,
                        });
                        await addToOpf(newPath, oldPath);

                        await loadEpub();
                        isProjectDirty = true;
                    } catch (e) {
                        alert("副本创建失败: " + e);
                    }
                } else if (action === "import-sibling") {
                    try {
                        const selected = await open({
                            multiple: false,
                            filters: [
                                {
                                    name: "HTML",
                                    extensions: ["html", "xhtml", "htm"],
                                },
                            ],
                        });
                        if (selected) {
                            const localPath = selected as string;
                            const fileName =
                                localPath.split(/[\\/]/).pop() ||
                                "imported.xhtml";
                            const currentDir = context.path.substring(
                                0,
                                context.path.lastIndexOf("/"),
                            );
                            const newEpubPath = currentDir + "/" + fileName;

                            const content = await invoke<string>(
                                "read_text_file",
                                { path: localPath },
                            );
                            await invoke("add_epub_file", {
                                epubPath,
                                filePath: newEpubPath,
                                content,
                            });
                            await addToOpf(newEpubPath, context.path);
                            await loadEpub();
                            isProjectDirty = true;
                        }
                    } catch (e) {
                        alert("导入失败: " + e);
                    }
                } else if (action === "new-sibling-html") {
                    const res = await showPromptDialog(
                        "新文件名",
                        "new_chapter.xhtml",
                    );
                    if (res && res.value) {
                        const fileName = res.value;
                        showPrompt = false;
                        const currentDir = context.path.substring(
                            0,
                            context.path.lastIndexOf("/"),
                        );
                        const newPath = currentDir + "/" + fileName;
                        const content =
                            '<?xml version="1.0" encoding="utf-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml">\n<head>\n<title></title>\n</head>\n<body>\n</body>\n</html>';

                        isPromptBusy = true;
                        try {
                            await invoke("add_epub_file", {
                                epubPath,
                                filePath: newPath,
                                content,
                            });
                            await addToOpf(newPath, context.path);
                            await loadEpub();
                            isProjectDirty = true;
                        } catch (e) {
                            alert("新建失败: " + e);
                        } finally {
                            isPromptBusy = false;
                            showPrompt = false;
                            promptResolve = null;
                        }
                    }
                } else if (action === "rename") {
                    const oldPath = context.path;
                    const oldName = oldPath.split("/").pop() || "";

                    const res = await showPromptDialog(
                        "请输入新文件名",
                        oldName,
                        true, // 显示“自动更新链接”选项
                    );

                    if (res && res.value && res.value !== oldName) {
                        const newName = res.value;
                        const doRefactor = res.confirmRefactor;
                        const newPath =
                            oldPath.substring(0, oldPath.lastIndexOf("/") + 1) +
                            newName;

                        // 设置加载状态，使弹窗保持打开并显示“正在更新...”
                        isPromptBusy = true;

                        try {
                            // 1. Rename file in backend
                            await invoke("rename_epub_file", {
                                epubPath,
                                oldPath,
                                newPath,
                            });

                            // 2. Sync UI Tabs & Selection
                            openTabs = openTabs.map((tab) => {
                                if (tab.path === oldPath) {
                                    return {
                                        ...tab,
                                        path: newPath,
                                        name: newName,
                                    };
                                }
                                return tab;
                            });
                            openTabs = openTabs;

                            if (selectedFile && selectedFile.path === oldPath) {
                                selectedFile = {
                                    ...selectedFile,
                                    path: newPath,
                                    name: newName,
                                } as EpubFileNode;
                            }

                            // 3. Refactor links if requested
                            if (doRefactor) {
                                let updatedCount = 0;
                                const everyFile = getAllFiles(fileTree);

                                for (const file of everyFile) {
                                    const isTextFile =
                                        file.file_type === "html" ||
                                        file.file_type === "xml" ||
                                        file.file_type === "css" ||
                                        file.name.endsWith(".opf") ||
                                        file.name.endsWith(".ncx");

                                    if (!isTextFile) continue;

                                    let targetFilePath = file.path;
                                    if (targetFilePath === oldPath)
                                        targetFilePath = newPath;

                                    let content = "";
                                    try {
                                        content = await invoke<string>(
                                            "read_epub_file_content",
                                            {
                                                epubPath,
                                                filePath: targetFilePath,
                                            },
                                        );
                                    } catch (e) {
                                        continue;
                                    }

                                    const escapedOld = oldName.replace(
                                        /[.*+?^${}()|[\]\\]/g,
                                        "\\$&",
                                    );
                                    const regexFinal = new RegExp(
                                        `([/"'])` + escapedOld + `([ "'#?])`,
                                        "g",
                                    );

                                    let newContent = content.replace(
                                        regexFinal,
                                        (match, prefix, suffix) => {
                                            return prefix + newName + suffix;
                                        },
                                    );

                                    if (newContent !== content) {
                                        await invoke("add_epub_file", {
                                            epubPath,
                                            filePath: targetFilePath,
                                            content: newContent,
                                        });
                                        updatedCount++;
                                    }
                                }
                                console.log(
                                    `[DEBUG] Refactor completed, updated ${updatedCount} files.`,
                                );
                            }

                            // 4. Reload tree & auto-expand (MUST be after link update)
                            await loadEpub();

                            isProjectDirty = true; // 标记为未保存

                            const newFolderPath = newPath.substring(
                                0,
                                newPath.lastIndexOf("/"),
                            );
                            if (newFolderPath) {
                                expandedFolders.add(newFolderPath);
                                expandedFolders = expandedFolders;
                            }
                        } catch (e) {
                            console.error("Rename/Refactor failed", e);
                            alert("操作失败: " + e);
                        } finally {
                            isPromptBusy = false;
                            showPrompt = false; // 操作完成后关闭对话框
                            promptResolve = null;
                        }
                    } else {
                        showPrompt = false; // 取消操作时关闭
                        promptResolve = null;
                    }
                } else if (action === "delete") {
                    const confirmed = await showConfirmDialog(
                        "确认删除",
                        `确定要删除 ${context.path} 吗?`,
                    );
                    if (confirmed) {
                        // 1. Delete file
                        await invoke("delete_epub_file", {
                            epubPath,
                            filePath: context.path,
                        });

                        // 2. Close tab if open
                        const wasOpenIndex = openTabs.findIndex(
                            (t) => t.path === context.path,
                        );
                        if (wasOpenIndex !== -1) {
                            openTabs = openTabs.filter(
                                (_, i) => i !== wasOpenIndex,
                            );

                            // If the deleted file was the currently selected one, switch to another tab or clear
                            if (
                                selectedFile &&
                                selectedFile.path === context.path
                            ) {
                                // If finding active tab index is needed, we can rely on selectedFile check
                                // Update selectedFile to the last available tab
                                if (openTabs.length > 0) {
                                    // Switch to last tab
                                    const newActive =
                                        openTabs[openTabs.length - 1];
                                    selectedFile = newActive;
                                    // Trigger file load logic?
                                    // Usually clicking a tab triggers 'selectFile'.
                                    // Here we might just set the variable, but 'selectFile' loads content.
                                    // Let's call selectFile logic or just set it and let reactivity handle if bindings exist.
                                    // But selectFile(newActive) is better.
                                    // Can we access selectFile here? It's defined below probably.
                                    // Let's just set 'selectedFile' and 'fileContent' manually for safety or rely on existing logic.
                                    // For now, just setting selectedFile might not load text.
                                    // But wait, the user just wants the tab to close.
                                    // If I set selectedFile = newActive, does the editor update?
                                    // Svelte reactivity should handle 'selectedFile' prop pass down.
                                    // But 'fileContent' variable needs update.
                                    // Calling selectFile(newActive) is best but it takes an event.
                                    // Let's emulate:
                                    // await selectFile(newActive);
                                    // But selectFile might not be async or exported.
                                } else {
                                    selectedFile = null;
                                    fileContent = "";
                                    activeTabIndex = -1;
                                }
                            }
                        }

                        await loadEpub();
                        isProjectDirty = true; // 标记为未保存
                    }
                } else if (action === "new-file") {
                    // Fix path calc: if folder, use path; if file, use parent.
                    let folderPath = context.path;
                    // Check if context is a file (context.type === 'file' or from context-type data)
                    // If we clicked on a folder node, type should be folder.
                    // But to be safe, if it looks like a file (has extension?), use parent.
                    // Better: check 'context-type'
                    // context comes from dataset.contextType
                    if (context.folderType && context.path.endsWith("/")) {
                        // It's likely a folder if folderType exists?
                    }
                    // Actually, let's look at how we bind data.
                    // If type is 'folder', context.path creates inside it.
                    // If type is 'file', context.path is the file, create sibling.

                    if (context.contextType === "file") {
                        folderPath = context.path.substring(
                            0,
                            context.path.lastIndexOf("/"),
                        );
                    }

                    const res = await showPromptDialog("文件名", "new.xhtml");
                    console.log("[DEBUG] new-file: res result:", res);
                    if (res && res.value) {
                        const fileName = res.value;
                        const filePath = folderPath + "/" + fileName;

                        isPromptBusy = true; // 开启忙碌状态

                        let content = "";
                        if (
                            fileName.endsWith(".xhtml") ||
                            fileName.endsWith(".html")
                        ) {
                            content =
                                '<?xml version="1.0" encoding="utf-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml">\n<head>\n<title></title>\n</head>\n<body>\n</body>\n</html>';
                        }
                        try {
                            await invoke("add_epub_file", {
                                epubPath,
                                filePath,
                                content,
                            });
                            console.log(
                                "[DEBUG] new-file: add_epub_file success",
                            );

                            await loadEpub();

                            // 标记为未保存
                            isProjectDirty = true;

                            // 展开父文件夹，确保新文件可见
                            expandedFolders.add(folderPath);
                            expandedFolders = expandedFolders;
                        } catch (e) {
                            console.error(
                                "[DEBUG] new-file: add_epub_file failed:",
                                e,
                            );
                            alert("创建失败: " + e);
                        } finally {
                            isPromptBusy = false;
                            showPrompt = false;
                            promptResolve = null;
                        }
                    }
                } else if (action === "select-in-tree") {
                    // TOC 选中文件
                    if (context.src) {
                        const targetPath = context.src.split("#")[0];
                        const findAndSelect = (
                            nodes: EpubFileNode[],
                        ): boolean => {
                            for (const node of nodes) {
                                // 模糊匹配：因为 src 可能是相对路径，而 node.path 是 ZIP 内全路径
                                if (
                                    node.path.endsWith(targetPath) ||
                                    targetPath.endsWith(node.path)
                                ) {
                                    // 1. 展开父文件夹
                                    const parts = node.path.split("/");
                                    let currentPath = "";
                                    for (let i = 0; i < parts.length - 1; i++) {
                                        currentPath +=
                                            (currentPath ? "/" : "") + parts[i];
                                        expandedFolders.add(currentPath);
                                    }
                                    expandedFolders = expandedFolders;

                                    // 2. 选中文件
                                    selectFile(node);
                                    return true;
                                }
                                if (node.children) {
                                    if (findAndSelect(node.children))
                                        return true;
                                }
                            }
                            return false;
                        };
                        findAndSelect(fileTree);
                    }
                } else if (action === "select-children") {
                    // TOC 选中卷下所有文件
                    // 逻辑：
                    // 1. 找到当前 TOC 节点（通过 src 查找？）
                    //    TOC 数据结构在 tocData 中。
                    //    context 只有 src 和 type。
                    //    我们需要遍历 tocData 找到这个节点，然后递归收集它的 children 的 src。
                    // 2. 将收集到的 src 转换为 file tree path (split('#')[0])
                    // 3. 将这些 paths 添加到 multiSelectedFiles

                    if (context.src) {
                        const targetSrc = context.src;

                        // 1. Find node and its parent in tocList
                        const findNodeAndParent = (
                            nodes: any[],
                            parent: any = null,
                        ): { node: any; parent: any } | null => {
                            for (const node of nodes) {
                                if (node.src === targetSrc)
                                    return { node, parent };
                                if (node.children) {
                                    const found = findNodeAndParent(
                                        node.children,
                                        node,
                                    );
                                    if (found) return found;
                                }
                            }
                            return null;
                        };

                        const result = findNodeAndParent(tocList);

                        if (result) {
                            let { node: tocNode, parent: parentNode } = result;

                            // If it's a leaf node (chapter), try to use parent (volume)
                            if (
                                (!tocNode.children ||
                                    tocNode.children.length === 0) &&
                                parentNode
                            ) {
                                tocNode = parentNode;
                            }

                            const filesToSelect = new Set<string>();

                            // 2. Collect all descendant srcs
                            const collectSrcs = (node: any) => {
                                if (node.src) {
                                    // Remove anchor
                                    const cleanPath = node.src.split("#")[0];
                                    // Resolve to full path if needed?
                                    // Usually src in TOC is relative to content or absolute in EPUB?
                                    // In our app, tocData usually has paths relative to root or OEBPS.
                                    // Let's match with flatHtmlFiles to be sure.

                                    // 模糊匹配：找到 flatHtmlFiles 中以 cleanPath 结尾的文件
                                    const file = flatHtmlFiles.find((f) =>
                                        f.path.endsWith(cleanPath),
                                    );
                                    if (file) filesToSelect.add(file.path);
                                }
                                if (node.children) {
                                    node.children.forEach(collectSrcs);
                                }
                            };

                            collectSrcs(tocNode); // Include self

                            // 3. Apply selection
                            if (filesToSelect.size > 0) {
                                // Add to existing or clear? Usually "Select" implies clearing others unless Ctrl held.
                                // But context menu action is isolated. Let's Clear then Add to be clean.
                                multiSelectedFiles.clear();
                                filesToSelect.forEach((p) =>
                                    multiSelectedFiles.add(p),
                                );
                                multiSelectedFiles = multiSelectedFiles; // trigger

                                // Auto expand folders for first item
                                const firstPath = filesToSelect
                                    .values()
                                    .next().value;
                                if (firstPath) {
                                    const parts = firstPath.split("/");
                                    let currentPath = "";
                                    for (let i = 0; i < parts.length - 1; i++) {
                                        currentPath +=
                                            (currentPath ? "/" : "") + parts[i];
                                        expandedFolders.add(currentPath);
                                    }
                                    expandedFolders = expandedFolders;
                                }
                            } else {
                                alert("未找到相关文件");
                            }
                        }
                    }
                } else if (action === "import-file") {
                    // 根据文件夹类型设置文件过滤器
                    const folderType = (context.folderType || "").toLowerCase();
                    let filters: { name: string; extensions: string[] }[] = [];

                    if (folderType === "text") {
                        filters = [
                            {
                                name: "XHTML/HTML 文件",
                                extensions: ["xhtml", "html", "htm"],
                            },
                        ];
                    } else if (folderType === "styles") {
                        filters = [{ name: "CSS 文件", extensions: ["css"] }];
                    } else if (folderType === "fonts") {
                        filters = [
                            {
                                name: "字体文件",
                                extensions: ["ttf", "otf", "woff", "woff2"],
                            },
                        ];
                    } else if (folderType === "images") {
                        filters = [
                            {
                                name: "图片文件",
                                extensions: [
                                    "jpg",
                                    "jpeg",
                                    "png",
                                    "gif",
                                    "svg",
                                    "webp",
                                ],
                            },
                        ];
                    } else {
                        filters = [{ name: "所有文件", extensions: ["*"] }];
                    }

                    // 打开文件选择对话框
                    const selected = await open({
                        multiple: true,
                        filters,
                    });

                    if (selected) {
                        const files = Array.isArray(selected)
                            ? selected
                            : [selected];
                        let importedCount = 0;

                        for (const filePath of files) {
                            try {
                                // 获取文件名
                                const fileName =
                                    filePath.split(/[\\/]/).pop() || "unknown";
                                // 计算目标路径
                                const targetPath =
                                    context.path + "/" + fileName;

                                // 读取文件内容
                                const fileData = await invoke<number[]>(
                                    "read_binary_file",
                                    {
                                        path: filePath,
                                    },
                                );

                                // 添加到 EPUB
                                await invoke("add_epub_file_binary", {
                                    epubPath,
                                    filePath: targetPath,
                                    content: fileData,
                                });

                                importedCount++;
                            } catch (e) {
                                console.error(`导入失败: ${filePath}`, e);
                            }
                        }

                        if (importedCount > 0) {
                            alert(`成功导入 ${importedCount} 个文件`);
                            await loadEpub();
                            // 展开导入到的目标文件夹
                            if (context.path) {
                                expandedFolders.add(context.path);
                                expandedFolders = expandedFolders;
                            }
                        }
                    }
                }
            } catch (err) {
                console.error("Action error:", err);
                alert("操作失败: " + err);
            }
        };

        const handleSelectAll = () => {
            epubCodeEditorComponent?.selectAll();
        };

        window.addEventListener("editor-select-all", handleSelectAll);
        window.addEventListener(
            "context-menu-action",
            handleContextMenuAction as unknown as EventListener,
        );
        return () => {
            window.removeEventListener("editor-select-all", handleSelectAll);
            window.removeEventListener(
                "context-menu-action",
                handleContextMenuAction as unknown as EventListener,
            );
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
        skipAssets: boolean = false,
    ): Promise<string> {
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, "text/html");

        const links = Array.from(
            doc.querySelectorAll('link[rel="stylesheet"]'),
        );
        const images = Array.from(doc.querySelectorAll("img"));
        // 支持 SVG 中的 image 元素 (xlink:href 或 href)
        const svgImages = Array.from(doc.querySelectorAll("image"));

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
        if (!skipAssets && cssPaths.length > 0) {
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
        const imageElemMap = new Map<string, Element[]>();

        for (const img of images) {
            const src = img.getAttribute("src");
            if (src && !src.startsWith("http") && !src.startsWith("data:")) {
                const imgPath = resolvePath(filePath, src);
                if (!assetCache.has(imgPath)) {
                    imagePaths.push(imgPath);
                }
                const existing = imageElemMap.get(imgPath) || [];
                existing.push(img);
                imageElemMap.set(imgPath, existing);
            }
        }

        // 处理 SVG image 元素 (xlink:href 或 href)
        for (const svgImg of svgImages) {
            // SVG image 可能使用 xlink:href 或 href
            const href =
                svgImg.getAttributeNS("http://www.w3.org/1999/xlink", "href") ||
                svgImg.getAttribute("href");
            if (href && !href.startsWith("http") && !href.startsWith("data:")) {
                const imgPath = resolvePath(filePath, href);
                if (!assetCache.has(imgPath)) {
                    imagePaths.push(imgPath);
                }
                const existing = imageElemMap.get(imgPath) || [];
                existing.push(svgImg);
                imageElemMap.set(imgPath, existing);
            }
        }

        // 5. 批量读取所有二进制资源（CSS 引用的字体 + 图片）
        const allBinaryPaths = [...binaryPaths, ...imagePaths];
        let binaryData: Record<string, number[]> = {};

        if (!skipAssets && allBinaryPaths.length > 0) {
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

        // 8. 处理图片 (包括 SVG image 元素)
        for (const [imgPath, imgElements] of imageElemMap) {
            const blobUrl = assetCache.get(imgPath);
            if (blobUrl) {
                for (const img of imgElements) {
                    if (img.tagName.toLowerCase() === "image") {
                        // SVG image 元素需要设置 href 和 xlink:href
                        img.setAttributeNS(
                            "http://www.w3.org/1999/xlink",
                            "href",
                            blobUrl,
                        );
                        img.setAttribute("href", blobUrl);
                    } else {
                        // 普通 img 元素
                        img.setAttribute("src", blobUrl);
                    }
                }
            }
        }

        // 注入全局样式：只移除html/body的默认边距，避免出现滚动条，并隐藏滚动条但保留滚动功能
        const globalStyle = doc.createElement("style");

        globalStyle.textContent = `
            /* 隐藏滚动条但保留滚动功能，不强制移除 margin/padding 以保留默认或书籍样式 */
            html { 
                overflow-x: hidden !important;
                scrollbar-width: none !important; /* Firefox */
                -ms-overflow-style: none !important; /* IE */
            }
            body {
                overflow-x: hidden !important;
                scrollbar-width: none !important;
                -ms-overflow-style: none !important;
            }
            /* Chrome/Safari 隐藏滚动条 */
            ::-webkit-scrollbar {
                display: none !important;
            }

            /* 默认隐藏 EPUB 注脚，模拟阅读器行为 */
            aside[epub\\:type="footnote"] {
                display: none;
            }

            /* 注脚弹窗样式 */
            .footnote-popup {
                position: fixed;
                background: #fff;
                border: 1px solid #ddd;
                box-shadow: 0 4px 12px rgba(0,0,0,0.15);
                padding: 12px;
                max-width: 80%;
                z-index: 9999;
                border-radius: 6px;
                font-size: 14px;
                color: #333;
                line-height: 1.5;
                pointer-events: auto;
                animation: fadeIn 0.2s ease;
            }
            @keyframes fadeIn {
                from { opacity: 0; transform: translateY(5px); }
                to { opacity: 1; transform: translateY(0); }
            }

            /* 选中颜色高亮 */
            ::selection {
                background-color: #ffeb3b !important;
                color: #000 !important;
            }
        `;
        doc.head.appendChild(globalStyle);

        // 注入滚动同步脚本：监听来自父窗口的滚动消息
        const syncScript = doc.createElement("script");
        syncScript.textContent = `

            // 接收父窗口消息
            // 自动处理头图全宽 (三面贴边)
            function fixHeaderImage() {
                try {
                    const body = document.body;
                    const style = window.getComputedStyle(body);
                    const paddingLeft = parseFloat(style.paddingLeft) || 0;
                    const paddingRight = parseFloat(style.paddingRight) || 0;
                    const paddingTop = parseFloat(style.paddingTop) || 0;
                    const marginLeft = parseFloat(style.marginLeft) || 0;
                    const marginRight = parseFloat(style.marginRight) || 0;
                    const marginTop = parseFloat(style.marginTop) || 0;

                    // 1. Find the first visual element
                    let target = null;
                    for (let i = 0; i < body.children.length; i++) {
                        const el = body.children[i];
                        // Skip non-visual or utility tags
                        if (['SCRIPT', 'STYLE', 'LINK', 'META', 'ASIDE', 'NOSCRIPT', 'TEMPLATE'].includes(el.tagName)) continue;
                        
                        // We found the first potential content element.
                        // Check if it qualifies as a header image.
                        // Must contain an image or be an image
                        const imgs = el.querySelectorAll('img, svg, image');
                        const isImgTag = ['IMG', 'SVG', 'IMAGE'].includes(el.tagName);
                        
                        if (isImgTag || imgs.length > 0) {
                             // Check text content length if it's a container
                             if (!isImgTag) {
                                 const textLen = el.innerText.replace(/\s/g, '').length;
                                 if (textLen > 50) {
                                     break; // Too much text, not a header image container
                                 }
                             }
                             target = el;
                        }
                        
                        // Stop searching after the first visual element
                        break;
                    }

                    if (!target) return;
                    
                    // 2. Validate Width Heuristic
                    // We only want to force full-bleed if the image is INTENDED to be wide.
                    // If it's a small logo or centered cover (e.g. width: 40%), we should leave it alone.
                    let shouldFix = false;
                    const innerImgs = target.querySelectorAll('img, svg, image');
                    
                    if (innerImgs.length > 0) {
                        const firstImg = innerImgs[0];
                        const rect = firstImg.getBoundingClientRect();
                        
                        if (rect.width === 0) {
                            // Image not loaded yet, wait for it
                            firstImg.addEventListener('load', fixHeaderImage);
                            return; 
                        }
                        // Only fix if image occupies > 60% of viewport
                        if (rect.width > window.innerWidth * 0.6) {
                            shouldFix = true;
                        }
                    } else if (['IMG', 'SVG', 'IMAGE'].includes(target.tagName)) {
                        const rect = target.getBoundingClientRect();
                        if (rect.width === 0) {
                             target.addEventListener('load', fixHeaderImage);
                             return;
                        }
                        if (rect.width > window.innerWidth * 0.6) {
                             shouldFix = true;
                        }
                    }
                    
                    if (!shouldFix) return;

                    // 3. Apply Absolute Positioning Strategy
                    if (target.dataset.headerFixed) return;
                    target.dataset.headerFixed = 'true';

                    // Force Styles matches viewport
                    target.style.setProperty('position', 'absolute', 'important');
                    target.style.setProperty('top', '0', 'important');
                    target.style.setProperty('left', '0', 'important');
                    target.style.setProperty('width', '100vw', 'important');
                    target.style.setProperty('max-width', 'none', 'important');
                    target.style.setProperty('box-sizing', 'border-box', 'important');
                    target.style.setProperty('margin', '0', 'important');
                    target.style.setProperty('padding', '0', 'important');
                    target.style.setProperty('z-index', '0', 'important');

                    // Force internal images to fill
                    innerImgs.forEach(img => {
                        img.style.setProperty('width', '100%', 'important');
                        img.style.setProperty('max-width', 'none', 'important');
                        img.style.setProperty('display', 'block', 'important');
                        img.style.setProperty('margin', '0', 'important');
                        img.style.setProperty('padding', '0', 'important');
                    });

                    // 4. Insert Spacer
                    const spacer = document.createElement('div');
                    spacer.style.width = '100%';
                    spacer.style.margin = '0';
                    spacer.style.padding = '0';
                    spacer.style.pointerEvents = 'none';
                    
                    // Insert spacer after target (or append if target is last)
                    if (target.nextSibling) {
                        target.parentElement.insertBefore(spacer, target.nextSibling);
                    } else {
                        target.parentElement.appendChild(spacer);
                    }

                    // 5. Sync Spacer Height
                    const updateSpacer = () => {
                        if (target && spacer) {
                            const height = target.getBoundingClientRect().height;
                            if (height > 0) {
                                spacer.style.height = height + 'px';
                            }
                        }
                    };

                    if (window.ResizeObserver) {
                        new ResizeObserver(updateSpacer).observe(target);
                    }
                    window.addEventListener('resize', updateSpacer);
                    setTimeout(updateSpacer, 50);
                    setTimeout(updateSpacer, 200);
                    setTimeout(updateSpacer, 1000);
                    innerImgs.forEach(img => img.addEventListener('load', updateSpacer));
                } catch (e) {
                    console.error('Header fix error:', e);
                }
            }
            
            // 尝试多次执行以应对加载延迟
            window.addEventListener('load', fixHeaderImage);
            window.addEventListener('DOMContentLoaded', fixHeaderImage);
            setTimeout(fixHeaderImage, 50);
            setTimeout(fixHeaderImage, 500);

            // 接收父窗口消息
            let isRemoteSelecting = false;
            window.addEventListener('message', function(event) {
                if (event.data) {
                    if (event.data.type === 'editorScroll') {
                        // 滚动同步 (百分比)
                        const scrollPercent = event.data.percent;
                        const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
                        const targetScroll = maxScroll * scrollPercent;
                        window.scrollTo({ top: targetScroll, behavior: 'smooth' });
                    } else if (event.data.type === 'editorClick') {
                        // 兼容旧的百分比点击 (Fallback)
                        const percent = event.data.percent;
                        const maxScroll = document.documentElement.scrollHeight - window.innerHeight;
                        const targetScroll = maxScroll * percent;
                        window.scrollTo({ top: targetScroll, behavior: 'smooth' });
                    } else if (event.data.type === 'editorSelection' || event.data.type === 'editorClickText') {
                        const text = event.data.text;
                        if (!text) return;
                        
                        // 先清除当前选中
                        isRemoteSelecting = true;
                        window.getSelection().removeAllRanges();
                        // 搜索文本
                        const found = window.find(text, false, false, true, false, false, false);
                        
                        if (found) {
                            const selection = window.getSelection();
                            if (selection.rangeCount > 0) {
                                // 点击时居中，拖拽选中时最近
                                const blockMode = event.data.type === 'editorClickText' ? 'center' : 'nearest';
                                selection.getRangeAt(0).startContainer.parentElement.scrollIntoView({ behavior: 'smooth', block: blockMode });
                            }
                            setTimeout(() => { isRemoteSelecting = false; }, 500);
                        } else {
                            // 未找到时，简单的 fallback (可选)
                            isRemoteSelecting = false;
                        }
                    }
                }
            });

             // 双向同步：点击预览区，通知编辑器
            // 双向同步：点击预览区，通知编辑器
            document.addEventListener('click', function(e) {
                // 忽略注脚点击和链接
                if (e.target.closest('[zy-footnote]') || e.target.closest('a')) return;

                let text = '';
                const selection = document.getSelection();
                if (selection && selection.toString().trim().length > 1) {
                    text = selection.toString();
                } else {
                    // 若无选中，取点击元素的文本 (适度截断)
                    const target = e.target;
                    text = target.innerText || target.textContent || '';
                    // 增加截断长度以确保完整匹配
                    if (text.length > 150) text = text.substring(0, 150);
                }

                if (text && text.trim().length >= 1) {
                    // Normalize
                    text = text.replace(/\s+/g, " ").trim();
                    
                    // 获取上下文 (父级Block元素的文本)
                    let context = "";
                    let p = e.target;
                    while (p && p !== document.body) {
                        const style = window.getComputedStyle(p);
                        if (style.display === 'block' || style.display === 'list-item') {
                            context = p.innerText || p.textContent || "";
                            // 增加 context 长度以改善匹配
                            if (context.length > 500) context = context.substring(0, 500);
                            break;
                        }
                        p = p.parentElement;
                    }
                    
                    window.parent.postMessage({ type: 'previewClick', text: text, context: context }, '*');
                }
            });

            // 双向同步：选中文本 (Debounced)
            let selectionTimeout;
            document.addEventListener('selectionchange', function() {
                if (isRemoteSelecting) return;
                
                clearTimeout(selectionTimeout);
                selectionTimeout = setTimeout(() => {
                    const selection = document.getSelection();
                    const text = selection ? selection.toString() : '';
                    if (text && text.length >= 1) { 
                        window.parent.postMessage({ type: 'previewSelection', text: text }, '*');
                    }
                }, 300); // 300ms debounce
            });

            // 注脚处理 (Hover)
            // 使用 mouseover/mouseout 代理
            document.addEventListener('mouseover', function(e) {
                let target = e.target;
                
                // 检查是否是注脚触发器
                let footnoteTrigger = null;
                let cursor = target;
                while (cursor && cursor !== document.body) {
                    if (cursor.getAttribute('zy-footnote') || (cursor.tagName === 'A' && cursor.getAttribute('epub:type') === 'noteref')) {
                        footnoteTrigger = cursor;
                        break;
                    }
                    cursor = cursor.parentElement;
                }

                if (footnoteTrigger) {
                    // 显示弹窗
                    let content = footnoteTrigger.getAttribute('zy-footnote');
                    if (!content && footnoteTrigger.tagName === 'A') {
                         const href = footnoteTrigger.getAttribute('href');
                         if (href && href.startsWith('#')) {
                             const noteEl = document.getElementById(href.substring(1));
                             if (noteEl) content = noteEl.innerText;
                         }
                    }

                    if (content) {
                        // 检查是否已存在
                        let popup = document.querySelector('.footnote-popup');
                        if (!popup) {
                            popup = document.createElement('div');
                            popup.className = 'footnote-popup';
                            document.body.appendChild(popup);
                        }
                        popup.innerText = content;
                        
                        // Positioning
                        const rect = footnoteTrigger.getBoundingClientRect();
                        popup.style.top = (rect.bottom + 5) + 'px';
                        popup.style.left = Math.max(10, Math.min(window.innerWidth - 300, rect.left)) + 'px';
                        popup.style.display = 'block';
                    }
                }
            });

            document.addEventListener('mouseout', function(e) {
                 // 简单的隐藏逻辑：如果在弹窗外移动，且移出的元素是注脚
                 // 更好的方式：检查鼠标是否进入了弹窗？
                 // 简化实现：移出 footnoteTrigger 就隐藏，除非移入的是 popup (太复杂了)
                 // 先简单实现：主要针对 footnoteTrigger 的 mouseleave
                 
                 let target = e.target;
                 if (target.getAttribute('zy-footnote') || (target.tagName === 'A' && target.getAttribute('epub:type') === 'noteref')) {
                     const popup = document.querySelector('.footnote-popup');
                     if (popup) {
                         popup.style.display = 'none';
                     }
                 }
            });

        `;
        doc.head.appendChild(syncScript);

        return doc.documentElement.outerHTML;
    }

    function hasUnsavedChanges(): boolean {
        return modifiedFiles.size > 0 || isProjectDirty;
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

                    // Update Preview (Stage 1: Fast render without assets)
                    const fastPreview = await processHtmlForPreview(
                        newContent,
                        selectedFile.path,
                        currentGeneration,
                        true, // skipAssets
                    );
                    if (fastPreview) {
                        previewContent = fastPreview;
                        // Don't cache fast preview to force full load next time?
                        // Actually, better caching logic needed, but for now ok.
                    }

                    // Update Preview (Stage 2: Full render with assets)
                    const fullPreview = await processHtmlForPreview(
                        newContent,
                        selectedFile.path,
                        currentGeneration,
                        false, // load assets
                    );
                    if (fullPreview) {
                        previewContent = fullPreview;
                        previewCache.set(selectedFile.path, fullPreview);
                    }
                }
            }, 500);
        }
    }

    async function saveEpub() {
        if (isSaving) return;
        isSaving = true;
        try {
            await invoke("save_epub_to_disk", { epubPath });
            isProjectDirty = false;
            // Clear modifiedFiles? No, modifiedFiles tracks editor vs temp.
            // If we saved temp to disk, temp is still "modified" vs what was in editor?
            // No, modifiedFiles means "Editor content" != "File content".
            // When we "saveCurrentFile", we write Editor -> Temp, and clear modifiedFiles.
            // So modifiedFiles should be empty if all files are saved to temp.
            // isProjectDirty means Temp != Original.
            // So this logic holds.
        } catch (e) {
            console.error("Save failed:", e);
            await confirm(`保存主要文件失败: ${e}`, {
                title: "错误",
                kind: "error",
            });
        } finally {
            isSaving = false;
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
            isProjectDirty = true;
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

    function expandParents(nodes: EpubFileNode[], targetPath: string): boolean {
        for (const node of nodes) {
            if (node.path === targetPath) return true;
            if (node.children) {
                if (expandParents(node.children, targetPath)) {
                    expandedFolders.add(node.path);
                    return true;
                }
            }
        }
        return false;
    }

    async function selectFile(
        file: EpubFileNode,
        event?: MouseEvent | KeyboardEvent,
    ) {
        if (file.file_type === "folder") return;

        // Multi-select logic
        if (event && (event as MouseEvent).ctrlKey) {
            if (multiSelectedFiles.has(file.path)) {
                multiSelectedFiles.delete(file.path);
            } else {
                multiSelectedFiles.add(file.path);
            }
            multiSelectedFiles = multiSelectedFiles; // trigger update
            // Also set as current selected file for preview? Yes.
        } else {
            // Normal click clears multi-select unless right-click
            // Note: Right-click usually doesn't trigger 'click' event but 'contextmenu'
            // We handle context menu separately.
            // But if user Left Clicks, clear multi-select
            multiSelectedFiles.clear();
            multiSelectedFiles.add(file.path);
            multiSelectedFiles = multiSelectedFiles;
        }

        // 增加代数，使得之前的 pending 请求失效
        currentGeneration++;
        const generation = currentGeneration;

        selectedFile = file;

        // 展开父目录并滚动到文件树位置
        expandParents(fileTree, file.path);
        expandedFolders = expandedFolders; // 触发更新
        await tick();
        const fileNode = document.querySelector(
            `.tree-node[data-path="${file.path.replace(/"/g, '\\"')}"]`,
        );
        if (fileNode) {
            fileNode.scrollIntoView({ block: "center", behavior: "smooth" });
        }

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

        // UX Optimization: Immediate Switch
        if (
            file.file_type === "html" ||
            file.name.endsWith(".xhtml") ||
            file.name.endsWith(".html")
        ) {
            activeTab = "preview";
        }

        // 立即清理旧内容，避免视觉混淆
        // 如果有内容缓存，先显示内容缓存
        if (fileContentCache.has(file.path)) {
            fileContent = fileContentCache.get(file.path)!;
        } else {
            fileContent = "加载中...";
        }

        // 如果是 HTML 文件且没命中预览缓存，显示加载中
        // 对于非 HTML 文件，保留当前预览内容
        const isHtml =
            file.file_type === "html" ||
            file.name.endsWith(".xhtml") ||
            file.name.endsWith(".html");
        if (isHtml && !previewCache.has(file.path)) {
            // 设为空字符串，让 placeholder div 显示而不是 iframe
            previewContent = "";
        }

        try {
            // 图片处理逻辑
            if (isImageFile(file.name)) {
                currentImageSrc = null;
                // 检查资源缓存
                if (assetCache.has(file.path)) {
                    currentImageSrc = assetCache.get(file.path)!;
                } else {
                    // 读取二进制
                    try {
                        const binaryData = await invoke<
                            Record<string, number[]>
                        >("read_epub_binary_batch", {
                            epubPath: epubPath,
                            filePaths: [file.path],
                        });
                        const data = binaryData[file.path];
                        if (data) {
                            const uint8Array = new Uint8Array(data);
                            let mimeType = "image/jpeg";
                            const lower = file.name.toLowerCase();
                            if (lower.endsWith(".png")) mimeType = "image/png";
                            else if (lower.endsWith(".gif"))
                                mimeType = "image/gif";
                            else if (lower.endsWith(".svg"))
                                mimeType = "image/svg+xml";
                            else if (lower.endsWith(".webp"))
                                mimeType = "image/webp";

                            const blob = new Blob([uint8Array], {
                                type: mimeType,
                            });
                            const url = URL.createObjectURL(blob);
                            assetCache.set(file.path, url);
                            currentImageSrc = url;
                        }
                    } catch (err) {
                        console.error("Failed to load image", err);
                        fileContent = "图片加载失败";
                    }
                }
                fileContent = ""; // Clear editor content
                // 不清除 previewContent，保留最近的 HTML 预览
                return;
            } else {
                currentImageSrc = null;
            }

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

            // 3. 仅对 HTML 文件进行预览处理
            if (
                file.file_type === "html" ||
                file.name.endsWith(".xhtml") ||
                file.name.endsWith(".html")
            ) {
                // Single-stage loading with full assets for visual consistency
                const processed = await processHtmlForPreview(
                    fileContent,
                    file.path,
                    generation,
                    false, // load all assets
                );

                if (currentGeneration === generation && processed) {
                    previewContent = processed;
                    previewCache.set(file.path, processed);
                    activeTab = "preview";

                    // 5. 触发相邻章节预加载 (在后台异步执行)
                    preloadNeighbors(file);
                }
            } else {
                // 对于非 HTML 文件（如 XML, OPF, NCX, CSS, 图片），保留当前预览
                // 如果之前有 HTML 预览，保持不变；否则预览区保持目录列表
                // (不清空 previewContent，所以保留最近的 HTML 预览)
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
    let spinePaths: string[] = []; // Store Spine reading order paths

    function toggleTocItem(id: string) {
        if (expandedTocItems.has(id)) {
            expandedTocItems.delete(id);
        } else {
            expandedTocItems.add(id);
        }
        expandedTocItems = expandedTocItems;
    }

    async function loadSpine() {
        spinePaths = [];
        try {
            // 1. Find rootfile (OPF)
            // Usually in META-INF/container.xml pointing to OPF, but here we scan OEBPS
            // Implementation shortcut: assume we know where OPF is or search for it.
            // Our file structure usually has OEBPS/content.opf
            let opfPath = "OEBPS/content.opf"; // Default guess

            // Try reading content directly
            let content = "";
            try {
                const res = await invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath,
                        filePaths: [opfPath],
                    },
                );
                if (res[opfPath]) {
                    content = res[opfPath];
                } else {
                    // Try another common path?
                    // For now just fail gracefully
                }
            } catch (e) {
                console.warn("OPF read failed:", e);
            }

            if (!content) return; // Exit if no content

            const parser = new DOMParser();
            const doc = parser.parseFromString(content, "text/xml");

            // Map id -> href (manifest)
            const manifestItems = Array.from(
                doc.querySelectorAll("manifest > item"),
            );
            const idToHref = new Map<string, string>();
            for (const item of manifestItems) {
                const id = item.getAttribute("id");
                const href = item.getAttribute("href");
                if (id && href) {
                    idToHref.set(id, href);
                }
            }

            // Get Spine idrefs
            const itemrefs = Array.from(
                doc.querySelectorAll("spine > itemref"),
            );
            const opfDir = opfPath.substring(0, opfPath.lastIndexOf("/"));

            for (const itemref of itemrefs) {
                const idref = itemref.getAttribute("idref");
                if (idref && idToHref.has(idref)) {
                    let href = idToHref.get(idref)!;
                    // Resolve to absolute path in zip (e.g. OEBPS/Text/foo.xhtml)
                    // If href is relative to opf
                    const fullPath = opfDir ? opfDir + "/" + href : href;
                    spinePaths.push(fullPath);
                }
            }
            console.log("[DEBUG] Spine paths loaded:", spinePaths.length);
        } catch (e) {
            console.warn("Failed to load Spine:", e);
        }
    }

    // Helper: Add new file to OPF Manifest and Spine (inserted AFTER a sibling)
    async function addToOpf(newFullPath: string, insertAfterFullPath: string) {
        try {
            let opfPath = "OEBPS/content.opf";
            const res = await invoke<Record<string, string>>(
                "read_epub_files_batch",
                {
                    epubPath,
                    filePaths: [opfPath],
                },
            );
            const opfContent = res[opfPath];
            if (!opfContent) throw new Error("无法读取 OPF 文件");

            const parser = new DOMParser();
            const doc = parser.parseFromString(opfContent, "text/xml");
            const opfDir = opfPath.substring(0, opfPath.lastIndexOf("/"));

            // 1. Calculate relative paths
            // newFullPath: OEBPS/Text/new.xhtml -> relative to OPF: Text/new.xhtml
            // We assume simple structure where opfDir prefixes logic works.
            const getRelPath = (full: string) =>
                full.startsWith(opfDir + "/")
                    ? full.substring(opfDir.length + 1)
                    : full;

            const newHref = getRelPath(newFullPath);
            const insertAfterHref = getRelPath(insertAfterFullPath);

            // 2. Add to Manifest
            const manifest = doc.querySelector("manifest");
            if (manifest) {
                const newItem = doc.createElement("item");
                // ID generation: "x" + filename slug or random
                const newId =
                    "x" +
                        newFullPath
                            .split("/")
                            .pop()
                            ?.replace(/[^a-zA-Z0-9]/g, "") || "newitem";
                newItem.setAttribute("id", newId);
                newItem.setAttribute("href", newHref);
                newItem.setAttribute("media-type", "application/xhtml+xml"); // Assume XHTML
                manifest.appendChild(newItem);

                // 3. Add to Spine
                const spine = doc.querySelector("spine");
                if (spine) {
                    // 查找 insertAfter 的 idref
                    // Find ID of neighbor
                    const neighborItem = manifest.querySelector(
                        `item[href="${insertAfterHref}"]`,
                    );
                    const neighborId = neighborItem
                        ? neighborItem.getAttribute("id")
                        : null;

                    const newItemRef = doc.createElement("itemref");
                    newItemRef.setAttribute("idref", newId);

                    if (neighborId) {
                        const neighborRef = spine.querySelector(
                            `itemref[idref="${neighborId}"]`,
                        );
                        if (neighborRef && neighborRef.nextSibling) {
                            spine.insertBefore(
                                newItemRef,
                                neighborRef.nextSibling,
                            );
                        } else {
                            spine.appendChild(newItemRef);
                        }
                    } else {
                        spine.appendChild(newItemRef);
                    }
                }
            }

            // Serialize and Save
            const serializer = new XMLSerializer();
            const newOpfContent = serializer.serializeToString(doc);

            await invoke("save_epub_file_content", {
                epubPath: epubPath,
                filePath: opfPath,
                content: newOpfContent,
            });
        } catch (e) {
            console.error("Failed to update OPF:", e);
            alert("更新 OPF 失败, 排序可能不正确: " + e);
        }
    }

    // New helper for assets (only Manifest, no Spine)
    async function addAssetToOpf(newFullPath: string) {
        try {
            let opfPath = "OEBPS/content.opf";
            const res = await invoke<Record<string, string>>(
                "read_epub_files_batch",
                {
                    epubPath,
                    filePaths: [opfPath],
                },
            );
            let opfContent = res[opfPath];
            if (!opfContent) return;

            const parser = new DOMParser();
            const doc = parser.parseFromString(opfContent, "text/xml");
            const opfDir = opfPath.substring(0, opfPath.lastIndexOf("/"));

            const getRelPath = (full: string) =>
                full.startsWith(opfDir + "/")
                    ? full.substring(opfDir.length + 1)
                    : full;

            const newHref = getRelPath(newFullPath);
            const manifest = doc.querySelector("manifest");

            if (manifest) {
                // Check duplicate
                if (manifest.querySelector(`item[href="${newHref}"]`)) {
                    console.log("Item already exists in manifest:", newHref);
                    return;
                }

                const newItem = doc.createElement("item");
                const newId =
                    "x" +
                        newFullPath
                            .split("/")
                            .pop()
                            ?.replace(/[^a-zA-Z0-9]/g, "") || "asset";

                // Guess media-type
                let mediaType = "application/octet-stream";
                if (newFullPath.endsWith(".css")) mediaType = "text/css";
                else if (
                    newFullPath.endsWith(".jpg") ||
                    newFullPath.endsWith(".jpeg")
                )
                    mediaType = "image/jpeg";
                else if (newFullPath.endsWith(".png")) mediaType = "image/png";
                else if (newFullPath.endsWith(".gif")) mediaType = "image/gif";
                else if (newFullPath.endsWith(".ttf")) mediaType = "font/ttf";
                else if (newFullPath.endsWith(".woff")) mediaType = "font/woff";
                else if (newFullPath.endsWith(".woff2"))
                    mediaType = "font/woff2";

                newItem.setAttribute("id", newId);
                newItem.setAttribute("href", newHref);
                newItem.setAttribute("media-type", mediaType);
                manifest.appendChild(newItem);

                const serializer = new XMLSerializer();
                const newOpf = serializer.serializeToString(doc);
                await invoke("save_epub_file_content", {
                    epubPath,
                    filePath: opfPath,
                    content: newOpf,
                });
            }
        } catch (e) {
            console.warn("addAssetToOpf failed", e);
        }
    }

    async function handleFileDrop(e: DragEvent) {
        console.log("File dropped", e.dataTransfer?.files);
        e.preventDefault();
        e.stopPropagation();
        if (!epubPath) {
            alert("错误: EPUB 路径丢失");
            return;
        }

        // Wait, regular web DnD: we can read file content via FileReader.
        if (
            e.dataTransfer &&
            e.dataTransfer.files &&
            e.dataTransfer.files.length > 0
        ) {
            const files = e.dataTransfer.files;
            const newSpineItems: string[] = [];
            const newAssetItems: string[] = [];
            const fileWrites: Promise<any>[] = [];

            // 1. Prepare all file writes
            for (let i = 0; i < files.length; i++) {
                const file = files[i];
                const name = file.name.toLowerCase();
                let targetFolder = "";
                let isSpineItem = false;

                if (name.endsWith(".html") || name.endsWith(".xhtml")) {
                    targetFolder = "OEBPS/Text";
                    isSpineItem = true;
                } else if (name.endsWith(".css")) {
                    targetFolder = "OEBPS/Styles";
                } else if (/\.(jpg|jpeg|png|gif|webp|svg|bmp)$/.test(name)) {
                    targetFolder = "OEBPS/Images";
                } else if (/\.(ttf|otf|woff|woff2)$/.test(name)) {
                    targetFolder = "OEBPS/Fonts";
                } else {
                    targetFolder = "OEBPS/Misc";
                }

                const newPath = targetFolder + "/" + file.name;

                if (isSpineItem) {
                    newSpineItems.push(newPath);
                } else {
                    newAssetItems.push(newPath);
                }

                // Prepare Payload for Batch Write
                // We convert everything to Vec<u8> (binary) for simplicity in Rust
                fileWrites.push(
                    (async () => {
                        const buffer = await file.arrayBuffer();
                        const uint8 = new Uint8Array(buffer);
                        return { path: newPath, content: Array.from(uint8) };
                    })(),
                );
            }

            // 2. Execute Batch Write
            try {
                const preparedFiles = await Promise.all(fileWrites);
                const filesMap: Record<string, number[]> = {};
                for (const f of preparedFiles) {
                    filesMap[f.path] = f.content;
                }

                await invoke("save_epub_files_batch", {
                    epubPath,
                    files: filesMap,
                });
            } catch (err) {
                console.error("Batch file write failed", err);
                alert("导入文件出错: " + err);
                return;
            }

            // 3. Update OPF once (Batch mode)
            try {
                // Manually implement simplified batch OPF update here to avoid N reads/writes
                // Reuse logic from addToOpf but for a list
                let opfPath = "OEBPS/content.opf";
                const res = await invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath,
                        filePaths: [opfPath],
                    },
                );
                const opfContent = res[opfPath];
                if (opfContent) {
                    const parser = new DOMParser();
                    const doc = parser.parseFromString(opfContent, "text/xml");
                    const opfDir = opfPath.substring(
                        0,
                        opfPath.lastIndexOf("/"),
                    );

                    const getRelPath = (full: string) =>
                        full.startsWith(opfDir + "/")
                            ? full.substring(opfDir.length + 1)
                            : full;

                    const manifest = doc.querySelector("manifest");
                    const spine = doc.querySelector("spine");

                    if (manifest && spine) {
                        // Add Assets
                        for (const path of newAssetItems) {
                            const href = getRelPath(path);
                            if (manifest.querySelector(`item[href="${href}"]`))
                                continue;

                            const newItem = doc.createElement("item");
                            // ID generation
                            const safeName =
                                path
                                    .split("/")
                                    .pop()
                                    ?.replace(/[^a-zA-Z0-9]/g, "") || "asset";
                            const newId =
                                "x" +
                                safeName +
                                Math.floor(Math.random() * 1000); // add random

                            // Media type guess
                            let mediaType = "application/octet-stream";
                            if (path.endsWith(".css")) mediaType = "text/css";
                            else if (/\.(jpg|jpeg|png)$/.test(path))
                                mediaType = "image/jpeg";
                            else if (path.endsWith(".gif"))
                                mediaType = "image/gif";
                            else if (path.endsWith(".svg"))
                                mediaType = "image/svg+xml";
                            else if (path.endsWith(".ttf"))
                                mediaType = "font/ttf";
                            else if (path.endsWith(".woff"))
                                mediaType = "font/woff";
                            else if (path.endsWith(".woff2"))
                                mediaType = "font/woff2";

                            newItem.setAttribute("id", newId);
                            newItem.setAttribute("href", href);
                            newItem.setAttribute("media-type", mediaType);
                            manifest.appendChild(newItem);
                        }

                        // Add Spine Items
                        for (const path of newSpineItems) {
                            const href = getRelPath(path);
                            // Manifest entry first
                            if (
                                !manifest.querySelector(`item[href="${href}"]`)
                            ) {
                                const newItem = doc.createElement("item");
                                const safeName =
                                    path
                                        .split("/")
                                        .pop()
                                        ?.replace(/[^a-zA-Z0-9]/g, "") ||
                                    "chapter";
                                const newId =
                                    "x" +
                                    safeName +
                                    Math.floor(Math.random() * 1000);
                                newItem.setAttribute("id", newId);
                                newItem.setAttribute("href", href);
                                newItem.setAttribute(
                                    "media-type",
                                    "application/xhtml+xml",
                                );
                                manifest.appendChild(newItem);

                                // Spine entry
                                const newItemRef = doc.createElement("itemref");
                                newItemRef.setAttribute("idref", newId);
                                spine.appendChild(newItemRef);
                            }
                        }

                        // Save OPF
                        const serializer = new XMLSerializer();
                        const newOpf = serializer.serializeToString(doc);
                        await invoke("save_epub_file_content", {
                            epubPath,
                            filePath: opfPath,
                            content: newOpf,
                        });
                    }
                }
            } catch (e) {
                console.error("Batch OPF update failed", e);
            }

            await loadEpub();
            isProjectDirty = true;
        }
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        e.dataTransfer!.dropEffect = "copy";
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

            // Text Folder Sorting (Priority: Spine > TOC > Alphabetical)
            if (parentName === "text") {
                // 1. Spine Order (User Custom Order)
                const spineIdx = spinePaths.indexOf(node.path);
                if (spineIdx !== -1) return spineIdx;

                // 2. TOC Order (Legacy fallback)
                const tocIdx = tocPaths.indexOf(node.path);
                if (tocIdx !== -1) return 10000 + tocIdx;

                // 3. Alphabetical (fallback) - Handled by sort() default string comp after weight
                return 20000;
            }

            return 0; // Default
        };

        const sortRecursive = (
            nodes: EpubFileNode[],
            parentName: string = "",
        ) => {
            nodes.sort((a, b) => {
                const wA = getWeight(a, parentName);
                const wB = getWeight(b, parentName);
                if (wA !== wB) return wA - wB;
                return a.name.localeCompare(b.name, undefined, {
                    numeric: true,
                });
            });

            nodes.forEach((node) => {
                if (node.children) {
                    node.children = sortRecursive(
                        node.children,
                        node.name.toLowerCase(),
                    );
                }
            });
            return [...nodes]; // 返回新引用的副本以触发响应式
        };

        fileTree = sortRecursive(fileTree);
        expandedFolders = expandedFolders; // Trigger reactivity
    }

    async function loadTOC() {
        // 每次刷新时重置 tocList，确保重新解析
        tocList = [];
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
            // 即使没有 TOC，也执行默认排序
            sortFileTree(fileTree, []);
            fileTree = [...fileTree]; // 使用新数组引用触发响应式更新
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
                fileTree = [...fileTree]; // 使用新数组引用触发响应式更新
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
            const sizeStr = file.size
                ? `${Math.round(file.size / 1024)}KB`
                : "";
            // 如果后端提供了分辨率，直接显示，否则显示“图片”
            return file.resolution
                ? `${file.resolution} ${sizeStr}`
                : `图片 ${sizeStr}`;
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

    // ========== 查找替换功能 ==========

    // 获取搜索范围内的文件列表
    function getFilesInScope(): EpubFileNode[] {
        switch (searchScope) {
            case "current":
                return selectedFile ? [selectedFile] : [];
            case "selected":
                if (multiSelectedFiles.size > 0) {
                    // Convert Set to Array of EpubFileNode
                    // This requires finding nodes by path.
                    // flatHtmlFiles only contains HTML.
                    // We need a map or traverse.
                    // For performance, we can just filter allFiles or Use flatHtmlFiles if we assume HTML only?
                    // Selected files might be CSS.
                    // Let's traverse tree or use a cache.
                    const selected: EpubFileNode[] = [];
                    const traverse = (nodes: EpubFileNode[]) => {
                        for (const node of nodes) {
                            if (multiSelectedFiles.has(node.path))
                                selected.push(node);
                            if (node.children) traverse(node.children);
                        }
                    };
                    traverse(fileTree);
                    return selected;
                }
                return selectedFile ? [selectedFile] : [];
            case "open":
                return openTabs.filter(
                    (tab) =>
                        tab.file_type === "html" ||
                        tab.file_type === "css" ||
                        tab.file_type === "xml" ||
                        tab.name.endsWith(".xhtml") ||
                        tab.name.endsWith(".html"),
                );
            case "html":
                return flatHtmlFiles;
            case "all":
                // 递归获取所有可编辑文件
                const allFiles: EpubFileNode[] = [];
                const collectFiles = (nodes: EpubFileNode[]) => {
                    for (const node of nodes) {
                        if (node.file_type !== "folder") {
                            if (isEditable(node.file_type)) {
                                allFiles.push(node);
                            }
                        }
                        if (node.children) {
                            collectFiles(node.children);
                        }
                    }
                };
                collectFiles(fileTree);
                return allFiles;
            default:
                return [];
        }
    }

    // 在当前文件中搜索
    function searchInCurrentFile(
        content: string,
        pattern: string,
        startPos: number = 0,
        direction: "down" | "up" = "down",
        useWrap: boolean = true,
    ): { from: number; to: number } | null {
        if (!pattern) return null;

        try {
            if (isRegex) {
                const regex = new RegExp(pattern, "g");
                const matches: { from: number; to: number }[] = [];
                let match;
                while ((match = regex.exec(content)) !== null) {
                    matches.push({
                        from: match.index,
                        to: match.index + match[0].length,
                    });
                }
                if (matches.length === 0) return null;

                if (direction === "down") {
                    for (const m of matches) {
                        if (m.from >= startPos) return m;
                    }
                    return useWrap && wrapAround ? matches[0] : null;
                } else {
                    for (let i = matches.length - 1; i >= 0; i--) {
                        if (matches[i].to <= startPos) return matches[i];
                    }
                    return useWrap && wrapAround
                        ? matches[matches.length - 1]
                        : null;
                }
            } else {
                // 普通文本搜索
                if (direction === "down") {
                    const index = content.indexOf(pattern, startPos);
                    if (index !== -1) {
                        return { from: index, to: index + pattern.length };
                    }
                    if (useWrap && wrapAround) {
                        const wrapIndex = content.indexOf(pattern, 0);
                        if (wrapIndex !== -1 && wrapIndex < startPos) {
                            return {
                                from: wrapIndex,
                                to: wrapIndex + pattern.length,
                            };
                        }
                    }
                } else {
                    const searchContent = content.substring(0, startPos);
                    const index = searchContent.lastIndexOf(pattern);
                    if (index !== -1) {
                        return { from: index, to: index + pattern.length };
                    }
                    if (useWrap && wrapAround) {
                        const wrapIndex = content.lastIndexOf(pattern);
                        if (wrapIndex !== -1 && wrapIndex >= startPos) {
                            return {
                                from: wrapIndex,
                                to: wrapIndex + pattern.length,
                            };
                        }
                    }
                }
            }
        } catch (e) {
            searchMessage = "正则表达式语法错误";
            return null;
        }
        return null;
    }

    // 向下查找
    async function findNext() {
        if (!findPattern) {
            searchMessage = "请输入查找内容";
            return;
        }

        // 保存到历史
        savePatternToHistory();

        const files = getFilesInScope();
        if (files.length === 0) {
            searchMessage = "没有可搜索的文件";
            return;
        }

        // 获取当前文件索引
        const currentPath = selectedFile?.path || "";
        let currentFileIndex = files.findIndex((f) => f.path === currentPath);
        if (currentFileIndex === -1) currentFileIndex = 0;

        // 获取当前编辑器的实际内容（而非缓存）
        let content = "";
        if (epubCodeEditorComponent) {
            const view = epubCodeEditorComponent.getView();
            if (view) {
                content = view.state.doc.toString();
            }
        }
        if (!content) content = fileContent;

        // 确定搜索起始位置
        let startPos = 0;
        if (currentMatchInfo && currentMatchInfo.filePath === currentPath) {
            startPos =
                searchDirection === "down"
                    ? currentMatchInfo.to
                    : currentMatchInfo.from;
        }

        // 在当前文件中搜索
        // 多文件模式下禁用 wrapAround，让搜索继续到下一个文件
        const isMultiFile = searchScope !== "current" && files.length > 1;
        let result = searchInCurrentFile(
            content,
            findPattern,
            startPos,
            searchDirection,
            !isMultiFile, // 多文件模式下不使用 wrap
        );

        if (result) {
            // 在当前文件找到匹配
            currentMatchInfo = {
                filePath: currentPath,
                from: result.from,
                to: result.to,
            };
            searchMessage = `找到匹配`;

            // 选中匹配内容
            if (epubCodeEditorComponent) {
                const view = epubCodeEditorComponent.getView();
                if (view) {
                    view.dispatch({
                        selection: { anchor: result.from, head: result.to },
                        scrollIntoView: true,
                    });
                    view.focus();
                }
            }
            return;
        }

        // 如果只搜索当前文件，没找到就结束
        if (searchScope === "current" || files.length === 1) {
            searchMessage = "未找到匹配";
            currentMatchInfo = null;
            return;
        }

        // 多文件搜索：继续搜索其他文件
        const direction = searchDirection === "down" ? 1 : -1;
        const filesSearched = new Set<string>();
        filesSearched.add(currentPath);

        let nextIndex = currentFileIndex;
        while (filesSearched.size < files.length) {
            nextIndex = (nextIndex + direction + files.length) % files.length;
            const nextFile = files[nextIndex];

            if (filesSearched.has(nextFile.path)) break;
            filesSearched.add(nextFile.path);

            // 获取文件内容
            let nextContent = fileContentCache.get(nextFile.path) || "";
            if (!nextContent) {
                try {
                    nextContent = await invoke<string>(
                        "read_epub_file_content",
                        {
                            epubPath: epubPath,
                            filePath: nextFile.path,
                        },
                    );
                    fileContentCache.set(nextFile.path, nextContent);
                } catch (e) {
                    continue;
                }
            }

            // 从文件开头搜索（多文件时不使用 wrapAround）
            let matchResult: { from: number; to: number } | null = null;
            try {
                if (isRegex) {
                    const regex = new RegExp(findPattern, "g");
                    let match;
                    const matches: { from: number; to: number }[] = [];
                    while ((match = regex.exec(nextContent)) !== null) {
                        matches.push({
                            from: match.index,
                            to: match.index + match[0].length,
                        });
                    }
                    if (matches.length > 0) {
                        matchResult =
                            searchDirection === "down"
                                ? matches[0]
                                : matches[matches.length - 1];
                    }
                } else {
                    const idx =
                        searchDirection === "down"
                            ? nextContent.indexOf(findPattern)
                            : nextContent.lastIndexOf(findPattern);
                    if (idx !== -1) {
                        matchResult = {
                            from: idx,
                            to: idx + findPattern.length,
                        };
                    }
                }
            } catch (e) {
                continue;
            }
            result = matchResult;

            if (result) {
                // 切换到该文件
                await selectFile(nextFile);
                await tick();
                await new Promise((resolve) => setTimeout(resolve, 100));

                // 从编辑器当前内容重新搜索匹配位置
                if (epubCodeEditorComponent) {
                    const view = epubCodeEditorComponent.getView();
                    if (view) {
                        const actualContent = view.state.doc.toString();

                        // 在实际编辑器内容中重新搜索
                        let actualResult: { from: number; to: number } | null =
                            null;
                        try {
                            if (isRegex) {
                                const regex = new RegExp(findPattern, "g");
                                let match;
                                const matches: { from: number; to: number }[] =
                                    [];
                                while (
                                    (match = regex.exec(actualContent)) !== null
                                ) {
                                    matches.push({
                                        from: match.index,
                                        to: match.index + match[0].length,
                                    });
                                }
                                if (matches.length > 0) {
                                    actualResult =
                                        searchDirection === "down"
                                            ? matches[0]
                                            : matches[matches.length - 1];
                                }
                            } else {
                                const idx =
                                    searchDirection === "down"
                                        ? actualContent.indexOf(findPattern)
                                        : actualContent.lastIndexOf(
                                              findPattern,
                                          );
                                if (idx !== -1) {
                                    actualResult = {
                                        from: idx,
                                        to: idx + findPattern.length,
                                    };
                                }
                            }
                        } catch (e) {
                            // ignore
                        }

                        if (actualResult) {
                            currentMatchInfo = {
                                filePath: nextFile.path,
                                from: actualResult.from,
                                to: actualResult.to,
                            };
                            searchMessage = `找到匹配 (${nextFile.name})`;

                            view.dispatch({
                                selection: {
                                    anchor: actualResult.from,
                                    head: actualResult.to,
                                },
                                scrollIntoView: true,
                            });
                            view.focus();
                        }
                    }
                }
                return;
            }
        }

        // 所有文件都搜索完毕，没找到
        searchMessage = "未找到匹配";
        currentMatchInfo = null;
    }

    // 向上查找
    async function findPrev() {
        const origDirection = searchDirection;
        searchDirection = searchDirection === "down" ? "up" : "down";
        await findNext();
        searchDirection = origDirection;
    }

    // 替换当前选中
    function performReplace() {
        if (!currentMatchInfo || !epubCodeEditorComponent) {
            searchMessage = "请先查找";
            return;
        }

        const view = epubCodeEditorComponent?.getView();
        if (!view) return;

        let replacement = replacePattern;

        // 文本模式：不替换标签内的内容
        if (textOnly && isRegex) {
            // 简化处理：暂时直接替换
        }

        view.dispatch({
            changes: {
                from: currentMatchInfo.from,
                to: currentMatchInfo.to,
                insert: replacement,
            },
        });

        // 调整匹配位置
        const diff =
            replacement.length - (currentMatchInfo.to - currentMatchInfo.from);
        currentMatchInfo.to = currentMatchInfo.from + replacement.length;

        searchMessage = "已替换";

        // 继续查找下一个
        findNext();
    }

    // 全部替换
    async function performReplaceAll() {
        if (!findPattern) {
            searchMessage = "请输入查找内容";
            return;
        }

        const files = getFilesInScope();
        let totalReplaced = 0;

        for (const file of files) {
            let content = fileContentCache.get(file.path) || "";
            if (!content) {
                try {
                    content = await invoke<string>("read_epub_file_content", {
                        epubPath: epubPath,
                        filePath: file.path,
                    });
                } catch (e) {
                    continue;
                }
            }

            let newContent = content;
            let replacedCount = 0;

            try {
                if (isRegex) {
                    const regex = new RegExp(findPattern, "g");
                    const matches = content.match(regex);
                    replacedCount = matches ? matches.length : 0;
                    newContent = content.replace(regex, replacePattern);
                } else {
                    // 计算替换次数
                    let pos = 0;
                    while ((pos = content.indexOf(findPattern, pos)) !== -1) {
                        replacedCount++;
                        pos += findPattern.length;
                    }
                    newContent = content
                        .split(findPattern)
                        .join(replacePattern);
                }
            } catch (e) {
                searchMessage = "正则表达式语法错误";
                return;
            }

            if (newContent !== content) {
                totalReplaced += replacedCount;
                fileContentCache.set(file.path, newContent);
                modifiedFiles.add(file.path);

                // 如果是当前文件，更新编辑器
                if (file.path === selectedFile?.path) {
                    fileContent = newContent;
                    if (epubCodeEditorComponent) {
                        epubCodeEditorComponent.resetDoc(newContent);
                    }
                }
            }
        }

        modifiedFiles = modifiedFiles; // 触发响应式
        searchMessage = `已替换 ${totalReplaced} 处`;
        currentMatchInfo = null;
    }

    // 计算匹配数量
    // 计算匹配数量
    async function countMatches() {
        try {
            if (!findPattern) {
                searchMessage = "请输入查找内容";
                return;
            }
            searchMessage = "计算中...";
            await tick();

            const files = getFilesInScope();
            const backendFiles = [];
            const localFiles = [];
            for (const file of files) {
                if (
                    (file.path === selectedFile?.path &&
                        epubCodeEditorComponent) ||
                    fileContentCache.has(file.path)
                ) {
                    localFiles.push(file);
                } else {
                    backendFiles.push(file.path);
                }
            }

            let totalCount = 0;
            // Backend
            if (backendFiles.length > 0) {
                searchMessage = `正在后台搜索 ${backendFiles.length} 个文件...`;
                await tick();
                try {
                    const count = await invoke("search_in_files", {
                        epubPath,
                        files: backendFiles,
                        pattern: findPattern,
                        isRegex,
                    });
                    totalCount += count as number;
                } catch (e) {
                    console.error(e);
                    // If backend fails, totalCount isn't incremented. User sees partial result.
                }
            }
            // Frontend
            if (localFiles.length > 0) {
                searchMessage = `正在搜索缓存文件...`;
                await tick();
                let processed = 0;
                for (const file of localFiles) {
                    processed++;
                    if (processed % 50 === 0)
                        await new Promise((r) => setTimeout(r, 0));
                    let content = "";
                    if (
                        file.path === selectedFile?.path &&
                        epubCodeEditorComponent
                    ) {
                        try {
                            content =
                                epubCodeEditorComponent
                                    .getView()
                                    ?.state.doc.toString() || fileContent;
                        } catch {
                            content = fileContent;
                        }
                    } else {
                        content = fileContentCache.get(file.path) || "";
                    }

                    try {
                        if (isRegex) {
                            totalCount += (
                                content.match(new RegExp(findPattern, "g")) ||
                                []
                            ).length;
                        } else {
                            let pos = 0;
                            while (
                                (pos = content.indexOf(findPattern, pos)) !== -1
                            ) {
                                totalCount++;
                                pos += findPattern.length;
                            }
                        }
                    } catch {
                        searchMessage = "正则表达式错误";
                        return;
                    }
                }
            }
            searchMessage = `共 ${totalCount} 处匹配`;
        } catch (e: any) {
            searchMessage = "错: " + e.message;
        }
    }

    async function countMatches_OLD() {
        try {
            if (!findPattern) {
                searchMessage = "请输入查找内容";
                return;
            }

            searchMessage = "计算中...";
            await tick();

            const files = getFilesInScope();

            // 1. 批量预加载内容 (优化速度)
            const filesToFetch = files.filter(
                (f) =>
                    !(
                        f.path === selectedFile?.path && epubCodeEditorComponent
                    ) && !fileContentCache.has(f.path),
            );

            if (filesToFetch.length > 0) {
                const BATCH_SIZE = 20;
                for (let i = 0; i < filesToFetch.length; i += BATCH_SIZE) {
                    const batch = filesToFetch.slice(i, i + BATCH_SIZE);
                    searchMessage = `正在加载文件 ${i}/${filesToFetch.length}...`;
                    await tick();

                    await Promise.all(
                        batch.map(async (file) => {
                            try {
                                const content = await invoke<string>(
                                    "read_epub_file_content",
                                    {
                                        epubPath: epubPath,
                                        filePath: file.path,
                                    },
                                );
                                fileContentCache.set(file.path, content);
                            } catch (e) {}
                        }),
                    );

                    await new Promise((r) => setTimeout(r, 0));
                }
            }

            let totalCount = 0;
            let processedCount = 0;

            searchMessage = `正在搜索...`;
            await tick();

            for (const file of files) {
                // 每处理几个文件让出主线程，避免界面卡死
                processedCount++;
                if (processedCount % 5 === 0) {
                    searchMessage = `搜索中 ${processedCount}/${files.length}`;
                    await new Promise((r) => setTimeout(r, 0));
                }

                let content = "";

                // 当前打开的文件，从编辑器获取实际内容
                if (
                    file.path === selectedFile?.path &&
                    epubCodeEditorComponent
                ) {
                    try {
                        const view = epubCodeEditorComponent.getView();
                        if (view) {
                            content = view.state.doc.toString();
                        } else {
                            content = fileContent;
                        }
                    } catch (e) {
                        content = fileContent;
                    }
                } else {
                    // 其他文件从缓存或后端获取
                    content = fileContentCache.get(file.path) || "";
                    if (!content) {
                        try {
                            content = await invoke<string>(
                                "read_epub_file_content",
                                {
                                    epubPath: epubPath,
                                    filePath: file.path,
                                },
                            );
                            fileContentCache.set(file.path, content);
                        } catch (e) {
                            continue;
                        }
                    }
                }

                try {
                    if (isRegex) {
                        const regex = new RegExp(findPattern, "g");
                        const matches = content.match(regex);
                        totalCount += matches ? matches.length : 0;
                    } else {
                        let pos = 0;
                        while (
                            (pos = content.indexOf(findPattern, pos)) !== -1
                        ) {
                            totalCount++;
                            pos += findPattern.length;
                        }
                    }
                } catch (e) {
                    searchMessage = "正则表达式语法错误";
                    return;
                }
            }

            searchMessage = `共 ${totalCount} 处匹配`;
        } catch (e: any) {
            console.error("Count error:", e);
            searchMessage = "计算出错: " + (e.message || "未知错误");
        }
    }

    // 关闭查找面板
    function closeFindReplace() {
        showFindReplace = false;
        showFindHistory = false;
        showReplaceHistory = false;
        currentMatchInfo = null;
        searchMessage = "";
    }

    // 保存查找历史到 localStorage
    function saveSearchHistory() {
        localStorage.setItem("epub-find-history", JSON.stringify(findHistory));
        localStorage.setItem(
            "epub-replace-history",
            JSON.stringify(replaceHistory),
        );
    }

    // 加载查找历史
    function loadSearchHistory() {
        try {
            const findH = localStorage.getItem("epub-find-history");
            const replaceH = localStorage.getItem("epub-replace-history");
            if (findH) findHistory = JSON.parse(findH);
            if (replaceH) replaceHistory = JSON.parse(replaceH);
            // 自动填充上次的值
            if (findHistory.length > 0) findPattern = findHistory[0];
            if (replaceHistory.length > 0) replacePattern = replaceHistory[0];
        } catch (e) {}
    }

    // 添加到历史记录
    function addToHistory(text: string, history: string[]): string[] {
        if (!text.trim()) return history;
        // 移除重复项
        const filtered = history.filter((h) => h !== text);
        // 添加到开头
        const newHistory = [text, ...filtered].slice(0, MAX_HISTORY);
        return newHistory;
    }

    // 从历史记录中删除
    function removeFromFindHistory(index: number) {
        findHistory = findHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

    function removeFromReplaceHistory(index: number) {
        replaceHistory = replaceHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

    // 选中历史记录
    function selectFindHistory(text: string) {
        findPattern = text;
        showFindHistory = false;
    }

    function selectReplaceHistory(text: string) {
        replacePattern = text;
        showReplaceHistory = false;
    }

    // 在执行查找时保存历史
    function savePatternToHistory() {
        if (findPattern) {
            findHistory = addToHistory(findPattern, findHistory);
        }
        if (replacePattern) {
            replaceHistory = addToHistory(replacePattern, replaceHistory);
        }
        saveSearchHistory();
    }

    function handleKeydown(e: KeyboardEvent) {
        // Ctrl+F 打开查找
        if (e.ctrlKey && e.key === "f") {
            e.preventDefault();
            e.stopPropagation();
            showFindReplace = true;
        }
        // Ctrl+H 打开替换
        if (e.ctrlKey && e.key === "h") {
            e.preventDefault();
            e.stopPropagation();
            showFindReplace = true;
        }
        // Escape 关闭
        if (e.key === "Escape" && showFindReplace) {
            closeFindReplace();
        }
        // F3 查找下一个
        if (e.key === "F3" && showFindReplace) {
            e.preventDefault();
            if (e.shiftKey) {
                findPrev();
            } else {
                findNext();
            }
        }
        // Ctrl+S 保存
        if (e.ctrlKey && e.key === "s") {
            e.preventDefault();
            e.stopPropagation();
            if (modifiedFiles.size > 0) {
                saveCurrentFile().then(() => {
                    saveEpub();
                });
            } else if (isProjectDirty) {
                saveEpub();
            }
        }
    }

    // 监听键盘事件 (capture phase to intercept before CodeMirror)
    onMount(() => {
        loadSearchHistory();
    });
</script>

<svelte:window
    on:keydown={handleKeydown}
    on:dragover={handleDragOver}
    on:drop={handleFileDrop}
/>

<!-- 自定义输入对话框 (替代 JavaScript prompt) -->
{#if showPrompt}
    <div class="prompt-overlay" on:click={handlePromptCancel}>
        <div class="prompt-dialog" on:click|stopPropagation>
            <div class="prompt-title">{promptTitle}</div>
            <input
                type="text"
                class="prompt-input"
                bind:value={promptValue}
                on:keydown={(e) => e.key === "Enter" && handlePromptConfirm()}
                disabled={isPromptBusy}
                autofocus
            />

            {#if promptOptions}
                <div class="prompt-options">
                    <label class="prompt-checkbox">
                        <input
                            type="checkbox"
                            bind:checked={promptCheckValue}
                            disabled={isPromptBusy}
                        />
                        自动更新其他文件中的链接引用
                    </label>
                </div>
            {/if}

            <div class="prompt-buttons">
                <button
                    class="prompt-btn cancel"
                    on:click={handlePromptCancel}
                    disabled={isPromptBusy}>取消</button
                >
                <button
                    class="prompt-btn confirm"
                    on:click={handlePromptConfirm}
                    disabled={isPromptBusy}
                >
                    {isPromptBusy ? "正在更新链接..." : "确定"}
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- 自定义确认对话框 -->
{#if showConfirm}
    <div
        class="prompt-overlay"
        on:click={handleConfirmCancel}
        role="presentation"
    >
        <div
            class="prompt-dialog"
            on:click|stopPropagation
            role="dialog"
            aria-modal="true"
        >
            <div class="prompt-title">{confirmTitle}</div>
            <div
                class="prompt-message"
                style="margin-bottom: 20px; color: #ccc;"
            >
                {confirmMessage}
            </div>
            <div class="prompt-buttons">
                <button class="prompt-btn cancel" on:click={handleConfirmCancel}
                    >取消</button
                >
                <button
                    class="prompt-btn confirm delete"
                    on:click={handleConfirmConfirm}>删除</button
                >
            </div>
        </div>
    </div>
{/if}

<div
    class="epub-editor"
    on:drop={handleFileDrop}
    on:dragover={handleDragOver}
    role="region"
    aria-label="File Drop Zone"
>
    {#if isLoading}
        <div class="loading">少女祈祷中······</div>
    {:else if error}
        <div class="error">{error}</div>
    {:else}
        <!-- 左侧：文件树 -->
        <aside class="file-tree">
            <div class="tree-header">
                <h3>文件结构</h3>
                <div class="header-actions">
                    <button
                        class="icon-btn"
                        on:click={saveEpub}
                        disabled={!isProjectDirty && modifiedFiles.size === 0}
                        title="保存所有更改 (Ctrl+S)"
                        class:dirty={isProjectDirty || modifiedFiles.size > 0}
                    >
                        💾
                    </button>
                    <button class="icon-btn" on:click={loadEpub} title="刷新">
                        🔄
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
                        {toggleFolder}
                        {selectFile}
                        {getFileIcon}
                        {getFileDescription}
                    />
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
                {#if isImageFile(selectedFile.name)}
                    <div class="image-preview-container">
                        {#if currentImageSrc}
                            <img
                                src={currentImageSrc}
                                alt={selectedFile.name}
                            />
                        {:else}
                            <div class="loading">加载图片中...</div>
                        {/if}
                    </div>
                {:else}
                    <div class="editor-content" bind:this={editorContentDiv}>
                        {#if isEditable(selectedFile.file_type)}
                            <EpubCodeEditor
                                bind:this={epubCodeEditorComponent}
                                doc={fileContent}
                                language={getFileLanguage(
                                    selectedFile.file_type,
                                )}
                                onChange={handleFileContentChange}
                                onSave={saveCurrentFile}
                                onClick={handleEditorClick}
                                onSelectionChange={handleEditorSelection}
                            />
                        {:else}
                            <pre class="code-block">{@html addLineNumbers(
                                    fileContent
                                        .replace(/</g, "&lt;")
                                        .replace(/>/g, "&gt;"),
                                )}</pre>
                        {/if}
                    </div>
                {/if}
            {:else}
                <div class="placeholder">点击左侧文件以查看内容</div>
            {/if}

            <!-- 查找替换面板 -->
            {#if showFindReplace}
                <div class="find-replace-panel">
                    <div class="fr-row">
                        <span class="fr-label">查找:</span>
                        <div class="fr-input-wrapper">
                            <input
                                type="text"
                                class="fr-input"
                                bind:value={findPattern}
                                on:keydown={(e) =>
                                    e.key === "Enter" && findNext()}
                                on:focus={() => {
                                    showFindHistory = false;
                                    showReplaceHistory = false;
                                }}
                            />
                            <button
                                class="fr-dropdown-btn"
                                on:click={() => {
                                    showFindHistory = !showFindHistory;
                                    showReplaceHistory = false;
                                }}
                                title="历史记录">▼</button
                            >
                            {#if showFindHistory && findHistory.length > 0}
                                <div class="fr-history-dropdown">
                                    {#each findHistory as item, i}
                                        <div class="fr-history-item">
                                            <span
                                                class="fr-history-text"
                                                on:click={() =>
                                                    selectFindHistory(item)}
                                                on:keydown={(e) =>
                                                    e.key === "Enter" &&
                                                    selectFindHistory(item)}
                                                role="button"
                                                tabindex="0">{item}</span
                                            >
                                            <button
                                                class="fr-history-del"
                                                on:click|stopPropagation={() =>
                                                    removeFromFindHistory(i)}
                                                title="删除">✕</button
                                            >
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                        <div class="fr-actions">
                            <button
                                class="fr-btn fr-btn-text"
                                on:click={findPrev}
                                title="上一个">上一个</button
                            >
                            <button
                                class="fr-btn fr-btn-text"
                                on:click={findNext}
                                title="下一个">下一个</button
                            >
                            <button
                                class="fr-btn fr-close-btn"
                                on:click={closeFindReplace}
                                title="关闭">✕</button
                            >
                        </div>
                    </div>
                    <div class="fr-row">
                        <span class="fr-label">替换:</span>
                        <div class="fr-input-wrapper">
                            <input
                                type="text"
                                class="fr-input"
                                bind:value={replacePattern}
                                on:focus={() => {
                                    showFindHistory = false;
                                    showReplaceHistory = false;
                                }}
                            />
                            <button
                                class="fr-dropdown-btn"
                                on:click={() => {
                                    showReplaceHistory = !showReplaceHistory;
                                    showFindHistory = false;
                                }}
                                title="历史记录">▼</button
                            >
                            {#if showReplaceHistory && replaceHistory.length > 0}
                                <div class="fr-history-dropdown">
                                    {#each replaceHistory as item, i}
                                        <div class="fr-history-item">
                                            <span
                                                class="fr-history-text"
                                                on:click={() =>
                                                    selectReplaceHistory(item)}
                                                on:keydown={(e) =>
                                                    e.key === "Enter" &&
                                                    selectReplaceHistory(item)}
                                                role="button"
                                                tabindex="0">{item}</span
                                            >
                                            <button
                                                class="fr-history-del"
                                                on:click|stopPropagation={() =>
                                                    removeFromReplaceHistory(i)}
                                                title="删除">✕</button
                                            >
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                        <div class="fr-actions">
                            <button
                                class="fr-btn fr-btn-text"
                                on:click={performReplace}
                                title="替换">替换</button
                            >
                            <button
                                class="fr-btn fr-btn-text fr-danger"
                                on:click={performReplaceAll}
                                title="全部替换">全部</button
                            >
                            <button
                                class="fr-btn fr-btn-text"
                                on:click={countMatches}
                                title="计数">计数</button
                            >
                        </div>
                    </div>
                    <div class="fr-row fr-options">
                        <span class="fr-label">模式:</span>
                        <select
                            class="fr-select fr-select-sm"
                            bind:value={isRegex}
                        >
                            <option value={false}>正常</option>
                            <option value={true}>正则</option>
                        </select>
                        <select class="fr-select" bind:value={searchScope}>
                            <option value="current">当前文件</option>
                            <option value="open">已打开</option>
                            <option value="html">HTML文件</option>
                            <option value="selected">选中文件</option>
                            <option value="all">所有文件</option>
                        </select>
                        <select
                            class="fr-select fr-select-sm"
                            bind:value={searchDirection}
                        >
                            <option value="down">下</option>
                            <option value="up">上</option>
                        </select>
                        <label class="fr-checkbox"
                            ><input
                                type="checkbox"
                                bind:checked={wrapAround}
                            />循环</label
                        >
                        <label class="fr-checkbox"
                            ><input
                                type="checkbox"
                                bind:checked={textOnly}
                            />文本</label
                        >
                        <span class="fr-message">{searchMessage}</span>
                    </div>
                </div>
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

                    {#if previewContent}
                        <!-- 如果有预览内容（HTML），显示iframe -->
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
                                ? "少女祈祷中······"
                                : "请从左侧选择一个文件"}
                        </div>
                    {/if}
                </div>
            {:else}
                <div class="toc-container">
                    {#if isTocLoading}
                        <div class="loading">少女祈祷中······</div>
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

    /* Ensure container is relative and takes full height for centering */
    .preview-container {
        position: relative;
        flex: 1;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }
    .epub-editor {
        display: flex;
        height: 100vh;
        background: #f5f5f5;
    }

    /* 主页面加载（大字体） */
    .loading,
    .error {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        font-size: 36px;
        color: #888;
        font-weight: 500;
        letter-spacing: 3px;
    }

    .error {
        color: #d32f2f;
    }

    /* 预览区占位符（小字体居中） */
    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        font-size: 18px;
        color: #999;
        font-weight: 400;
        letter-spacing: 2px;
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
        justify-content: center; /* Center the title */
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

    /* 编辑器 */
    .editor-pane {
        flex: 1;
        display: flex;
        flex-direction: column;
        background: #fff;
        border-right: 1px solid #ddd;
        min-width: 0; /* 关键：允许 flex 子项收缩，从而触发内部滚动 */
        position: relative; /* 使查找替换面板的 absolute 定位相对于此元素 */
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
        width: 320px; /* Standard Android/iPhone Width */
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
        flex: 1; /* Grow vertically */
        width: 100%; /* Fill sidebar width */
        display: flex;
        flex-direction: column;
        overflow-y: auto; /* Allow scrolling if phone height > window height */
        overflow-x: hidden; /* Prevent horizontal scrollbar */
        background: #f0f0f0; /* Darker background to distinguish phone frame */
        align-items: center; /* Center horizontally */
        justify-content: flex-start; /* Align to top (placeholder will center itself) */
        padding: 0; /* No padding */
    }

    .mobile-frame {
        width: 100%; /* 320px */
        height: 711px; /* 20:9 ratio based on 320px width */
        flex: 0 0 auto; /* Fixed height, don't grow/shrink */
        background: #fff;
        border: 1px solid #ddd;
        box-sizing: border-box; /* Prevent border from adding width */
        box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1); /* Shadow for depth */
        /* overflow-y: auto;  <- Internal scroll is handled by iframe content usually? No, iframe has own scroll. */
        /* But we want the FRAME to be the viewport. */
        overflow: hidden; /* Hide overflow outside the "phone screen" */
        position: relative;
    }

    .mobile-frame::-webkit-scrollbar {
        display: none;
    }

    /* 封面专用框架：自动高度，无固定比例 */
    .mobile-frame-cover {
        height: auto !important;
        max-height: none !important;
        flex: 0 0 auto !important;
    }

    .mobile-frame-cover iframe {
        height: auto !important;
        min-height: 200px;
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

    /* 查找替换面板 */
    .find-replace-panel {
        background: #f0f0f0;
        border-top: 1px solid #ccc;
        padding: 6px 10px;
        font-size: 13px;
        flex-shrink: 0;
    }

    .fr-row {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-bottom: 6px;
    }

    .fr-actions {
        display: flex;
        gap: 4px;
        width: 154px; /* 固定宽度以对齐两行按钮 */
        justify-content: space-between;
        flex-shrink: 0;
    }

    .fr-row:last-child {
        margin-bottom: 0;
    }

    .fr-label {
        font-weight: 600;
        color: #555;
        min-width: 40px;
        text-align: right;
    }

    .fr-input {
        flex: 1;
        padding: 6px 10px;
        border: 1px solid #ccc;
        border-radius: 4px;
        font-size: 13px;
        font-family: "Consolas", "Monaco", monospace;
        background: #fff;
    }

    .fr-input:focus {
        outline: none;
        border-color: #2196f3;
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-btn {
        padding: 4px 8px;
        border: 1px solid #bbb;
        border-radius: 4px;
        background: linear-gradient(to bottom, #fff, #e8e8e8);
        cursor: pointer;
        font-size: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        white-space: nowrap;
    }

    .fr-btn-text {
        padding: 4px 10px;
    }

    .fr-btn:hover {
        background: linear-gradient(to bottom, #f5f5f5, #ddd);
        border-color: #999;
    }

    .fr-btn:active {
        background: #ddd;
    }

    .fr-danger {
        background: linear-gradient(to bottom, #fff0e6, #ffddcc);
        border-color: #e65c00;
    }

    .fr-danger:hover {
        background: linear-gradient(to bottom, #ffe6d9, #ffcc99);
    }

    .fr-close-btn {
        background: linear-gradient(to bottom, #fee, #fcc);
        border-color: #d99;
    }

    .fr-close-btn:hover {
        background: linear-gradient(to bottom, #fdd, #faa);
        border-color: #c66;
    }

    .fr-select-sm {
        min-width: 50px;
        padding: 4px 6px;
    }

    .fr-select {
        padding: 4px 8px;
        border: 1px solid #ccc;
        border-radius: 4px;
        background: #fff;
        font-size: 12px;
        min-width: 75px;
    }

    .fr-select:focus {
        outline: none;
        border-color: #2196f3;
    }

    .fr-options {
        font-size: 12px;
    }

    .fr-checkbox {
        display: flex;
        align-items: center;
        gap: 4px;
        cursor: pointer;
        color: #555;
    }

    .fr-checkbox input {
        margin: 0;
        cursor: pointer;
    }

    .fr-message {
        margin-left: auto;
        color: #2196f3;
        font-weight: 500;
        min-width: 80px;
        text-align: right;
    }

    /* 输入框包装器（含下拉按钮） */
    .fr-input-wrapper {
        flex: 1;
        display: flex;
        position: relative;
    }

    .fr-input-wrapper .fr-input {
        flex: 1;
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
        border-right: none;
    }

    .fr-dropdown-btn {
        padding: 6px 8px;
        border: 1px solid #ccc;
        border-left: none;
        border-top-right-radius: 4px;
        border-bottom-right-radius: 4px;
        background: linear-gradient(to bottom, #fff, #e8e8e8);
        cursor: pointer;
        font-size: 10px;
        color: #666;
    }

    .fr-dropdown-btn:hover {
        background: linear-gradient(to bottom, #f5f5f5, #ddd);
    }

    /* 历史记录下拉菜单 */
    .fr-history-dropdown {
        position: absolute;
        top: 100%;
        left: 0;
        right: 0;
        background: #fff;
        border: 1px solid #ccc;
        border-radius: 4px;
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
        z-index: 200;
        max-height: 200px;
        overflow-y: auto;
    }

    .fr-history-item {
        display: flex;
        align-items: center;
        padding: 6px 10px;
        border-bottom: 1px solid #eee;
    }

    .fr-history-item:last-child {
        border-bottom: none;
    }

    .fr-history-item:hover {
        background: #f5f5f5;
    }

    .fr-history-text {
        flex: 1;
        cursor: pointer;
        font-family: "Consolas", "Monaco", monospace;
        font-size: 12px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .fr-history-del {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 12px;
        color: #999;
        padding: 2px 6px;
        border-radius: 3px;
        margin-left: 8px;
    }

    .fr-history-del:hover {
        background: #ffdddd;
        color: #d32f2f;
    }
    .image-preview-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: #f0f0f0;
        overflow: auto;
        padding: 20px;
        height: 100%;
    }

    .image-preview-container img {
        max-width: 100%;
        max-height: 100%;
        box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
        /* 棋盘格背景 */
        background-color: #fff;
        background-image: linear-gradient(45deg, #eee 25%, transparent 25%),
            linear-gradient(-45deg, #eee 25%, transparent 25%),
            linear-gradient(45deg, transparent 75%, #eee 75%),
            linear-gradient(-45deg, transparent 75%, #eee 75%);
        background-size: 20px 20px;
        background-position:
            0 0,
            0 10px,
            10px -10px,
            -10px 0px;
    }

    .loading {
        color: #666;
        font-size: 14px;
    }

    /* 自定义输入对话框样式 */
    .prompt-overlay {
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 999999;
    }

    .prompt-dialog {
        background: white;
        border-radius: 12px;
        padding: 24px;
        min-width: 320px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    }

    .prompt-title {
        font-size: 16px;
        font-weight: 600;
        margin-bottom: 16px;
        color: #333;
    }

    .prompt-input {
        width: 100%;
        padding: 10px 12px;
        border: 1px solid #ddd;
        border-radius: 6px;
        font-size: 14px;
        margin-bottom: 16px;
        box-sizing: border-box;
    }

    .prompt-input:focus {
        outline: none;
        border-color: #4a90d9;
        box-shadow: 0 0 0 2px rgba(74, 144, 217, 0.2);
    }

    .prompt-options {
        margin-bottom: 20px;
    }

    .prompt-checkbox {
        display: flex;
        align-items: center;
        gap: 10px;
        font-size: 13px;
        color: #555;
        cursor: pointer;
        user-select: none;
    }

    .prompt-checkbox input {
        width: 16px;
        height: 16px;
        cursor: pointer;
        margin: 0;
    }

    .prompt-buttons {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }

    .prompt-btn {
        padding: 8px 16px;
        border: none;
        border-radius: 6px;
        font-size: 14px;
        cursor: pointer;
        transition: all 0.2s;
    }

    .prompt-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .prompt-btn.cancel {
        background: #f0f0f0;
        color: #666;
    }

    .prompt-btn.cancel:hover {
        background: #e0e0e0;
    }

    .prompt-btn.confirm {
        background: #4a90d9;
        color: white;
    }

    .prompt-btn.confirm:hover {
        background: #3a80c9;
    }
</style>
