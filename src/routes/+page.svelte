<script lang="ts">
    import { onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, emit } from "@tauri-apps/api/event";
    import { open, save, message, ask } from "@tauri-apps/plugin-dialog";
    // import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs"; // Removed to force use of custom backend
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import Editor from "$lib/Editor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";

    // --- [1. 完整的接口定义] ---
    interface RawChapter {
        title: string;
        line_number: number;
        level: number;
        is_meta: boolean;
        word_count: number;
    }
    interface TocNode {
        id: string;
        title: string;
        line_number: number;
        type: "Volume" | "Chapter" | "Meta";
        word_count: number;
        children: TocNode[];
        expanded: boolean;
        parentId?: string;
        level?: number;
    }
    interface MatchLocation {
        line: number;
        start_char: number;
        end_char: number;
    }
    interface SearchResult {
        found: boolean;
        count: number;
        matches: MatchLocation[];
    }
    interface FlatNode {
        id: string;
        line: number;
        parentId?: string;
        title: string;
        type: "Volume" | "Chapter" | "Meta";
        word_count: number;
        level?: number;
    }
    interface CheckItem {
        id: string;
        title: string;
        line: number;
        msg: string;
        val: number | string;
        parentId?: string;
    }
    interface HistoryMeta {
        filename: string;
        path: string;
        timestamp: number;
        size: number;
    }

    // --- [2. 默认配置 (三大正则回归 & 新增动态生成规则)] ---
    interface CustomRegexRule {
        level: number;
        pattern: string;
    }

    const DEFAULT_SETTINGS = {
        customRegexRules: [
            { level: 1, pattern: "^\\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*" },
            { level: 1, pattern: "^\\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*" },
            { level: 3, pattern: "^\\s*(第[一二三四五六七八九十百千万0-9]+(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+).*" }
        ] as CustomRegexRule[],
        wordCountThreshold: 8000,
        clearHistoryOnSave: false,
        defaultEpubStyles: { "main.css": "", "font.css": "" },
        wordWrap: true,
        showWhitespace: false,
        showLineBreaks: false,
        // Legacy fallbacks for compatibility
        volRegex: "^\\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*",
        chapRegex: "^\\s*(第[一二三四五六七八九十百千万0-9]+(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+).*",
        metaRegex: "^\\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*",
    };

    const REGEX_PRESETS = [
        { label: "自定义", value: "" },
        { label: "^\\s*第[一二三..]+章.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+章.*$" },
        { label: "^\\s*第[一二三..]+卷.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+卷.*$" },
        { label: "^\\s*第[一二三..]+回.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+回.*$" },
        { label: "^\\s*第[一二三..]+节.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+节.*$" },
        { label: "^\\s*第\\d+章.*$", value: "^\\s*第\\d+章.*$" },
        { label: "^\\s*(序[1-9言曲]?|(内容)?简介|后记|尾声)$", value: "^\\s*(序[1-9言曲]?|(内容)?简介|后记|尾声)$" },
        { label: "^\\s*\\d+\\s*$", value: "^\\s*\\d+\\s*$" }
    ];

    // --- [3. 核心状态] ---
    let filePath = "请打开一本小说...";
    let fileContent = "";
    let tocTree: TocNode[] = [];
    let flatToc: FlatNode[] = [];
    let stats = { volumes: 0, chapters: 0 };
    let activeChapterId = "";
    let editorComponent: Editor;

    let showSidebar = true; // State
    // [Removed duplicate declarations]
    let isLoading = false;
    let isLoadingFile = false;
    let isModified = false;
    let isSaving = false;
    let isMobile = false;
    // 导航锁：点击目录跳转时暂时屏蔽滚动监听，防止目录乱跳
    let isNavigating = false;
    let scrollTimeout: any = null;
    let navTimer: any = null;
    let hasInitialized = false;

    // 面板显示状态
    let showSettingsPanel = false;
    let settingsActiveTab = "display"; // 'display' | 'toc'
    let showEpubModal = false;
    let showCheckPanel = false;
    let showHistoryPanel = false;
    let showRestoreConfirm = false;
    let restoreTargetSnapshot: any = null;
    let epubGenerationStatus: "idle" | "generating" | "success" = "idle";

    // 功能数据
    let epubMeta = {
        title: "书名",
        creator: "作者",
        publisher: "",
        date: new Date().toISOString().split("T")[0],
        uuid: crypto.randomUUID(),
        md5: "",
        cover_path: "",
        description: "",
        styles: { "main.css": "", "font.css": "" },
        assets: [] as { name: string, path: string, category: string }[],
    };
    let coverPreviewUrl: string | null = null;
    let showAdvancedEpub = false;
    let customMetadata: { key: string; value: string }[] = [];
    let appSettings = { ...DEFAULT_SETTINGS };
    let historyList: HistoryMeta[] = [];

    // 查找替换状态
    let findPattern = "";
    let replacePattern = "";
    let replaceMsg = "";
    let isRegex = false;
    let allMatches: MatchLocation[] = [];
    let currentMatchIndex = -1;

    // 内容检查状态
    let isCheckModeOn = true;
    let invalidSequenceIds = new Set<string>();
    let sequenceErrors: CheckItem[] = [];
    let wordCountErrors: CheckItem[] = [];
    let titleErrors: CheckItem[] = []; // 新增：空标题检查
    let checkCollapseState = { seq: false, title: false, word: false };
    let longPressTimer: any;
    let autoRefreshTimer: any;

    // 拖拽与坐标
    let findPanelPos = { x: 0, y: 0 };
    let checkPanelPos = { x: 0, y: 0 };
    let isDragging = false;
    let dragStart = { x: 0, y: 0 };
    let activeDragTarget = "find"; // 'find' or 'check'

    // Close Dialog State
    let showCloseDialog = false;
    let isDialogSaving = false;
    let lastGeneratedEpubPath = ""; // New state variable

    function handleDialogSave() {
        isDialogSaving = true;
        saveFile()
            .then(async () => {
                // After save, exit
                await invoke("exit_app");
                // Window destroys, state doesn't matter much but good practice
            })
            .catch(() => {
                isDialogSaving = false;
            });
    }

    async function handleDialogDiscard() {
        // Discard changes: Clear cache and exit
        localStorage.removeItem("app-crash-recovery");
        await invoke("exit_app");
    }

    function handleDialogCancel() {
        showCloseDialog = false;
    }

    function startDrag(e: MouseEvent, target: "find" | "check") {
        if (
            (e.target as HTMLElement).tagName === "INPUT" ||
            (e.target as HTMLElement).tagName === "BUTTON" ||
            (e.target as HTMLElement).classList.contains("err-tag")
        )
            return;
        isDragging = true;
        activeDragTarget = target;
        const currentPos = target === "find" ? findPanelPos : checkPanelPos;
        dragStart = {
            x: e.clientX - currentPos.x,
            y: e.clientY - currentPos.y,
        };
        window.addEventListener("mousemove", handleDrag);
        window.addEventListener("mouseup", stopDrag);
    }
    function handleDrag(e: MouseEvent) {
        if (!isDragging) return;
        const newPos = {
            x: e.clientX - dragStart.x,
            y: e.clientY - dragStart.y,
        };
        if (activeDragTarget === "find") findPanelPos = newPos;
        else checkPanelPos = newPos;
    }
    function stopDrag() {
        isDragging = false;
        window.removeEventListener("mousemove", handleDrag);
        window.removeEventListener("mouseup", stopDrag);
    }

    onMount(() => {
        console.log("App mounting...");
        window.addEventListener("error", (e) => {
            console.error("Global Error Caught:", e.message, e.error);
        });
        window.addEventListener("unhandledrejection", (e) => {
            console.error("Unhandled Promise Rejection:", e.reason);
        });

        let unlistenClose: any;
        let unlistenDragDrop: any;

        const init = async () => {
            const { getCurrentWindow, LogicalPosition } = await import("@tauri-apps/api/window");
            const appWindow = getCurrentWindow();
            const label = appWindow.label;

            // 1. 窗口位置恢复
            const savedPos = localStorage.getItem("window_pos_" + label);
            if (savedPos) {
                try {
                    const { x, y } = JSON.parse(savedPos);
                    await appWindow.setPosition(new LogicalPosition(x, y));
                } catch (e) {}
            }

            // 监听移动并保存
            appWindow.listen("tauri://move", async () => {
                try {
                    const pos = await appWindow.outerPosition();
                    localStorage.setItem("window_pos_" + label, JSON.stringify(pos));
                } catch (e) {}
            });

            // Listen for restore request from EPUB window
            appWindow.listen("restore-main-window", async () => {
                console.log("Received restore-main-window event");
                await appWindow.show();
                await appWindow.setFocus();
            });

            // 监听系统文件拖放：将 txt/md/epub 直接拖到窗口即可打开
            try {
                unlistenDragDrop = await appWindow.onDragDropEvent(async (event: any) => {
                    const payload = event?.payload;
                    if (!payload || payload.type !== "drop") return;
                    const paths: string[] = Array.isArray(payload.paths) ? payload.paths : [];
                    if (paths.length === 0) return;
                    await openDroppedFile(paths);
                });
            } catch (e) {
                console.warn("注册拖放监听失败:", e);
            }

            // 2. 移动端检测
            if (window.innerWidth < 768) {
                isMobile = true;
                showSidebar = false;
            }

            // 3. 读取设置
            const stored = localStorage.getItem("app-settings");
            if (stored) {
                try {
                    let parsed = JSON.parse(stored);
                    appSettings = { ...DEFAULT_SETTINGS, ...parsed };
                    
                    // 核心初始化：确保 customRegexRules 存在并按照预期结构映射
                    if (!appSettings.customRegexRules || !Array.isArray(appSettings.customRegexRules)) {
                        appSettings.customRegexRules = [
                            { level: 1, pattern: appSettings.metaRegex || DEFAULT_SETTINGS.metaRegex },
                            { level: 1, pattern: appSettings.volRegex || DEFAULT_SETTINGS.volRegex },
                            { level: 3, pattern: appSettings.chapRegex || DEFAULT_SETTINGS.chapRegex }
                        ];
                    } else {
                        // 迁移：如果包含 type，无缝转换为 level
                        appSettings.customRegexRules = appSettings.customRegexRules.map((r: any) => {
                            if (typeof r.type === "string") {
                                let level = 3;
                                if (r.type === "Volume" || r.type === "Meta") level = 1;
                                return { level, pattern: r.pattern };
                            }
                            return r;
                        });
                    }

                    // 迁移旧版正则：将 [章回] 替换为排除"回合"的模式
                    appSettings.customRegexRules.forEach(r => {
                        if (r.pattern && r.pattern.includes("[章回]")) {
                            r.pattern = r.pattern.replace("[章回]", "(?:[章节]|回(?:[^合]|$))");
                        }
                    });
                } catch (e) {}
            }

            // 4. 崩溃恢复逻辑
            const savedState = localStorage.getItem("app-crash-recovery");
            if (savedState) {
                try {
                    const state = JSON.parse(savedState);
                    if (state.filePath && state.filePath !== "请打开一本小说...") {
                        filePath = state.filePath;
                        let diskContent = "";
                        try {
                            diskContent = await invoke("read_text_file", { path: filePath });
                        } catch (e) {
                            console.warn("File read fail:", e);
                        }

                        if (state.isModified && state.content && state.content !== diskContent) {
                            fileContent = state.content;
                            isModified = true;
                        } else {
                            fileContent = diskContent;
                            isModified = false;
                            if (state.isModified)
                                localStorage.removeItem("app-crash-recovery");
                        }

                        if (fileContent) {
                            await tick();
                            editorComponent?.resetDoc(fileContent);
                            await scanToc(fileContent);
                            epubMeta = extractMetadata(fileContent, filePath);
                            updateMd5(fileContent);
                            if (state.scrollLine) {
                                setTimeout(() => editorComponent?.scrollToLine(state.scrollLine), 200);
                            }
                        }
                    }
                } catch (e) {
                    console.error("Recovery failed:", e);
                    localStorage.removeItem("app-crash-recovery");
                }
            }

            // 5. 文件关联启动
            setTimeout(async () => {
                const launchArg = await invoke<string | null>("get_launch_args");
                if (launchArg) openLocalFile(launchArg, true);
                hasInitialized = true;
            }, 500);

            // 6. 关闭拦截
            await appWindow.setTitle("TEpub-Editor-TXT");
            unlistenClose = await appWindow.onCloseRequested(async (event) => {
                if (isModified) {
                    event.preventDefault();
                    showCloseDialog = true;
                } else {
                    await invoke("exit_app");
                }
            });
        };

        init();

        // 监听全选事件
        const handleSelectAll = () => {
            editorComponent?.selectAll();
        };
        window.addEventListener("editor-select-all", handleSelectAll);

        return () => {
            if (unlistenClose) unlistenClose();
            if (unlistenDragDrop) unlistenDragDrop();
            window.removeEventListener("editor-select-all", handleSelectAll);
        };
    });

    // --- [4. 核心逻辑实现] ---

    async function updateMd5(content: string) {
        try {
            epubMeta.md5 = await invoke("calculate_md5", { content });
        } catch (e) {}
    }

    function extractMetadata(content: string, path: string) {
        const meta = {
            title: "书名",
            creator: "作者",
            publisher: "",
            date: new Date().toISOString().split("T")[0],
            uuid: crypto.randomUUID(),
            md5: epubMeta.md5 || "",
            cover_path: epubMeta.cover_path || "",
            description: "",
            styles: { ...epubMeta.styles },
            assets: [...epubMeta.assets] as { name: string, path: string, category: string }[],
        };

        // 默认书名
        const basename = path.split(/[\\/]/).pop()?.replace(/\.[^/.]+$/, "") || "未命名";
        meta.title = basename;

        try {
            const lines = content.split("\n").map(l => l.trim()).filter(l => l.length > 0);
            if (lines.length > 0) {
                const firstLine = lines[0];
                const secondLine = lines.length > 1 ? lines[1] : "";

                // 规则 1: 书名号提取 《...》
                const bracketMatch = firstLine.match(/《([^》]+)》/);
                if (bracketMatch) {
                    meta.title = bracketMatch[1].trim();
                }
                // 规则 2: "书名：" 前缀
                else if (firstLine.match(/^(?:书名|小说名|Title)[\s:：]+(.*)/i)) {
                    meta.title = firstLine.replace(/^(?:书名|小说名|Title)[\s:：]+/i, "").trim();
                }
                // 规则 3: 双行关联 (如果第二行是作者，第一行通常是书名)
                else if (secondLine.match(/^(?:作者|Author|By)[\s:：~]*(.*)/i)) {
                    meta.title = firstLine.trim();
                }
            }

            // 提取作者 (严格限制在前 2 行)
            const first2LinesForAuthor = lines.slice(0, 2).join("\n");
            const authorMatch = first2LinesForAuthor.match(/(?:^|\n)\s*(?:作者|Author|By)[\s:：~]*([^\n\r]+)/i);
            if (authorMatch && authorMatch[1]) {
                meta.creator = authorMatch[1].trim();
            }

            // 规则 4: 书名兜底（前两行未识别出《》或书名：时）
            if (!meta.title || meta.title === "书名" || meta.title === meta.creator) {
                 meta.title = basename;
            }

            // 3. 简介 (更精准的正则：保留首行缩进)
            const descMatch = content.match(/(?:^|\n)[^\S\n]*(?:内容)?(?:简介|Intro|Description)[\t ]*[:：]?[\t ]*(?:\r?\n)?([\s\S]+?)(?=\n\s*(?:第[零一二三四五六七八九十百千万0-9]+[卷部章回|卷部]|Chapter\s*\d+)|$)/i);
            if (descMatch && descMatch[1]) {
                const desc = descMatch[1].replace(/\s+$/, ""); // 仅修剪尾部空白
                if (desc.length > 0) {
                    meta.description = desc.length > 3000 ? desc.substring(0, 3000) + "..." : desc;
                }
            }
        } catch (e) {
            console.log("Metadata extract failed", e);
        }
        return meta;
    }

    function refreshEpubMetadata() {
        if (!fileContent) return;
        const fresh = extractMetadata(fileContent, filePath);
        
        // 如果文件内容已修改，或者当前仍是默认占位符，则更新主要字段
        const isTitleDefault = epubMeta.title === "书名" || !epubMeta.title;
        const isCreatorDefault = epubMeta.creator === "作者" || !epubMeta.creator;

        if (isModified || isTitleDefault) epubMeta.title = fresh.title;
        if (isModified || isCreatorDefault) epubMeta.creator = fresh.creator;
        if (isModified || !epubMeta.description) epubMeta.description = fresh.description;
        
        // 加载自定义内置样式 (如果存在)
        if (appSettings.defaultEpubStyles) {
            if (!epubMeta.styles["main.css"]) epubMeta.styles["main.css"] = appSettings.defaultEpubStyles["main.css"];
            if (!epubMeta.styles["font.css"]) epubMeta.styles["font.css"] = appSettings.defaultEpubStyles["font.css"];
        }

        // UUID 保持不变除非为空
        if (!epubMeta.uuid) epubMeta.uuid = fresh.uuid;
        // 强制重新计算 MD5
        updateMd5(fileContent);
    }

    async function loadCoverPreview() {
        if (!epubMeta.cover_path) {
            if (coverPreviewUrl) {
                URL.revokeObjectURL(coverPreviewUrl);
                coverPreviewUrl = null;
            }
            return;
        }
        try {
            const normalizedPath = normalizeLocalPath(epubMeta.cover_path);
            if (normalizedPath !== epubMeta.cover_path) {
                epubMeta.cover_path = normalizedPath;
            }
            const data = await invoke<number[]>("read_binary_file", { path: normalizedPath });
            const ext = normalizedPath.split('.').pop()?.toLowerCase() || 'jpg';
            const mimeMap: Record<string, string> = { jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', webp: 'image/webp', gif: 'image/gif' };
            const mime = mimeMap[ext] || 'image/jpeg';
            const blob = new Blob([new Uint8Array(data)], { type: mime });
            if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
            coverPreviewUrl = URL.createObjectURL(blob);
        } catch (e) {
            console.error('封面预览加载失败:', e);
            coverPreviewUrl = null;
        }
    }

    function normalizeLocalPath(path: string): string {
        if (!path || !path.startsWith("file://")) return path;
        try {
            const url = new URL(path);
            if (url.protocol !== "file:") return path;
            let pathname = decodeURIComponent(url.pathname || "");
            if (/^\/[A-Za-z]:/.test(pathname)) {
                pathname = pathname.slice(1);
            }
            return pathname || path;
        } catch {
            return path;
        }
    }

    function extractPickedPath(selection: string | string[]): string {
        const raw = Array.isArray(selection) ? selection[0] : selection;
        return normalizeLocalPath(raw);
    }

    async function openAdvancedEpubMetadata() {
        try {
            const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
            
            // 检查窗口是否已存在
            const existing = await WebviewWindow.getByLabel("epub-metadata");
            if (existing) {
                await existing.setFocus();
                return;
            }

            const win = new WebviewWindow("epub-metadata", {
                url: "/epub-metadata",
                title: "高级选项",
                width: 450,
                height: 480,
                resizable: true,
                decorations: true,
                center: true,
            });

            // 监听初始化请求
            win.once("metadata-window-ready", async () => {
                await emit("init-metadata", {
                    meta: {
                        publisher: epubMeta.publisher,
                        uuid: epubMeta.uuid,
                        md5: epubMeta.md5,
                        styles: { ...epubMeta.styles },
                        assets: [...epubMeta.assets]
                    },
                    custom: customMetadata
                });
            });

            // 监听更新
            const unlisten = await listen("update-metadata", (event: any) => {
                const { meta, custom, persistCss } = event.payload;
                epubMeta.publisher = meta.publisher;
                epubMeta.uuid = meta.uuid;
                epubMeta.md5 = meta.md5;
                epubMeta.styles = { ...meta.styles };
                epubMeta.assets = [...(meta.assets || [])];
                customMetadata = [...custom];

                if (persistCss) {
                    appSettings.defaultEpubStyles = { ...meta.styles };
                    localStorage.setItem("app-settings", JSON.stringify(appSettings));
                    console.log("Persisted custom styles to settings");
                }

                console.log("Updated metadata from window:", event.payload);
            });

            win.once("tauri://destroyed", () => {
                unlisten();
            });

        } catch (e) {
            message("打开高级设置失败: " + e, { kind: "error" });
        }
    }

    function saveStateToCache(line: number) {
        if (isLoadingFile) return;
        // 限制缓存大小，防止 localStorage 溢出
        const state = {
            filePath,
            isModified,
            scrollLine: line,
            content:
                isModified && fileContent.length < 3000000 ? fileContent : null,
        };
        localStorage.setItem("app-crash-recovery", JSON.stringify(state));
    }

    async function openLocalFile(path: string, initialLaunch = false) {
        try {
            if (path) {
                // 检查是否是 EPUB 文件
                if (path.toLowerCase().endsWith(".epub")) {
                    const encodedPath = encodeURIComponent(path);
                    console.log("打开 EPUB 文件:", path);

                    if (initialLaunch) {
                        // Initial launch: Reuse the main window
                        const { getCurrentWindow, LogicalSize } = await import(
                            "@tauri-apps/api/window"
                        );
                        const appWindow = getCurrentWindow();
                        await appWindow.setTitle("TEpub-Editor-EPUB");
                        await appWindow.setSize(new LogicalSize(1200, 800));
                        window.location.href = `/epub-editor?file=${encodedPath}`;
                        return;
                    }

                    try {
                        // 打开新窗口显示 EPUB 编辑器
                        const { WebviewWindow } = await import(
                            "@tauri-apps/api/webviewWindow"
                        );

                        // 确保路径正确编码
                        console.log("编码后路径:", encodedPath);

                        const epubWindow = new WebviewWindow(
                            "epub-editor-" + Date.now(),
                            {
                                url: `/epub-editor?file=${encodedPath}`,
                                title: "TEpub-Editor-EPUB",
                                width: 1200,
                                height: 740,
                                dragDropEnabled: true,
                                center: true, // Center the window
                            },
                        );

                        // 这里的事件监听可能不触发，改为直接执行隐藏逻辑
                        // Logic: Close main window if it's empty
                        console.log(
                            "Checking if main window should hide (Immediate). Content length:",
                            fileContent ? fileContent.length : 0,
                        );

                        const current = getCurrentWindow();
                        // 强制隐藏：只要不是在编辑已有的文件（通过内容是否为空判断），就隐藏
                        if (!fileContent || fileContent.trim().length === 0) {
                            console.log("Hiding main window...");
                            await current.hide();
                        } else {
                            console.log(
                                "Main window kept open. Content exists.",
                            );
                        }

                        // 无论隐藏与否，都监听错误
                        epubWindow.once("tauri://error", (e) => {
                            console.error("窗口创建失败:", e);
                            message("打开 EPUB 编辑器失败: " + e, {
                                title: "错误",
                                kind: "error",
                            });
                        });

                        // 监听已销毁事件：当 EPUB 窗口关闭时，恢复主窗口显示
                        epubWindow.once("tauri://destroyed", async () => {
                            console.log(
                                "EPUB window destroyed, restoring main window...",
                            );
                            const current = getCurrentWindow();
                            await current.show();
                            await current.setFocus();
                        });
                    } catch (e) {
                        console.error("EPUB 窗口打开错误:", e);
                        await message("打开 EPUB 编辑器失败: " + e, {
                            title: "错误",
                            kind: "error",
                        });
                    }
                    return; // EPUB 文件处理完毕，直接返回
                }

                isLoading = true;
                isLoadingFile = true;
                filePath = path;

                // 读取原生文本并施加终极降维打击：强力规范化换行符！
                let rawContent = await invoke<string>("read_text_file", {
                    path: filePath,
                });
                let content = rawContent.replace(
                    /\r\n|\r|\u2028|\u2029/g,
                    "\n",
                );

                // 【终极排错大招】防巨型单行核武器：如果此文件的恶劣排版中包含超乎想象的巨龙行（>800字没有一个物理回车），
                // CodeMirror 会在拖拽选区或滚动时因为几何测算彻底超载，并导致 posAtCoordsInline 读出 null 空指针崩溃。
                // 解决方案：为所有超长异端长句智能注入真正的换行！
                content = content
                    .split("\n")
                    .map((line) => {
                        if (line.length > 800) {
                            // 遇到八百字不换行的“伪文字段落”，在句号/叹号/问号后（包裹着引号时也行），并且后面跟着空格或什么都没有的地方，强制斩断加回车
                            return line.replace(
                                /([。\.\!\?][”’」』]*)(?=\s|\S)/g,
                                "$1\n",
                            );
                        }
                        return line;
                    })
                    .join("\n");

                fileContent = content;

                // 提取元数据
                epubMeta = extractMetadata(content, path);
                customMetadata = []; // 重置自定义元数据

                editorComponent?.resetDoc(content);
                isModified = false;
                updateMd5(content);
                await scanToc(content);
                if (isCheckModeOn) runFullCheck();

                isLoading = false;
                localStorage.removeItem("app-crash-recovery");
                setTimeout(() => {
                    isLoadingFile = false;
                }, 100);
            }
        } catch (e) {
            isLoading = false;
            console.error("Open file failed:", e);
            message(`打开文件失败: ${e}`, { kind: "error" });
        }
    }

    async function openDroppedFile(paths: string[]) {
        if (!paths || paths.length === 0) return;
        const firstSupported = paths.find((p) => /\.(txt|md|epub)$/i.test(p));
        if (!firstSupported) {
            await message("仅支持拖入 TXT/MD/EPUB 文件", { kind: "warning" });
            return;
        }
        await openLocalFile(firstSupported);
    }

    async function selectFile() {
        try {
            const selected = await open({
                multiple: false,
                filters: [
                    {
                        name: "所有支持的文件",
                        extensions: ["txt", "md", "epub"],
                    },
                    { name: "文本文件", extensions: ["txt", "md"] },
                    { name: "EPUB 文件", extensions: ["epub"] },
                ],
            });
            if (selected) {
                await openLocalFile(selected.toString());
            }
        } catch (e) {
            console.error("Select file failed:", e);
        }
    }

    async function saveFile() {
        if (!fileContent || isSaving) return;
        isSaving = true;
        try {
            if (filePath.startsWith("请打开")) {
                const path = await save({
                    filters: [{ name: "Text", extensions: ["txt"] }],
                });
                if (!path) {
                    isSaving = false;
                    return;
                }
                filePath = path;
            }
            // await writeTextFile(filePath, fileContent);
            await invoke("save_text_file", {
                path: filePath,
                content: fileContent,
            });
            // 调用后端保存历史
            await invoke("save_history", {
                originalPath: filePath,
                content: fileContent,
            }).catch(() => {});

            isModified = false;
            // Clear crash recovery on explicit save
            localStorage.removeItem("app-crash-recovery");
            // saveStateToCache(0); // Optional: re-save cleanslate or just remove. Removing is safer.
            updateMd5(fileContent);
            await scanToc(fileContent);
            // await message("保存成功！"); // 移除弹窗，保持静默成功
        } catch (e) {
            await message(`保存失败: ${e}\n请确保已授予“所有文件访问权限”`, {
                kind: "error",
            });
        } finally {
            isSaving = false;
        }
    }

    // --- TOC 解析与同步 (含双向绑定) ---
    async function scanToc(textOverride?: string) {
        const text = textOverride ?? fileContent;
        if (!text) return;
        try {
            // 调用 Rust 正则扫描
            const rawList = await invoke<RawChapter[]>("scan_chapters", {
                content: text,
                rules: appSettings.customRegexRules,
            });

            const tree: TocNode[] = [];
            flatToc = [];
            let uid = 0;
            let parentStack: TocNode[] = [];

            // 构建嵌套树
            for (const item of rawList) {
                // Determine legacy type for UI styling backward compatibility
                const computedType = item.is_meta ? "Meta" : (item.level === 1 ? "Volume" : "Chapter");
                
                const node: TocNode = {
                    id: `n-${uid++}`,
                    title: item.title,
                    line_number: item.line_number,
                    type: computedType,
                    word_count: item.word_count,
                    children: [],
                    expanded: true,
                };

                const flatNode: FlatNode = {
                    id: node.id,
                    line: node.line_number,
                    title: node.title,
                    type: computedType,
                    word_count: node.word_count,
                };

                // Remove parents that are closed by this node
                while (parentStack.length > 0) {
                    let top = parentStack[parentStack.length - 1];
                    // If the parent is not Meta, and its level is STRICTLY smaller, it IS a valid parent.
                    // Wait, our level is 1..5. Smaller number = higher level (like h1). 
                    // So valid parent must have level < item.level.
                    // Example: Volume is level 1. Chapter is level 3. Parent (1) < Node (3).
                    // If we encounter another Level 1, we pop the previous Level 1 because 1 is not < 1.
                    if (top.level !== undefined && top.level < item.level && top.type !== "Meta") {
                        break;
                    }
                    if (top.type === "Volume" && item.level > 1) {
                        break; // Fallback for safely catching missing level property in old node trees
                    }
                    parentStack.pop();
                }

                if (parentStack.length > 0) {
                    let parent = parentStack[parentStack.length - 1];
                    // Only volume nodes act as parents in the tree representation 
                    node.parentId = parent.id;
                    parent.children.push(node);
                    flatNode.parentId = parent.id;
                } else {
                    tree.push(node);
                }

                // Add non-meta elements to stack using dynamic levels
                (node as any).level = item.level;
                parentStack.push(node);
                flatToc.push(flatNode);
            }
            flatToc = [...flatToc];
            tocTree = tree;

            // 更新统计
            let v = 0,
                c = 0;
            tocTree.forEach((n) => {
                if (n.type === "Volume") {
                    v++;
                    c += n.children.length;
                } else if (n.type === "Chapter") c++;
            });
            stats = { volumes: v, chapters: c };

            if (isCheckModeOn) runFullCheck();
        } catch (e) {}
    }
    let saveCacheTimer: ReturnType<typeof setTimeout> | null = null;

    let lastNavChapterId: string | null = null; // 导航锁定的章节ID
    let lastNavLine: number = 0; // 导航锁定章节的行号

    // 编辑器滚动时触发：高亮侧边栏
    async function handleScroll(state: {
        top: number;
        bottom: number;
        isAtBottom: boolean;
    }) {
        // 防抖: 每2秒最多保存一次状态到 localStorage（只保存 top 行号）
        if (!saveCacheTimer) {
            saveCacheTimer = setTimeout(() => {
                saveCacheTimer = null;
                saveStateToCache(state.top);
            }, 2000);
        }
        if (flatToc.length === 0) return;
        if (isNavigating) return; // 正在手动跳转，忽略滚动监听

        // 倒序查找上下边界分别对应的章节
        // 注意：CM6 使用 scrollIntoView(y:"start") 时，章节标题往往排在视口顶部往下 5-20 行的地方。
        // 所以我们加上 10 行的容差。如果某个章节标题出现在视口顶部这 10 行内，我们就认为当前处于该章节。
        let foundTop: FlatNode | null = null;
        let foundBottom: FlatNode | null = null;

        for (let i = flatToc.length - 1; i >= 0; i--) {
            if (!foundTop && flatToc[i].line <= state.top + 10) {
                foundTop = flatToc[i];
            }
            if (!foundBottom && flatToc[i].line <= state.bottom) {
                foundBottom = flatToc[i];
            }
            if (foundTop && foundBottom) break;
        }

        // 默认高亮视口最上方的章节
        // 但如果已经滚到了文档绝对底部，则高亮视口最下方的章节
        // 这样可以完美解决最后几章很短导致无法滚动到顶部时的高亮错位问题
        let found = state.isAtBottom ? foundBottom : foundTop;

        if (found && found.id !== activeChapterId) {
            activeChapterId = found.id;

            // 如果是卷内章节，确保父卷展开
            if (found.parentId) {
                const p = tocTree.find((n) => n.id === found!.parentId);
                if (p && !p.expanded) {
                    p.expanded = true;
                    tocTree = [...tocTree];
                    await tick();
                }
            }

            // 侧边栏自动滚动
            await tick();
            const el = document.getElementById(`toc-${activeChapterId}`);
            const tocList = document.querySelector(".toc-list");
            if (el && tocList) {
                const elRect = el.getBoundingClientRect();
                const listRect = tocList.getBoundingClientRect();
                // 仅当目标不在可视区域的中心时才微调滚动，避免频繁触发 reflow 抖动
                if (
                    elRect.top < listRect.top + 50 ||
                    elRect.bottom > listRect.bottom - 50
                ) {
                    const scrollAmount =
                        elRect.top -
                        listRect.top -
                        listRect.height / 2 +
                        elRect.height / 2;
                    tocList.scrollBy({ top: scrollAmount, behavior: "smooth" });
                }
            }
        }
    }

    // 处理选择时的目录同步
    async function handleSelectionChange(line: number) {
        if (isNavigating) return;
        handleScroll({ top: line, bottom: line, isAtBottom: false });
    }

    // 统一处理章节跳转点击
    function handleChapterClick(id: string, line: number) {
        console.log("handleChapterClick", id, line);

        // 1. 清理旧定时器
        if (scrollTimeout) {
            clearTimeout(scrollTimeout);
            scrollTimeout = null;
        }

        // 2. 开启导航锁
        isNavigating = true;

        // 3. 立即更新高亮 + 设置导航锁定目标
        activeChapterId = id;
        lastNavChapterId = id;
        lastNavLine = line;

        // 4. 执行滚动
        if (editorComponent) {
            editorComponent.scrollToLine(line, true);
        } else {
            console.error("Editor component not ready");
        }

        // 5. 手动滚动侧边栏（因为 handleScroll 被锁住了）
        requestAnimationFrame(() => {
            const el = document.getElementById(`toc-${id}`);
            if (el) {
                el.scrollIntoView({ behavior: "smooth", block: "center" });
            }
        });

        // 6. 解锁导航锁（handleScroll 会通过 lastNavChapterId 继续保护高亮）
        scrollTimeout = setTimeout(() => {
            isNavigating = false;
            scrollTimeout = null;
        }, 1200);
    }

    // --- 检查逻辑 ---
    function toggleCheckMode() {
        isCheckModeOn = !isCheckModeOn;
        if (isCheckModeOn) {
            scanToc();
            runFullCheck();
        } else {
            invalidSequenceIds.clear();
            tocTree = [...tocTree];
        }
    }

    function startLongPress(e: Event) {
        if (isMobile) {
            e.preventDefault();
            (document.activeElement as HTMLElement)?.blur();
        }
        longPressTimer = setTimeout(() => {
            closeAllPanels();
            showCheckPanel = true;
            runFullCheck();
        }, 600);
    }

    // PC 端鼠标长按支持
    function handleMouseDown() {
        longPressTimer = setTimeout(() => {
            // closeAllPanels(); // 允许和其他面板共存
            showCheckPanel = true;
            // 初始化位置
            if (checkPanelPos.x === 0 && checkPanelPos.y === 0) {
                checkPanelPos = { x: window.innerWidth / 2 - 150, y: 100 };
            }
            runFullCheck();
        }, 600);
    }

    // 中文数字转阿拉伯数字
    function chineseToNum(cn: string): number {
        const charMap: Record<string, number> = {
            零: 0,
            〇: 0,
            一: 1,
            二: 2,
            两: 2,
            三: 3,
            四: 4,
            五: 5,
            六: 6,
            七: 7,
            八: 8,
            九: 9,
            十: 10,
            百: 100,
            千: 1000,
            万: 10000,
        };
        let result = 0,
            current = 0;
        for (const c of cn) {
            const v = charMap[c];
            if (v === undefined) return -1;
            if (v >= 10) {
                if (current === 0) current = 1;
                if (v === 10000) {
                    result = (result + current) * v;
                    current = 0;
                } else {
                    current *= v;
                    result += current;
                    current = 0;
                }
            } else {
                current = current * 10 + v;
            }
        }
        return result + current;
    }

    // 从标题中提取章节序号，优先匹配"第X章/回/节"格式
    function extractChapterNum(title: string): number {
        // 1. 优先匹配 "第X章/回/节" 格式（支持中文数字和阿拉伯数字）
        const m = title.match(
            /第\s*([0-9零一二三四五六七八九十百千万〇两]+)\s*[章回节]/,
        );
        if (m) {
            const raw = m[1];
            // 纯阿拉伯数字
            if (/^\d+$/.test(raw)) return parseInt(raw);
            // 中文数字
            return chineseToNum(raw);
        }
        // 2. 降级：标题以纯数字开头（如 "101 黑暗"）
        const m2 = title.match(/^(\d+)/);
        if (m2) return parseInt(m2[1]);
        return -1;
    }

    function runFullCheck() {
        sequenceErrors = [];
        wordCountErrors = [];
        titleErrors = [];
        invalidSequenceIds.clear();
        let lastNum = -1;
        for (const node of flatToc) {
            if (node.type === "Chapter") {
                const num = extractChapterNum(node.title);
                if (num !== -1) {
                    if (lastNum !== -1 && num !== lastNum + 1) {
                        invalidSequenceIds.add(node.id);
                        sequenceErrors.push({
                            id: node.id,
                            title: node.title,
                            line: node.line,
                            msg: `(${lastNum}-${num})`,
                            val: num,
                        });
                    }
                    lastNum = num;
                }

                // 空标题检查: 仅包含数字、序号，没有具体内容
                if (
                    /^第\s*[0-9零一二三四五六七八九十百千万]+\s*[章卷回节]\s*$/.test(
                        node.title.trim(),
                    ) ||
                    /^\d+$/.test(node.title.trim())
                ) {
                    titleErrors.push({
                        id: node.id,
                        title: node.title,
                        line: node.line,
                        msg: "无标题",
                        val: 0,
                    });
                }

                if (node.word_count > appSettings.wordCountThreshold) {
                    wordCountErrors.push({
                        id: node.id,
                        title: node.title,
                        line: node.line,
                        msg: `超标`,
                        val: node.word_count,
                    });
                }
            } else if (node.type === "Volume") {
                // 新卷开始，重置序号计数
                lastNum = -1;
            }
        }
        tocTree = [...tocTree]; // 触发 Svelte 更新
    }

    // --- 查找替换逻辑 ---
    async function findNext() {
        if (!allMatches || allMatches.length === 0) await performFind();
        if (allMatches && allMatches.length > 0) {
            currentMatchIndex = (currentMatchIndex + 1) % allMatches.length;
            replaceMsg = `第 ${currentMatchIndex + 1}/${allMatches.length} 处`;
            editorComponent.selectMatch(
                allMatches[currentMatchIndex].line,
                allMatches[currentMatchIndex].start_char,
                allMatches[currentMatchIndex].end_char,
            );
        }
    }

    async function findPrev() {
        if (!allMatches || allMatches.length === 0) await performFind();
        if (allMatches && allMatches.length > 0) {
            currentMatchIndex =
                (currentMatchIndex - 1 + allMatches.length) % allMatches.length;
            replaceMsg = `第 ${currentMatchIndex + 1}/${allMatches.length} 处`;
            editorComponent.selectMatch(
                allMatches[currentMatchIndex].line,
                allMatches[currentMatchIndex].start_char,
                allMatches[currentMatchIndex].end_char,
            );
        }
    }

    async function performFind() {
        if (!fileContent || !findPattern) return;
        try {
            const res = await invoke<SearchResult>("advanced_search", {
                content: fileContent,
                pattern: findPattern,
                isRegex,
            });
            if (res.found) {
                allMatches = res.matches;
                currentMatchIndex = 0;
                replaceMsg = `第 1/${res.count} 处`;
                editorComponent.selectMatch(
                    allMatches[0].line,
                    allMatches[0].start_char,
                    allMatches[0].end_char,
                );
            } else {
                allMatches = [];
                replaceMsg = "未找到";
            }
        } catch (e) {
            replaceMsg = "正则错误";
        }
    }

    async function performReplaceAll() {
        if (!fileContent || !findPattern) return;
        const confirmed = await ask("确定执行全书替换吗？此操作无法撤销。", {
            kind: "warning",
        });
        if (!confirmed) return;

        try {
            const res = await invoke<string>("advanced_replace", {
                content: fileContent,
                pattern: findPattern,
                replacement: replacePattern,
                isRegex,
            });
            fileContent = res;
            editorComponent.resetDoc(res);
            replaceMsg = "替换完成";
            allMatches = [];
        } catch (e) {
            replaceMsg = "替换失败";
        }
    }

    // --- EPUB 导出 ---
    async function generateEpub() {
        if (!fileContent) return;

        // 必填项检查 (仅书名如果不填会无法生成有效OPF，其他可选)
        if (!epubMeta.title || epubMeta.title.trim() === "") {
            // 尝试使用文件名作为默认书名
            const basename =
                filePath
                    .split(/[\\/]/)
                    .pop()
                    ?.replace(/\.[^/.]+$/, "") || "未命名书籍";
            epubMeta.title = basename;
        }
        if (!epubMeta.uuid) epubMeta.uuid = crypto.randomUUID();
        // MD5 应该在文件加载时已计算，防卫性保留
        if (!epubMeta.md5) await updateMd5(fileContent);

        epubGenerationStatus = "generating";
        isLoading = true;
        try {
            const savePath = await save({
                filters: [{ name: "EPUB", extensions: ["epub"] }],
                defaultPath: epubMeta.title + ".epub",
            });
            if (!savePath) {
                isLoading = false;
                epubGenerationStatus = "idle";
                return;
            }

            let chapters = await invoke<RawChapter[]>("scan_chapters", {
                content: fileContent,
                rules: appSettings.customRegexRules,
            });

            // 智能清洗
            const cleanRegex =
                /^(\s*(?:第[零一二三四五六七八九十百千万0-9]+[卷部章回]|Chapter\s*\d+|楔子|序[章言]?))\s*[:：]\s*/;
            chapters = chapters.map((c) => {
                c.title = c.title.replace(cleanRegex, "$1 ");
                return c;
            });

            await invoke("export_epub", {
                savePath,
                content: fileContent,
                chapters,
                metadata: {
                    title: epubMeta.title,
                    creator: epubMeta.creator,
                    publisher: epubMeta.publisher,
                    date: epubMeta.date,
                    uuid: epubMeta.uuid,
                    md5: epubMeta.md5,
                    cover_path: epubMeta.cover_path,
                    description: epubMeta.description,
                    main_css: epubMeta.styles["main.css"],
                    font_css: epubMeta.styles["font.css"],
                    assets: epubMeta.assets,
                    ...Object.fromEntries(customMetadata.map(m => [m.key, m.value]))
                },
            });
            // 制作成功：设置状态为成功，在UI上显示操作按钮
            epubGenerationStatus = "success";

            // 保存此时的路径供按钮使用
            // (We can assume 'savePath' is available, but we need to store it in a state variable
            // if we want the button in HTML to access it easily?
            // actually 'savePath' is local. Let's create a module-level variable or just use the closure if we were inline.
            // Let's add a state variable `lastGeneratedEpubPath`.
            lastGeneratedEpubPath = savePath;
        } catch (e) {
            // 失败时显示错误并重置状态
            await message("制作失败: " + e, { kind: "error" });
            epubGenerationStatus = "idle";
        } finally {
            isLoading = false;
        }
    }

    async function confirmRestore() {
        if (!restoreTargetSnapshot) return;

        try {
            // 1. 先保存当前版本为新历史
            if (filePath && fileContent) {
                await invoke("save_snapshot", {
                    path: filePath,
                    content: fileContent,
                });
            }

            // 2. 执行回退
            fileContent = await invoke("read_text_file", {
                path: restoreTargetSnapshot.path,
            });
            editorComponent.resetDoc(fileContent);

            // 3. 关闭所有弹窗并重新扫描目录
            showRestoreConfirm = false;
            closeAllPanels();
            await scanToc();
        } catch (e) {
            await message("回退失败: " + e, { kind: "error" });
        }
    }

    function closeAllPanels() {
        showSettingsPanel = false;
        showEpubModal = false;
        showCheckPanel = false;
        showHistoryPanel = false;
    }
</script>

<svelte:head>
    <meta name="theme-color" content="#f3f3f3" />
    <meta
        name="viewport"
        content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover"
    />
</svelte:head>


<!-- <ContextMenu /> --> <!-- Removed duplicate at top -->


<main class="app-container" on:contextmenu|preventDefault>
    <header class="toolbar">
        <div class="btn-group">
            <button class="btn-primary" on:click={selectFile}>📂</button>
            <button
                class={isModified ? "btn-save-modified" : "btn-save-default"}
                on:click={saveFile}>💾</button
            >
            <button
                class="btn-secondary"
                on:click={() => editorComponent.triggerUndo()}>↩️</button
            >
            <button
                class="btn-secondary"
                on:click={() => editorComponent.triggerRedo()}>↪️</button
            >
            <button
                class="btn-secondary"
                on:click={() => (showSidebar = !showSidebar)}>📖</button
            >
            <button
                class="btn-secondary"
                on:click={() => {
                    closeAllPanels();
                    refreshEpubMetadata();
                    showEpubModal = true;
                    // 重置EPUB制作状态
                    epubGenerationStatus = "idle";
                    loadCoverPreview();
                }}>📚</button
            >
            <button
                class="btn-secondary"
                on:click={() => {
                    closeAllPanels();
                    showSettingsPanel = true;
                }}>⚙️</button
            >
        </div>
        <button
            class="btn-secondary"
            title="查找与替换 (Ctrl+F)"
            on:click={() => {
                closeAllPanels();
                if (editorComponent) {
                    editorComponent.openSearchWindow();
                }
            }}>🔍</button
        >
    </header>

    <div class="main-body">
        {#if showSidebar && isMobile}
            <div
                role="presentation"
                class="sidebar-mask"
                on:click={() => (showSidebar = false)}
            ></div>
        {/if}

        {#if showSidebar}
            <aside class="sidebar">
                <!-- 头部固定，不再随列表滚动 -->
                <div class="sidebar-header-fixed">
                    <div class="sidebar-header-row">
                        <span>{stats.volumes}卷 {stats.chapters}章</span>
                        <div class="header-btns">
                            <button
                                class="icon-btn"
                                title="全部展开/折叠"
                                on:click={() => {
                                    const anyExpanded = tocTree.some(n => n.expanded);
                                    const targetState = !anyExpanded;
                                    tocTree.forEach(n => n.expanded = targetState);
                                    tocTree = [...tocTree];
                                }}>⇅</button
                            >
                            <button
                                class="mini-btn {isCheckModeOn ? 'active' : ''}"
                                on:mousedown={handleMouseDown}
                                on:mouseup={() => clearTimeout(longPressTimer)}
                                on:mouseleave={() =>
                                    clearTimeout(longPressTimer)}
                                on:click={toggleCheckMode}>检查</button
                            >
                        </div>
                    </div>
                </div>

                <div class="toc-list">
                    {#each tocTree as node (node.id)}
                        <div
                            role="button"
                            tabindex="0"
                            id={`toc-${node.id}`}
                            class="toc-item {node.type === 'Volume'
                                ? 'vol-title'
                                : ''} {activeChapterId === node.id
                                ? 'active'
                                : ''}"
                            on:click={() =>
                                node.type === "Volume"
                                    ? ((node.expanded = !node.expanded),
                                      (tocTree = [...tocTree]))
                                    : editorComponent.scrollToLine(
                                          node.line_number,
                                      )}
                            on:keydown={() => {}}
                        >
                            {#if node.type === "Volume"}
                                <span class="arrow"
                                    >{node.expanded ? "▼" : "▶"}</span
                                >
                            {/if}
                            <span
                                class="toc-title {invalidSequenceIds.has(
                                    node.id,
                                )
                                    ? 'text-error'
                                    : ''}">{node.title}</span
                            >
                            <span class="toc-count"
                                >{node.type === "Volume"
                                    ? node.children.length
                                    : node.word_count}</span
                            >
                        </div>

                        {#if node.expanded}
                            {#each node.children as child (child.id)}
                                <div
                                    role="button"
                                    tabindex="0"
                                    id={`toc-${child.id}`}
                                    class="toc-item indent {activeChapterId ===
                                    child.id
                                        ? 'active'
                                        : ''}"
                                    on:click={() =>
                                        handleChapterClick(
                                            child.id,
                                            child.line_number,
                                        )}
                                    on:keydown={() => {}}
                                >
                                    <span
                                        class="toc-title {invalidSequenceIds.has(
                                            child.id,
                                        )
                                            ? 'text-error'
                                            : ''}">{child.title}</span>
                                    <span class="toc-count"
                                        >{child.word_count}</span
                                    >
                                </div>
                            {/each}
                        {/if}
                    {/each}
                </div>
            </aside>
        {/if}

        <section class="editor-wrapper">
            {#if isLoading}<div class="loading">加载中...</div>{/if}
            <Editor
                bind:this={editorComponent}
                doc={fileContent}
                titleLines={flatToc.map((n) => n.line)}
                onChange={(v) => {
                    fileContent = v;
                    isModified = true;
                    // Debounced TOC Sync
                    clearTimeout(autoRefreshTimer);
                    autoRefreshTimer = setTimeout(() => scanToc(v), 200);
                }}
                onScroll={handleScroll}
                onSelectionChange={handleSelectionChange}
                wordWrap={appSettings.wordWrap}
                showWhitespace={appSettings.showWhitespace}
                showLineBreaks={appSettings.showLineBreaks}
            />
        </section>
    </div>

    {#if showSettingsPanel || showEpubModal || showHistoryPanel}
        <div
            role="presentation"
            class="modal-overlay"
            on:click={closeAllPanels}
        >
            <div
                role="presentation"
                class="modal-content"
                on:click|stopPropagation
            >
                {#if showSettingsPanel}
                    <div class="p-header">
                        <span>偏好设置</span>
                        <button class="icon-close" on:click={closeAllPanels}
                            >✕</button
                        >
                    </div>
                    
                    <div class="settings-tabs">
                        <button class="tab-btn {settingsActiveTab === 'display' ? 'active' : ''}" on:click={() => settingsActiveTab = 'display'}>显示</button>
                        <button class="tab-btn {settingsActiveTab === 'toc' ? 'active' : ''}" on:click={() => settingsActiveTab = 'toc'}>目录</button>
                    </div>
                    <div class="p-body">
                        {#if settingsActiveTab === 'display'}
                            <div class="set-row">
                                <label for="wordWrap">自动换行:</label>
                                <input id="wordWrap" type="checkbox" bind:checked={appSettings.wordWrap} style="width: auto;"/>
                            </div>
                            <div class="set-row">
                                <label for="showWhitespace">显示空格:</label>
                                <input id="showWhitespace" type="checkbox" bind:checked={appSettings.showWhitespace} style="width: auto;"/>
                            </div>
                            <div class="set-row">
                                <label for="showLineBreaks">显示换行符:</label>
                                <input id="showLineBreaks" type="checkbox" bind:checked={appSettings.showLineBreaks} style="width: auto;"/>
                            </div>
                            <!-- 撤销开关 -->
                            <div class="set-row">
                                <label for="wth">单章字数检查:</label>
                                <input
                                    id="wth"
                                    type="number"
                                    bind:value={appSettings.wordCountThreshold}
                                    style="width: 80px;"
                                />
                            </div>
                            
                            <div class="set-row">
                                <label for="clh">保存清空撤销:</label>
                                <input id="clh" type="checkbox" bind:checked={appSettings.clearHistoryOnSave} style="width: auto;"/>
                            </div>
                        {:else}
                            <div class="rules-header">正则表达式</div>
                            <div class="rules-list">
                                {#each appSettings.customRegexRules as rule, idx}
                                    <div class="rule-item" style="gap: 8px; align-items: center;">
                                        <select class="rule-type" bind:value={rule.level} style="width: 80px; flex-shrink: 0;">
                                            <option value={1}>层级 1</option>
                                            <option value={2}>层级 2</option>
                                            <option value={3}>层级 3</option>
                                            <option value={4}>层级 4</option>
                                            <option value={5}>层级 5</option>
                                        </select>
                                        
                                        <div class="rule-input-group">
                                            <input
                                                class="rule-input"
                                                type="text"
                                                bind:value={rule.pattern}
                                                placeholder="输入正则表达式"
                                            />
                                            <div class="rule-arrow-visual">▼</div>
                                            <select
                                                class="rule-hidden-select"
                                                on:change={(e) => {
                                                    const val = e.currentTarget.value;
                                                    if(val) rule.pattern = val;
                                                    e.currentTarget.value = "";
                                                }}
                                            >
                                                <option value="">选择预设正则</option>
                                                {#each REGEX_PRESETS as preset}
                                                    {#if preset.value}
                                                        <option value={preset.value}>{preset.value}</option>
                                                    {/if}
                                                {/each}
                                            </select>
                                        </div>

                                        <button class="rule-btn remove" on:click={() => {
                                            appSettings.customRegexRules.splice(idx, 1);
                                            appSettings.customRegexRules = [...appSettings.customRegexRules];
                                        }}>－</button>
                                    </div>
                                {/each}
                            </div>
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 10px;">
                                <button class="grid-btn" style="padding: 4px 10px; font-size: 13px;" on:click={() => {
                                    appSettings.customRegexRules = [
                                        { level: 1, pattern: DEFAULT_SETTINGS.metaRegex },
                                        { level: 1, pattern: DEFAULT_SETTINGS.volRegex },
                                        { level: 3, pattern: DEFAULT_SETTINGS.chapRegex }
                                    ];
                                }}>↺ 还原正则</button>
                                <button class="grid-btn" style="padding: 4px 10px; font-size: 13px; color: #0066b8; border-color: #0066b8;" on:click={() => {
                                    appSettings.customRegexRules.push({ level: 3, pattern: "" });
                                    appSettings.customRegexRules = [...appSettings.customRegexRules];
                                }}>＋ 新增正则</button>
                            </div>

                        {/if}

                        <!-- 底部通用按钮：放在一行，且放在 if/else 外部 -->
                        <div style="display:flex; gap:10px; margin-top:15px;">
                            <button
                                class="grid-btn blue"
                                style="flex:1;"
                                on:click={() => {
                                    // Backward compatibility: keep legacy string arrays synced for older clients, ignoring them in main logic
                                    try {
                                        const vols = appSettings.customRegexRules.filter(r => r.level === 1).map(r => `(${r.pattern})`);
                                        const chaps = appSettings.customRegexRules.filter(r => r.level >= 2).map(r => `(${r.pattern})`);
                                        appSettings.volRegex = vols.length > 0 ? vols.join("|") : "^$"; 
                                        appSettings.chapRegex = chaps.length > 0 ? chaps.join("|") : "^$";
                                    } catch (e) {}

                                    localStorage.setItem(
                                        "app-settings",
                                        JSON.stringify(appSettings),
                                    );
                                    closeAllPanels();
                                    scanToc();
                                }}>保存并应用</button
                            >
                            <button
                                class="grid-btn"
                                style="flex:1;"
                                on:click={async () => {
                                    historyList = await invoke(
                                        "get_history_list",
                                        {
                                            originalPath: filePath,
                                        },
                                    );
                                    showHistoryPanel = true;
                                    showSettingsPanel = false;
                                }}>历史版本</button
                            >
                        </div>
                    </div>
                {:else if showEpubModal}
                    <div class="p-header">
                        <span>制作 EPUB</span>
                        <button class="icon-close" on:click={closeAllPanels}>✕</button>
                    </div>
                    <div class="p-body epub-modal-body">
                        <div class="epub-main-layout">
                            <!-- 左侧：主要信息 -->
                            <div class="epub-fields-column">
                                <div class="set-row compact">
                                    <label for="et">书名:</label>
                                    <input id="et" type="text" bind:value={epubMeta.title} class="epub-input-small" />
                                </div>
                                <div class="set-row compact">
                                    <label for="ec">作者:</label>
                                    <input id="ec" type="text" bind:value={epubMeta.creator} class="epub-input-small" />
                                </div>
                                <div class="set-row compact align-start">
                                    <label for="ed">简介:</label>
                                    <textarea
                                        id="ed"
                                        rows="6"
                                        bind:value={epubMeta.description}
                                        class="epub-textarea"
                                        placeholder="请输入书籍简介..."
                                    ></textarea>
                                </div>
                            </div>

                            <!-- 右侧：封面预览 -->
                            <div class="epub-cover-column">
                                <div
                                    class="epub-cover-preview"
                                    on:click={async () => {
                                        const s = await open({
                                            filters: [{ name: "Image", extensions: ["jpg", "png", "jpeg", "webp"] }],
                                        });
                                        if (s) {
                                            epubMeta.cover_path = extractPickedPath(s as string | string[]);
                                            await loadCoverPreview();
                                        }
                                    }}
                                    role="button"
                                    tabindex="0"
                                    on:keydown={(e) => e.key === 'Enter' && (e.target as HTMLElement).click()}
                                >
                                    {#if coverPreviewUrl}
                                        <img src={coverPreviewUrl} alt="封面预检" />
                                        <div class="cover-hint">点击更换封面</div>
                                    {:else if epubMeta.cover_path}
                                        <div class="no-cover">
                                            <span>⏳</span>
                                            <span>加载中...</span>
                                        </div>
                                    {:else}
                                        <div class="no-cover">
                                            <span>➕</span>
                                            <span>添加封面</span>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>

                        <div class="epub-modal-footer">
                            <button class="epub-cancel" on:click={openAdvancedEpubMetadata}>
                                高级选项
                            </button>
                            <button
                                class="epub-confirm"
                                disabled={epubGenerationStatus === "generating"}
                                on:click={generateEpub}
                            >
                                {epubGenerationStatus === "generating"
                                    ? "制作中..."
                                    : "开始制作"}
                            </button>
                        </div>
                    </div>
                {:else if showHistoryPanel}
                    <div class="p-header">
                        <div style="display:flex; align-items:center;">
                            <button
                                class="icon-close"
                                style="font-size:18px; margin-right:8px; transform:rotate(180deg);"
                                on:click={() => {
                                    showHistoryPanel = false;
                                    showSettingsPanel = true;
                                }}>➜</button
                            >
                            <span>历史版本</span>
                        </div>
                        <button class="icon-close" on:click={closeAllPanels}
                            >✕</button
                        >
                    </div>
                    <div class="p-body scroll-p">
                        {#each historyList as h}
                            <button
                                class="hist-item"
                                on:click={() => {
                                    restoreTargetSnapshot = h;
                                    showRestoreConfirm = true;
                                }}
                            >
                                <span
                                    >{new Date(
                                        h.timestamp * 1000,
                                    ).toLocaleString()}</span
                                >
                                <span>{(h.size / 1024).toFixed(1)}KB</span>
                            </button>
                        {:else}
                            <div class="empty-msg">暂无历史快照</div>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>
    {/if}

    <!-- 历史回退确认弹窗 -->
    {#if showRestoreConfirm}
        <div
            role="presentation"
            class="modal-overlay"
            on:click={() => {
                showRestoreConfirm = false;
                restoreTargetSnapshot = null;
            }}
        >
            <div
                role="presentation"
                class="modal-content"
                style="max-width: 400px; padding: 30px; text-align: center;"
                on:click|stopPropagation
            >
                <div
                    style="font-size: 18px; margin-bottom: 20px; font-weight: bold;"
                >
                    确认回退到历史版本？
                </div>
                <div style="color: #666; margin-bottom: 30px; line-height:1.6;">
                    当前版本将自动保存为新的历史记录。<br />
                    此操作可以再次回退。
                </div>
                <div style="display: flex; gap: 12px; justify-content: center;">
                    <button
                        class="btn-small"
                        style="flex: 1; max-width: 120px;"
                        on:click={() => {
                            showRestoreConfirm = false;
                            restoreTargetSnapshot = null;
                        }}
                    >
                        取消
                    </button>
                    <button
                        class="btn-small"
                        style="flex: 1; max-width: 120px; background: linear-gradient(135deg, #0066b8, #0088dd); color: white; border: none;"
                        on:click={confirmRestore}
                    >
                        确认回退
                    </button>
                </div>
            </div>
        </div>
    {/if}

    {#if showCheckPanel}
        <div
            class="check-panel"
            style="left: {checkPanelPos.x}px; top: {checkPanelPos.y}px;"
        >
            <!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
            <div
                class="find-header"
                on:mousedown={(e) => startDrag(e, "check")}
                role="application"
                aria-label="拖拽以移动内容检查面板"
            >
                <span class="find-title">全书检查</span>
                <button
                    class="icon-close"
                    on:click={() => (showCheckPanel = false)}>✕</button
                >
            </div>
            <div
                class="find-body scroll-p"
                style="max-height: 400px; overflow-y: auto;"
            >
                <!-- 章节跳转连贯性 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.seq = !checkCollapseState.seq)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.seq = !checkCollapseState.seq)}
                    >
                        <span>{checkCollapseState.seq ? "▶" : "▼"} 断序检查 ({sequenceErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.seq}
                        <div class="tag-list">
                            {#each sequenceErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >
                                    <span class="err-tag-msg">{e.msg}</span>
                                    <span class="err-tag-title">{e.title}</span>
                                </button>
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 标题空 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.title = !checkCollapseState.title)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.title = !checkCollapseState.title)}
                    >
                        <span>{checkCollapseState.title ? "▶" : "▼"} 标题内容 ({titleErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.title}
                        <div class="tag-list">
                            {#each titleErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >{e.title}</button>
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 字数 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.word = !checkCollapseState.word)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.word = !checkCollapseState.word)}
                    >
                        <span>{checkCollapseState.word ? "▶" : "▼"} 字数检查 ({wordCountErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.word}
                        <div class="tag-list">
                            {#each wordCountErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >{e.title} ({e.val})</button
                                >
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</main>

<!-- Context Menu -->
<ContextMenu />

{#if showCloseDialog}
    <div class="dialog-overlay">
        <div class="dialog">
            <div class="dialog-header">未保存的更改</div>
            <div class="dialog-content">
                当前文件包含未保存的更改，是否保存并退出？
            </div>
            <div class="dialog-actions">
                <!-- 假设 saveFile 已存在 -->
                <button
                    class="btn primary"
                    on:click={handleDialogSave}
                    disabled={isDialogSaving}
                >
                    {isDialogSaving ? "保存中..." : "保存"}
                </button>
                <button
                    class="btn danger"
                    on:click={handleDialogDiscard}
                    disabled={isDialogSaving}>不保存</button
                >
                <button
                    class="btn secondary"
                    on:click={handleDialogCancel}
                    disabled={isDialogSaving}>取消</button
                >
            </div>
        </div>
    </div>
{/if}

<style>
    /* Dialog Styles (Matched with Epub Editor) */
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
        z-index: 2000; /* High z-index */
    }

    .dialog {
        background: white;
        padding: 20px;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        min-width: 300px;
        color: #333;
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
        font-size: 14px;
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



    .set-row {
        display: flex;
        align-items: center;
        margin-bottom: 12px;
        gap: 12px;
    }
    .set-row label {
        color: #444;
        font-size: 14px;
    }
    .set-row input[type="checkbox"] {
        width: 16px;
        height: 16px;
        cursor: pointer;
    }

    :global(body) {
        margin: 0;
        background: #fff;
        overflow: hidden;
        -webkit-touch-callout: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        user-select: none;

        font-family: system-ui;
    }
    .app-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
    }
    .toolbar {
        padding-top: env(safe-area-inset-top);
        background: #f3f3f3;
        height: 44px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-left: 10px;
        padding-right: 10px;
        border-bottom: 1px solid #ddd;
        z-index: 100;
    }
    .btn-group {
        display: flex;
        gap: 6px;
    }
    button {
        height: 34px;
        min-width: 40px;
        border-radius: 6px;
        border: 1px solid #ccc;
        background: #fff;
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: #eee;
        transform: scale(0.96);
    }
    .btn-primary {
        background: #0066b8;
        color: #fff;
        border: none;
    }
    .btn-save-modified {
        background: #d32f2f;
        color: #fff;
        border: none;
        animation: pulse 2s infinite;
    }
    @keyframes pulse {
        0% {
            opacity: 1;
        }
        50% {
            opacity: 0.7;
        }
        100% {
            opacity: 1;
        }
    }

    .main-body {
        flex: 1;
        display: flex;
        overflow: hidden;
        position: relative;
    }
    .sidebar {
        width: 280px;
        background: #f8f8f8;
        border-right: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
    }

    .sidebar-header-fixed {
        background: #eee;
        border-bottom: 1px solid #ddd;
        flex-shrink: 0;
        z-index: 20;
    }
    .sidebar-header-row {
        padding: 10px;
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        font-weight: bold;
        align-items: center;
    }
    .header-btns {
        display: flex;
        gap: 5px;
    }
    .icon-btn {
        width: 26px;
        height: 26px;
        padding: 0;
        font-size: 14px;
        border: 1px solid #ccc;
        background: #fff;
        cursor: pointer;
        border-radius: 4px;
    }

    .toc-list {
        flex: 1;
        overflow-y: auto;
    }
    .toc-item {
        padding: 12px;
        font-size: 14px;
        border-bottom: 1px solid #eee;
        display: flex;
        /* justify-content: space-between; Removed to fix centering issue */
        align-items: center;
        cursor: pointer;
        cursor: pointer;
        position: relative; /* Fix z-index stacking */
        z-index: 1;
    }
    .indent {
        padding-left: 28px;
        background: #fafafa;
    }
    .toc-item.active {
        background: #d4e8fa;
        color: #0066b8;
        border-left: 4px solid #0066b8;
        font-weight: bold;
    }
    /* 卷标吸顶 */
    .vol-title {
        background: #eaeaea;
        font-weight: bold;
        position: sticky;
        top: 0;
        z-index: 10;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    }
    .text-error {
        color: #d32f2f;
        font-weight: bold;
    }
    .toc-count {
        color: #999;
        font-size: 11px;
        margin-left: auto; /* Push to right */
    }
    .arrow {
        font-size: 10px;
        margin-right: 8px;
        color: #888;
        width: 12px;
        display: inline-block;
    }
    .mini-btn {
        font-size: 11px;
        height: 26px;
        padding: 0 10px;
        border-radius: 4px;
        border: 1px solid #ccc;
        background: #fff;
    }
    .mini-btn.active {
        background: #0066b8;
        color: #fff;
    }

    .editor-wrapper {
        flex: 1;
        overflow: hidden;
        position: relative;
    }
    .loading {
        position: absolute;
        inset: 0;
        background: rgba(255, 255, 255, 0.8);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 50;
    }

    .epub-textarea {
        flex: 1;
        padding: 8px;
        background: #fdfdfd;
        border: 1px solid #ddd;
        border-radius: 4px;
        resize: vertical;
        font-family: inherit;
        font-size: 13px;
        line-height: 1.6;
        /* text-indent: 2em; 移除强制缩进，使用原文缩进 */
    }

    .p-header {
        width: 100%;
        box-sizing: border-box;
        padding: 10px 15px;
        background: #f8fafc;
        border-bottom: 1px solid #eceff1;
        display: flex;
        justify-content: space-between;
        align-items: center;
        user-select: none;
    }
    .p-body {
        padding: 15px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .set-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 15px;
        gap: 10px;
    }
    .set-row label {
        width: 110px;
        flex-shrink: 0;
        font-weight: bold;
        color: #444;
    }
    .set-row input {
        flex: 1;
        padding: 8px;
        border: 1px solid #ddd;
        border-radius: 6px;
        font-size: 15px;
        background: #fff;
    }
    .modal-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
        padding: 20px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }
    .modal-content {
        background: #fff;
        width: 100%;
        max-width: 520px;
        border-radius: 20px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }

    /* 偏好设置面板增强样式 */
    .settings-tabs {
        display: flex;
        gap: 15px;
        padding: 5px 20px 10px;
        border-bottom: 1px solid #eee;
    }
    .settings-tabs .tab-btn {
        background: none;
        border: none;
        border-radius: 6px;
        padding: 6px 14px;
        font-size: 15px;
        font-weight: bold;
        color: #777;
        cursor: pointer;
        transition: 0.2s;
        height: auto;
        min-width: 0;
    }
    .settings-tabs .tab-btn:hover {
        background: #f0f0f0;
    }
    .settings-tabs .tab-btn.active {
        color: #0066b8;
        background: #e3f2fd;
    }

    .rules-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .rule-item {
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .rule-type {
        width: 85px;
        height: 32px;
        padding: 0 5px;
        border: 1px solid #ccc;
        border-radius: 6px;
        font-size: 13px;
        background: #fff;
    }
    .rule-input-group {
        display: flex;
        flex: 1;
        align-items: center;
        position: relative;
    }
    .rule-input {
        flex: 1;
        height: 32px;
        padding: 0 10px;
        border: 1px solid #ccc;
        border-right: none;
        border-radius: 6px 0 0 6px;
        font-size: 13px;
        background: #fff;
        min-width: 0;
        z-index: 1;
    }
    .rule-arrow-visual {
        width: 32px;
        height: 32px;
        border: 1px solid #ccc;
        border-radius: 0 6px 6px 0;
        background: #f9f9f9;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #666;
        font-size: 10px;
        flex-shrink: 0;
    }
    .rule-hidden-select {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        opacity: 0;
        appearance: none;
        -webkit-appearance: none;
        cursor: pointer;
        z-index: 2;
        clip-path: inset(0 0 0 calc(100% - 32px));
    }
    .rule-btn {
        width: 32px;
        height: 32px;
        min-width: 32px;
        border: 1px solid #ccc;
        border-radius: 6px;
        background: #fff;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: 0.1s;
    }
    .rule-btn:hover {
        background: #fee;
        color: #d32f2f;
        border-color: #ffcdd2;
    }
    .rules-header {
        font-size: 13px;
        font-weight: bold;
        color: #666;
        margin-bottom: 5px;
    }

    /* EPUB 制作面板重构样式 */
    .epub-modal-body {
        max-width: 680px !important;
        font-size: 13px;
        color: #444;
    }

    .epub-main-layout {
        display: flex;
        gap: 24px;
        margin-bottom: 20px;
    }

    .epub-fields-column {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .epub-cover-column {
        width: 160px;
        flex-shrink: 0;
    }

    .set-row.compact {
        margin-bottom: 0;
        gap: 12px;
        align-items: center;
    }
    .set-row.align-start {
        align-items: flex-start !important;
    }
    .set-row.align-start label {
        margin-top: 10px !important;
    }

    .set-row.compact label {
        width: 50px;
        font-weight: 500;
        color: #666;
        font-size: 13px;
        margin: 0;
    }

    .epub-input-small {
        height: 32px !important;
        font-size: 13px !important;
        padding: 0 10px !important;
        border: 1px solid #ddd;
        border-radius: 4px;
        width: 100%;
    }

    .epub-cover-preview {
        width: 100%;
        height: 220px;
        border: 2px dashed #eee;
        border-radius: 8px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        overflow: hidden;
        position: relative;
        background: #fafafa;
        transition: all 0.2s;
    }

    .epub-cover-preview:hover {
        border-color: #0088dd;
        background: #f0f8ff;
    }

    .epub-cover-preview img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .epub-cover-preview .no-cover {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        color: #aaa;
    }

    .epub-cover-preview .no-cover span:first-child {
        font-size: 24px;
    }

    .cover-hint {
        position: absolute;
        bottom: 0;
        width: 100%;
        background: rgba(0,0,0,0.5);
        color: white;
        font-size: 11px;
        padding: 4px 0;
        text-align: center;
        opacity: 0;
        transition: opacity 0.2s;
    }

    .epub-cover-preview:hover .cover-hint {
        opacity: 1;
    }

    .epub-modal-footer {
        display: flex;
        gap: 12px;
        margin-top: 10px;
    }

    .epub-modal-footer button {
        flex: 1;
        height: 40px;
        font-size: 14px;
    }

    /* 检查面板样式 */
    .check-panel {
        position: fixed;
        width: 300px;
        background: rgba(255, 255, 255, 0.95);
        backdrop-filter: blur(8px);
        border: 1px solid rgba(0, 0, 0, 0.1);
        border-radius: 12px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
        z-index: 1000;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }
    .check-sec {
        padding: 8px 12px;
        border-bottom: 1px solid #eee;
    }
    .check-sec:last-child {
        border-bottom: none;
    }
    .sec-title {
        font-size: 13px;
        font-weight: bold;
        color: #444;
        cursor: pointer;
        padding: 4px 0;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .tag-list {
        display: flex;
        flex-direction: column;
        gap: 6px;
        padding: 8px 0;
    }
    .err-tag {
        background: #fff5f5;
        border: 1px solid #ffcdd2;
        border-radius: 6px;
        padding: 6px 10px;
        font-size: 12px;
        color: #d32f2f;
        cursor: pointer;
        text-align: left;
        line-height: 1.4;
        transition: 0.2s;
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 8px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        justify-content: flex-start;
    }
    .err-tag:hover {
        background: #ffebee;
        border-color: #ef5350;
    }
    .err-tag-title {
        font-weight: bold;
    }
    .err-tag-msg {
        font-size: 11px;
        color: #f44336;
        font-weight: bold;
        font-family: monospace;
    }
    
    .find-header {
        padding: 10px 14px;
        background: #f8fafc;
        border-bottom: 1px solid #eee;
        display: flex;
        justify-content: space-between;
        align-items: center;
        cursor: move;
    }
    .find-title {
        font-size: 14px;
        font-weight: bold;
        color: #333;
    }
    .icon-close {
        background: none;
        border: none;
        font-size: 16px;
        color: #999;
        cursor: pointer;
        padding: 4px;
        min-width: 0;
    }
    .icon-close:hover {
        color: #f44336;
    }
    .scroll-p::-webkit-scrollbar {
        width: 6px;
    }
    .scroll-p::-webkit-scrollbar-thumb {
        background: #ddd;
        border-radius: 3px;
    }
</style>
