<script lang="ts">
    import { onMount, tick } from "svelte";
    import { platform, type PlatformUnlisten } from "$lib/platform";
    import { base } from "$app/paths";
    import { goto } from "$app/navigation";
    import {
        isInHiddenView,
        onlyWhenActive,
        viewUrl,
    } from "$lib/keepAlive/viewState";
    import { closeCurrentTab, usesMultiWindow } from "$lib/secondaryTabs";

    // 同窗口保活：别的标签的页面也一直挂着，window 级监听要先判当前视图
    const activeOnly = onlyWhenActive();
    // getContext 只能在初始化时读，先取出本页自己的地址
    const ownUrl = viewUrl();
    import TocNode from "$lib/TocNode.svelte";
    import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";
    import FileTreePanel from "$lib/epubEditor/FileTreePanel.svelte";
    import TabsBar from "$lib/epubEditor/TabsBar.svelte";
    import FindReplacePanel from "$lib/epubEditor/FindReplacePanel.svelte";
    import PreviewPane from "$lib/epubEditor/PreviewPane.svelte";
    import FontPreviewPanel from "$lib/epubEditor/FontPreviewPanel.svelte";
    import Toast from "$lib/Toast.svelte";
    import { toastInfo, toastSuccess, toastError } from "$lib/toast";
    import { loadAppSettings, type GlobalAppSettings } from "$lib/appSettings";
    import {
        DEFAULT_FONT_PREVIEW_TEXT,
        type FontGlyphAnalyzeResult,
        accumulateCharUsage,
        buildFontFamilyAliases,
        collectSelectorsUsingFont,
        createFontPreviewFamily,
        fontFileStem,
        getBinaryMimeType,
        isFontFile,
        resolveCssHrefToZipPath,
        styleAttributeUsesFont,
        summarizeGlyphUsage,
    } from "$lib/fontTools";
    import {
        EPUB_SEARCH_OPTIONS_KEY,
        EPUB_FIND_HISTORY_KEY,
        EPUB_REPLACE_HISTORY_KEY,
    } from "$lib/storageKeys";
    import {
        countMatchesInContent,
        filterFilesInScope,
        findNextMatch,
        findNextMatchWrapped,
        replaceAllInContent,
        type FindQuery,
        type MatchRange,
    } from "$lib/findReplace";

    // Setup Close Listener
    onMount(() => {
        const win = platform.getCurrentWindow();
        let unlistenClose: (() => void) | null = null;
        let isUnmounted = false;

        // NOTE: Do NOT emit restore-main-window here. It should only be emitted
        // after the user confirms closing (or if there are no unsaved changes).
        // The actual restoration is handled by the confirmation dialog logic.
        win.listen("tauri://close-requested", async (event) => {
            // Prevent default close, show confirmation if needed
            // The 'beforeunload' or custom logic should handle this
            console.log("EPUB window close requested");
            // Do NOT emit restore-main-window here as it's too early
        }).then((unlisten) => {
            if (isUnmounted) {
                unlisten();
            } else {
                unlistenClose = unlisten;
            }
        });

        return () => {
            isUnmounted = true;
            unlistenClose?.();
        };
    });

    interface EpubFileNode {
        name: string;
        path: string;
        file_type: string;
        size?: number;
        title?: string;
        resolution?: string;
        children?: EpubFileNode[];
    }

    interface EpubFileMeta {
        path: string;
        title?: string;
        resolution?: string;
    }

    // 后台按需补全文件 title / resolution，不阻塞文件树首屏渲染
    async function loadFileMetaInBackground(
        epubPath: string,
        nodes: EpubFileNode[],
    ) {
        const flat = flattenFiles(nodes);
        const needMeta = flat.filter(
            (n) => n.file_type === "html" || n.file_type === "image",
        );
        if (needMeta.length === 0) return;
        try {
            const metas = await platform.invoke<EpubFileMeta[]>(
                "load_epub_file_meta",
                {
                    epubPath,
                    filePaths: needMeta.map((n) => n.path),
                },
            );
            const metaMap = new Map<string, EpubFileMeta>();
            for (const m of metas) metaMap.set(m.path, m);
            // 把补全到的 title/resolution 回写到 fileTree（浅拷贝触发响应式更新）
            fileTree = fileTree.map((node) => applyMeta(node, metaMap));
        } catch (e) {
            console.warn("文件元信息补全失败:", e);
        }
    }

    function applyMeta(
        node: EpubFileNode,
        metaMap: Map<string, EpubFileMeta>,
    ): EpubFileNode {
        const meta = metaMap.get(node.path);
        const updated = { ...node };
        if (meta) {
            if (meta.title) updated.title = meta.title;
            if (meta.resolution) updated.resolution = meta.resolution;
        }
        if (node.children) {
            updated.children = node.children.map((c) => applyMeta(c, metaMap));
        }
        return updated;
    }

    // Validation Error Interface
    interface ValidationError {
        type: "tag" | "img";
        message: string;
        line: number;
    }

    let epubPath = "";
    let sourceEpubPath = "";
    let fileTree: EpubFileNode[] = [];
    let selectedFile: EpubFileNode | null = null;
    let fileContent = "";
    let previewContent = "";
    let isLoading = true;
    let loadingMessage = "正在加载 EPUB...";
    let error = "";
    let unlistenPrepareStage: PlatformUnlisten | undefined;
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

    let currentGeneration = 0;
    let blobUrls: string[] = [];
    let assetCache: Map<string, string> = new Map();
    let fontBinaryCache: Map<string, Uint8Array> = new Map();
    let fileContentCache: Map<string, string> = new Map();
    let previewCache: Map<string, string> = new Map();
    let previewSourcePath = "";

    let flatHtmlFiles: EpubFileNode[] = [];

    let previewPane: PreviewPane | null = null;
    let editorContentDiv: HTMLElement | null = null;
    let epubCodeEditorComponent: EpubCodeEditor | null = null;

    async function closeEpubEditorWindow() {
        // 桌面端 label 恒为字符串：main 窗口回首页，其余窗口 destroy（与原
        // getCurrentWindow().label 判断等价）。web 端 label 为 null，标签页
        // window.close() 对非脚本打开的页面是 no-op，回首页是最接近
        // "关闭编辑器"的真实降级。
        // 同窗口模式（默认）下没有独立窗口可销毁：标签栏里还留着书库就回书库，
        // 否则回工具箱。
        const label = await platform.getCurrentWindowLabel();
        if (!usesMultiWindow()) {
            // 关掉编辑器时把自己的标签一并摘掉，否则回到上一页后标签栏
            // 里还挂着这本已经关掉的书；优先回书库，其它情况回最近一个标签
            const remaining = closeCurrentTab();
            const hasLibrary = remaining?.id.startsWith("/library");
            // goto 走 SPA 导航：location.href 会整页重载，标签栏跟着闪一下，
            // 且 remaining.href 已含 base 前缀，再拼一次会把路径拼坏
            await goto(
                hasLibrary ? `${base}/library` : remaining?.href || `${base}/`,
            );
            return;
        }
        if (label === "main" || label === null) {
            window.location.href = `${base}/`;
            return;
        }
        await platform.getCurrentWindow().destroy();
    }

    let openTabs: EpubFileNode[] = [];
    let activeTabIndex: number = -1;
    let tabsBar: TabsBar | null = null;

    let currentImageSrc: string | null = null;
    let currentFontSrc: string | null = null;
    let currentFontFamily = "";
    let currentFontInternalName = "";
    let fontPreviewText = DEFAULT_FONT_PREVIEW_TEXT;
    let fontStatsLoading = false;
    let fontUsedCharCount = 0;
    let fontMissingGlyphCharCount = 0;
    let fontMissingChars: string[] = [];
    let fontStatsError = "";
    let currentFontStatsToken = 0;
    let currentFontFace: FontFace | null = null;

    function isImageFile(name: string) {
        return /\.(jpg|jpeg|png|gif|webp|svg|bmp)$/i.test(name);
    }

    interface EpubPrepareResult {
        source_path: string;
        processed_path: string;
        changed: boolean;
        action: string;
    }

    interface EpubPrepareStageEvent {
        epubPath: string;
        message: string;
    }

    function showEpubPrepareNotice(prepared: EpubPrepareResult) {
        if (window.self !== window.top) {
            return;
        }

        if (
            !prepared.changed ||
            !prepared.processed_path ||
            prepared.processed_path === prepared.source_path
        ) {
            return;
        }

        toastInfo(
            [
                "EPUB 打开前需要预处理。",
                prepared.action || "已自动生成可读的工作副本。",
                "原文件未被修改，当前查看的是处理后的副本。",
                "源文件：" + prepared.source_path,
                "当前文件：" + prepared.processed_path,
            ].join("\n"),
        );
    }

    function resetFontPreviewState() {
        currentFontStatsToken++;
        if (currentFontFace) {
            document.fonts.delete(currentFontFace);
            currentFontFace = null;
        }
        currentFontSrc = null;
        currentFontFamily = "";
        currentFontInternalName = "";
        fontStatsLoading = false;
        fontUsedCharCount = 0;
        fontMissingGlyphCharCount = 0;
        fontMissingChars = [];
        fontStatsError = "";
    }

    // 预览滚动条开关（设置 → 通用），默认关闭
    let appSettings: GlobalAppSettings = loadAppSettings();
    let restoreSettingsListener: (() => void) | null = null;

    let showPrompt = false;
    let promptTitle = "";
    let promptValue = "";
    let promptOptions = false;
    let promptCheckValue = true;
    let isPromptBusy = false;
    let promptInput: HTMLInputElement | null = null;
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
            setTimeout(() => promptInput?.focus(), 0);
            console.log("[DEBUG] showPrompt set to true");
        });
    }

    function handlePromptConfirm() {
        if (isPromptBusy) return;
        console.log("[DEBUG] handlePromptConfirm called, value:", promptValue);
        if (promptResolve) {
            promptResolve({
                value: promptValue,
                confirmRefactor: promptCheckValue,
            });
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

    function handlePromptInputKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            handlePromptConfirm();
            return;
        }
        if (e.key === "Escape") {
            e.preventDefault();
            handlePromptCancel();
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

    function handlePromptOverlayClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            handlePromptCancel();
        }
    }

    function handlePromptOverlayKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            handlePromptCancel();
        }
    }

    function handleConfirmOverlayClick(e: MouseEvent) {
        if (e.target === e.currentTarget) {
            handleConfirmCancel();
        }
    }

    function handleConfirmOverlayKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            handleConfirmCancel();
        }
    }

    let showFindReplace = true;
    let findPattern = "";
    let replacePattern = "";
    let isRegex = false;
    let searchScope: "all" | "html" | "selected" | "open" | "current" =
        "current";
    let searchDirection: "down" | "up" = "down";
    let wrapAround = true;
    let textOnly = false;

    if (typeof localStorage !== "undefined") {
        const savedOptions = localStorage.getItem(EPUB_SEARCH_OPTIONS_KEY);
        if (savedOptions) {
            try {
                const options = JSON.parse(savedOptions);
                isRegex = options.isRegex ?? false;
                searchScope = options.searchScope ?? "current";
                searchDirection = options.searchDirection ?? "down";
                wrapAround = options.wrapAround ?? true;
                textOnly = options.textOnly ?? false;
            } catch (e) {}
        }
    }

    $: if (typeof localStorage !== "undefined") {
        localStorage.setItem(
            EPUB_SEARCH_OPTIONS_KEY,
            JSON.stringify({
                isRegex,
                searchScope,
                searchDirection,
                wrapAround,
                textOnly,
            }),
        );
    }
    let searchMessage = "";
    let currentMatchInfo: {
        filePath: string;
        from: number;
        to: number;
    } | null = null;

    let findHistory: string[] = [];
    let replaceHistory: string[] = [];

    // Multi-select
    let multiSelectedFiles = new Set<string>();
    const MAX_HISTORY = 10;

    // 编辑器 -> 预览：按「点击的段落 / 选中的文字」定位，而不是按滚动比例。
    // 滚动联动会让预览跟着手抖，且两边的换行密度不同，比例对不上位置。
    // 预览 iframe 已被撑到正文高度、由外层 .preview-container 滚动，
    // 所以由 PreviewPane 把命中位置换算成外层 scrollTop。
    function revealInPreview(text: string, mode: "center" | "nearest") {
        const clean = text.replace(/[\s ]+/g, " ").trim();
        if (!clean) return;
        previewPane?.revealText(clean, mode);
    }

    // 用户在编辑器里点某一行：把该行的可见文字同步到预览
    function handleEditorClick(line: number) {
        const view = epubCodeEditorComponent?.getView();
        if (!view) return;
        const lineContent = view.state.doc.line(line).text;
        const cleanText = lineContent
            .replace(/<[^>]+>/g, " ")
            .replace(/[\s ]+/g, " ")
            .trim();
        if (cleanText.length < 2) return;
        revealInPreview(cleanText, "center");
    }

    // 在编辑器里拖选一段文字：预览滚动到离当前视野最近的命中处
    let editorSelectionTimeout: ReturnType<typeof setTimeout> | null = null;
    function handleEditorSelection(text: string) {
        if (editorSelectionTimeout) clearTimeout(editorSelectionTimeout);
        editorSelectionTimeout = setTimeout(() => {
            const cleanText = text.replace(/<[^>]+>/g, "");
            if (cleanText.replace(/[\s ]+/g, "").length < 2) return;
            revealInPreview(cleanText, "nearest");
        }, 300);
    }

    // 预览 -> 编辑器：点了预览里的某段文字，把编辑器定位到对应位置
    function handlePreviewClick(payload: { text: string; context: string }) {
        if (payload.context && payload.context.length >= 3) {
            epubCodeEditorComponent?.selectTextWithContext(
                payload.text,
                payload.context,
            );
        } else {
            epubCodeEditorComponent?.selectText(payload.text);
        }
    }

    // HTML 判定的唯一来源：flatHtmlFiles（flattenFiles）与查找替换的
    // 「HTML文件」范围（filterFilesInScope 的 isHtml）都用它，避免两处各写一套。
    function isHtmlNode(node: EpubFileNode): boolean {
        return (
            (node.file_type === "html" ||
                node.name.endsWith(".xhtml") ||
                node.name.endsWith(".html")) &&
            !node.name.endsWith(".opf") &&
            !node.name.endsWith(".ncx")
        );
    }

    function flattenFiles(nodes: EpubFileNode[]): EpubFileNode[] {
        let result: EpubFileNode[] = [];
        for (const node of nodes) {
            if (isHtmlNode(node)) {
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

    async function computeFontUsageStats(
        fontFile: EpubFileNode,
        fontFamily: string,
        extraAliases: string[] = [],
    ): Promise<void> {
        const token = ++currentFontStatsToken;
        fontStatsLoading = true;
        fontStatsError = "";
        fontUsedCharCount = 0;
        fontMissingGlyphCharCount = 0;
        fontMissingChars = [];

        try {
            const allFiles = getAllFiles(fileTree);
            const htmlFiles = allFiles.filter(
                (f) =>
                    f.file_type === "html" ||
                    f.name.endsWith(".xhtml") ||
                    f.name.endsWith(".html"),
            );
            const cssFiles = allFiles.filter(
                (f) => f.file_type === "css" || f.name.endsWith(".css"),
            );

            const htmlPaths = htmlFiles.map((f) => f.path);
            const cssPaths = cssFiles.map((f) => f.path);

            let htmlContents: Record<string, string> = {};
            let cssContents: Record<string, string> = {};

            if (htmlPaths.length > 0) {
                htmlContents = await platform.invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath,
                        filePaths: htmlPaths,
                    },
                );
            }

            if (cssPaths.length > 0) {
                cssContents = await platform.invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath,
                        filePaths: cssPaths,
                    },
                );
            }

            if (token !== currentFontStatsToken) return;

            const familyAliases = buildFontFamilyAliases({
                fontFamily,
                fontFileName: fontFile.name,
                extraAliases,
                cssContents,
                fontPath: fontFile.path,
            });

            const parser = new DOMParser();
            const usageMap = new Map<string, number>();

            for (const htmlFile of htmlFiles) {
                const htmlPath = htmlFile.path;
                const html = htmlContents[htmlPath] || "";
                if (!html) continue;

                const doc = parser.parseFromString(html, "text/html");
                const matchedElements = new Set<Element>();
                const selectors: string[] = [];

                for (const styleEl of Array.from(
                    doc.querySelectorAll("style"),
                )) {
                    selectors.push(
                        ...collectSelectorsUsingFont(
                            styleEl.textContent || "",
                            familyAliases,
                        ),
                    );
                }

                for (const linkEl of Array.from(
                    doc.querySelectorAll('link[rel="stylesheet"]'),
                )) {
                    const href = linkEl.getAttribute("href") || "";
                    if (!href) continue;
                    const cssPath = resolveCssHrefToZipPath(htmlPath, href);
                    const cssContent = cssContents[cssPath] || "";
                    if (!cssContent) continue;
                    selectors.push(
                        ...collectSelectorsUsingFont(cssContent, familyAliases),
                    );
                }

                selectors.forEach((selector) => {
                    try {
                        doc.querySelectorAll(selector).forEach((el) =>
                            matchedElements.add(el),
                        );
                    } catch {
                        // ignore invalid selectors
                    }
                });

                doc.querySelectorAll("[style]").forEach((el) => {
                    const style = el.getAttribute("style") || "";
                    if (styleAttributeUsesFont(style, familyAliases)) {
                        matchedElements.add(el);
                    }
                });

                const walker = doc.createTreeWalker(
                    doc.body || doc.documentElement,
                    NodeFilter.SHOW_TEXT,
                );
                let node: Node | null = walker.nextNode();

                while (node) {
                    const textNode = node as Text;
                    const parent = textNode.parentElement;
                    if (parent) {
                        let cursor: Element | null = parent;
                        let isCovered = false;
                        while (cursor) {
                            if (matchedElements.has(cursor)) {
                                isCovered = true;
                                break;
                            }
                            if (cursor === doc.body) {
                                break;
                            }
                            cursor = cursor.parentElement;
                        }

                        if (isCovered) {
                            accumulateCharUsage(
                                textNode.nodeValue || "",
                                usageMap,
                            );
                        }
                    }

                    node = walker.nextNode();
                }
            }

            const analyze = await platform.invoke<FontGlyphAnalyzeResult>(
                "analyze_epub_font_glyphs",
                {
                    epubPath,
                    filePath: fontFile.path,
                    chars: Array.from(usageMap.keys()),
                },
            );

            if (token !== currentFontStatsToken) return;

            if (analyze.internal_names?.length) {
                currentFontInternalName = analyze.internal_names[0];
            }

            if (analyze.unsupported_reason) {
                fontStatsError = "统计失败：" + analyze.unsupported_reason;
                return;
            }

            const missingSummary = summarizeGlyphUsage(
                usageMap,
                analyze.missing_chars || [],
            );
            fontUsedCharCount = missingSummary.usedCount;
            fontMissingGlyphCharCount = missingSummary.missingCount;
            fontMissingChars = missingSummary.missingChars;
        } catch (e) {
            fontStatsError = "统计失败：" + e;
        } finally {
            if (token === currentFontStatsToken) {
                fontStatsLoading = false;
            }
        }
    }

    // 保活时读本视图自己的地址：全局 page.url 会随别的标签跳转而变，
    // 切回来时重新读会导致把别的书当成当前书
    const routeFileParam = () => ownUrl.searchParams.get("file") || "";

    const loadEpub = async () => {
        const routePath = routeFileParam();
        const isNewRoutePath =
            routePath && routePath !== sourceEpubPath && routePath !== epubPath;
        const requestedPath = isNewRoutePath
            ? routePath
            : epubPath || routePath;

        isLoading = true;
        loadingMessage = "正在加载 EPUB...";
        fontPreviewText = DEFAULT_FONT_PREVIEW_TEXT;
        if (!requestedPath) {
            error = "未提供 EPUB 文件路径。";
            isLoading = false;
            return;
        }

        try {
            blobUrls.forEach((url) => URL.revokeObjectURL(url));
            blobUrls = [];
            assetCache.clear();
            fontBinaryCache.clear();
            fileContentCache.clear();
            previewCache.clear();
            resetFontPreviewState();
            previewSourcePath = "";
            loadingMessage = "准备 EPUB 文件...";

            const prepared = await platform.invoke<EpubPrepareResult>(
                "prepare_epub_for_open",
                {
                    epubPath: requestedPath,
                },
            );
            epubPath = prepared.processed_path || requestedPath;
            sourceEpubPath = prepared.source_path || requestedPath;

            loadingMessage = "正在解压 EPUB 文件树...";
            fileTree = await platform.invoke<EpubFileNode[]>("extract_epub", {
                epubPath,
            });
            loadingMessage = "正在整理 EPUB 文件树...";
            showEpubPrepareNotice(prepared);

            const ensureStandardFolders = (nodes: EpubFileNode[]) => {
                const oebpsNode = nodes.find(
                    (node) => node.name.toLowerCase() === "oebps",
                );
                if (!oebpsNode?.children) return;

                for (const folderName of ["Images", "Fonts"]) {
                    const exists = oebpsNode.children.some(
                        (child) =>
                            child.name === folderName &&
                            child.file_type === "folder",
                    );
                    if (!exists) {
                        oebpsNode.children.push({
                            name: folderName,
                            path: "OEBPS/" + folderName,
                            file_type: "folder",
                            children: [],
                        });
                    }
                }
            };
            ensureStandardFolders(fileTree);

            isProjectDirty = false;
            modifiedFiles.clear();
            modifiedFiles = new Set(modifiedFiles);

            flatHtmlFiles = flattenFiles(fileTree);
            await loadSpine();
            await loadTOC();

            const autoExpand = (nodes: EpubFileNode[]) => {
                nodes.forEach((node) => {
                    const name = node.name.toLowerCase();
                    if (name === "oebps") {
                        expandedFolders.add(node.path);
                        node.children?.forEach((child) => {
                            if (child.name.toLowerCase() === "text") {
                                expandedFolders.add(child.path);
                            }
                        });
                    }
                    if (node.children) autoExpand(node.children);
                });
            };
            autoExpand(fileTree);
            expandedFolders = new Set(expandedFolders);

            isLoading = false;

            // 后台补全 HTML 标题与图片分辨率，不阻塞首屏
            loadFileMetaInBackground(epubPath, fileTree);

            precacheCssAssets(fileTree).then(() => {
                console.log("CSS pre-caching completed");
                if (activeTabIndex === -1 && !selectedFile) {
                    const firstHtml = flatHtmlFiles.find(
                        (file) =>
                            file.file_type === "html" ||
                            file.name.endsWith(".xhtml") ||
                            file.name.endsWith(".html"),
                    );
                    if (firstHtml) {
                        selectFile(firstHtml);
                    } else if (flatHtmlFiles.length > 0) {
                        selectFile(flatHtmlFiles[0]);
                    }
                }
            });
        } catch (e) {
            error = "打开 EPUB 失败：" + e;
            isLoading = false;
        }
    };

    async function preloadFile(file: EpubFileNode) {
        if (!file) return;
        const filePath = file.path;

        let content = "";
        if (fileContentCache.has(filePath)) {
            content = fileContentCache.get(filePath)!;
        } else {
            try {
                content = await platform.invoke<string>(
                    "read_epub_file_content",
                    {
                        epubPath: epubPath,
                        filePath: filePath,
                    },
                );
                fileContentCache.set(filePath, content);
            } catch (e) {
                console.warn("Failed to process preview asset " + filePath, e);
                return;
            }
        }

        if (!previewCache.has(filePath)) {
            try {
                const processed = await processHtmlForPreview(
                    content,
                    filePath,
                    currentGeneration,
                );
                if (processed) {
                    previewCache.set(filePath, processed);
                }
            } catch (e) {
                console.warn("Failed to process preview asset " + filePath, e);
            }
        }
    }

    function preloadNeighbors(currentFile: EpubFileNode) {
        if (flatHtmlFiles.length === 0) return;
        const index = flatHtmlFiles.findIndex(
            (f) => f.path === currentFile.path,
        );
        if (index === -1) return;

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
            cssContents = await platform.invoke<Record<string, string>>(
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
            binaryData = await platform.invoke<Record<string, number[]>>(
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
            "Pre-cached " +
                Object.keys(cssContents).length +
                " CSS files and " +
                Object.keys(binaryData).length +
                " binary assets",
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
        expandedFolders = new Set(expandedFolders);
    }

    onMount(() => {
        window.addEventListener("beforeunload", handleBeforeUnload);

        let unlistenClose: (() => void) | null = null;
        const setupCloseHandler = async () => {
            try {
                // 窗口 label 不可变，mount 时取一次即可（桌面端恒为字符串）。
                const label = await platform.getCurrentWindowLabel();
                unlistenClose = await platform.onCurrentWindowCloseRequested(
                    async (event) => {
                        if (hasUnsavedChanges()) {
                            event.preventDefault();
                            closeContext = "app";
                            showCloseDialog = true;
                        } else if (label === "main") {
                            event.preventDefault();
                            await closeEpubEditorWindow();
                        }
                    },
                );
            } catch (e) {
                console.warn("Tauri close handler init failed:", e);
            }
        };
        setupCloseHandler();

        // Calling loadEpub (now top-level)
        platform
            .listen<EpubPrepareStageEvent>("epub-prepare-stage", (event) => {
                if (!isLoading) return;
                const routePath = routeFileParam();
                const activePath = sourceEpubPath || epubPath || routePath;
                if (
                    event.payload.epubPath &&
                    activePath &&
                    event.payload.epubPath !== activePath
                )
                    return;
                loadingMessage = event.payload.message;
            })
            .then((fn) => {
                unlistenPrepareStage = fn;
            })
            .finally(() => {
                loadEpub();
            });

        const blockGlobalSearch = (e: KeyboardEvent) => {
            // 保活：隐藏的编辑器也挂着这条捕获监听，不判当前视图会把
            // 当前标签的 Ctrl+F 也拦下来
            if (!activeOnly()) return;
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
                e.preventDefault();
                showFindReplace = true;
                setTimeout(() => {
                    const input = document.getElementById(
                        "epub-search-input-fr",
                    );
                    if (input) input.focus();
                }, 50);
            }
        };
        window.addEventListener("keydown", blockGlobalSearch, true);

        return () => {
            window.removeEventListener("keydown", blockGlobalSearch, true);
            window.removeEventListener("beforeunload", handleBeforeUnload);
            if (unlistenClose) unlistenClose();
            unlistenPrepareStage?.();
            cleanupBlobUrls();
        };
    });

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
                        multiSelectedFiles = new Set(multiSelectedFiles);
                    }
                } else if (action === "save-as") {
                    try {
                        const res = await platform.invoke<
                            Record<string, string>
                        >("read_epub_files_batch", {
                            epubPath,
                            filePaths: [context.path],
                        });
                        const content = res[context.path];
                        if (!content) throw new Error("读取导出内容失败。");
                        const savedPath = await platform.saveDialog({
                            defaultPath: context.path.split("/").pop(),
                            filters: [
                                {
                                    name: "Text",
                                    extensions: ["xhtml", "html", "txt"],
                                },
                            ],
                        });
                        if (savedPath) {
                            await platform.invoke("save_text_file", {
                                path: savedPath,
                                content,
                            });
                            toastSuccess("文件导出成功。");
                        }
                    } catch (e) {
                        toastError("导出文件失败：" + e);
                    }
                } else if (action === "duplicate") {
                    try {
                        const oldPath = context.path;
                        const res = await platform.invoke<
                            Record<string, string>
                        >("read_epub_files_batch", {
                            epubPath,
                            filePaths: [oldPath],
                        });
                        const content = res[oldPath];
                        if (!content) throw new Error("读取文件内容失败");
                        // Simple duplicate naming: foo.xhtml -> foo_copy.xhtml
                        const parts = oldPath.split(".");
                        const ext = parts.pop();
                        const base = parts.join(".");
                        const newPath = base + "_copy." + ext;

                        await platform.invoke("add_epub_file", {
                            epubPath,
                            filePath: newPath,
                            content,
                        });
                        await addToOpf(newPath, oldPath);

                        await loadEpub();
                        isProjectDirty = true;
                    } catch (e) {
                        toastError("复制文件失败：" + e);
                    }
                } else if (action === "import-sibling") {
                    try {
                        const selected = await platform.openDialog({
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

                            const content = await platform.invoke<string>(
                                "read_text_file",
                                { path: localPath },
                            );
                            await platform.invoke("add_epub_file", {
                                epubPath,
                                filePath: newEpubPath,
                                content,
                            });
                            await addToOpf(newEpubPath, context.path);
                            await loadEpub();
                            isProjectDirty = true;
                        }
                    } catch (e) {
                        toastError("重命名文件失败：" + e);
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
                            await platform.invoke("add_epub_file", {
                                epubPath,
                                filePath: newPath,
                                content,
                            });
                            await addToOpf(newPath, context.path);
                            await loadEpub();
                            isProjectDirty = true;
                        } catch (e) {
                            toastError("删除文件失败：" + e);
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
                        "输入新文件名",
                        oldName,
                        true,
                    );

                    if (res && res.value && res.value !== oldName) {
                        const newName = res.value;
                        const doRefactor = res.confirmRefactor;
                        const newPath =
                            oldPath.substring(0, oldPath.lastIndexOf("/") + 1) +
                            newName;

                        isPromptBusy = true;

                        try {
                            // 1. Rename file in backend
                            await platform.invoke("rename_epub_file", {
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
                            openTabs = [...openTabs];

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
                                        content = await platform.invoke<string>(
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
                                        "([/\"'])" + escapedOld + "([ \"'#?])",
                                        "g",
                                    );

                                    let newContent = content.replace(
                                        regexFinal,
                                        (match, prefix, suffix) => {
                                            return prefix + newName + suffix;
                                        },
                                    );

                                    if (newContent !== content) {
                                        await platform.invoke("add_epub_file", {
                                            epubPath,
                                            filePath: targetFilePath,
                                            content: newContent,
                                        });
                                        updatedCount++;
                                    }
                                }
                                console.log(
                                    "[DEBUG] Refactor completed, updated " +
                                        updatedCount +
                                        " files.",
                                );
                            }

                            // 4. Reload tree & auto-expand (MUST be after link update)
                            await loadEpub();

                            isProjectDirty = true;

                            const newFolderPath = newPath.substring(
                                0,
                                newPath.lastIndexOf("/"),
                            );
                            if (newFolderPath) {
                                expandedFolders.add(newFolderPath);
                                expandedFolders = new Set(expandedFolders);
                            }
                        } catch (e) {
                            console.error("Rename/Refactor failed", e);
                            toastError("重命名/重构失败：" + e);
                        } finally {
                            isPromptBusy = false;
                            showPrompt = false;
                            promptResolve = null;
                        }
                    } else {
                        showPrompt = false;
                        promptResolve = null;
                    }
                } else if (action === "delete") {
                    const confirmed = await showConfirmDialog(
                        "确认删除",
                        "确定删除 " + context.path + " 吗？",
                    );
                    if (confirmed) {
                        // 1. Delete file
                        await platform.invoke("delete_epub_file", {
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
                        isProjectDirty = true;
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

                        isPromptBusy = true;

                        let content = "";
                        if (
                            fileName.endsWith(".xhtml") ||
                            fileName.endsWith(".html")
                        ) {
                            content =
                                '<?xml version="1.0" encoding="utf-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml">\n<head>\n<title></title>\n</head>\n<body>\n</body>\n</html>';
                        }
                        try {
                            await platform.invoke("add_epub_file", {
                                epubPath,
                                filePath,
                                content,
                            });
                            console.log(
                                "[DEBUG] new-file: add_epub_file success",
                            );

                            await loadEpub();

                            isProjectDirty = true;

                            expandedFolders.add(folderPath);
                            expandedFolders = new Set(expandedFolders);
                        } catch (e) {
                            console.error(
                                "[DEBUG] new-file: add_epub_file failed:",
                                e,
                            );
                            toastError("保存文件失败：" + e);
                        } finally {
                            isPromptBusy = false;
                            showPrompt = false;
                            promptResolve = null;
                        }
                    }
                } else if (action === "select-in-tree") {
                    if (context.src) {
                        const targetPath = context.src.split("#")[0];
                        const findAndSelect = (
                            nodes: EpubFileNode[],
                        ): boolean => {
                            for (const node of nodes) {
                                if (
                                    node.path.endsWith(targetPath) ||
                                    targetPath.endsWith(node.path)
                                ) {
                                    const parts = node.path.split("/");
                                    let currentPath = "";
                                    for (let i = 0; i < parts.length - 1; i++) {
                                        currentPath +=
                                            (currentPath ? "/" : "") + parts[i];
                                        expandedFolders.add(currentPath);
                                    }
                                    expandedFolders = new Set(expandedFolders);

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
                } else if (
                    action === "collapse-this" ||
                    action === "collapse-all"
                ) {
                    const tocId = context.id;
                    console.log(
                        "[DEBUG] Collapse action:",
                        action,
                        "tocId:",
                        tocId,
                    );
                    console.log("[DEBUG] tocNodeRefs:", tocNodeRefs);
                    console.log("[DEBUG] Has ref?", !!tocNodeRefs[tocId]);

                    if (action === "collapse-this") {
                        if (tocId && tocNodeRefs[tocId]) {
                            const nodeRef = tocNodeRefs[tocId];
                            console.log("[DEBUG] nodeRef:", nodeRef);
                            if (nodeRef.collapseThis) {
                                console.log("[DEBUG] Calling collapseThis");
                                nodeRef.collapseThis();
                            }
                        } else {
                            console.log(
                                "[DEBUG] No ref found for tocId:",
                                tocId,
                            );
                        }
                    } else if (action === "collapse-all") {
                        console.log("[DEBUG] Collapsing all TOC nodes");
                        Object.values(tocNodeRefs).forEach((nodeRef) => {
                            if (nodeRef && nodeRef.collapseThis) {
                                nodeRef.collapseThis();
                            }
                        });
                    }
                } else if (action === "select-children") {
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
                                multiSelectedFiles = new Set(
                                    multiSelectedFiles,
                                ); // trigger

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
                                    expandedFolders = new Set(expandedFolders);
                                }
                            } else {
                                toastInfo("该目录项下未找到文件。");
                            }
                        }
                    }
                } else if (action === "import-file") {
                    const folderType = (context.folderType || "").toLowerCase();
                    let filters: { name: string; extensions: string[] }[] = [];

                    if (folderType === "text") {
                        filters = [
                            {
                                name: "XHTML/HTML files",
                                extensions: ["xhtml", "html", "htm"],
                            },
                        ];
                    } else if (folderType === "styles") {
                        filters = [{ name: "CSS files", extensions: ["css"] }];
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
                        filters = [{ name: "全部文件", extensions: ["*"] }];
                    }

                    const selected = await platform.openDialog({
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
                                const fileName =
                                    filePath.split(/[\\/]/).pop() || "unknown";
                                const targetPath =
                                    context.path + "/" + fileName;

                                const fileData = await platform.invoke<
                                    number[]
                                >("read_binary_file", {
                                    path: filePath,
                                });

                                await platform.invoke("add_epub_file_binary", {
                                    epubPath,
                                    filePath: targetPath,
                                    content: fileData,
                                });

                                importedCount++;
                            } catch (e) {
                                console.error(
                                    "Failed to import file " + filePath,
                                    e,
                                );
                            }
                        }

                        if (importedCount > 0) {
                            toastSuccess(
                                "已导入 " + importedCount + " 个文件。",
                            );
                            await loadEpub();
                            if (context.path) {
                                expandedFolders.add(context.path);
                                expandedFolders = new Set(expandedFolders);
                            }
                        }
                    }
                }
            } catch (err) {
                console.error("Action error:", err);
                toastError("操作失败：" + err);
            }
        };

        const handleSelectAll = () => {
            // 保活：隐藏的编辑器页面也会收到这条全局事件
            if (!activeOnly()) return;
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
        fontBinaryCache.clear();
        fileContentCache.clear();
        previewCache.clear();
        resetFontPreviewState();
    }

    function getTauriCspNonce() {
        const nonceElement = document.querySelector(
            "style[nonce], script[nonce]",
        ) as HTMLElement | null;
        return nonceElement?.nonce || nonceElement?.getAttribute("nonce") || "";
    }

    function applyCspNonce(element: Element, nonce = getTauriCspNonce()) {
        if (!nonce) return;
        element.setAttribute("nonce", nonce);
        if ("nonce" in element) {
            (element as HTMLElement & { nonce: string }).nonce = nonce;
        }
    }

    function applyPreviewCspNonce(doc: Document) {
        const nonce = getTauriCspNonce();
        if (!nonce) return;
        doc.querySelectorAll("style, script").forEach((element) => {
            applyCspNonce(element, nonce);
        });
    }

    function resolvePath(basePath: string, relativePath: string): string {
        const stack = basePath.split("/");
        stack.pop();

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
        const svgImages = Array.from(doc.querySelectorAll("image"));

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

        let cssContents: Record<string, string> = {};
        if (!skipAssets && cssPaths.length > 0) {
            try {
                cssContents = await platform.invoke<Record<string, string>>(
                    "read_epub_files_batch",
                    {
                        epubPath: epubPath,
                        filePaths: cssPaths,
                    },
                );
            } catch (e) {
                console.error("Failed to load editor selection state", e);
            }
        }

        for (const cssPath of cssPaths) {
            const cachedCss = fileContentCache.get(cssPath);
            if (cachedCss !== undefined) {
                cssContents[cssPath] = cachedCss;
            }
        }

        if (currentGeneration !== generation) return "";

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

        for (const svgImg of svgImages) {
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

        const allBinaryPaths = [...binaryPaths, ...imagePaths];
        let binaryData: Record<string, number[]> = {};

        if (!skipAssets && allBinaryPaths.length > 0) {
            try {
                binaryData = await platform.invoke<Record<string, number[]>>(
                    "read_epub_binary_batch",
                    {
                        epubPath: epubPath,
                        filePaths: allBinaryPaths,
                    },
                );
            } catch (e) {
                console.error("Failed to sync editor preview", e);
            }
        }

        if (currentGeneration !== generation) return "";

        for (const [path, data] of Object.entries(binaryData)) {
            const uint8Array = new Uint8Array(data);

            let mimeType = "application/octet-stream";
            const lower = path.toLowerCase();
            if (lower.endsWith(".ttf")) mimeType = "font/ttf";
            else if (lower.endsWith(".woff")) mimeType = "font/woff";
            else if (lower.endsWith(".woff2")) mimeType = "font/woff2";
            else if (lower.endsWith(".otf")) mimeType = "font/otf";
            else if (lower.endsWith(".eot"))
                mimeType = "application/vnd.ms-fontobject";
            else if (lower.endsWith(".svg")) mimeType = "image/svg+xml";
            else if (lower.endsWith(".webp")) mimeType = "image/webp";

            const blob = new Blob([uint8Array], { type: mimeType });
            const blobUrl = URL.createObjectURL(blob);
            blobUrls.push(blobUrl);
            assetCache.set(path, blobUrl);
        }

        for (const [cssPath, cssContent] of Object.entries(cssContents)) {
            let processedCss = cssContent;
            const assets = cssAssetMap.get(cssPath);

            if (assets) {
                for (const asset of assets) {
                    const blobUrl = assetCache.get(asset.path);
                    if (blobUrl) {
                        processedCss = processedCss
                            .split(asset.original)
                            .join('url("' + blobUrl + '")');
                    }
                }
            }

            const link = cssLinkMap.get(cssPath);
            if (link) {
                const style = doc.createElement("style");
                applyCspNonce(style);
                style.textContent = processedCss;
                link.replaceWith(style);
            }
        }
        for (const [imgPath, imgElements] of imageElemMap) {
            const blobUrl = assetCache.get(imgPath);
            if (blobUrl) {
                for (const img of imgElements) {
                    if (img.tagName.toLowerCase() === "image") {
                        img.setAttributeNS(
                            "http://www.w3.org/1999/xlink",
                            "href",
                            blobUrl,
                        );
                        img.setAttribute("href", blobUrl);
                    } else {
                        img.setAttribute("src", blobUrl);
                    }
                }
            }
        }

        const globalStyle = doc.createElement("style");
        applyCspNonce(globalStyle);

        globalStyle.textContent = [
            "html {",
            "    overflow-x: hidden !important;",
            "    scrollbar-width: none !important;",
            "    -ms-overflow-style: none !important;",
            "}",
            "body {",
            "    overflow-x: hidden !important;",
            "    scrollbar-width: none !important;",
            "    -ms-overflow-style: none !important;",
            "}",
            "::-webkit-scrollbar {",
            "    display: none !important;",
            "}",
            'aside[epub\:type="footnote"] {',
            "    display: none;",
            "}",
            ".footnote-popup {",
            "    position: fixed;",
            "    background: #fff;",
            "    border: 1px solid #ddd;",
            "    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);",
            "    padding: 12px;",
            "    max-width: 80%;",
            "    z-index: 9999;",
            "    border-radius: 6px;",
            "    font-size: 14px;",
            "    color: #333;",
            "    line-height: 1.5;",
            "    pointer-events: auto;",
            "    animation: fadeIn 0.2s ease;",
            "}",
            ".footnote-popup img {",
            "    max-width: 100%;",
            "    height: auto;",
            "}",
            "@keyframes fadeIn {",
            "    from { opacity: 0; transform: translateY(5px); }",
            "    to { opacity: 1; transform: translateY(0); }",
            "}",
            "::selection {",
            "    background-color: #ffeb3b !important;",
            "    color: #000 !important;",
            "}",
        ].join("\n");
        doc.head.appendChild(globalStyle);

        const syncScriptCode = `
document.addEventListener("click", function (event) {
    var clicked = event.target;
    var trigger = clicked && clicked.closest ? clicked.closest("a") : null;
    if (!trigger) return;
    if (trigger.classList.contains("duokan-footnote-back")) {
      event.preventDefault();
      event.stopPropagation();
      var activeBackdrop = trigger.closest(".footnote-popup-backdrop");
      if (activeBackdrop) activeBackdrop.remove();
      return;
    }
    var epubType = trigger.getAttribute("epub:type") || "";
    var isNoteref = trigger.getAttribute("role") === "doc-noteref"
      || trigger.classList.contains("duokan-footnote")
      || epubType.split(/\\s+/).indexOf("noteref") !== -1;
    if (!isNoteref) return;
    var href = trigger.getAttribute("href") || "";
    if (href.charAt(0) !== "#") return;
    var note = document.getElementById(decodeURIComponent(href.slice(1)));
    if (!note) return;
    event.preventDefault();
    event.stopPropagation();
    var existing = document.querySelector(".footnote-popup-backdrop");
    if (existing) existing.remove();
    var backdrop = document.createElement("div");
    backdrop.className = "footnote-popup-backdrop";
    backdrop.style.cssText = "position:fixed;inset:0;z-index:9998;background:rgba(15,23,42,.72);display:flex;align-items:center;justify-content:center;padding:20px;box-sizing:border-box";
    var popup = document.createElement("div");
    popup.className = "footnote-popup";
    popup.innerHTML = note.innerHTML;
    popup.style.cssText = "position:relative;width:min(90vw,720px);max-height:88vh;overflow:auto;background:#fff;padding:16px;box-sizing:border-box;border-radius:8px";
    backdrop.appendChild(popup);
    backdrop.addEventListener("click", function (closeEvent) {
        if (closeEvent.target === backdrop) backdrop.remove();
    });
    document.body.appendChild(backdrop);
}, true);`;
        const syncScript = doc.createElement("script");
        applyCspNonce(syncScript);
        syncScript.textContent = syncScriptCode;
        doc.head.appendChild(syncScript);

        applyPreviewCspNonce(doc);

        return doc.documentElement.outerHTML;
    }

    function hasUnsavedChanges(): boolean {
        return modifiedFiles.size > 0 || isProjectDirty;
    }

    function validateHtml(content: string, currentPath: string) {
        void currentPath;
        const errors: ValidationError[] = [];
        const newErrorLines: number[] = [];
        const lines = content.split("\n");
        const tagStack: { tag: string; line: number }[] = [];
        const tagRegex = /<\/?([a-zA-Z0-9]+)[^>]*>/g;

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const lineNum = i + 1;
            tagRegex.lastIndex = 0;

            let match: RegExpExecArray | null;
            while ((match = tagRegex.exec(line)) !== null) {
                const fullTag = match[0];
                const tagName = match[1].toLowerCase();

                if (
                    fullTag.endsWith("/>") ||
                    ["br", "hr", "img", "input", "meta", "link"].includes(
                        tagName,
                    )
                ) {
                    continue;
                }

                if (fullTag.startsWith("</")) {
                    let matchIndex = -1;
                    for (let j = tagStack.length - 1; j >= 0; j--) {
                        if (tagStack[j].tag === tagName) {
                            matchIndex = j;
                            break;
                        }
                    }

                    if (matchIndex !== -1) {
                        const popped = tagStack.splice(matchIndex);
                        for (let k = 1; k < popped.length; k++) {
                            errors.push({
                                type: "tag",
                                message:
                                    "Line " +
                                    popped[k].line +
                                    ": unclosed tag <" +
                                    popped[k].tag +
                                    ">",
                                line: popped[k].line,
                            });
                            newErrorLines.push(popped[k].line);
                        }
                    } else {
                        errors.push({
                            type: "tag",
                            message:
                                "Line " +
                                lineNum +
                                ": extra closing tag </" +
                                tagName +
                                ">",
                            line: lineNum,
                        });
                        newErrorLines.push(lineNum);
                    }
                } else {
                    tagStack.push({ tag: tagName, line: lineNum });
                }
            }
        }

        for (const unclosed of tagStack) {
            errors.push({
                type: "tag",
                message:
                    "Line " +
                    unclosed.line +
                    ": unclosed tag <" +
                    unclosed.tag +
                    ">",
                line: unclosed.line,
            });
            newErrorLines.push(unclosed.line);
        }

        previewError = errors;
        errorLines = newErrorLines;
    }

    let validationTimer: any = null;
    function handleFileContentChange(newContent: string) {
        fileContent = newContent;
        if (selectedFile) {
            fileContentCache.set(selectedFile.path, newContent);
            modifiedFiles.add(selectedFile.path);
            modifiedFiles = new Set(modifiedFiles); // reactivity

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
                        previewSourcePath = selectedFile.path;
                    }
                } else if (
                    selectedFile?.file_type === "css" ||
                    selectedFile?.name.endsWith(".css")
                ) {
                    previewCache.clear();
                    if (!previewSourcePath) return;

                    const previewHtml =
                        fileContentCache.get(previewSourcePath) ??
                        (await platform.invoke<string>(
                            "read_epub_file_content",
                            {
                                epubPath,
                                filePath: previewSourcePath,
                            },
                        ));

                    fileContentCache.set(previewSourcePath, previewHtml);

                    const refreshedPreview = await processHtmlForPreview(
                        previewHtml,
                        previewSourcePath,
                        currentGeneration,
                        false,
                    );

                    if (refreshedPreview) {
                        previewContent = refreshedPreview;
                        previewCache.set(previewSourcePath, refreshedPreview);
                    }
                }
            }, 500);
        }
    }

    async function persistModifiedFiles() {
        if (modifiedFiles.size === 0) return;

        const pendingPaths = Array.from(modifiedFiles);
        await Promise.all(
            pendingPaths.map(async (path) => {
                const content = fileContentCache.get(path);
                if (content === undefined) return;

                await platform.invoke("save_epub_file_content", {
                    epubPath,
                    filePath: path,
                    content,
                });
            }),
        );

        modifiedFiles.clear();
        modifiedFiles = new Set(modifiedFiles);
        isProjectDirty = true;
    }

    async function saveAllChanges() {
        if (isSaving) return;
        isSaving = true;
        try {
            await persistModifiedFiles();
            await platform.invoke("save_epub_to_disk", { epubPath });
            isProjectDirty = false;
        } catch (e) {
            console.error("保存失败:", e);
            await platform.confirm("保存失败：" + e, {
                title: "错误",
                kind: "error",
            });
        } finally {
            isSaving = false;
        }
    }

    async function saveEpub() {
        await saveAllChanges();
    }

    async function saveCurrentFile() {
        if (!selectedFile) return;
        isSaving = true;
        try {
            await platform.invoke("save_epub_file_content", {
                epubPath: epubPath,
                filePath: selectedFile.path,
                content: fileContent,
            });
            modifiedFiles.delete(selectedFile.path);
            modifiedFiles = new Set(modifiedFiles);
            isProjectDirty = true;
        } catch (e) {
            console.error("保存失败:", e);
            await platform.confirm("保存失败：" + e, {
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
            e.returnValue = "有未保存的更改，确定离开吗？";
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
            multiSelectedFiles = new Set(multiSelectedFiles); // trigger update
            // Also set as current selected file for preview? Yes.
        } else {
            // Normal click clears multi-select unless right-click
            // Note: Right-click usually doesn't trigger 'click' event but 'contextmenu'
            // We handle context menu separately.
            // But if user Left Clicks, clear multi-select
            multiSelectedFiles.clear();
            multiSelectedFiles.add(file.path);
            multiSelectedFiles = new Set(multiSelectedFiles);
        }

        currentGeneration++;
        const generation = currentGeneration;

        selectedFile = file;

        expandParents(fileTree, file.path);
        expandedFolders = new Set(expandedFolders);
        await tick();
        const escapedPath = file.path.replace(/"/g, '\"');
        const fileNode = document.querySelector(
            '.tree-node[data-path="' + escapedPath + '"]',
        );
        if (fileNode) {
            fileNode.scrollIntoView({ block: "center", behavior: "smooth" });
        }

        const existingIndex = openTabs.findIndex(
            (tab) => tab.path === file.path,
        );
        if (existingIndex >= 0) {
            activeTabIndex = existingIndex;
            await tick();
            tabsBar?.scrollTabIntoView(existingIndex);
        } else {
            openTabs.push(file);
            activeTabIndex = openTabs.length - 1;
            openTabs = [...openTabs];

            await tick();
            tabsBar?.scrollToEnd();
        }

        if (previewCache.has(file.path)) {
            fileContent = fileContentCache.get(file.path) || "";
            previewContent = previewCache.get(file.path)!;
            previewSourcePath = file.path;
            activeTab = "preview";
            preloadNeighbors(file);
            return;
        }

        if (fileContentCache.has(file.path)) {
            fileContent = fileContentCache.get(file.path)!;
        } else {
            fileContent = "字体预览加载失败。";
        }

        // UX Optimization: Immediate Switch for HTML files
        // Only auto-switch if no preview content yet (to avoid clearing existing preview)
        const isHtml =
            file.file_type === "html" ||
            file.name.endsWith(".xhtml") ||
            file.name.endsWith(".html");

        if (isHtml) {
            // Only switch to preview tab if we already have preview content OR if we'll load it
            // Don't clear previewContent here - let it retain the previous preview until new one loads
            activeTab = "preview";

            // Only clear preview if we don't have it cached (show loading placeholder)
            if (!previewCache.has(file.path)) {
                previewContent = "";
            }
        }

        try {
            if (isImageFile(file.name)) {
                resetFontPreviewState();
                currentImageSrc = null;
                if (assetCache.has(file.path)) {
                    currentImageSrc = assetCache.get(file.path)!;
                } else {
                    try {
                        const binaryData = await platform.invoke<
                            Record<string, number[]>
                        >("read_epub_binary_batch", {
                            epubPath: epubPath,
                            filePaths: [file.path],
                        });
                        const data = binaryData[file.path];
                        if (data) {
                            const uint8Array = new Uint8Array(data);
                            const blob = new Blob([uint8Array], {
                                type: getBinaryMimeType(file.name),
                            });
                            const url = URL.createObjectURL(blob);
                            assetCache.set(file.path, url);
                            currentImageSrc = url;
                        }
                    } catch (err) {
                        console.error("Failed to load image", err);
                        fileContent = "图片预览加载失败。";
                    }
                }
                fileContent = ""; // Clear editor content
                return;
            } else {
                currentImageSrc = null;
            }

            if (isFontFile(file.name)) {
                resetFontPreviewState();

                let fontBinary = fontBinaryCache.get(file.path);
                if (!fontBinary) {
                    const binaryData = await platform.invoke<
                        Record<string, number[]>
                    >("read_epub_binary_batch", {
                        epubPath,
                        filePaths: [file.path],
                    });
                    const data = binaryData[file.path];
                    if (!data) {
                        throw new Error("font binary data missing");
                    }
                    fontBinary = new Uint8Array(data);
                    fontBinaryCache.set(file.path, fontBinary);
                }

                if (assetCache.has(file.path)) {
                    currentFontSrc = assetCache.get(file.path)!;
                } else {
                    const blob = new Blob([fontBinary], {
                        type: getBinaryMimeType(file.name),
                    });
                    const url = URL.createObjectURL(blob);
                    assetCache.set(file.path, url);
                    currentFontSrc = url;
                }

                let internalNames: string[] = [];
                try {
                    const inspect =
                        await platform.invoke<FontGlyphAnalyzeResult>(
                            "analyze_epub_font_glyphs",
                            {
                                epubPath,
                                filePath: file.path,
                                chars: [],
                            },
                        );
                    internalNames = inspect.internal_names || [];
                    if (internalNames.length > 0) {
                        currentFontInternalName = internalNames[0];
                    }
                } catch (inspectErr) {
                    console.warn(
                        "Failed to read font internal names",
                        inspectErr,
                    );
                }

                const renderFamily = createFontPreviewFamily();
                const buffer = fontBinary.buffer.slice(
                    fontBinary.byteOffset,
                    fontBinary.byteOffset + fontBinary.byteLength,
                );
                const fontFace = new FontFace(renderFamily, buffer);
                await fontFace.load();
                document.fonts.add(fontFace);
                currentFontFace = fontFace;
                currentFontFamily = renderFamily;

                const statFamily = internalNames[0] || fontFileStem(file.name);
                await computeFontUsageStats(file, statFamily, internalNames);

                fileContent = "";
                return;
            } else {
                resetFontPreviewState();
            }

            let content = "";

            if (fileContentCache.has(file.path)) {
                content = fileContentCache.get(file.path)!;
            } else {
                content = await platform.invoke<string>(
                    "read_epub_file_content",
                    {
                        epubPath: epubPath,
                        filePath: file.path,
                    },
                );

                fileContentCache.set(file.path, content);
            }

            if (currentGeneration !== generation) return;

            fileContent = content;

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
                    previewSourcePath = file.path;
                    activeTab = "preview";

                    preloadNeighbors(file);
                }
            } else {
            }
        } catch (e) {
            if (currentGeneration === generation) {
                fileContent = "预览源加载失败：" + e;
                previewContent = "预览源加载失败：" + e;
            }
        }
    }

    function switchTab(index: number) {
        if (index < 0 || index >= openTabs.length) return;
        activeTabIndex = index;
        const tab = openTabs[index];
        selectedFile = tab;

        if (fileContentCache.has(tab.path)) {
            fileContent = fileContentCache.get(tab.path)!;
        }
        if (previewCache.has(tab.path)) {
            previewContent = previewCache.get(tab.path)!;
            previewSourcePath = tab.path;
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
        modifiedFiles.delete(tab.path);
        modifiedFiles = new Set(modifiedFiles);

        openTabs.splice(index, 1);
        openTabs = [...openTabs];

        if (openTabs.length === 0) {
            activeTabIndex = -1;
            selectedFile = null;
            fileContent = "";
            previewContent = "";
        } else {
            if (index === activeTabIndex) {
                const newIndex =
                    index >= openTabs.length ? openTabs.length - 1 : index;
                switchTab(newIndex);
            } else if (index < activeTabIndex) {
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
                        await platform.invoke("save_epub_file_content", {
                            epubPath: epubPath,
                            filePath: pendingCloseFile.path,
                            content: contentToSave,
                        });
                        modifiedFiles.delete(pendingCloseFile.path);
                        modifiedFiles = new Set(modifiedFiles);
                    } catch (e) {
                        console.error("弹窗保存失败:", e);
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
                        await platform.invoke("save_epub_file_content", {
                            epubPath: epubPath,
                            filePath: path,
                            content: content,
                        });
                    }
                });
                await Promise.all(tasks);
                modifiedFiles.clear();
                modifiedFiles = new Set(modifiedFiles);

                await closeEpubEditorWindow();
            } catch (e) {
                isSaving = false;
                await platform.confirm("保存失败：" + e, { kind: "error" });
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
                modifiedFiles = new Set(modifiedFiles);
            }
            showCloseDialog = false;
            if (pendingCloseIndex !== -1) {
                doCloseTab(pendingCloseIndex);
            }
        } else {
            // App Logic: Discard all
            await closeEpubEditorWindow();
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

    interface TocItem {
        id: string;
        label: string;
        src: string;
        children?: TocItem[];
    }

    let activeTab: "preview" | "toc" = "toc";
    let tocList: TocItem[] = [];
    let isTocLoading = false;
    let expandedTocItems: Set<string> = new Set();
    let tocNodeRefs: Record<string, any> = {};
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
                const res = await platform.invoke<Record<string, string>>(
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
            const res = await platform.invoke<Record<string, string>>(
                "read_epub_files_batch",
                {
                    epubPath,
                    filePaths: [opfPath],
                },
            );
            const opfContent = res[opfPath];
            if (!opfContent) throw new Error("读取 OPF 内容失败。");

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
                    // Find ID of neighbor
                    const neighborItem = manifest.querySelector(
                        'item[href="' + insertAfterHref + '"]',
                    );
                    const neighborId = neighborItem
                        ? neighborItem.getAttribute("id")
                        : null;

                    const newItemRef = doc.createElement("itemref");
                    newItemRef.setAttribute("idref", newId);

                    if (neighborId) {
                        const neighborRef = spine.querySelector(
                            'itemref[idref="' + neighborId + '"]',
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

            await platform.invoke("save_epub_file_content", {
                epubPath: epubPath,
                filePath: opfPath,
                content: newOpfContent,
            });
        } catch (e) {
            console.error("Failed to update OPF:", e);
            toastError("更新 OPF 失败：" + e);
        }
    }

    // New helper for assets (only Manifest, no Spine)
    async function addAssetToOpf(newFullPath: string) {
        try {
            let opfPath = "OEBPS/content.opf";
            const res = await platform.invoke<Record<string, string>>(
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
                if (manifest.querySelector('item[href="' + newHref + '"]')) {
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
                await platform.invoke("save_epub_file_content", {
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
        if (isInHiddenView(e.target)) return;
        console.log("File dropped", e.dataTransfer?.files);
        e.preventDefault();
        e.stopPropagation();
        if (!epubPath) {
            toastError("向 EPUB 添加资源失败。");
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

                await platform.invoke("save_epub_files_batch", {
                    epubPath,
                    files: filesMap,
                });
            } catch (err) {
                console.error("Batch file write failed", err);
                toastError("批量写入文件失败：" + err);
                return;
            }

            // 3. Update OPF once (Batch mode)
            try {
                // Manually implement simplified batch OPF update here to avoid N reads/writes
                // Reuse logic from addToOpf but for a list
                let opfPath = "OEBPS/content.opf";
                const res = await platform.invoke<Record<string, string>>(
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
                            if (
                                manifest.querySelector(
                                    'item[href="' + href + '"]',
                                )
                            )
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
                                !manifest.querySelector(
                                    'item[href="' + href + '"]',
                                )
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
                        await platform.invoke("save_epub_file_content", {
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
        if (isInHiddenView(e.target)) return;
        e.preventDefault();
        e.dataTransfer!.dropEffect = "copy";
    }

    function parseNavPoints(container: Element): TocItem[] {
        const items: TocItem[] = [];
        for (const child of Array.from(container.children)) {
            if (child.tagName.toLowerCase() === "navpoint") {
                const id =
                    child.getAttribute("id") ||
                    Math.random().toString(36).substr(2, 9);
                const label =
                    child.querySelector(":scope > navLabel > text")
                        ?.textContent || "Untitled TOC item";
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
            return [...nodes];
        };

        fileTree = sortRecursive(fileTree);
        expandedFolders = new Set(expandedFolders); // Trigger reactivity
    }

    async function loadTOC() {
        tocList = [];
        isTocLoading = true;

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
            console.warn("toc.ncx not found or could not be parsed.");
            sortFileTree(fileTree, []);
            fileTree = [...fileTree];
            isTocLoading = false;
            return;
        }

        try {
            const ncxContent = await platform.invoke<string>(
                "read_epub_file_content",
                {
                    epubPath: epubPath,
                    filePath: ncxNode.path,
                },
            );

            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(ncxContent, "text/xml");
            const navMap = xmlDoc.querySelector("navMap");

            if (navMap) {
                tocList = parseNavPoints(navMap);

                tocNcxPath = ncxNode.path;
                const tocPaths: string[] = [];
                const collectPaths = (items: TocItem[]) => {
                    for (const item of items) {
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

                sortFileTree(fileTree, tocPaths);
                fileTree = [...fileTree];
            }

            tocNcxPath = ncxNode.path;
        } catch (e) {
            console.error("Failed to refresh TOC selection", e);
        } finally {
            isTocLoading = false;
        }
    }

    let tocNcxPath = "";

    function handleTocClick(src: string) {
        let [relativePath, anchor] = src.split("#");

        const targetPath = resolvePath(tocNcxPath, relativePath);

        function expandParentFolders(path: string) {
            const parts = path.split("/");
            let currentPath = "";
            for (let i = 0; i < parts.length - 1; i++) {
                currentPath += (i > 0 ? "/" : "") + parts[i];
                expandedFolders.add(currentPath);
            }
            expandedFolders = new Set(expandedFolders);
        }

        function findAndSelect(nodes: EpubFileNode[]): boolean {
            for (const node of nodes) {
                if (node.path === targetPath) {
                    selectFile(node);

                    setTimeout(() => {
                        const fileElement = document.querySelector(
                            '.file-node[data-path="' + targetPath + '"]',
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
    }

    function highlightHTML(code: string): string {
        let result = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");

        const placeholders: string[] = [];
        let placeholderIndex = 0;

        result = result.replace(/(&lt;!--[\s\S]*?--&gt;)/g, (match) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                '<span class="comment">' + match + "</span>";
            return placeholder;
        });

        result = result.replace(/(&lt;\/?)(\w+)/g, (match, p1, p2) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                p1 + '<span class="tag">' + p2 + "</span>";
            return placeholder;
        });

        result = result.replace(/(\s)([\w-]+)(=)/g, (match, p1, p2, p3) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                p1 + '<span class="attr">' + p2 + "</span>" + p3;
            return placeholder;
        });

        result = result.replace(/="([^"]*)"/g, (match, p1) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                '="<span class=\\"string\\">' + p1 + '</span>\\"';
            return placeholder;
        });

        placeholders.forEach((value, index) => {
            result = result.replace("___PH" + index + "___", value);
        });

        return result;
    }

    function highlightCSS(code: string): string {
        let result = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");

        const placeholders: string[] = [];
        let placeholderIndex = 0;

        result = result.replace(/(\/\*[\s\S]*?\*\/)/g, (match) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                '<span class="comment">' + match + "</span>";
            return placeholder;
        });

        result = result.replace(/([\w-]+)(\s*)(:)/g, (match, p1, p2, p3) => {
            const placeholder = "___PH" + placeholderIndex + "___";
            placeholders[placeholderIndex++] =
                '<span class="property">' + p1 + "</span>" + p2 + p3;
            return placeholder;
        });

        placeholders.forEach((value, index) => {
            result = result.replace("___PH" + index + "___", value);
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

    function addLineNumbers(highlighted: string): string {
        const lines = highlighted.split("\n");
        return lines
            .map((line, i) => {
                const lineNum = i + 1;
                return (
                    '<div class="line-with-number"><span class="line-number">' +
                    lineNum +
                    '</span><span class="line-content">' +
                    (line || " ") +
                    "</span></div>"
                );
            })
            .join("");
    }

    function getFilesInScope(): EpubFileNode[] {
        const allFiles = getAllFiles(fileTree);
        const selected = allFiles.filter((node) =>
            multiSelectedFiles.has(node.path),
        );
        return filterFilesInScope(searchScope, {
            allFiles,
            currentFile: selectedFile,
            openFiles: openTabs,
            selectedFiles: selected,
            isHtml: isHtmlNode,
            isEditable: (node) => isEditable(node.file_type),
        });
    }

    function currentFindQuery(): FindQuery {
        return { pattern: findPattern, mode: isRegex ? "regex" : "text" };
    }

    function searchInCurrentFile(
        content: string,
        pattern: string,
        startPos: number = 0,
        direction: "down" | "up" = "down",
        useWrap: boolean = true,
    ): { from: number; to: number } | null {
        if (!pattern) return null;

        try {
            return findNextMatchWrapped(
                content,
                { pattern, mode: isRegex ? "regex" : "text" },
                startPos,
                direction,
                useWrap && wrapAround,
            );
        } catch {
            searchMessage = "\u641C\u7D22\u6A21\u5F0F\u65E0\u6548\u3002";
            return null;
        }
    }

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
                    nextContent = await platform.invoke<string>(
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
            let matchResult: MatchRange | null = null;
            try {
                matchResult = findNextMatch(
                    nextContent,
                    currentFindQuery(),
                    searchDirection === "down" ? 0 : nextContent.length,
                    searchDirection,
                );
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
                        let actualResult: MatchRange | null = null;
                        try {
                            actualResult = findNextMatch(
                                actualContent,
                                currentFindQuery(),
                                searchDirection === "down"
                                    ? 0
                                    : actualContent.length,
                                searchDirection,
                            );
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
                    content = await platform.invoke<string>(
                        "read_epub_file_content",
                        {
                            epubPath: epubPath,
                            filePath: file.path,
                        },
                    );
                } catch (e) {
                    continue;
                }
            }

            let newContent = content;
            let replacedCount = 0;

            try {
                const replaced = replaceAllInContent(
                    content,
                    currentFindQuery(),
                    replacePattern,
                );
                newContent = replaced.content;
                replacedCount = replaced.count;
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

        modifiedFiles = new Set(modifiedFiles); // 触发响应式
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
                    const count = await platform.invoke("search_in_files", {
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
                        totalCount += countMatchesInContent(
                            content,
                            currentFindQuery(),
                        );
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

    // 关闭查找面板
    function closeFindReplace() {
        showFindReplace = false;
        currentMatchInfo = null;
        searchMessage = "";
    }

    // 保存查找历史到 localStorage

    function saveSearchHistory() {
        localStorage.setItem(
            EPUB_FIND_HISTORY_KEY,
            JSON.stringify(findHistory),
        );
        localStorage.setItem(
            EPUB_REPLACE_HISTORY_KEY,
            JSON.stringify(replaceHistory),
        );
    }

    function loadSearchHistory() {
        try {
            const findH = localStorage.getItem(EPUB_FIND_HISTORY_KEY);
            const replaceH = localStorage.getItem(EPUB_REPLACE_HISTORY_KEY);
            if (findH) findHistory = JSON.parse(findH);
            if (replaceH) replaceHistory = JSON.parse(replaceH);
            if (findHistory.length > 0) findPattern = findHistory[0];
            if (replaceHistory.length > 0) replacePattern = replaceHistory[0];
        } catch (e) {}
    }

    function addToHistory(text: string, history: string[]): string[] {
        if (!text.trim()) return history;
        const filtered = history.filter((h) => h !== text);
        const newHistory = [text, ...filtered].slice(0, MAX_HISTORY);
        return newHistory;
    }

    function removeFromFindHistory(index: number) {
        findHistory = findHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

    function removeFromReplaceHistory(index: number) {
        replaceHistory = replaceHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

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
        // 保活：隐藏的编辑器也挂着 <svelte:window>，不判当前视图会把
        // 可见标签的 Ctrl+F / Ctrl+S 都抢走
        if (isInHiddenView(e.target)) return;
        if (e.ctrlKey && e.key === "f") {
            e.preventDefault();
            e.stopPropagation();
            showFindReplace = true;
        }
        if (e.ctrlKey && e.key === "h") {
            e.preventDefault();
            e.stopPropagation();
            showFindReplace = true;
        }
        if (e.key === "Escape" && showFindReplace) {
            closeFindReplace();
        }
        if (e.key === "F3" && showFindReplace) {
            e.preventDefault();
            if (e.shiftKey) {
                findPrev();
            } else {
                findNext();
            }
        }
        if (e.ctrlKey && e.key === "s") {
            e.preventDefault();
            e.stopPropagation();
            if (modifiedFiles.size > 0 || isProjectDirty) {
                saveAllChanges();
            }
        }
    }

    onMount(() => {
        loadSearchHistory();
        // 编辑器是独立窗口，设置窗口改完开关后要立刻同步过来。
        // 自定义事件只在本窗口内冒泡，另一个 WebviewWindow 改 localStorage
        // 时只有 storage 事件会跨窗口送达，两个都要听
        const syncSettings = () => {
            appSettings = loadAppSettings();
        };
        window.addEventListener("tepub-settings-updated", syncSettings);
        window.addEventListener("storage", syncSettings);
        restoreSettingsListener = () => {
            window.removeEventListener("tepub-settings-updated", syncSettings);
            window.removeEventListener("storage", syncSettings);
        };
        return () => {
            restoreSettingsListener?.();
            restoreSettingsListener = null;
        };
    });
</script>

<svelte:window
    on:keydown={handleKeydown}
    on:dragover={handleDragOver}
    on:drop={handleFileDrop}
/>

<Toast />

{#if showPrompt}
    <div
        class="prompt-overlay"
        role="button"
        tabindex="-1"
        on:click={handlePromptOverlayClick}
        on:keydown={handlePromptOverlayKeydown}
    >
        <div
            class="prompt-dialog"
            role="dialog"
            aria-modal="true"
            tabindex="-1"
        >
            <div class="prompt-title">{promptTitle}</div>
            <input
                type="text"
                bind:this={promptInput}
                bind:value={promptValue}
                class="prompt-input"
                on:keydown={handlePromptInputKeydown}
                disabled={isPromptBusy}
            />
            {#if promptOptions}
                <div class="prompt-check">
                    <label>
                        <input
                            type="checkbox"
                            bind:checked={promptCheckValue}
                            disabled={isPromptBusy}
                        />
                        &#33258;&#21160;&#26356;&#26032;&#20854;&#20182;&#25991;&#20214;&#20013;&#30340;&#38142;&#25509;&#24341;&#29992;
                    </label>
                </div>
            {/if}

            <div class="prompt-buttons">
                <button
                    class="prompt-btn cancel"
                    on:click={handlePromptCancel}
                    disabled={isPromptBusy}
                >
                    &#21462;&#28040;
                </button>
                <button
                    class="prompt-btn confirm"
                    on:click={handlePromptConfirm}
                    disabled={isPromptBusy}
                >
                    {isPromptBusy
                        ? "\u6B63\u5728\u66F4\u65B0\u94FE\u63A5..."
                        : "\u786E\u5B9A"}
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- 闂備胶鍘ч〃搴㈢濠婂牊鍋╅柣鎰靛墰閳绘洖顭跨捄鍝勵槵闁逞屽墯椤洭骞忕€ｎ喖围闁告粈绀佸▓鎼佹煟閻樺弶澶勭紒璇插暣閵?-->
{#if showConfirm}
    <div
        class="prompt-overlay"
        role="button"
        tabindex="-1"
        on:click={handleConfirmOverlayClick}
        on:keydown={handleConfirmOverlayKeydown}
    >
        <div
            class="prompt-dialog"
            role="dialog"
            aria-modal="true"
            tabindex="-1"
        >
            <div class="prompt-title">{confirmTitle}</div>
            <div
                class="prompt-message"
                style="margin-bottom: 20px; color: #ccc;"
            >
                {confirmMessage}
            </div>
            <div class="prompt-buttons">
                <button
                    class="prompt-btn cancel"
                    on:click={handleConfirmCancel}
                >
                    &#21462;&#28040;
                </button>
                <button
                    class="prompt-btn confirm delete"
                    on:click={handleConfirmConfirm}
                >
                    &#21024;&#38500;
                </button>
            </div>
        </div>
    </div>
{/if}
{#if isLoading}
    <div class="loading page-loading">{loadingMessage}</div>
{:else if error}
    <div class="error page-error">{error}</div>
{:else}
    <div class="epub-editor">
        <FileTreePanel
            {fileTree}
            {expandedFolders}
            {selectedFile}
            {multiSelectedFiles}
            {isSaving}
            isDirty={isProjectDirty || modifiedFiles.size > 0}
            onToggleFolder={toggleFolder}
            onSelectFile={selectFile}
            onSaveAll={saveAllChanges}
            onRefresh={loadEpub}
        />

        <main class="editor-pane">
            <TabsBar
                bind:this={tabsBar}
                {openTabs}
                {activeTabIndex}
                {modifiedFiles}
                onSelectTab={switchTab}
                onCloseTab={closeTab}
            />

            {#if selectedFile}
                {#if isImageFile(selectedFile.name)}
                    <div class="image-preview-container">
                        {#if currentImageSrc}
                            <img
                                src={currentImageSrc}
                                alt={selectedFile.name}
                            />
                        {:else}
                            <div class="loading loading-inline">
                                &#21152;&#36733;&#22270;&#29255;&#20013;...
                            </div>
                        {/if}
                    </div>
                {:else if isFontFile(selectedFile.name)}
                    <FontPreviewPanel
                        fileName={selectedFile.name}
                        internalName={currentFontInternalName}
                        fontFamily={currentFontFamily}
                        bind:previewText={fontPreviewText}
                        statsLoading={fontStatsLoading}
                        statsError={fontStatsError}
                        usedCharCount={fontUsedCharCount}
                        missingGlyphCharCount={fontMissingGlyphCharCount}
                        missingChars={fontMissingChars}
                    />
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
                                onOpenSearch={() => {
                                    showFindReplace = true;
                                    setTimeout(() => {
                                        const input = document.getElementById(
                                            "epub-search-input-fr",
                                        );
                                        if (input) input.focus();
                                        else {
                                            const inputs =
                                                document.querySelectorAll(
                                                    ".fr-input",
                                                );
                                            if (inputs.length > 0)
                                                (
                                                    inputs[0] as HTMLElement
                                                ).focus();
                                        }
                                    }, 50);
                                }}
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
                <div class="placeholder">
                    &#35831;&#20174;&#24038;&#20391;&#36873;&#25321;&#19968;&#20010;&#25991;&#20214;
                </div>
            {/if}

            {#if showFindReplace && selectedFile && isEditable(selectedFile.file_type)}
                <FindReplacePanel
                    bind:findPattern
                    bind:replacePattern
                    bind:isRegex
                    bind:searchScope
                    bind:searchDirection
                    bind:wrapAround
                    bind:textOnly
                    {findHistory}
                    {replaceHistory}
                    {searchMessage}
                    onFindNext={findNext}
                    onFindPrev={findPrev}
                    onCountMatches={countMatches}
                    onReplace={performReplace}
                    onReplaceAll={performReplaceAll}
                    onRemoveFindHistory={removeFromFindHistory}
                    onRemoveReplaceHistory={removeFromReplaceHistory}
                />
            {/if}
        </main>

        <aside class="preview-pane">
            <div class="preview-header">
                <div class="tabs">
                    <button
                        class="tab"
                        class:active={activeTab === "preview"}
                        on:click={() => (activeTab = "preview")}
                    >
                        &#39044;&#35272;
                    </button>
                    <button
                        class="tab"
                        class:active={activeTab === "toc"}
                        on:click={() => {
                            activeTab = "toc";
                        }}
                    >
                        &#30446;&#24405;
                    </button>
                </div>
            </div>
            <div class="preview-stack">
                <div
                    class="preview-slot"
                    class:is-hidden={activeTab !== "preview"}
                >
                    <PreviewPane
                        bind:this={previewPane}
                        content={previewContent}
                        errors={previewError}
                        hasSelectedFile={!!selectedFile}
                        onPreviewClick={handlePreviewClick}
                        showScrollbar={appSettings.previewScrollbar}
                        darkPaper={appSettings.uiTheme === "dark"}
                    />
                </div>
                <div class="preview-slot" class:is-hidden={activeTab !== "toc"}>
                    <div class="toc-container">
                        {#if isTocLoading}
                            <div class="loading toc-loading">
                                &#23569;&#22899;&#31048;&#31095;&#20013;&#183;&#183;&#183;&#183;&#183;&#183;
                            </div>
                        {:else if tocList.length === 0}
                            <div class="empty">
                                &#26242;&#26080;&#30446;&#24405;&#25110;&#26410;&#25214;&#21040;
                                toc.ncx
                            </div>
                        {:else}
                            <div class="toc-list">
                                {#each tocList as item}
                                    <TocNode
                                        bind:this={tocNodeRefs[item.id]}
                                        {item}
                                        onSelect={handleTocClick}
                                    />
                                {/each}
                            </div>
                        {/if}
                    </div>
                </div>
            </div>
        </aside>
    </div>
{/if}

<!-- Context Menu -->
<ContextMenu />

{#if showCloseDialog}
    <div class="dialog-overlay">
        <div class="dialog">
            <div class="dialog-header">
                &#26410;&#20445;&#23384;&#30340;&#26356;&#25913;
            </div>
            <div class="dialog-content">
                {#if closeContext === "tab"}
                    &#25991;&#20214; "{pendingCloseFile?.name}"
                    &#26377;&#26410;&#20445;&#23384;&#30340;&#26356;&#25913;&#65292;&#26159;&#21542;&#20445;&#23384;&#65311;
                {:else}
                    &#24744;&#26377; {modifiedFiles.size} &#20010;&#25991;&#20214;&#21253;&#21547;&#26410;&#20445;&#23384;&#30340;&#26356;&#25913;&#65292;&#26159;&#21542;&#20445;&#23384;&#25152;&#26377;&#24182;&#36864;&#20986;&#65311;
                {/if}
            </div>
            <div class="dialog-actions">
                <button
                    class="btn primary"
                    on:click={handleDialogSave}
                    disabled={isSaving}
                >
                    {isSaving ? "\u4FDD\u5B58\u4E2D..." : "\u4FDD\u5B58"}
                </button>
                <button
                    class="btn danger"
                    on:click={handleDialogDiscard}
                    disabled={isSaving}
                >
                    &#19981;&#20445;&#23384;
                </button>
                <button
                    class="btn secondary"
                    on:click={handleDialogCancel}
                    disabled={isSaving}
                >
                    &#21462;&#28040;
                </button>
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
        border-radius: var(--radius-sm);
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
        color: var(--color-text);
    }

    .dialog-actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
    }

    .btn {
        padding: 8px 16px;
        border-radius: var(--radius-xs);
        border: none;
        cursor: pointer;
        font-weight: 500;
    }

    .btn.primary {
        background: var(--color-accent);
        color: var(--color-accent-contrast);
    }

    .btn.danger {
        background: var(--color-danger);
        color: var(--color-accent-contrast);
    }

    .btn.secondary {
        background: var(--color-border);
        color: var(--color-text);
    }

    .epub-editor {
        display: flex;
        height: 100vh;
        background: var(--color-surface-soft);
    }

    .page-loading,
    .page-error {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        min-height: 100vh;
        font-size: 36px;
        color: var(--color-muted);
        font-weight: 500;
        letter-spacing: 3px;
    }

    .error {
        color: var(--color-danger);
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 100%;
        height: 100%;
        font-size: 18px;
        color: var(--color-muted);
        font-weight: 400;
        letter-spacing: 2px;
    }

    :global(body) {
        margin: 0;
        padding: 0;
        overflow: hidden;
        font-family:
            -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen,
            Ubuntu, Cantarell, "Open Sans", "Helvetica Neue", sans-serif;
    }

    .editor-pane {
        flex: 1;
        background: var(--color-surface);
        display: flex;
        flex-direction: column;
        border-left: 1px solid var(--color-border);
        border-right: 1px solid var(--color-border);
        min-width: 0;
        min-height: 0;
        position: relative;
    }

    .code-block :global(.line-with-number) {
        display: flex;
        line-height: 1.5;
        margin: 0;
        padding: 0;
    }

    .code-block :global(.line-number) {
        color: #858585;
        text-align: right;
        padding-right: 12px;
        user-select: none;
        border-right: 1px solid var(--color-border);
        min-width: 40px;
        background: var(--color-surface-soft);
        flex-shrink: 0;
    }

    .code-block :global(.line-content) {
        padding-left: 8px;
        flex: 1;
        white-space: pre-wrap;
    }

    .editor-content {
        flex: 1;
        min-height: 0;
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

    .code-block :global(.tag) {
        color: #0000ff;
        font-weight: 600;
    }

    .code-block :global(.attr) {
        color: #ff0000;
    }

    .code-block :global(.string) {
        color: var(--color-accent-deep);
    }

    .code-block :global(.comment) {
        color: #008000;
        font-style: italic;
    }

    .code-block :global(.property) {
        color: #ff0000;
    }

    .code-block :global(.value) {
        color: var(--color-accent-deep);
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

    .preview-pane {
        width: 320px; /* Standard Android/iPhone Width */
        background: var(--color-surface);
        display: flex;
        flex-direction: column;
        border-left: 1px solid var(--color-border);
    }

    /* 预览/目录常驻挂载、只切换显隐。
       用 {#if} 销毁重建会让 iframe 每次回来都重量一次高度，
       外层滚动条先消失再出现，肉眼就是「闪一下」 */
    .preview-stack {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-rows: minmax(0, 1fr);
    }

    .preview-slot {
        grid-area: 1 / 1;
        min-width: 0;
        min-height: 0;
        display: flex;
        flex-direction: column;
    }

    /* 用 visibility 而不是 display:none —— 后者让 iframe 脱离布局，
       切回来时整篇重新排版，书页高度先短后长，滚动条会闪一下 */
    .preview-slot.is-hidden {
        visibility: hidden;
        pointer-events: none;
    }

    /* 兜底：隐藏层里的元素若自带 visibility: visible（内联或组件内样式），
       会盖过继承来的 hidden 单独画在预览区上（目录树的展开箭头就踩过）。
       后代一律压住，显示回来时选择器失配，自然恢复 */
    .preview-slot.is-hidden :global(*) {
        visibility: hidden !important;
    }

    .preview-header {
        height: 40px; /* Matched with tabs-bar */
        background: var(--color-surface-soft);
        border-bottom: 1px solid var(--color-border);
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
        color: var(--color-muted);
        cursor: pointer;
        border-bottom: 2px solid transparent;
        transition: all 0.2s;
        font-weight: bold; /* Matched with tree-header h3 */
    }

    .tab:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .tab.active {
        color: var(--color-accent);
        border-bottom: 2px solid var(--color-accent);
        background: var(--color-surface);
    }

    .toc-container {
        flex: 1;
        overflow-y: auto;
        background: var(--color-surface);
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
        color: var(--color-muted);
        font-size: 14px;
    }

    .placeholder {
        display: flex;
        align-items: center;
        justify-content: center;
        height: 100%;
        width: 100%;
        color: var(--color-muted);
        font-size: 14px;
    }

    ::-webkit-scrollbar {
        width: 8px;
        height: 8px;
    }

    ::-webkit-scrollbar-track {
        background: var(--color-surface-soft);
    }

    ::-webkit-scrollbar-thumb {
        background: var(--color-muted);
        border-radius: var(--radius-xs);
    }

    ::-webkit-scrollbar-thumb:hover {
        background: var(--color-text-soft);
    }

    .image-preview-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: var(--color-hover);
        overflow: auto;
        padding: 20px;
        height: 100%;
    }

    .image-preview-container img {
        max-width: 100%;
        max-height: 100%;
        box-shadow: var(--shadow-sm);
        background-color: var(--color-surface);
        background-image:
            linear-gradient(45deg, var(--color-border) 25%, transparent 25%),
            linear-gradient(-45deg, var(--color-border) 25%, transparent 25%),
            linear-gradient(45deg, transparent 75%, var(--color-border) 75%),
            linear-gradient(-45deg, transparent 75%, var(--color-border) 75%);
        background-size: 20px 20px;
        background-position:
            0 0,
            0 10px,
            10px -10px,
            -10px 0px;
    }

    .loading-inline {
        color: var(--color-muted);
        font-size: 14px;
    }

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
        border-radius: var(--radius-md);
        padding: 24px;
        min-width: 320px;
        box-shadow: var(--shadow-pop);
    }

    .prompt-title {
        font-size: 16px;
        font-weight: 600;
        margin-bottom: 16px;
        color: var(--color-text);
    }

    .prompt-input {
        width: 100%;
        padding: 10px 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        font-size: 14px;
        margin-bottom: 16px;
        box-sizing: border-box;
    }

    .prompt-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: 0 0 0 2px rgba(74, 144, 217, 0.2);
    }

    .prompt-buttons {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }

    .prompt-btn {
        padding: 8px 16px;
        border: none;
        border-radius: var(--radius-xs);
        font-size: 14px;
        cursor: pointer;
        transition: all 0.2s;
    }

    .prompt-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }

    .prompt-btn.cancel {
        background: var(--color-hover);
        color: var(--color-muted);
    }

    .prompt-btn.cancel:hover {
        background: var(--color-border);
    }

    .prompt-btn.confirm {
        background: var(--color-accent);
        color: var(--color-accent-contrast);
    }

    .prompt-btn.confirm:hover {
        background: #3a80c9;
    }

    /* Modern UI overrides */
    :global(body) {
        background: var(--gradient-app);
        color: var(--color-text);
        font-family: var(--font-ui);
    }

    .epub-editor {
        background: rgba(238, 244, 248, 0.72);
        color: var(--color-text);
    }

    .preview-header {
        background: var(--color-surface-raised);
        border-bottom: 1px solid var(--color-border);
        backdrop-filter: blur(18px) saturate(1.12);
    }

    .preview-header {
        height: 52px;
    }

    .editor-pane {
        background: var(--color-surface);
        border-left: 0;
        border-right: 1px solid var(--color-border);
        box-shadow: inset 1px 0 0 var(--color-surface-raised);
    }

    .editor-content {
        background: var(--color-surface);
    }

    .preview-pane {
        width: 340px;
        background: var(--color-surface-raised);
        border-left: 1px solid var(--color-border);
        box-shadow: -10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .tabs {
        gap: 8px;
        padding: 7px 10px 0;
        box-sizing: border-box;
    }

    .tab {
        border-radius: 999px 999px 0 0;
        color: var(--color-muted);
        font-size: 16px;
        transition:
            background var(--transition-fast),
            color var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .tab:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .tab.active {
        background: var(--color-surface);
        color: var(--color-accent-deep);
        border-bottom-color: var(--color-accent);
        box-shadow: 0 -6px 16px rgba(23, 36, 52, 0.05);
    }

    .toc-container {
        background: var(--color-surface-raised);
        border-radius: var(--radius-md);
    }

    .placeholder,
    .toc-container .empty,
    .toc-container .loading,
    .loading {
        color: var(--color-muted);
        letter-spacing: 0.03em;
    }

    .error {
        color: var(--color-danger);
    }

    /* NOTE: 原为 .image-preview-container, .font-preview-container 共享一条
       渐变背景规则；字体份已随 FontPreviewPanel 组件下沉，此处保留图片份。 */
    .image-preview-container {
        background: linear-gradient(
            180deg,
            rgba(247, 250, 253, 0.94),
            rgba(238, 244, 248, 0.94)
        );
    }

    .image-preview-container img {
        border-radius: var(--radius-md);
        box-shadow: var(--shadow-md);
    }

    .dialog-overlay,
    .prompt-overlay {
        background: rgba(14, 24, 36, 0.36);
        backdrop-filter: blur(10px);
    }

    .dialog,
    .prompt-dialog {
        background: var(--color-surface-raised);
        border: 1px solid var(--color-surface-raised);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-pop);
        color: var(--color-text);
        backdrop-filter: blur(18px) saturate(1.08);
    }

    .dialog {
        min-width: 340px;
        padding: 24px;
    }

    .dialog-header,
    .prompt-title {
        color: var(--color-text);
    }

    .dialog-content,
    .prompt-message {
        color: var(--color-text-soft) !important;
    }

    .btn,
    .prompt-btn {
        border-radius: var(--radius-sm);
        border: 1px solid var(--color-border);
        background: linear-gradient(
            180deg,
            var(--color-surface),
            var(--color-surface-soft)
        );
        color: var(--color-text-soft);
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast),
            color var(--transition-fast),
            transform var(--transition-fast),
            box-shadow var(--transition-fast);
    }

    .btn:hover:not(:disabled),
    .prompt-btn:hover:not(:disabled) {
        background: var(--color-hover);
        border-color: var(--color-border-strong);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    .btn.primary,
    .prompt-btn.confirm {
        background: var(--gradient-accent);
        border-color: transparent;
        color: var(--color-accent-contrast);
        box-shadow: none;
    }

    .btn.danger,
    .prompt-btn.confirm.delete {
        background: var(--gradient-danger);
        border-color: transparent;
        color: var(--color-surface);
    }

    .btn.secondary,
    .prompt-btn.cancel {
        background: var(--color-surface-raised);
        color: var(--color-text-soft);
    }

    .prompt-input {
        border-color: var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
    }

    .prompt-input:focus {
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
    }

    ::-webkit-scrollbar-track {
        background: rgba(226, 235, 244, 0.72);
    }

    ::-webkit-scrollbar-thumb {
        background: linear-gradient(
            180deg,
            var(--color-border-strong),
            var(--color-border)
        );
        border-radius: 999px;
    }

    /* Right preview tabs: compact underline, no pill shadow. */
    .preview-pane .tabs {
        gap: 0;
        padding: 0 12px;
        align-items: stretch;
    }

    .preview-pane .tab {
        margin: 0 10px;
        padding: 0;
        border-radius: 0;
        background: transparent;
        border-bottom: 2px solid transparent;
        box-shadow: none;
        color: var(--color-text-soft);
    }

    .preview-pane .tab:hover {
        background: transparent;
        color: var(--color-text);
    }

    .preview-pane .tab.active {
        background: transparent;
        border-bottom-color: var(--color-accent);
        box-shadow: none;
        color: var(--color-accent-deep);
    }
</style>
