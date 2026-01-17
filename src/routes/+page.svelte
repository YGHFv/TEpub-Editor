<script lang="ts">
    import { onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { open, save, message, ask } from "@tauri-apps/plugin-dialog";
    import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import Editor from "$lib/Editor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";

    // --- [1. 完整的接口定义] ---
    interface RawChapter {
        title: string;
        line_number: number;
        toc_type: "Volume" | "Chapter" | "Meta";
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

    // --- [2. 默认配置 (三大正则回归)] ---
    const DEFAULT_SETTINGS = {
        volRegex: "^\\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*",
        chapRegex:
            "^\\s*(第[一二三四五六七八九十百千万0-9]+[章回]|Chapter\\s*\\d+).*",
        metaRegex: "^\\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*", // 之前丢失的
        wordCountThreshold: 8000,
        clearHistoryOnSave: false,
    };

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
    let showFindReplace = false;
    let showSettingsPanel = false;
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
        publisher: "出版社",
        date: new Date().toISOString().split("T")[0],
        uuid: crypto.randomUUID(),
        md5: "",
        cover_path: "",
    };
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
    let isCheckModeOn = false;
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
        let unlisten: any;

        const init = async () => {
            // 1. 移动端检测
            if (window.innerWidth < 768) {
                isMobile = true;
                showSidebar = false;
            }

            // 2. 读取设置
            const stored = localStorage.getItem("app-settings");
            if (stored)
                try {
                    appSettings = {
                        ...DEFAULT_SETTINGS,
                        ...JSON.parse(stored),
                    };
                } catch (e) {}

            // 3. 崩溃恢复逻辑 (完整保留)
            const savedState = localStorage.getItem("app-crash-recovery");
            if (savedState) {
                try {
                    const state = JSON.parse(savedState);
                    if (
                        state.filePath &&
                        state.filePath !== "请打开一本小说..."
                    ) {
                        filePath = state.filePath;
                        // 只有当有未保存内容时才恢复 content，否则读文件
                        if (state.isModified && state.content) {
                            fileContent = state.content;
                            isModified = true;
                        } else {
                            try {
                                fileContent = await readTextFile(filePath);
                            } catch (e) {}
                        }

                        if (fileContent) {
                            await tick(); // 等待编辑器挂载
                            editorComponent?.resetDoc(fileContent);
                            await scanToc(fileContent);
                            updateMd5(fileContent);
                            // 恢复滚动位置
                            if (state.scrollLine)
                                setTimeout(
                                    () =>
                                        editorComponent?.scrollToLine(
                                            state.scrollLine,
                                        ),
                                    200,
                                );
                        }
                    }
                } catch (e) {}
            }
            setTimeout(() => {
                hasInitialized = true;
            }, 500);

            // 4. 防止误触退出
            const appWindow = getCurrentWindow();
            unlisten = await appWindow.onCloseRequested(async (event) => {
                if (isModified) {
                    event.preventDefault();
                    const confirmed = await ask(
                        "当前文件有未保存的修改，确定要退出吗？",
                        { title: "未保存警告", kind: "warning" },
                    );
                    if (confirmed) {
                        localStorage.removeItem("app-crash-recovery");
                        await invoke("exit_app");
                    }
                } else {
                    await invoke("exit_app");
                }
            });
        };

        init();

        return () => {
            if (unlisten) unlisten();
        };
    });

    // --- [4. 核心逻辑实现] ---

    async function updateMd5(content: string) {
        try {
            epubMeta.md5 = await invoke("calculate_md5", { content });
        } catch (e) {}
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

    async function selectFile() {
        try {
            const selected = await open({
                multiple: false,
                filters: [{ name: "Text", extensions: ["txt", "md"] }],
            });
            if (selected) {
                isLoading = true;
                isLoadingFile = true;
                filePath = selected as string;

                // 自动填充 EPUB 书名
                const basename =
                    filePath
                        .split(/[\\/]/)
                        .pop()
                        ?.replace(/\.[^/.]+$/, "") || "未命名";
                epubMeta.title = basename;

                const content = await readTextFile(filePath);
                fileContent = content;

                editorComponent?.resetDoc(content);
                isModified = false;
                updateMd5(content);
                await scanToc(content);

                isLoading = false;
                localStorage.removeItem("app-crash-recovery");
                setTimeout(() => {
                    isLoadingFile = false;
                }, 100);
            }
        } catch (e) {
            isLoading = false;
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
            await writeTextFile(filePath, fileContent);
            // 调用后端保存历史
            await invoke("save_history", {
                originalPath: filePath,
                content: fileContent,
            }).catch(() => {});

            isModified = false;
            saveStateToCache(0); // 保存成功后更新缓存状态
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
                volreg: appSettings.volRegex,
                chapreg: appSettings.chapRegex,
                metareg: appSettings.metaRegex,
            });

            const tree: TocNode[] = [];
            flatToc = [];
            let curVol: TocNode | null = null;
            let uid = 0;

            // 构建嵌套树
            for (const item of rawList) {
                const node: TocNode = {
                    id: `n-${uid++}`,
                    title: item.title,
                    line_number: item.line_number,
                    type: item.toc_type,
                    word_count: item.word_count,
                    children: [],
                    expanded: true,
                };

                // 压平数组用于滚动查找
                const flatNode: FlatNode = {
                    id: node.id,
                    line: node.line_number,
                    title: node.title,
                    type: node.type,
                    word_count: node.word_count,
                };

                if (item.toc_type === "Volume") {
                    curVol = node;
                    tree.push(node);
                    flatToc.push(flatNode);
                } else if (item.toc_type === "Chapter" && curVol) {
                    node.parentId = curVol.id;
                    curVol.children.push(node);
                    flatNode.parentId = curVol.id;
                    flatToc.push(flatNode);
                } else {
                    tree.push(node);
                    flatToc.push(flatNode);
                }
            }
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

    // 编辑器滚动时触发：高亮侧边栏
    async function handleScroll(line: number) {
        saveStateToCache(line);
        if (flatToc.length === 0) return;
        if (isNavigating) return; // 正在手动跳转，忽略滚动监听

        // 二分查找或倒序查找当前章节
        let found: FlatNode | null = null;
        // Editor 现在传递的是【屏幕中心】的行号，所以直接比较即可
        for (let i = flatToc.length - 1; i >= 0; i--) {
            if (flatToc[i].line <= line) {
                found = flatToc[i];
                break;
            }
        }

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
            if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
        }
    }

    // 处理搜索/选择时的目录同步（绕过导航锁）
    async function handleSelectionChange(line: number) {
        if (isNavigating) return;
        // 这里也可以加少量防抖
        handleScroll(line);
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

        // 3. 立即更新高亮
        activeChapterId = id;

        // 4. 执行滚动
        if (editorComponent) {
            editorComponent.scrollToLine(line);
        } else {
            console.error("Editor component not ready");
        }

        // 5. 手动滚动侧边栏（因为 handleScroll 被锁住了）
        requestAnimationFrame(() => {
            const el = document.getElementById(`toc-${id}`);
            if (el) {
                console.log("handleChapterClick: scrolling sidebar to", id);
                el.scrollIntoView({ behavior: "smooth", block: "center" });
            } else {
                console.warn("handleChapterClick: TOC element not found", id);
            }
        });

        // 6. 设置解锁定时器
        scrollTimeout = setTimeout(() => {
            isNavigating = false;
            scrollTimeout = null;
        }, 600);
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

    function runFullCheck() {
        sequenceErrors = [];
        wordCountErrors = [];
        titleErrors = [];
        invalidSequenceIds.clear();
        let lastNum = -1;
        for (const node of flatToc) {
            if (node.type === "Chapter") {
                const num = parseInt(node.title.match(/\d+/)?.[0] || "-1");
                if (num !== -1) {
                    if (lastNum !== -1 && num !== lastNum + 1) {
                        invalidSequenceIds.add(node.id);
                        sequenceErrors.push({
                            id: node.id,
                            title: node.title,
                            line: node.line,
                            msg: `跳跃: ${lastNum}->${num}`,
                            val: num,
                        });
                    }
                    lastNum = num;
                }

                // 空标题检查: 仅包含数字、序号，没有具体内容
                // 匹配 "第xxx章" 后仅有空白
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
                volreg: appSettings.volRegex,
                chapreg: appSettings.chapRegex,
                metareg: appSettings.metaRegex,
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
                metadata: epubMeta,
            });
            // 制作成功：设置状态为成功，不显示弹窗
            epubGenerationStatus = "success";
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
            fileContent = await readTextFile(restoreTargetSnapshot.path);
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
        showFindReplace = false;
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

<ContextMenu />

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
                    showEpubModal = true;
                    updateMd5(fileContent);
                    // 确保书名已填充
                    if (
                        epubMeta.title === "书名" &&
                        filePath !== "请打开一本小说..."
                    ) {
                        const basename =
                            filePath
                                .split(/[\\/]/)
                                .pop()
                                ?.replace(/\.[^/.]+$/, "") || "未命名";
                        epubMeta.title = basename;
                    }
                    // 重置EPUB制作状态
                    epubGenerationStatus = "idle";
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
            on:click={() => {
                closeAllPanels();
                showFindReplace = !showFindReplace;
                // 重置位置到默认(如果未初始化)
                if (findPanelPos.x === 0 && findPanelPos.y === 0) {
                    findPanelPos = { x: window.innerWidth - 340, y: 60 };
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
                                    tocTree.forEach(
                                        (n) => (n.expanded = !n.expanded),
                                    );
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
                            <span class="toc-count">{node.word_count}</span>
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
                                            : ''}">{child.title}</span
                                    >
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
            />
        </section>
    </div>

    {#if showFindReplace}
        <div
            class="find-panel"
            style="left: {findPanelPos.x}px; top: {findPanelPos.y}px;"
            on:mousedown={(e) => startDrag(e, "find")}
        >
            <div class="find-header">
                <span class="drag-title">查找与替换 (可拖拽)</span>
                <button
                    class="icon-close"
                    on:click={() => (showFindReplace = false)}>✕</button
                >
            </div>
            <div class="find-body">
                <div class="find-grid">
                    <div class="input-group">
                        <input
                            type="text"
                            bind:value={findPattern}
                            placeholder="查找..."
                            on:keydown={(e) =>
                                e.key === "Enter" && performFind()}
                        />
                        <label class="regex-tag"
                            ><input
                                type="checkbox"
                                bind:checked={isRegex}
                            />.*</label
                        >
                    </div>
                    <div class="input-group">
                        <input
                            type="text"
                            bind:value={replacePattern}
                            placeholder="替换为..."
                        />
                    </div>
                </div>

                <div class="msg-bar-compact">{replaceMsg || " "}</div>

                <div class="action-bar">
                    <div
                        class="nav-btns"
                        style="flex: 1; display:flex; gap:8px"
                    >
                        <button
                            class="btn-small"
                            style="flex:1"
                            on:click={findPrev}>↑ 向上查找</button
                        >
                        <button
                            class="btn-small"
                            style="flex:1"
                            on:click={findNext}>↓ 向下查找</button
                        >
                    </div>
                </div>
                <div class="action-bar" style="margin-top:8px">
                    <div class="op-btns" style="flex: 1; display:flex; gap:8px">
                        <button
                            class="btn-small"
                            style="flex:1"
                            on:click={() =>
                                editorComponent.replaceSelection(
                                    replacePattern,
                                )}>替换</button
                        >
                        <button
                            class="btn-small btn-dang"
                            style="flex:1"
                            on:click={performReplaceAll}>全部替换</button
                        >
                    </div>
                </div>
            </div>
        </div>
    {/if}

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
                    <div class="p-body">
                        <div class="set-row">
                            <label for="vreg">卷正则:</label><input
                                id="vreg"
                                type="text"
                                bind:value={appSettings.volRegex}
                            />
                        </div>
                        <div class="set-row">
                            <label for="creg">章正则:</label><input
                                id="creg"
                                type="text"
                                bind:value={appSettings.chapRegex}
                            />
                        </div>
                        <div class="set-row">
                            <label for="mreg">Meta正则:</label><input
                                id="mreg"
                                type="text"
                                bind:value={appSettings.metaRegex}
                            />
                        </div>
                        <!-- 合并：字数阈值 和 撤销开关 -->
                        <div class="set-row">
                            <label for="wth">字数阈值:</label>
                            <input
                                id="wth"
                                type="number"
                                bind:value={appSettings.wordCountThreshold}
                                style="flex:1"
                            />

                            <div
                                style="display:flex; align-items:center; margin-left:10px; flex-shrink:0;"
                            >
                                <label
                                    for="clh"
                                    style="width:auto; margin-right:5px; font-weight:normal;"
                                    >保存清空撤销</label
                                >
                                <input
                                    id="clh"
                                    type="checkbox"
                                    bind:checked={
                                        appSettings.clearHistoryOnSave
                                    }
                                    style="width:auto !important; margin:0;"
                                />
                            </div>
                        </div>

                        <!-- 底部按钮：放在一行 -->
                        <div style="display:flex; gap:10px; margin-top:10px;">
                            <button
                                class="grid-btn blue"
                                style="flex:1;"
                                on:click={() => {
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
                        <button class="icon-close" on:click={closeAllPanels}
                            >✕</button
                        >
                    </div>
                    <div class="p-body">
                        <div class="set-row">
                            <label for="et">书名:</label><input
                                id="et"
                                type="text"
                                bind:value={epubMeta.title}
                            />
                        </div>
                        <div class="set-row">
                            <label for="ec">作者:</label><input
                                id="ec"
                                type="text"
                                bind:value={epubMeta.creator}
                            />
                        </div>
                        <div class="set-row">
                            <label for="ep">出版社:</label><input
                                id="ep"
                                type="text"
                                bind:value={epubMeta.publisher}
                            />
                        </div>
                        <div class="set-row">
                            <label>UUID:</label><input
                                type="text"
                                value={epubMeta.uuid}
                                readonly
                                style="font-size:10px; background:#f5f5f5"
                            />
                        </div>
                        <div class="set-row">
                            <label>MD5:</label><input
                                type="text"
                                value={epubMeta.md5}
                                readonly
                                style="font-size:10px; background:#f5f5f5"
                            />
                        </div>
                        <div class="set-row">
                            <label>封面:</label><button
                                class="mini-btn"
                                on:click={async () => {
                                    const s = await open({
                                        filters: [
                                            {
                                                name: "Image",
                                                extensions: ["jpg", "png"],
                                            },
                                        ],
                                    });
                                    if (s) epubMeta.cover_path = s as string;
                                }}
                                >{epubMeta.cover_path
                                    ? "已选"
                                    : "选择图片"}</button
                            >
                        </div>
                        {#if epubGenerationStatus === "idle"}
                            <button
                                class="grid-btn blue full-row"
                                style="height:44px; margin-top:10px;"
                                on:click={generateEpub}>开始生成</button
                            >
                        {:else if epubGenerationStatus === "generating"}
                            <button
                                class="grid-btn full-row"
                                disabled
                                style="height:44px; margin-top:10px; opacity:0.6; cursor:not-allowed;"
                                >正在制作...</button
                            >
                        {:else if epubGenerationStatus === "success"}
                            <button
                                class="grid-btn epub-success full-row"
                                style="height:44px; margin-top:10px;"
                                on:click={() => {
                                    showEpubModal = false;
                                    epubGenerationStatus = "idle";
                                }}>制作完成 ✓</button
                            >
                        {/if}
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
            on:mousedown={(e) => startDrag(e, "check")}
        >
            <div class="find-header">
                <span class="drag-title">内容检查 (可拖拽)</span>
                <button
                    class="icon-close"
                    on:click={() => (showCheckPanel = false)}>✕</button
                >
            </div>
            <div
                class="find-body scroll-p"
                style="max-height: 400px; overflow-y: auto;"
            >
                <!-- 断序 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        on:click={() =>
                            (checkCollapseState.seq = !checkCollapseState.seq)}
                    >
                        <span
                            >{checkCollapseState.seq ? "▶" : "▼"} 断序章节 ({sequenceErrors.length})</span
                        >
                    </div>
                    {#if !checkCollapseState.seq}
                        <div class="tag-list">
                            {#each sequenceErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() =>
                                        handleChapterClick(e.id, e.line)}
                                    >{e.title} ({e.msg})</button
                                >
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 标题空 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        on:click={() =>
                            (checkCollapseState.title =
                                !checkCollapseState.title)}
                    >
                        <span
                            >{checkCollapseState.title ? "▶" : "▼"} 标题空内容 ({titleErrors.length})</span
                        >
                    </div>
                    {#if !checkCollapseState.title}
                        <div class="tag-list">
                            {#each titleErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() =>
                                        handleChapterClick(e.id, e.line)}
                                    >{e.title}</button
                                >
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 字数 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        on:click={() =>
                            (checkCollapseState.word =
                                !checkCollapseState.word)}
                    >
                        <span
                            >{checkCollapseState.word ? "▶" : "▼"} 字数超标 ({wordCountErrors.length})</span
                        >
                    </div>
                    {#if !checkCollapseState.word}
                        <div class="tag-list">
                            {#each wordCountErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() =>
                                        handleChapterClick(e.id, e.line)}
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

<style>
    :global(body) {
        margin: 0;
        background: #fff;
        overflow: hidden;
        -webkit-touch-callout: none;
        -webkit-user-select: none;
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

    /* 查找面板 - 紧凑型设计 */
    .find-panel {
        position: fixed;
        background: #fff;
        border: 1px solid #ccc;
        box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
        border-radius: 8px;
        width: 300px;
        z-index: 1000;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        font-size: 13px;
    }
    .find-header {
        background: #f5f5f5;
        padding: 8px 12px;
        cursor: move;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid #ddd;
        user-select: none;
    }

    .check-panel {
        position: fixed;
        background: #fff;
        border: 1px solid #ccc;
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
        border-radius: 8px;
        width: 320px;
        z-index: 1100; /* Higher than find panel */
        display: flex;
        flex-direction: column;
        font-size: 13px;
        max-height: 80vh;
        overflow: hidden;
    }
    .check-sec {
        margin-bottom: 10px;
        border-bottom: 1px dashed #eee;
        padding-bottom: 5px;
    }
    .sec-title {
        font-weight: bold;
        margin-bottom: 5px;
        cursor: pointer;
        user-select: none;
        background: #fafafa;
        padding: 4px;
        border-radius: 4px;
    }
    .sec-title:hover {
        background: #f0f0f0;
    }
    .tag-list {
        display: flex;
        flex-wrap: wrap;
        gap: 5px;
    }
    .err-tag {
        border: none;
        background: #fff3e0;
        color: #e65100;
        font-size: 11px;
        padding: 2px 6px;
        border-radius: 4px;
        cursor: pointer;
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .err-tag:hover {
        background: #ffe0b2;
    }
    .drag-title {
        font-weight: bold;
        color: #555;
        font-size: 12px;
    }
    .icon-close {
        background: none;
        border: none;
        font-size: 16px;
        width: 20px;
        min-width: unset; /* Override global button min-width */
        height: 20px;
        padding: 0;
        line-height: 1;
        color: #888;
        cursor: pointer;
    }
    .icon-close:hover {
        color: #d32f2f;
    }

    .find-body {
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .find-grid {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .input-group {
        display: flex;
        align-items: center;
        border: 1px solid #ddd;
        border-radius: 4px;
        overflow: hidden;
        height: 28px;
    }
    .input-group input[type="text"] {
        flex: 1;
        border: none;
        padding: 4px 8px;
        outline: none;
        font-size: 13px;
        height: 100%;
    }
    .regex-tag {
        background: #eee;
        padding: 0 6px;
        border-left: 1px solid #ddd;
        display: flex;
        align-items: center;
        gap: 4px;
        font-size: 11px;
        height: 100%;
        color: #666;
        cursor: pointer;
    }

    .msg-bar-compact {
        height: 16px;
        font-size: 11px;
        color: #e65100;
        text-align: right;
    }

    .action-bar {
        display: flex;
        justify-content: space-between;
        gap: 8px;
    }
    .nav-btns {
        display: flex;
        gap: 4px;
    }
    .nav-btns button {
        width: 28px;
        height: 28px;
        padding: 0;
        border: 1px solid #ddd;
        border-radius: 4px;
        background: #fff;
        cursor: pointer;
    }
    .nav-btns button:hover {
        background: #f0f0f0;
    }

    .op-btns {
        display: flex;
        gap: 6px;
    }
    .btn-small {
        padding: 0 10px;
        height: 28px;
        font-size: 12px;
        border-radius: 4px;
        border: 1px solid #ccc;
        background: #fff;
        cursor: pointer;
    }
    .btn-small:hover {
        background: #f5f5f5;
        border-color: #bbb;
    }
    .btn-dang {
        color: #d32f2f;
        border-color: #ffcdd2;
        background: #ffebee;
    }
    .btn-dang:hover {
        background: #ffcdd2;
    }

    /* 弹窗样式 - 绝对居中 */
    .modal-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
        padding: 20px;
        backdrop-filter: blur(2px);
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
    .p-header {
        width: 100%;
        box-sizing: border-box;
        padding: 12px 18px;
        background: #f0f0f0;
        font-weight: bold;
        border-bottom: 1px solid #ddd;
        font-size: 16px;
        display: flex;
        justify-content: space-between;
        align-items: center;
        flex-shrink: 0;
    }
    .p-body {
        padding: 20px;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }
    .scroll-p {
        max-height: 60vh;
        overflow-y: auto;
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
    .set-row input,
    .set-row button.mini-btn {
        width: auto !important;
        flex: 1;
        padding: 8px !important;
        border: 1px solid #ddd !important;
        border-radius: 6px !important;
        font-size: 15px !important;
        background: #fff !important;
        height: auto !important;
        line-height: 1.5 !important;
        box-sizing: border-box !important;
        display: block !important;
        min-height: 38px !important;
    }

    .err-tag {
        margin: 3px;
        padding: 6px 14px;
        background: #fee;
        color: #c00;
        border: 1px solid #fcc;
        border-radius: 20px;
        font-size: 13px;
    }
    .hist-item {
        display: flex;
        justify-content: space-between;
        padding: 16px;
        border-bottom: 1px solid #eee;
        width: 100%;
        background: #fff;
    }
    .sec-title {
        font-weight: bold;
        font-size: 14px;
        border-left: 5px solid #0066b8;
        padding-left: 10px;
        margin-bottom: 10px;
    }
    .empty-msg {
        text-align: center;
        color: #999;
        padding: 20px;
    }

    /* EPUB制作完成按钮样式 - 墨蓝色渐变 */
    .epub-success {
        background: linear-gradient(
            135deg,
            #1e3a8a 0%,
            #3b82f6 100%
        ) !important;
        color: white !important;
        border: none !important;
        font-weight: 600;
        box-shadow: 0 4px 12px rgba(30, 58, 138, 0.3);
    }
    .epub-success:active {
        background: linear-gradient(
            135deg,
            #1e40af 0%,
            #2563eb 100%
        ) !important;
        transform: scale(0.98);
    }

    .sidebar-mask {
        position: absolute;
        inset: 0;
        background: rgba(0, 0, 0, 0.4);
        z-index: 90;
    }
    @media (max-width: 768px) {
        .sidebar {
            position: absolute;
            z-index: 1000;
            left: 0;
            top: 0;
            bottom: 0;
            width: 85%;
            box-shadow: 15px 0 50px rgba(0, 0, 0, 0.3);
        }
    }
</style>
