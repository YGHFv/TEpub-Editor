<script lang="ts">
    import { base } from "$app/paths";
    import { viewUrl } from "$lib/keepAlive/viewState";

    // 保活视图里页面常驻，全局 page.url 会随别的标签跳转而变
    const ownUrl = viewUrl();
    import { onDestroy, onMount, tick } from "svelte";
    import { EditorView } from "@codemirror/view";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";
    import FileTreeItem from "$lib/FileTreeItem.svelte";
    // 与桌面端共用同一份图标映射（emoji 在 web 端也会因系统字体不同而变形）
    import { getFileIcon } from "$lib/epubEditor/fileIcons";
    import {
        countMatchesInContent,
        filterFilesInScope,
        findNextMatch,
        replaceAllInContent,
        type FindQuery,
    } from "$lib/findReplace";
    import TocNode from "$lib/TocNode.svelte";
    import MobileSheet from "$lib/mobile/MobileSheet.svelte";
    import ToolImportPage from "$lib/ToolImportPage.svelte";
    import {
        describePreviewClick,
        revealTextInPreview,
    } from "$lib/epubEditor/previewSync";
    import { measureEpubContentHeight } from "$lib/epubEditor/contentHeight";
    import { hasUnsavedEpubChanges } from "$lib/unsavedChanges";
    import {
        applyPaperTheme,
        paperBackground,
        paperTextColor,
    } from "$lib/epubPaperTheme";
    import {
        loadAppSettings,
        resolveUiTheme,
        THEME_RESOLVED_EVENT,
        type GlobalAppSettings,
    } from "$lib/appSettings";
    import {
        EPUB_FIND_HISTORY_KEY,
        EPUB_REPLACE_HISTORY_KEY,
    } from "$lib/storageKeys";
    import {
        addWebEpubResource,
        deleteWebEpubResource,
        exportWebEpubBlob,
        guessWebEpubMediaType,
        loadWebEpub,
        normalizeZipPath,
        readWebEpubBlob,
        readWebEpubText,
        renameWebEpubResource,
        updateWebEpubMetadata,
        updateWebEpubText,
        type WebEpubDocument,
        type WebEpubFileEntry,
        type WebEpubMetadata,
        type WebEpubNavItem,
    } from "$lib/webEpub";
    import { resolveEpubCover } from "$lib/epubEditor/cover";
    import { downloadBrowserBlob } from "$lib/webFileWorkflow";
    import { setTabDetail, tabIdFor } from "$lib/secondaryTabs";
    import {
        registerKeepAliveGuard,
        unregisterKeepAliveGuard,
    } from "$lib/keepAlive/dirtyGuard";
    import { clientLayout, isMobileUserAgent } from "$lib/clientProfile";

    type WebFileTreeNode = {
        name: string;
        path: string;
        file_type:
            | "folder"
            | "html"
            | "css"
            | "xml"
            | "image"
            | "font"
            | "text"
            | "other";
        children?: WebFileTreeNode[];
        file?: WebEpubFileEntry;
        size?: number;
        mediaType?: string;
        kind?: WebEpubFileEntry["kind"];
        editable?: boolean;
    };

    type WebTocTreeNode = {
        id: string;
        label: string;
        src: string;
        children?: WebTocTreeNode[];
    };

    function appPath(path: string) {
        return `${base}${path.startsWith("/") ? path : `/${path}`}`;
    }

    const FIND_MODE_OPTIONS = [
        { value: "text", label: "普通文本" },
        { value: "regex", label: "正则表达式" },
    ];

    const SEARCH_SCOPE_OPTIONS = [
        { value: "current", label: "当前文件" },
        { value: "open", label: "已打开" },
        { value: "html", label: "HTML文件" },
        { value: "selected", label: "选中文件" },
        { value: "all", label: "全部文件" },
    ];

    const SEARCH_DIRECTION_OPTIONS = [
        { value: "down", label: "向下" },
        { value: "up", label: "向上" },
    ];

    let fileInput: HTMLInputElement | null = null;
    let resourceInput: HTMLInputElement | null = null;
    let doc: WebEpubDocument | null = null;
    let selectedFile: WebEpubFileEntry | null = null;
    let selectedImage: WebEpubFileEntry | null = null;
    let editorText = "";
    let savedEditorText = "";
    let previewHtml = "";
    let previewBlobUrls: string[] = [];
    let imagePreviewUrl = "";
    let coverPreviewUrl = "";
    let epubCodeEditorComponent: EpubCodeEditor | null = null;
    let metadataDraft: WebEpubMetadata = emptyMetadata();
    let metadataDirty = false;
    let documentDirty = false;
    let busy = false;
    let status = "选择 EPUB 文件后在浏览器内解包，编辑文本资源并导出新文件。";
    let activeTab: "files" | "metadata" = "files";
    let rightTab: "preview" | "toc" = "preview";
    let restorePageOverflow: (() => void) | null = null;
    let restoreSettingsListener: (() => void) | null = null;
    /** 本页在顶部标签栏里的 id（含 ?mode=reader），用于上报悬停说明 */
    const ownTabId = tabIdFor(ownUrl.pathname, ownUrl.search);
    let expandedFolders = new Set<string>();
    let multiSelectedFiles = new Set<string>();
    let selectedTreePath = "";
    let pendingImportFolder = "";
    let fileMenu: { path: string; x: number; y: number } | null = null;
    let fileTitleMap = new Map<string, string>();
    let findPattern = "";
    let replacePattern = "";
    let findHistory: string[] = [];
    let replaceHistory: string[] = [];
    let showFindHistory = false;
    let showReplaceHistory = false;
    let isRegex = false;
    let searchScope: "current" | "open" | "html" | "selected" | "all" =
        "current";
    let searchDirection: "down" | "up" = "down";
    let wrapAround = true;
    let textOnly = false;
    let searchMessage = "";
    let currentMatchInfo: { path: string; from: number; to: number } | null =
        null;
    let previewBuildId = 0;
    // 预览滚动条开关（设置 → 通用），默认关闭
    let appSettings: GlobalAppSettings = loadAppSettings();
    let resolvedTheme = resolveUiTheme(appSettings.uiTheme);
    let previewFrameHeight = 0;
    let previewIframeEl: HTMLIFrameElement | null = null;
    let previewBodyEl: HTMLDivElement | null = null;
    let readerPreviewFontSize = 18;
    let readerTocCollapsed = false;
    const readerMode = ownUrl.searchParams.get("mode") === "reader";

    /**
     * 手机形态：三栏叠成一格，靠底部页签切换。
     *
     * 为什么不给手机写第二套 DOM：整页 3900 行，业务逻辑（查找替换、预览同步、
     * 资源增删）全绑在现有节点上。复制一份等于以后每个 bug 都要修两遍。
     * 这里只在**手机**下加页签、加动作条，桌面渲染出的节点一个不变。
     */
    let phoneLayout = false;
    $: phoneLayout = $clientLayout === "phone";

    /** 手机上当前显示哪一栏。桌面忽略这个值 */
    let mobilePane: "tree" | "editor" | "preview" = "tree";
    /** 手机文件树筛选（桌面不渲染输入框，值恒为空串，不影响 fileTree） */
    let phoneTreeFilter = "";
    /** 手机上的查找条默认收起：它一展开就吃掉半屏，编辑区没法用 */
    let phoneFindOpen = false;
    let phoneMoreOpen = false;
    let phoneGotoOpen = false;
    let phoneGotoValue = "";
    /** 真机触摸：软键盘要关掉自动大写/纠错，代码里那些属性全是负作用 */
    let touchInput = false;
    $: touchInput = phoneLayout && isMobileUserAgent();
    let phoneLine = 1;

    /**
     * 切到编辑栏并刷新状态栏里的行号。
     *
     * 必须等一帧再读行号：切换前编辑器所在的那一栏是 visibility:hidden，
     * CodeMirror 的测量在这种状态下拿到的是旧值（有些浏览器直接返回 0），
     * 立刻读会把「行 1」写进动作条，切过去之后又不刷新。
     */
    function showEditorPane() {
        mobilePane = "editor";
        void tick().then(syncPhoneLine);
    }

    /** 切走编辑栏时把焦点收掉：隐藏的 CodeMirror 不该继续吃键盘输入 */
    function leaveEditorPane() {
        if (mobilePane !== "editor") return;
        const view = epubCodeEditorComponent?.getView();
        view?.contentDOM.blur();
    }

    function selectPane(pane: "tree" | "editor" | "preview") {
        leaveEditorPane();
        mobilePane = pane;
        if (pane === "editor") void tick().then(syncPhoneLine);
    }

    function syncPhoneLine() {
        phoneLine = epubCodeEditorComponent?.currentLine() || 1;
    }

    function handlePhoneGoto() {
        const line = parseInt(phoneGotoValue, 10);
        if (Number.isFinite(line) && line > 0) {
            epubCodeEditorComponent?.gotoLine(line);
            syncPhoneLine();
        }
        phoneGotoOpen = false;
        phoneGotoValue = "";
    }

    function handlePhoneSymbol(text: string) {
        epubCodeEditorComponent?.insertText(text);
    }

    function togglePhoneWrap() {
        epubCodeEditorComponent?.toggleLineWrap();
    }

    function adjustPhoneFont(delta: number) {
        const current = epubCodeEditorComponent?.getFontSize() ?? 14;
        epubCodeEditorComponent?.setFontSize(current + delta);
    }

    const PHONE_SYMBOLS = [
        "    ",
        ":",
        "=",
        "{",
        "}",
        "[",
        "]",
        "(",
        ")",
        "<",
        ">",
        '"',
        "'",
        "/",
        "\\",
        "|",
        "&",
        "+",
        "-",
        "*",
        "#",
        "_",
    ];
    /** 空格键显示成「Tab」而不是一片空白，否则那一格看起来像少了按钮 */
    function phoneSymbolLabel(symbol: string) {
        return symbol === "    " ? "Tab" : symbol;
    }

    $: headTitle = readerMode
        ? "Web EPUB 阅读器 - TEpub Editor"
        : "Web EPUB 编辑器 - TEpub Editor";
    $: importHeading = readerMode
        ? "选择 EPUB 文件开始阅读"
        : "选择 EPUB 文件开始编辑";
    $: importDescription = readerMode
        ? "导入后在浏览器内解包，左侧显示目录，中间以宽屏滚动预览方式阅读。"
        : "支持无 DRM 的标准 EPUB，导入后可编辑文件结构、资源和元数据。";
    $: importAction = readerMode ? "选择 EPUB 文件" : "选择 EPUB 文件";
    $: currentPreviewTitle = selectedFile
        ? fileTitleMap.get(selectedFile.path) || selectedFile.name
        : doc?.metadata.title || doc?.fileName || "未选择章节";

    /* 刻意不在 previewHtml 清空时把 previewFrameHeight 归零：
       换章节时 previewHtml 会先被置空再填新内容，归零会让新 iframe
       先以 min-height 挂载、量完再长高，外层滚动条就闪一下。
       保留上一次高度，新章节挂载即占住位置，只做一次高度修正。 */

    /* 让 iframe 长到内容高度，滚动交给外面那个框。
       普通预览的框是手机框（.mobile-frame）：它自己有固定尺寸，正文在框内
       滚动，手机不会随章节变长而拉成一条长纸带；阅读模式的框是宽屏页
       （.reader-frame），没有高度上限，仍由外层 .preview-body 滚动。
       两种情况下"可见高度"都是父框的 clientHeight，量法一致。 */
    function measurePreviewFrame() {
        const frame = previewIframeEl;
        if (!frame) return;
        let innerDoc: Document | null = null;
        try {
            innerDoc = frame.contentDocument;
        } catch {
            innerDoc = null;
        }
        const innerBody = innerDoc?.body;
        if (!innerBody) return;
        // 内容高度的量法见 $lib/epubEditor/contentHeight：
        // `html, body { height: 100% }` 的书会让 scrollHeight 随 iframe 一起长，
        // 直接拿来设高度会变成自持回路（iframe→body→量得更高→iframe）
        const contentHeight = measureEpubContentHeight(innerBody);
        if (contentHeight <= 0) return;
        // 短章节（封面这类）也要占满一屏，否则框里下面会空出一块底色；
        // 长章节则用实测内容高度，让框自己滚动。
        //
        // 量谁的高度要看模式：
        // - 手机框有 height:100% + max-height，高度由预览列决定，直接量它；
        // - 阅读模式的 .reader-frame 是 height:auto，高度就是 iframe 的高度，
        //   量它等于拿自己的结果当输入（永远收敛不了），要量外层 .preview-body。
        const shell = frame.parentElement;
        const host = readerMode
            ? frame.closest<HTMLElement>(".preview-body")
            : shell;
        // 目录页签下预览区是 visibility:hidden，尺寸仍在；
        // 真的量不到（列被压成 0）时保留上一次高度，避免切回来先缩再长
        if (host && host.clientHeight <= 0) return;
        let availableHeight = 0;
        if (host) {
            const hostStyle = getComputedStyle(host);
            availableHeight = Math.floor(
                host.clientHeight -
                    parseFloat(hostStyle.paddingTop) -
                    parseFloat(hostStyle.paddingBottom),
            );
        }
        previewFrameHeight = Math.max(contentHeight, availableHeight);
    }

    // 图片解码、字体回流都会改变高度，分几个时间点补测
    function schedulePreviewMeasure() {
        measurePreviewFrame();
        requestAnimationFrame(measurePreviewFrame);
        setTimeout(measurePreviewFrame, 80);
        setTimeout(measurePreviewFrame, 320);
        setTimeout(measurePreviewFrame, 900);
    }

    /* 编辑器 -> 预览：按「点击的段落 / 选中的文字」定位，而不是按滚动比例。
       滚动联动会让预览跟着抖动，两边换行密度也不同，百分比对不上位置。
       预览 iframe 已被撑到正文高度、由 .preview-body 滚动，
       所以把命中位置换算成外层 scrollTop（由 previewSync 完成）。 */
    function revealInPreview(text: string, mode: "center" | "nearest") {
        const clean = text.replace(/[\s ]+/g, " ").trim();
        if (!clean) return;
        revealTextInPreview(previewBodyEl, previewIframeEl, clean, { mode });
    }

    function handleEditorClick(line: number) {
        const view = epubCodeEditorComponent?.getView();
        if (!view) return;
        const cleanText = view.state.doc
            .line(line)
            .text.replace(/<[^>]+>/g, " ")
            .replace(/[\s ]+/g, " ")
            .trim();
        if (cleanText.length < 2) return;
        revealInPreview(cleanText, "center");
    }

    let editorSelectionTimeout: ReturnType<typeof setTimeout> | null = null;
    function handleEditorSelection(text: string) {
        if (editorSelectionTimeout) clearTimeout(editorSelectionTimeout);
        editorSelectionTimeout = setTimeout(() => {
            const plain = text.replace(/<[^>]+>/g, "");
            if (plain.replace(/[\s ]+/g, "").length < 2) return;
            revealInPreview(plain, "nearest");
        }, 300);
    }

    /* 预览 -> 编辑器：点了预览里的某段文字，把编辑器定位过去。
       有块级上下文时用上下文匹配，能区分重复的短句。 */
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

    /* 正文内部的尺寸一变就重量高度。
       只靠几个固定 setTimeout 会漏掉「晚到的图片把内容撑高」：
       iframe 停在旧高度，内部就长出滚动条，而且再也不会消失。 */
    let previewContentObserver: ResizeObserver | null = null;
    function watchPreviewContent() {
        previewContentObserver?.disconnect();
        const innerBody = previewIframeEl?.contentDocument?.body;
        if (!innerBody || typeof ResizeObserver === "undefined") return;
        previewContentObserver = new ResizeObserver(() =>
            measurePreviewFrame(),
        );
        previewContentObserver.observe(innerBody);
    }

    function trackPreviewWidth(node: HTMLElement) {
        let lastWidth = node.clientWidth;
        let lastHeight = node.clientHeight;
        const observer = new ResizeObserver(() => {
            // 把可视高度写到 CSS 变量，供 iframe 在实测完成前先占满一屏，
            // 避免首帧只有一个默认高度、随后再长高的跳动
            const style = getComputedStyle(node);
            const inner =
                node.clientHeight -
                parseFloat(style.paddingTop) -
                parseFloat(style.paddingBottom);
            if (inner > 0) node.style.setProperty("--preview-h", `${inner}px`);

            const width = node.clientWidth;
            const height = node.clientHeight;
            if (width <= 0) return;
            // 宽高任一变化都要重量：宽度决定正文折行（内容高度），
            // 高度决定手机框的可视高度——窗口拉高拉矮后不重量，
            // iframe 会停在旧高度，正文上下多出空白或被裁
            if (
                Math.abs(width - lastWidth) < 2 &&
                Math.abs(height - lastHeight) < 2
            ) {
                return;
            }
            lastWidth = width;
            lastHeight = height;
            schedulePreviewMeasure();
        });
        observer.observe(node);
        return {
            destroy: () => {
                observer.disconnect();
                previewContentObserver?.disconnect();
                previewContentObserver = null;
            },
        };
    }

    /* 预览点击 -> 编辑器定位。每次换章节都要重新绑到新文档上 */
    let unbindPreviewClick: (() => void) | null = null;
    function bindPreviewClickHandler() {
        unbindPreviewClick?.();
        unbindPreviewClick = null;
        const innerDoc = previewIframeEl?.contentDocument;
        if (!innerDoc) return;
        const handler = (event: Event) => {
            const payload = describePreviewClick(
                event.target as Element | null,
            );
            if (payload) handlePreviewClick(payload);
        };
        innerDoc.addEventListener("click", handler, true);
        unbindPreviewClick = () =>
            innerDoc.removeEventListener("click", handler, true);
    }

    function handlePreviewFrameLoad() {
        schedulePreviewMeasure();
        watchPreviewContent();
        bindPreviewClickHandler();
        // 深色主题下书自己的深色文字会消失在深色纸上。安装版在
        // PreviewPane 里调 applyPaperTheme 处理，web 版此前没接这一步。
        applyPreviewPaperTheme();
    }

    function applyPreviewPaperTheme() {
        try {
            applyPaperTheme(
                previewIframeEl?.contentDocument,
                resolveUiTheme(appSettings.uiTheme) === "dark",
            );
        } catch {
            // 跨源或文档已销毁：预览本身还能看，不值得让整页报错
        }
    }

    // 主题切换时 srcdoc 会重建，但设置变化不一定走重建（比如只在
    // 另一窗口改），这里补一次，和安装版的 $effect 对齐
    $: if (appSettings.uiTheme) applyPreviewPaperTheme();

    $: dirty = editorText !== savedEditorText;
    $: hasUnsavedChanges = hasUnsavedEpubChanges(
        readerMode,
        Boolean(doc),
        dirty,
        metadataDirty,
        documentDirty,
    );
    $: editableFiles = doc?.files.filter((file) => file.editable) ?? [];
    $: imageFiles = doc?.files.filter((file) => file.kind === "image") ?? [];
    $: imageCount =
        doc?.files.filter((file) => file.kind === "image").length ?? 0;
    $: textCount = doc?.files.filter((file) => file.editable).length ?? 0;
    $: fileTree = doc
        ? buildFileTree(
              doc.files,
              doc.spine
                  .map((item) => item.manifest?.fullPath || "")
                  .filter(Boolean),
              collectNavPaths(doc.navItems),
          )
        : [];
    $: selectedTreeNode = selectedTreePath
        ? { path: selectedTreePath }
        : selectedFile || selectedImage;
    $: tocTree = doc ? buildTocTree(doc.navItems) : [];

    /**
     * 手机文件树筛选。
     *
     * 不能只判 `phoneTreeFilter.trim()`：这个输入框渲染在 `{#if phoneLayout}`
     * 里面，而本页是保活视图 —— 手机上输入 "nav"，再切到桌面视图，输入框
     * 连同清除按钮一起消失，但状态还在组件里，桌面文件树会被静默过滤成子集，
     * 用户没有任何入口发现或清除它。所以判定必须同时要求 phoneLayout。
     */
    function filterFileTree(
        nodes: WebFileTreeNode[],
        needle: string,
    ): WebFileTreeNode[] {
        const result: WebFileTreeNode[] = [];
        for (const node of nodes) {
            if (node.file_type === "folder") {
                // 文件夹只要还有命中的子孙就保留，并把子集一起换掉
                const children = filterFileTree(node.children || [], needle);
                if (children.length) result.push({ ...node, children });
                continue;
            }
            if (node.path.toLowerCase().includes(needle)) result.push(node);
        }
        return result;
    }

    /** 筛选时把所有存活文件夹都算作展开，否则用户得手动一层层点开找命中项 */
    function collectFolderPaths(nodes: WebFileTreeNode[], into: Set<string>) {
        for (const node of nodes) {
            if (node.file_type !== "folder") continue;
            into.add(node.path);
            collectFolderPaths(node.children || [], into);
        }
        return into;
    }

    $: phoneTreeNeedle = phoneLayout ? phoneTreeFilter.trim().toLowerCase() : "";
    $: visibleFileTree = phoneTreeNeedle
        ? filterFileTree(fileTree, phoneTreeNeedle)
        : fileTree;
    $: visibleExpandedFolders = phoneTreeNeedle
        ? collectFolderPaths(visibleFileTree, new Set<string>())
        : expandedFolders;

    function fileByPath(path: string | undefined) {
        if (!doc || !path) return null;
        return doc.files.find((file) => file.path === path) || null;
    }

    function handleBeforeUnload(event: BeforeUnloadEvent) {
        if (!hasUnsavedChanges) return;
        event.preventDefault();
        event.returnValue = "";
    }

    function isEditablePath(path: string | undefined) {
        return !!fileByPath(path)?.editable;
    }

    function treeFileType(
        kind: WebEpubFileEntry["kind"],
    ): WebFileTreeNode["file_type"] {
        if (kind === "xhtml") return "html";
        return kind;
    }

    function collectNavPaths(items: WebEpubNavItem[]) {
        const paths: string[] = [];
        for (const item of items) {
            if (item.fullPath) paths.push(item.fullPath);
            if (item.children) paths.push(...collectNavPaths(item.children));
        }
        return paths;
    }

    function sortTreeNodes(
        nodes: WebFileTreeNode[],
        parentName = "",
        spinePaths: string[] = [],
        tocPaths: string[] = [],
    ) {
        const rootPriority = ["oebps", "meta-inf"];
        const oebpsFilePriority = ["content.opf", "toc.ncx"];
        const oebpsFolderPriority = ["text", "styles", "fonts", "images"];

        function weight(node: WebFileTreeNode) {
            const name = node.name.toLowerCase();
            if (!parentName) {
                const index = rootPriority.indexOf(name);
                return index >= 0 ? index : 100;
            }
            if (parentName === "oebps") {
                if (node.file_type !== "folder") {
                    const index = oebpsFilePriority.indexOf(name);
                    return index >= 0 ? index : 200;
                }
                const index = oebpsFolderPriority.indexOf(name);
                return index >= 0 ? 300 + index : 400;
            }
            if (parentName === "text") {
                const spineIndex = spinePaths.indexOf(node.path);
                if (spineIndex >= 0) return spineIndex;
                const tocIndex = tocPaths.indexOf(node.path);
                if (tocIndex >= 0) return 10000 + tocIndex;
                return 20000;
            }
            return 0;
        }

        nodes.sort((left, right) => {
            const leftWeight = weight(left);
            const rightWeight = weight(right);
            if (leftWeight !== rightWeight) return leftWeight - rightWeight;
            return left.name.localeCompare(right.name, "zh-Hans-CN", {
                numeric: true,
                sensitivity: "base",
            });
        });
        for (const node of nodes) {
            if (node.children)
                sortTreeNodes(
                    node.children,
                    node.name.toLowerCase(),
                    spinePaths,
                    tocPaths,
                );
        }
        return nodes;
    }

    function buildFileTree(
        files: WebEpubFileEntry[],
        spinePaths: string[] = [],
        tocPaths: string[] = [],
    ) {
        const roots: WebFileTreeNode[] = [];
        const folders = new Map<string, WebFileTreeNode>();

        function getFolder(
            path: string,
            name: string,
            target: WebFileTreeNode[],
        ) {
            const existing = folders.get(path);
            if (existing) return existing;
            const folder: WebFileTreeNode = {
                name,
                path,
                file_type: "folder",
                children: [],
            };
            folders.set(path, folder);
            target.push(folder);
            return folder;
        }

        for (const file of files) {
            const parts = file.path.split("/").filter(Boolean);
            let target = roots;
            let folderPath = "";
            for (const part of parts.slice(0, -1)) {
                folderPath = folderPath ? `${folderPath}/${part}` : part;
                const folder = getFolder(folderPath, part, target);
                target = folder.children || [];
            }
            target.push({
                name: file.name,
                path: file.path,
                file_type: treeFileType(file.kind),
                file,
                size: file.size,
                mediaType: file.mediaType,
                kind: file.kind,
                editable: file.editable,
            });
        }

        return sortTreeNodes(roots, "", spinePaths, tocPaths);
    }

    function defaultExpandedFolders(files: WebEpubFileEntry[]) {
        const next = new Set<string>();
        for (const file of files) {
            const parts = file.path.split("/").filter(Boolean);
            let folderPath = "";
            for (const part of parts.slice(0, -1)) {
                folderPath = folderPath ? `${folderPath}/${part}` : part;
                if (!folderPath.includes("/")) next.add(folderPath);
            }
        }
        return next;
    }

    function expandParentFolders(path: string) {
        const parts = path.split("/").filter(Boolean);
        const next = new Set(expandedFolders);
        let folderPath = "";
        for (const part of parts.slice(0, -1)) {
            folderPath = folderPath ? `${folderPath}/${part}` : part;
            next.add(folderPath);
        }
        expandedFolders = next;
    }

    async function revealTreePath(path: string) {
        activeTab = "files";
        expandParentFolders(path);
        await tick();
        const escapedPath =
            typeof CSS !== "undefined" && CSS.escape
                ? CSS.escape(path)
                : path.replace(/["\\]/g, "\\$&");
        const element = document.querySelector(
            `.tree-node[data-path="${escapedPath}"]`,
        );
        element?.scrollIntoView({ block: "center", behavior: "smooth" });
    }

    function toggleFolder(path: string) {
        const next = new Set(expandedFolders);
        if (next.has(path)) next.delete(path);
        else next.add(path);
        expandedFolders = next;
    }

    async function selectTreeFile(node: WebFileTreeNode) {
        const file = node.file || fileByPath(node.path);
        if (!file) return;
        selectedTreePath = file.path;
        expandParentFolders(file.path);
        // 手机上点了文件就该直接看到内容，不该再让用户自己点一下「编辑」页签；
        // 桌面不动（三栏同时可见，切页签反而是干扰）
        if (phoneLayout) showEditorPane();
        if (file.kind === "image") {
            await openImage(file);
            return;
        }
        if (file.editable) {
            await openFile(file);
            return;
        }
        status = `${file.path} 暂不支持在 Web 版中直接编辑。`;
    }

    function getFileDescription(node: WebFileTreeNode) {
        if (node.file_type === "folder") return "";
        if (node.file_type === "html") {
            const title = fileTitleMap.get(node.path);
            if (title) return title;
        }
        const name = node.name.toLowerCase();
        if (name === "container.xml") return "容器配置";
        if (name.endsWith(".opf")) return "元数据";
        if (name.includes("toc") || name.endsWith(".ncx")) return "目录结构";
        if (node.file_type === "css") return "样式表";
        if (node.file_type === "font")
            return `font · ${fileSizeLabel(node.size || 0)}`;
        if (node.file_type === "image")
            return `image · ${fileSizeLabel(node.size || 0)}`;
        return `${node.file_type} · ${fileSizeLabel(node.size || 0)}`;
    }

    function getFileLanguage(
        kind: WebEpubFileEntry["kind"],
    ): "html" | "css" | "xml" | "other" {
        if (kind === "xhtml") return "html";
        if (kind === "css") return "css";
        if (kind === "xml") return "xml";
        return "other";
    }

    function handleEditorChange(content: string) {
        editorText = content;
        if (selectedFile?.kind !== "xhtml") previewHtml = "";
    }

    function currentEditorContent() {
        return (
            epubCodeEditorComponent?.getView()?.state.doc.toString() ??
            editorText
        );
    }

    function currentEditorCursor(
        direction: "down" | "up",
        fallbackLength: number,
    ) {
        const selection =
            epubCodeEditorComponent?.getView()?.state.selection.main;
        if (!selection) return direction === "up" ? fallbackLength : 0;
        return direction === "up" ? selection.from : selection.to;
    }

    function selectEditorRange(from: number, to: number) {
        const view = epubCodeEditorComponent?.getView();
        if (!view) return;
        view.focus();
        view.dispatch({
            selection: { anchor: from, head: to },
            effects: EditorView.scrollIntoView(from, { y: "center" }),
        });
    }

    function buildTocTree(items: WebEpubNavItem[], prefix = "toc") {
        return items.map((item, index): WebTocTreeNode => ({
            id: `${prefix}-${index}`,
            label: item.label || item.href || `目录 ${index + 1}`,
            src: item.fullPath,
            children: item.children?.length
                ? buildTocTree(item.children, `${prefix}-${index}`)
                : undefined,
        }));
    }

    function pickFile() {
        // 换书会把整个 doc 连同未暂存的修改一起丢掉。桌面有 beforeunload 兜底
        // 关闭/跳转，但"再选一本书"是页内动作，不会被它拦住 —— 手机上更容易
        // 误触（「更多」里的第一项就是它），所以只在手机形态确认一次。
        // 桌面保持原有行为：点「打开文件」直接弹文件选择框。
        if (phoneLayout && doc && hasUnsavedChanges) {
            // 判据是 hasUnsavedChanges（含已暂存但尚未导出的 documentDirty），
            // 文案不能只说「未暂存」：用户先点过「暂存当前文件」再换书时，
            // 没有任何未暂存内容，被丢掉的恰恰是已暂存的那份。
            const ok = window.confirm(
                "当前 EPUB 有未导出的修改（含已暂存内容），换书会丢弃这些改动。是否继续？",
            );
            if (!ok) return;
        }
        fileInput?.click();
    }

    function emptyMetadata(): WebEpubMetadata {
        return {
            title: "",
            creator: "",
            language: "zh-CN",
            identifier: "",
            description: "",
            publisher: "",
            date: "",
            subject: "",
        };
    }

    function resetMetadataDraft() {
        metadataDraft = doc ? { ...doc.metadata } : emptyMetadata();
        metadataDirty = false;
    }

    function markMetadataDirty() {
        metadataDirty = true;
    }

    function extractXhtmlTitle(source: string) {
        try {
            const parsed = new DOMParser().parseFromString(
                source,
                "application/xhtml+xml",
            );
            const title =
                parsed
                    .querySelector("title")
                    ?.textContent?.replace(/\s+/g, " ")
                    .trim() ||
                parsed
                    .querySelector("h1,h2,h3")
                    ?.textContent?.replace(/\s+/g, " ")
                    .trim() ||
                "";
            return title.slice(0, 80);
        } catch {
            const match = source.match(
                /<(?:title|h1|h2|h3)[^>]*>([\s\S]*?)<\/(?:title|h1|h2|h3)>/i,
            );
            return (match?.[1] || "")
                .replace(/<[^>]+>/g, " ")
                .replace(/\s+/g, " ")
                .trim()
                .slice(0, 80);
        }
    }

    async function loadFileTitles() {
        if (!doc) {
            fileTitleMap = new Map();
            return;
        }
        const next = new Map<string, string>();
        const files = doc.files.filter((file) => file.kind === "xhtml");
        for (const file of files) {
            try {
                const title = extractXhtmlTitle(
                    await readWebEpubText(doc, file.path),
                );
                if (title) next.set(file.path, title);
            } catch {
                // Ignore unreadable title candidates; opening the file will still report the real error.
            }
        }
        fileTitleMap = next;
    }

    onMount(() => {
        loadSearchHistory();
        const bodyOverflow = document.body.style.overflow;
        const htmlOverflow = document.documentElement.style.overflow;
        document.body.style.overflow = "hidden";
        document.documentElement.style.overflow = "hidden";
        restorePageOverflow = () => {
            document.body.style.overflow = bodyOverflow;
            document.documentElement.style.overflow = htmlOverflow;
        };
        // 编辑器常驻在独立窗口/标签页里，改完设置回来要立刻生效。
        // 自定义事件只在本窗口冒泡，跨窗口改 localStorage 只有 storage 事件会送达
        const syncSettings = () => {
            const next = loadAppSettings();
            const nextResolvedTheme = resolveUiTheme(next.uiTheme);
            const themeChanged = nextResolvedTheme !== resolvedTheme;
            resolvedTheme = nextResolvedTheme;
            appSettings = next;
            // 预览文档的纸色是在生成 srcdoc 时写死的，主题一变必须重算，
            // 否则深色界面里会留着一整页浅色书页
            if (themeChanged && previewHtml && selectedFile?.kind === "xhtml") {
                void refreshPreviewFromEditor();
            }
        };
        window.addEventListener("tepub-settings-updated", syncSettings);
        window.addEventListener(THEME_RESOLVED_EVENT, syncSettings);
        window.addEventListener("storage", syncSettings);
        restoreSettingsListener = () => {
            window.removeEventListener("tepub-settings-updated", syncSettings);
            window.removeEventListener(THEME_RESOLVED_EVENT, syncSettings);
            window.removeEventListener("storage", syncSettings);
        };

        // 手机端保活裁剪会为了省内存卸载最久没访问的页面。带着未暂存修改时
        // 必须否决 —— 否则用户从编辑器逛两页回来，改动已经无声消失。
        // 桌面版永远不裁（trimTabs 只在手机外壳调用），这条登记对桌面无副作用。
        registerKeepAliveGuard(ownTabId, () => hasUnsavedChanges);
    });

    onDestroy(() => {
        unregisterKeepAliveGuard(ownTabId);
        revokePreviewUrls();
        revokeImagePreviewUrl();
        revokeCoverPreviewUrl();
        restorePageOverflow?.();
        restoreSettingsListener?.();
    });

    /**
     * 把当前打开的书 / 文件报到顶部标签栏的悬停提示里。
     *
     * web 版是用 <input type=file> 选书的，地址里没有 ?file=，标签栏原本
     * 只能显示「EPUB 编辑器」。按「书 → 当前文件」分两行给出，和标签栏
     * 「短标签 + 悬停看全」的约定一致。
     */
    function publishTabDetail() {
        if (!doc) {
            setTabDetail(ownTabId, "");
            return;
        }
        const book = doc.metadata.title || doc.fileName;
        const lines = [book];
        const currentPath =
            selectedFile?.path || selectedImage?.path || selectedTreePath;
        if (currentPath && currentPath !== doc.opfPath) {
            lines.push(`当前文件：${currentPath}`);
        } else if (!currentPath) {
            lines.push(
                `${doc.files.length} 个文件 · ${doc.spine.length} 个阅读项`,
            );
        }
        setTabDetail(ownTabId, lines.join("\n"));
    }

    /**
     * 导入时做过的自动修复要告诉用户 —— 否则他看到的是一本「改过名字」
     * 的书，导出后和源文件不一样，却不知道是哪一步改的。
     */
    function normalizationSuffix(current: WebEpubDocument) {
        const notes: string[] = [];
        if (current.normalization.repairedEncryption)
            notes.push("已解除伪加密");
        if (current.normalization.xorKey !== null) notes.push("已解混淆");
        if (current.normalization.renamedEntries > 0)
            notes.push(
                `已恢复 ${current.normalization.renamedEntries} 个可读文件名`,
            );
        return notes.length ? `；${notes.join("，")}` : "";
    }

    async function loadEpubFile(file: File) {
        busy = true;
        status = "正在解包 EPUB...";
        selectedFile = null;
        editorText = "";
        savedEditorText = "";
        selectedImage = null;
        selectedTreePath = "";
        fileTitleMap = new Map();
        previewHtml = "";
        resetMetadataDraft();
        revokePreviewUrls();
        revokeImagePreviewUrl();
        revokeCoverPreviewUrl();

        try {
            doc = await loadWebEpub(file);
            documentDirty = false;
            expandedFolders = defaultExpandedFolders(doc.files);
            resetMetadataDraft();
            await loadFileTitles();
            // 阅读模式没有元数据面板，封面取来也没地方画，白占一个 blob URL
            if (!readerMode) await loadCoverPreview();
            publishTabDetail();
            status = `已导入 ${file.name}，读取到 ${doc.files.length} 个文件 / ${doc.spine.length} 个阅读项${normalizationSuffix(doc)}`;
            const firstEditable =
                doc.spine
                    .map((item) => fileByPath(item.manifest?.fullPath))
                    .find(
                        (entry): entry is WebEpubFileEntry => !!entry?.editable,
                    ) ||
                doc.files.find((entry) => entry.editable) ||
                null;
            busy = false;
            if (firstEditable) await openFile(firstEditable, true);
        } catch (error) {
            doc = null;
            fileTitleMap = new Map();
            resetMetadataDraft();
            selectedImage = null;
            revokeImagePreviewUrl();
            revokeCoverPreviewUrl();
            status = `导入失败：${String(error)}`;
        } finally {
            busy = false;
            if (fileInput) fileInput.value = "";
        }
    }

    async function onFileChange(event: Event) {
        const file = (event.currentTarget as HTMLInputElement).files?.[0];
        if (!file) return;
        await loadEpubFile(file);
    }

    function handleImportFiles(event: CustomEvent<File[]>) {
        const file = event.detail[0];
        if (file) void loadEpubFile(file);
    }

    function persistCurrentFile(options: { refreshPreview?: boolean } = {}) {
        if (readerMode) return;
        if (!doc || !selectedFile || !dirty) return;
        const content = currentEditorContent();
        editorText = content;
        updateWebEpubText(doc, selectedFile.path, content);
        documentDirty = true;
        savedEditorText = content;
        if (options.refreshPreview && selectedFile.kind === "xhtml") {
            void refreshPreviewFromEditor();
        }
    }

    async function openFile(file: WebEpubFileEntry, force = false) {
        if (!doc || !file.editable || (busy && !force)) return false;
        persistCurrentFile({ refreshPreview: false });
        busy = true;
        status = `正在读取 ${file.path}`;
        try {
            selectedFile = file;
            selectedTreePath = file.path;
            expandParentFolders(file.path);
            editorText = await readWebEpubText(doc, file.path);
            savedEditorText = editorText;
            if (file.kind === "xhtml") {
                /* 不在这里清空 previewHtml：清空会让 .mobile-frame 整块卸载，
                   预览区先被空态（直角虚线框）顶上约 400ms，等新章节构建完
                   才变回圆角框 —— 观感上就是"先闪一下直角"。
                   安装版是保留上一章画面直到新内容就绪（见
                   routes/epub-editor/+page.svelte 里 previewContent 的处理），
                   这里与它对齐：refreshPreviewFor 拿到新 html 后一次性替换。 */
                rightTab = "preview";
                void refreshPreviewFor(file, editorText);
            } else {
                revokePreviewUrls();
                previewHtml = "";
            }
            currentMatchInfo = null;
            publishTabDetail();
            status = `已打开 ${file.path}`;
            return true;
        } catch (error) {
            status = `读取失败：${String(error)}`;
            return false;
        } finally {
            busy = false;
        }
    }

    async function openPath(path: string | undefined) {
        const file = fileByPath(path);
        if (!file) return false;
        if (file.kind === "image") return await openImage(file);
        if (file.editable) return await openFile(file);
        return false;
    }

    function stageCurrentFile() {
        persistCurrentFile({ refreshPreview: true });
    }

    async function refreshPreviewFromEditor() {
        if (!doc || !selectedFile || selectedFile.kind !== "xhtml") {
            previewHtml = "";
            revokePreviewUrls();
            return;
        }
        await refreshPreviewFor(selectedFile, currentEditorContent());
    }

    async function refreshPreviewFor(file: WebEpubFileEntry, source: string) {
        if (!doc || file.kind !== "xhtml") return;
        const requestId = ++previewBuildId;
        try {
            const result = await buildPreviewHtmlFor(file, source);
            if (
                requestId !== previewBuildId ||
                selectedFile?.path !== file.path
            ) {
                for (const url of result.urls) URL.revokeObjectURL(url);
                return;
            }
            revokePreviewUrls();
            previewBlobUrls = result.urls;
            previewHtml = result.html;
            rightTab = "preview";
        } catch (error) {
            if (requestId !== previewBuildId) return;
            previewHtml = "";
            status = `预览失败：${String(error)}`;
            revokePreviewUrls();
        }
    }

    function prepareOpfMutation() {
        if (!doc || selectedFile?.path !== doc.opfPath || !dirty) return true;
        const ok = window.confirm(
            "当前 OPF 源码有未暂存修改，继续会先暂存 OPF 再写入资源清单。是否继续？",
        );
        if (!ok) return false;
        stageCurrentFile();
        return true;
    }

    async function syncOpenOpfSource() {
        if (!doc || selectedFile?.path !== doc.opfPath) return;
        editorText = await readWebEpubText(doc, doc.opfPath);
        savedEditorText = editorText;
        previewHtml = "";
        revokePreviewUrls();
    }

    function revokeImagePreviewUrl() {
        if (imagePreviewUrl) URL.revokeObjectURL(imagePreviewUrl);
        imagePreviewUrl = "";
    }

    function revokeCoverPreviewUrl() {
        if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
        coverPreviewUrl = "";
    }

    async function loadCoverPreview() {
        revokeCoverPreviewUrl();
        const current = doc;
        if (!current) return;
        try {
            // 按声明解析：properties="cover-image" → <meta name="cover">
            // → guide 包装页里的图 → 名字像封面的图 → 最大的图
            const cover = await resolveEpubCover({
                files: current.files,
                hints: current.coverHints,
                readText: (path) => readWebEpubText(current, path),
            });
            if (!cover || doc !== current) return;
            const blob = await readWebEpubBlob(
                current,
                cover.path,
                mimeForPath(cover.path),
            );
            if (doc !== current) return;
            coverPreviewUrl = URL.createObjectURL(blob);
        } catch {
            coverPreviewUrl = "";
        }
    }

    async function openImage(file: WebEpubFileEntry, force = false) {
        if (!doc || file.kind !== "image" || (busy && !force)) return false;
        persistCurrentFile({ refreshPreview: false });
        busy = true;
        status = `正在读取图片 ${file.path}`;
        try {
            const blob = await readWebEpubBlob(
                doc,
                file.path,
                mimeForPath(file.path),
            );
            revokeImagePreviewUrl();
            selectedImage = file;
            selectedFile = null;
            selectedTreePath = file.path;
            expandParentFolders(file.path);
            editorText = "";
            savedEditorText = "";
            previewHtml = "";
            revokePreviewUrls();
            currentMatchInfo = null;
            imagePreviewUrl = URL.createObjectURL(blob);
            publishTabDetail();
            status = `已打开图片 ${file.path}`;
            return true;
        } catch (error) {
            status = `图片读取失败：${String(error)}`;
            return false;
        } finally {
            busy = false;
        }
    }

    function safeResourceName(name: string) {
        const cleaned = name
            .trim()
            .replace(/[\\:*?"<>|]+/g, "_")
            .replace(/\s+/g, "_");
        return cleaned || "resource";
    }

    function uniqueResourcePath(path: string) {
        if (!doc) return normalizeZipPath(path);
        const clean = normalizeZipPath(path);
        if (!doc.zip.file(clean)) return clean;
        const dot = clean.lastIndexOf(".");
        const slash = clean.lastIndexOf("/");
        const base = dot > slash ? clean.slice(0, dot) : clean;
        const ext = dot > slash ? clean.slice(dot) : "";
        let index = 2;
        let next = `${base}-${index}${ext}`;
        while (doc.zip.file(next)) {
            index += 1;
            next = `${base}-${index}${ext}`;
        }
        return next;
    }

    function defaultImportPath(fileName: string) {
        if (!doc) return safeResourceName(fileName);
        const mediaType = guessWebEpubMediaType(fileName);
        const name = safeResourceName(fileName);
        if (mediaType.startsWith("image/"))
            return uniqueResourcePath(`${doc.opfDir}Images/${name}`);
        if (mediaType === "text/css")
            return uniqueResourcePath(`${doc.opfDir}Styles/${name}`);
        if (mediaType.includes("font"))
            return uniqueResourcePath(`${doc.opfDir}Fonts/${name}`);
        if (mediaType === "application/xhtml+xml")
            return uniqueResourcePath(`${doc.opfDir}Text/${name}`);
        return uniqueResourcePath(`${doc.opfDir}${name}`);
    }

    function folderImportPath(folderPath: string, fileName: string) {
        const folder = normalizeZipPath(folderPath);
        const name = safeResourceName(fileName);
        return uniqueResourcePath(folder ? `${folder}/${name}` : name);
    }

    function pickFolderResourceImport(
        node: WebFileTreeNode,
        event?: MouseEvent,
    ) {
        event?.stopPropagation();
        if (!doc || busy || node.file_type !== "folder") return;
        pendingImportFolder = node.path;
        resourceInput?.click();
    }

    async function dropFilesIntoFolder(node: WebFileTreeNode, files: FileList) {
        if (!doc || busy || node.file_type !== "folder" || files.length === 0)
            return;
        await importResourceFiles(Array.from(files), node.path, false);
    }

    async function importResourceFiles(
        files: File[],
        targetFolder = "",
        askPath = true,
    ) {
        if (!doc || files.length === 0) return;
        if (!prepareOpfMutation()) {
            if (resourceInput) resourceInput.value = "";
            return;
        }
        busy = true;
        const importedPaths: string[] = [];
        let lastEntry: WebEpubFileEntry | undefined;
        try {
            for (const file of files) {
                const initialPath = targetFolder
                    ? folderImportPath(targetFolder, file.name)
                    : defaultImportPath(file.name);
                const requestedPath =
                    askPath && files.length === 1
                        ? window.prompt("导入到 EPUB 内部路径：", initialPath)
                        : initialPath;
                if (!requestedPath) continue;
                const path = uniqueResourcePath(requestedPath);
                const mediaType = file.type || guessWebEpubMediaType(path);
                const addToSpine =
                    mediaType === "application/xhtml+xml" &&
                    (files.length === 1
                        ? window.confirm("将该 XHTML 加入阅读顺序？")
                        : /\/text\//i.test(path));
                status = `正在导入资源 ${path}`;
                const entry = await addWebEpubResource(doc, {
                    path,
                    content: await file.arrayBuffer(),
                    mediaType,
                    addToSpine,
                    afterPath: selectedFile?.path,
                });
                importedPaths.push(path);
                expandParentFolders(path);
                lastEntry = entry;
            }
            doc = doc;
            documentDirty = true;
            await loadCoverPreview();
            await loadFileTitles();
            if (lastEntry && files.length === 1) {
                if (lastEntry.editable) await openFile(lastEntry, true);
                else if (lastEntry.kind === "image")
                    await openImage(lastEntry, true);
                else selectedTreePath = lastEntry.path;
            } else {
                await syncOpenOpfSource();
            }
            status =
                importedPaths.length > 0
                    ? `已导入 ${importedPaths.length} 个资源。`
                    : "未导入资源。";
        } catch (error) {
            status = `导入资源失败：${String(error)}`;
        } finally {
            busy = false;
            pendingImportFolder = "";
            if (resourceInput) resourceInput.value = "";
        }
    }

    async function onResourceImport(event: Event) {
        const files = Array.from(
            (event.currentTarget as HTMLInputElement).files || [],
        );
        if (!doc || files.length === 0) return;
        await importResourceFiles(
            files,
            pendingImportFolder,
            !pendingImportFolder,
        );
    }

    /**
     * 文件树「更多操作」浮层的尺寸。
     *
     * 这里是**外框**尺寸，不是 CSS 里的 width:104px：104 是内容宽，还要加
     * 左右各 6px 内边距和 1px 边框，实测外框 118px。以前按 104 算最大 left，
     * 点在屏幕最右边的按钮上时浮层右边缘会超出视口 14px（390 宽下量到
     * right=396），「删除」两个字被切掉一截。
     *
     * 高度随布局变：桌面两项 30px → 78px；手机抬到触控尺寸后是 106px。
     * 浮层是点出来之后才挂载的，量不到 DOM，只能按 CSS 预算。
     */
    const FILE_MENU_WIDTH = 120;

    function fileMenuHeight() {
        return phoneLayout ? 112 : 88;
    }

    function openFileMenu(node: WebFileTreeNode, event: MouseEvent) {
        if (node.file_type === "folder" || !node.path) return;
        event.stopPropagation();
        // 桌面左栏最宽 340px，点了永远在屏内；手机文件树铺满整屏，
        // 「更多操作」按钮是行内最后一个元素，点它在 x≈屏宽-27，
        // 不收敛的话浮层大半落在视口外，重命名/删除点不到。
        // 桌面保持原位置（左栏最宽 340px、菜单贴着鼠标），只在手机形态收敛。
        if (!phoneLayout) {
            fileMenu = { path: node.path, x: event.clientX, y: event.clientY };
            return;
        }
        const maxX = window.innerWidth - FILE_MENU_WIDTH - 8;
        // 不能按 innerHeight 收敛：底部导航（z-index 60）压在浮层（z-index 30）
        // 上面，滚到文件树底部时点最后几行的「更多」，「删除」正好落在导航
        // 底下 —— 看得见却点不中。按可见内容区的底边算。
        // /mobile 子树没有底部导航，回退到窗口底边。
        const nav = document.querySelector(".m-nav");
        const bottom = nav ? nav.getBoundingClientRect().top : window.innerHeight;
        const maxY = bottom - fileMenuHeight() - 8;
        fileMenu = {
            path: node.path,
            x: Math.max(8, Math.min(event.clientX, maxX)),
            y: Math.max(8, Math.min(event.clientY, maxY)),
        };
    }

    function closeFileMenu() {
        fileMenu = null;
    }

    async function deleteResourcePath(path: string) {
        if (!doc || busy) return;
        const current = fileByPath(path);
        if (!current) return;
        const currentPath = current.path;
        if (currentPath === doc.opfPath) {
            status = "不能删除 OPF 核心文件。";
            return;
        }
        const ok = window.confirm(
            `确认删除资源？\n${currentPath}\n\n会同步移除 OPF manifest/spine 引用，导出后生效。`,
        );
        if (!ok) return;
        busy = true;
        status = `正在删除 ${currentPath}`;
        try {
            await deleteWebEpubResource(doc, currentPath);
            doc = doc;
            documentDirty = true;
            if (selectedFile?.path === currentPath) {
                selectedFile = null;
                editorText = "";
                savedEditorText = "";
                previewHtml = "";
                revokePreviewUrls();
            }
            if (selectedImage?.path === currentPath) {
                selectedImage = null;
                revokeImagePreviewUrl();
            }
            selectedTreePath = "";
            await loadCoverPreview();
            await loadFileTitles();
            status = `已删除 ${currentPath}`;
        } catch (error) {
            status = `删除失败：${String(error)}`;
        } finally {
            busy = false;
            closeFileMenu();
        }
    }

    function renamedPathForInput(path: string, input: string) {
        const value = input.trim().replace(/[\\:*?"<>|]+/g, "_");
        if (!value) return "";
        if (value.includes("/")) return normalizeZipPath(value);
        return normalizeZipPath(`${dirname(path)}${value}`);
    }

    async function renameResourcePath(path: string) {
        if (!doc || busy) return;
        const current = fileByPath(path);
        if (!current) return;
        if (path === doc.opfPath) {
            status = "不能重命名 OPF 核心文件。";
            return;
        }
        const requestedName = window.prompt(
            "新的文件名或 EPUB 内部路径：",
            current.name,
        );
        if (requestedName === null) return;
        const nextPath = renamedPathForInput(path, requestedName);
        if (!nextPath || nextPath === path) return;
        if (selectedFile?.path === path && dirty) stageCurrentFile();
        busy = true;
        status = `正在重命名 ${path}`;
        try {
            const renamed = await renameWebEpubResource(doc, path, nextPath);
            doc = doc;
            documentDirty = true;
            if (selectedFile?.path === path) {
                selectedFile = renamed || fileByPath(nextPath);
                selectedTreePath = nextPath;
            } else if (selectedImage?.path === path) {
                selectedImage = renamed || fileByPath(nextPath);
                selectedTreePath = nextPath;
            } else if (selectedTreePath === path) {
                selectedTreePath = nextPath;
            }
            expandParentFolders(nextPath);
            await loadCoverPreview();
            await loadFileTitles();
            await syncOpenOpfSource();
            status = `已重命名为 ${nextPath}`;
        } catch (error) {
            status = `重命名失败：${String(error)}`;
        } finally {
            busy = false;
            closeFileMenu();
        }
    }

    async function saveMetadata() {
        if (!doc || busy || !metadataDirty) return;
        if (selectedFile?.path === doc.opfPath && dirty) {
            const ok = window.confirm(
                "当前 OPF 源码有未暂存修改，保存元数据会重新写入 OPF。是否继续？",
            );
            if (!ok) return;
        }
        busy = true;
        status = "正在写入 OPF 元数据...";
        try {
            const saved = await updateWebEpubMetadata(doc, metadataDraft);
            documentDirty = true;
            metadataDraft = { ...saved };
            metadataDirty = false;
            if (selectedFile?.path === doc.opfPath) {
                editorText = await readWebEpubText(doc, doc.opfPath);
                savedEditorText = editorText;
                previewHtml = "";
                revokePreviewUrls();
            }
            status = "元数据已写入，导出 EPUB 后生效。";
        } catch (error) {
            status = `元数据保存失败：${String(error)}`;
        } finally {
            busy = false;
        }
    }

    async function exportEpub() {
        if (!doc || busy) return;
        if (dirty) stageCurrentFile();
        busy = true;
        status = "正在重新打包 EPUB...";
        try {
            const blob = await exportWebEpubBlob(doc);
            const fileName = outputFileName();
            // 隐藏锚点 + 延后回收 URL，两件事都在这个共用函数里
            // （手机 WebView 上同步 revoke 会导致"点了导出没反应"）
            downloadBrowserBlob(blob, fileName);
            documentDirty = false;
            status = `已导出 ${fileName}`;
        } catch (error) {
            status = `导出失败：${String(error)}`;
        } finally {
            busy = false;
        }
    }

    function outputFileName() {
        if (!doc) return "edited.epub";
        const stem = doc.fileName.replace(/\.epub$/i, "") || "edited";
        return `${stem}.web-edited.epub`;
    }

    function fileSizeLabel(size: number) {
        if (!size) return "-";
        if (size < 1024) return `${size} B`;
        if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
        return `${(size / 1024 / 1024).toFixed(1)} MB`;
    }

    function dirname(path: string) {
        const index = path.lastIndexOf("/");
        return index >= 0 ? path.slice(0, index + 1) : "";
    }

    function resolveRelativePath(basePath: string, href: string) {
        const cleanHref = href.split("#")[0] || "";
        if (
            !cleanHref ||
            /^(data:|blob:|https?:|mailto:|tel:|#)/i.test(cleanHref)
        )
            return "";
        try {
            return normalizeZipPath(
                `${dirname(basePath)}${decodeURIComponent(cleanHref).replace(/^\//, "")}`,
            );
        } catch {
            return normalizeZipPath(
                `${dirname(basePath)}${cleanHref.replace(/^\//, "")}`,
            );
        }
    }

    function mimeForPath(path: string) {
        const file = fileByPath(path);
        if (file?.mediaType) return file.mediaType;
        const ext = path.split(".").pop()?.toLowerCase() || "";
        if (ext === "css") return "text/css";
        if (ext === "svg") return "image/svg+xml";
        if (ext === "png") return "image/png";
        if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
        if (ext === "webp") return "image/webp";
        if (ext === "gif") return "image/gif";
        if (ext === "woff") return "font/woff";
        if (ext === "woff2") return "font/woff2";
        return "application/octet-stream";
    }

    function previewableSpineFiles() {
        if (!doc) return [];
        return doc.spine
            .map((item) => fileByPath(item.manifest?.fullPath))
            .filter(
                (file): file is WebEpubFileEntry =>
                    !!file && file.kind === "xhtml" && file.editable,
            );
    }

    function currentPreviewIndex() {
        if (!selectedFile) return -1;
        return previewableSpineFiles().findIndex(
            (file) => file.path === selectedFile?.path,
        );
    }

    function previewNavLabel() {
        const files = previewableSpineFiles();
        const index = currentPreviewIndex();
        if (index < 0 || files.length === 0) return "未定位到书脊";
        return `${index + 1} / ${files.length}`;
    }

    function canPreviewRelative(delta: number) {
        const files = previewableSpineFiles();
        const index = currentPreviewIndex();
        if (files.length === 0) return false;
        const nextIndex = index < 0 ? 0 : index + delta;
        return nextIndex >= 0 && nextIndex < files.length;
    }

    function canAdjustReaderFont(delta: number) {
        const nextSize = readerPreviewFontSize + delta;
        return nextSize >= 14 && nextSize <= 26;
    }

    async function adjustReaderFont(delta: number) {
        if (!readerMode || busy || !canAdjustReaderFont(delta)) return;
        readerPreviewFontSize += delta;
        if (selectedFile?.kind === "xhtml") await refreshPreviewFromEditor();
    }

    function toggleReaderToc() {
        if (!readerMode) return;
        readerTocCollapsed = !readerTocCollapsed;
    }

    function revokePreviewUrls() {
        for (const url of previewBlobUrls) URL.revokeObjectURL(url);
        previewBlobUrls = [];
    }

    function makePreviewUrl(blob: Blob, type: string, urls: string[]) {
        const url = URL.createObjectURL(
            blob.type === type ? blob : new Blob([blob], { type }),
        );
        urls.push(url);
        return url;
    }

    function rewriteCssUrls(
        css: string,
        cssPath: string,
        urlByPath: Map<string, string>,
    ) {
        return css.replace(
            /url\(\s*(['"]?)(.*?)\1\s*\)/gi,
            (match, quote, rawUrl) => {
                const targetPath = resolveRelativePath(cssPath, rawUrl.trim());
                const url = targetPath ? urlByPath.get(targetPath) : "";
                return url ? `url(${quote || ""}${url}${quote || ""})` : match;
            },
        );
    }

    const XLINK_NS = "http://www.w3.org/1999/xlink";

    /** 把一处相对引用换成 blob URL；换不上就原样返回 */
    function previewUrlFor(
        htmlPath: string,
        value: string,
        urlByPath: Map<string, string>,
    ) {
        const targetPath = resolveRelativePath(htmlPath, value);
        return targetPath ? urlByPath.get(targetPath) || "" : "";
    }

    /** srcset：逗号分隔的「地址 描述符」，只换地址，描述符（2x / 640w）留着 */
    function rewriteSrcset(
        value: string,
        htmlPath: string,
        urlByPath: Map<string, string>,
    ) {
        const parts = value.split(",");
        let changed = false;
        const rewritten = parts.map((part) => {
            const trimmed = part.trim();
            if (!trimmed) return part;
            const spaceIndex = trimmed.search(/\s/);
            const url = spaceIndex < 0 ? trimmed : trimmed.slice(0, spaceIndex);
            const descriptor = spaceIndex < 0 ? "" : trimmed.slice(spaceIndex);
            const replacement = previewUrlFor(htmlPath, url, urlByPath);
            if (!replacement || replacement === url) return part;
            changed = true;
            return `${replacement}${descriptor}`;
        });
        return changed ? rewritten.join(", ") : "";
    }

    /**
     * 把章节里所有相对资源换成 blob URL。
     *
     * 必须覆盖的几处（少一处就有一类书显示空白）：
     * - 普通 src/href/poster；
     * - SVG 的 `xlink:href`：封面包装页 `<svg><image xlink:href="../Images/
     *   cover.jpg"/></svg>` 就是这样写的。它在 DOM 里是**带命名空间**的属性，
     *   getAttribute("href") 读不到，得走 getAttributeNS；
     * - srcset（浏览器优先用它，漏改的话即使 src 换好了也加载失败）；
     * - `<object data>`；
     * - 行内 style 属性里的 url()，以及页面里 style 标签内的 url()。
     */
    function rewriteHtmlResources(
        source: string,
        htmlPath: string,
        urlByPath: Map<string, string>,
    ) {
        const htmlDoc = new DOMParser().parseFromString(source, "text/html");
        const attrs = ["src", "href", "poster", "data"];
        for (const element of Array.from(htmlDoc.querySelectorAll("*"))) {
            for (const attr of attrs) {
                const value = element.getAttribute(attr);
                if (!value) continue;
                const url = previewUrlFor(htmlPath, value, urlByPath);
                if (url) element.setAttribute(attr, url);
            }

            // 带命名空间的 xlink:href：serialize 时保留原前缀，iframe 能认
            const xlinkHref = element.getAttributeNS(XLINK_NS, "href");
            if (xlinkHref) {
                const url = previewUrlFor(htmlPath, xlinkHref, urlByPath);
                if (url) element.setAttributeNS(XLINK_NS, "xlink:href", url);
            } else if (element.hasAttribute("xlink:href")) {
                const raw = element.getAttribute("xlink:href") || "";
                const url = previewUrlFor(htmlPath, raw, urlByPath);
                if (url) element.setAttribute("xlink:href", url);
            }

            const srcset = element.getAttribute("srcset");
            if (srcset) {
                const rewritten = rewriteSrcset(srcset, htmlPath, urlByPath);
                if (rewritten) element.setAttribute("srcset", rewritten);
            }

            const inlineStyle = element.getAttribute("style");
            if (inlineStyle && /url\s*\(/i.test(inlineStyle)) {
                element.setAttribute(
                    "style",
                    rewriteCssUrls(inlineStyle, htmlPath, urlByPath),
                );
            }
        }
        for (const style of Array.from(htmlDoc.querySelectorAll("style"))) {
            const css = style.textContent || "";
            if (!/url\s*\(/i.test(css)) continue;
            style.textContent = rewriteCssUrls(css, htmlPath, urlByPath);
        }
        const style = htmlDoc.createElement("style");
        // 阅读模式的字号调节要生效，只能由外层注入；普通预览保持书的原样
        const readerFontRule = readerMode
            ? `body { font-size: ${readerPreviewFontSize}px; }`
            : "";
        // 底色在生成 srcdoc 时按当前主题写死：交给运行时的 applyPaperTheme
        // 去改已经晚了，iframe 会先按浅色画一帧（切页面时整屏闪白）
        const dark = resolveUiTheme(appSettings.uiTheme) === "dark";
        const paperBg = paperBackground(dark);
        const paperText = paperTextColor(dark);
        // 浅色模式下不注入任何底色：安装版 PreviewPane 只注滚动条和深色纸色，
        // 书页透出 iframe 自身的 --color-surface（纯白）。这里多铺一层米色纸
        // 就会让两边看起来一个是米白一个是纯白。
        const paperRule = dark ? `body { background: ${paperBg}; }` : "";
        style.textContent = `
      /* 这里只放"不这样做就会坏"的规则，排版一律交给书自己：
         安装版除了滚动条什么都不注入，web 端多一条 margin/padding 重置
         或 img 宽度上限，同一本书两边的换行位置和图片尺寸就会不一样。 */
      /* 深色纸色只铺 body，html 必须保持透明 —— html 一旦有不透明底色，
         body 的背景就不再向画布传播，整页背景图（如 body.Fullscreen1
         的 1080x2347 底图）会被裁进只有几十像素高的 body 盒子里，
         预览只剩顶部一条。 */
      html { overflow-x: hidden; }
      body { overflow-x: hidden; }
      ${paperRule}
      ${readerFontRule}
      /* 滚动统一交给外层 .preview-body。iframe 在过渡高度上会短暂出现内部
         滚动条，它把正文宽度挤窄 ~15px、文字重排后内容反而更高，
         量出的高度永远追不上，那条滚动条就会「闪出来不消失」。 */
      html, body { scrollbar-width: none !important; -ms-overflow-style: none !important; }
      ::-webkit-scrollbar { width: 0 !important; height: 0 !important; display: none !important; }
      .tepub-footnote-backdrop { position: fixed; inset: 0; z-index: 9998; display: flex; align-items: center; justify-content: center; padding: 20px; box-sizing: border-box; background: rgba(15, 23, 42, .72); }
      .tepub-footnote-dialog { position: relative; width: min(90vw, 720px); max-height: 88vh; overflow: auto; padding: 16px; box-sizing: border-box; border-radius: 8px; background: ${dark ? paperBg : "#fff"}; color: ${paperText}; box-shadow: 0 16px 48px rgba(0, 0, 0, .32); }
      .tepub-footnote-dialog img { display: block; max-width: 100%; max-height: calc(88vh - 40px); width: auto; height: auto; margin: 0 auto; }
      .tepub-footnote-dialog figure { margin: 0; }
    `;
        htmlDoc.head.prepend(style);
        const script = htmlDoc.createElement("script");
        script.textContent = `
      document.addEventListener("click", function (event) {
        var clicked = event.target;
        var trigger = clicked && clicked.closest ? clicked.closest("a") : null;
        if (!trigger) return;
        if (trigger.classList.contains("duokan-footnote-back")) {
          event.preventDefault();
          event.stopPropagation();
          var activeBackdrop = trigger.closest(".tepub-footnote-backdrop");
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
        var old = document.querySelector(".tepub-footnote-backdrop");
        if (old) old.remove();
        var backdrop = document.createElement("div");
        backdrop.className = "tepub-footnote-backdrop";
        var dialog = document.createElement("div");
        dialog.className = "tepub-footnote-dialog";
        dialog.innerHTML = note.innerHTML;
        backdrop.appendChild(dialog);
        backdrop.addEventListener("click", function (closeEvent) {
          if (closeEvent.target === backdrop) backdrop.remove();
        });
        document.body.appendChild(backdrop);
      }, true);
    `;
        htmlDoc.head.appendChild(script);
        return `<!doctype html>${htmlDoc.documentElement.outerHTML}`;
    }

    function collectHtmlPreviewRefs(source: string, htmlPath: string) {
        const assets = new Set<string>();
        const css = new Set<string>();
        const htmlDoc = new DOMParser().parseFromString(source, "text/html");
        const addPath = (path: string) => {
            const file = fileByPath(path);
            if (!file || file.path === htmlPath) return;
            if (file.kind === "css") css.add(file.path);
            else assets.add(file.path);
        };
        for (const element of Array.from(htmlDoc.querySelectorAll("*"))) {
            // 属性名必须和 rewriteHtmlResources 保持一致：这里漏收集，
            // 那边就没有 blob URL 可换，图照样是破的
            for (const attr of ["src", "href", "poster", "data"]) {
                const value = element.getAttribute(attr);
                if (!value) continue;
                const path = resolveRelativePath(htmlPath, value);
                if (path) addPath(path);
            }
            const xlinkHref =
                element.getAttributeNS(XLINK_NS, "href") ||
                element.getAttribute("xlink:href") ||
                "";
            if (xlinkHref) {
                const path = resolveRelativePath(htmlPath, xlinkHref);
                if (path) addPath(path);
            }
            const srcset = element.getAttribute("srcset") || "";
            if (srcset) {
                for (const part of srcset.split(",")) {
                    const trimmed = part.trim();
                    if (!trimmed) continue;
                    const url = trimmed.split(/\s+/)[0];
                    const path = resolveRelativePath(htmlPath, url);
                    if (path) addPath(path);
                }
            }
            const inlineStyle = element.getAttribute("style");
            if (inlineStyle) {
                for (const path of collectCssUrlPaths(inlineStyle, htmlPath))
                    addPath(path);
            }
        }
        for (const style of Array.from(htmlDoc.querySelectorAll("style"))) {
            for (const path of collectCssUrlPaths(
                style.textContent || "",
                htmlPath,
            ))
                addPath(path);
        }
        return { assets, css };
    }

    function collectCssUrlPaths(css: string, cssPath: string) {
        const paths = new Set<string>();
        css.replace(
            /url\(\s*(['"]?)(.*?)\1\s*\)/gi,
            (_match, _quote, rawUrl) => {
                const path = resolveRelativePath(cssPath, rawUrl.trim());
                if (path) paths.add(path);
                return "";
            },
        );
        css.replace(
            /@import\s+(?:url\(\s*)?(['"])(.*?)\1\s*\)?/gi,
            (_match, _quote, rawUrl) => {
                const path = resolveRelativePath(cssPath, rawUrl.trim());
                if (path) paths.add(path);
                return "";
            },
        );
        return paths;
    }

    async function buildPreviewHtmlFor(
        currentFile: WebEpubFileEntry,
        source: string,
    ) {
        if (!doc) return { html: "", urls: [] };
        const urls: string[] = [];
        const urlByPath = new Map<string, string>();
        const refs = collectHtmlPreviewRefs(source, currentFile.path);
        const cssContent = new Map<string, string>();
        const cssQueue = Array.from(refs.css);
        const seenCss = new Set<string>();

        while (cssQueue.length > 0) {
            const path = cssQueue.shift()!;
            if (seenCss.has(path)) continue;
            seenCss.add(path);
            const file = fileByPath(path);
            const zipFile = file ? doc.zip.file(file.path) : null;
            if (!file || !zipFile) continue;
            const css = await zipFile.async("string");
            cssContent.set(file.path, css);
            for (const refPath of collectCssUrlPaths(css, file.path)) {
                const refFile = fileByPath(refPath);
                if (!refFile) continue;
                if (refFile.kind === "css") cssQueue.push(refFile.path);
                else refs.assets.add(refFile.path);
            }
        }

        for (const path of refs.assets) {
            const file = fileByPath(path);
            if (!file) continue;
            const zipFile = doc.zip.file(file.path);
            if (!zipFile) continue;
            const blob = await zipFile.async("blob");
            urlByPath.set(
                file.path,
                makePreviewUrl(blob, mimeForPath(file.path), urls),
            );
        }

        for (const [path, css] of cssContent) {
            const rewritten = rewriteCssUrls(css, path, urlByPath);
            urlByPath.set(
                path,
                makePreviewUrl(
                    new Blob([rewritten], { type: "text/css" }),
                    "text/css",
                    urls,
                ),
            );
        }

        return {
            html: rewriteHtmlResources(source, currentFile.path, urlByPath),
            urls,
        };
    }

    async function buildPreviewHtml() {
        if (!doc || !selectedFile) return "";
        const result = await buildPreviewHtmlFor(selectedFile, editorText);
        revokePreviewUrls();
        previewBlobUrls = result.urls;
        return result.html;
    }

    async function previewCurrentFile() {
        if (!selectedFile || busy) return;
        if (selectedFile.kind !== "xhtml") {
            status = "当前文件不是 XHTML/HTML 章节，暂不能预览。";
            return;
        }
        busy = true;
        status = `正在生成 ${selectedFile.path} 的预览`;
        revokePreviewUrls();
        try {
            previewHtml = await buildPreviewHtml();
            rightTab = "preview";
            status = `已生成 ${selectedFile.path} 的预览`;
        } catch (error) {
            previewHtml = "";
            status = `预览失败：${String(error)}`;
            revokePreviewUrls();
        } finally {
            busy = false;
        }
    }

    async function previewFile(file: WebEpubFileEntry) {
        if (busy || file.kind !== "xhtml") return;
        if (selectedFile?.path === file.path) {
            await refreshPreviewFromEditor();
            return;
        }
        await openFile(file);
    }

    async function previewRelative(delta: number) {
        const files = previewableSpineFiles();
        if (files.length === 0) return;
        const index = currentPreviewIndex();
        const nextIndex = index < 0 ? 0 : index + delta;
        const target = files[nextIndex];
        if (target) await previewFile(target);
    }

    async function previewPath(path: string | undefined) {
        const file = fileByPath(path);
        if (!file) return false;
        if (file.kind === "xhtml") {
            await previewFile(file);
            return true;
        }
        return await openPath(path);
    }

    async function handleTocSelect(path: string) {
        const opened = readerMode
            ? await previewPath(path)
            : await openPath(path);
        if (opened) await revealTreePath(path);
        // 手机上点目录里的章节，期望是"开始读这一章"，不是"跳回文件列表找它"
        if (opened && phoneLayout) selectPane("preview");
    }

    function addToHistory(value: string, history: string[]) {
        const clean = value.trim();
        if (!clean) return history;
        return [clean, ...history.filter((item) => item !== clean)].slice(
            0,
            20,
        );
    }

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
            findHistory = JSON.parse(
                localStorage.getItem(EPUB_FIND_HISTORY_KEY) || "[]",
            );
            replaceHistory = JSON.parse(
                localStorage.getItem(EPUB_REPLACE_HISTORY_KEY) || "[]",
            );
            if (findHistory[0]) findPattern = findHistory[0];
            if (replaceHistory[0]) replacePattern = replaceHistory[0];
        } catch {
            findHistory = [];
            replaceHistory = [];
        }
    }

    function rememberSearchInputs() {
        if (findPattern) findHistory = addToHistory(findPattern, findHistory);
        if (replacePattern)
            replaceHistory = addToHistory(replacePattern, replaceHistory);
        saveSearchHistory();
    }

    // 点输入框本身不再切换历史面板，避免和 ▾ 按钮的语义重复
    function closeHistoryPanels() {
        showFindHistory = false;
        showReplaceHistory = false;
    }

    function selectFindHistory(value: string) {
        findPattern = value;
        showFindHistory = false;
    }

    function selectReplaceHistory(value: string) {
        replacePattern = value;
        showReplaceHistory = false;
    }

    function removeFromFindHistory(index: number) {
        findHistory = findHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

    function removeFromReplaceHistory(index: number) {
        replaceHistory = replaceHistory.filter((_, i) => i !== index);
        saveSearchHistory();
    }

    function closeFindReplace() {
        showFindHistory = false;
        showReplaceHistory = false;
        currentMatchInfo = null;
        searchMessage = "";
    }

    function getFilesInScope() {
        if (!doc) return [];
        const selected = Array.from(multiSelectedFiles)
            .map((path) => fileByPath(path))
            .filter((file): file is WebEpubFileEntry => !!file);
        return filterFilesInScope(searchScope, {
            allFiles: doc.files,
            currentFile: selectedFile,
            // Web 版编辑器没有多标签，一次只打开一个文件，
            // 所以「已打开」范围就等于当前文件。
            openFiles: selectedFile ? [selectedFile] : [],
            selectedFiles: selected,
            isHtml: (file) => file.kind === "xhtml",
            isEditable: (file) => file.editable,
        });
    }

    function currentFindQuery(): FindQuery {
        return { pattern: findPattern, mode: isRegex ? "regex" : "text" };
    }

    async function contentForFile(file: WebEpubFileEntry) {
        if (!doc) return "";
        if (selectedFile?.path === file.path) return currentEditorContent();
        return readWebEpubText(doc, file.path);
    }

    async function selectMatch(
        file: WebEpubFileEntry,
        from: number,
        to: number,
    ) {
        if (selectedFile?.path !== file.path) {
            const opened =
                file.kind === "image"
                    ? await openImage(file)
                    : await openFile(file);
            if (!opened) return;
        }
        currentMatchInfo = { path: file.path, from, to };
        await tick();
        selectEditorRange(from, to);
    }

    async function findNextInDirection(direction: "down" | "up") {
        if (!doc || !selectedFile) return;
        if (!findPattern) {
            searchMessage = "请输入查找内容";
            return;
        }
        rememberSearchInputs();
        const files = getFilesInScope();
        if (files.length === 0) {
            searchMessage = "没有可搜索文件";
            return;
        }
        const currentIndex = Math.max(
            0,
            files.findIndex((file) => file.path === selectedFile?.path),
        );
        const ordered =
            direction === "up"
                ? [
                      ...files.slice(0, currentIndex + 1).reverse(),
                      ...files.slice(currentIndex + 1).reverse(),
                  ]
                : [
                      ...files.slice(currentIndex),
                      ...files.slice(0, currentIndex),
                  ];

        try {
            const query = currentFindQuery();
            for (const file of ordered) {
                const content = await contentForFile(file);
                const isCurrent = file.path === selectedFile.path;
                const cursor = isCurrent
                    ? currentMatchInfo?.path === file.path
                        ? direction === "up"
                            ? currentMatchInfo.from - 1
                            : currentMatchInfo.to
                        : currentEditorCursor(direction, content.length)
                    : direction === "up"
                      ? content.length
                      : 0;
                let match = findNextMatch(content, query, cursor, direction);
                if (!match && wrapAround && isCurrent && files.length === 1) {
                    match = findNextMatch(
                        content,
                        query,
                        direction === "up" ? content.length : 0,
                        direction,
                    );
                }
                if (match) {
                    await selectMatch(file, match.from, match.to);
                    searchMessage = `已定位 ${file.name}`;
                    return;
                }
                if (!wrapAround && isCurrent) break;
            }
        } catch {
            searchMessage = "正则表达式错误";
            return;
        }
        searchMessage = "未找到匹配内容";
    }

    async function findNext() {
        await findNextInDirection(searchDirection);
    }

    async function findPrev() {
        await findNextInDirection(searchDirection === "down" ? "up" : "down");
    }

    async function performReplace() {
        if (
            !selectedFile ||
            !currentMatchInfo ||
            currentMatchInfo.path !== selectedFile.path
        ) {
            searchMessage = "请先查找";
            return;
        }
        const before = editorText.slice(0, currentMatchInfo.from);
        const after = editorText.slice(currentMatchInfo.to);
        editorText = `${before}${replacePattern}${after}`;
        currentMatchInfo = {
            path: selectedFile.path,
            from: currentMatchInfo.from,
            to: currentMatchInfo.from + replacePattern.length,
        };
        rememberSearchInputs();
        await tick();
        selectEditorRange(currentMatchInfo.from, currentMatchInfo.to);
        searchMessage = "已替换";
        await findNext();
    }

    async function performReplaceAll() {
        if (!doc || !findPattern) {
            searchMessage = "请输入查找内容";
            return;
        }
        rememberSearchInputs();
        let total = 0;
        try {
            const query = currentFindQuery();
            for (const file of getFilesInScope()) {
                const original = await contentForFile(file);
                const replaced = replaceAllInContent(
                    original,
                    query,
                    replacePattern,
                );
                if (replaced.count === 0) continue;
                total += replaced.count;
                updateWebEpubText(doc, file.path, replaced.content);
                documentDirty = true;
                if (selectedFile?.path === file.path) {
                    editorText = replaced.content;
                    savedEditorText = replaced.content;
                }
            }
            if (selectedFile?.kind === "xhtml")
                await refreshPreviewFromEditor();
            await loadFileTitles();
            currentMatchInfo = null;
            searchMessage = `已替换 ${total} 处`;
        } catch (error) {
            searchMessage = `替换失败：${String(error)}`;
        }
    }

    async function countMatches() {
        if (!findPattern) {
            searchMessage = "请输入查找内容";
            return;
        }
        let total = 0;
        try {
            const query = currentFindQuery();
            for (const file of getFilesInScope()) {
                const content = await contentForFile(file);
                total += countMatchesInContent(content, query);
            }
            searchMessage = `共 ${total} 处匹配`;
        } catch {
            searchMessage = "正则表达式错误";
        }
    }
</script>

<svelte:head>
    <title>{headTitle}</title>
</svelte:head>

<svelte:window on:click={closeFileMenu} on:beforeunload={handleBeforeUnload} />

<div class="web-epub-page">
    <input
        bind:this={fileInput}
        class="file-input"
        type="file"
        accept=".epub,application/epub+zip"
        on:change={onFileChange}
    />
    <input
        bind:this={resourceInput}
        class="file-input"
        type="file"
        multiple
        on:change={onResourceImport}
    />

    {#if !doc}
        <div class="import-shell">
            <ToolImportPage
                mark={readerMode ? "READ" : "EPUB"}
                kicker={readerMode ? "EPUB READER" : "EPUB EDITOR"}
                title={readerMode ? "EPUB 阅读器" : "EPUB 编辑器"}
                description={importDescription}
                privacy={`文件仅在当前设备中读取，选择后继续进入原有${readerMode ? "阅读" : "编辑"}页面。`}
                outputLabel={readerMode ? "阅读格式" : "编辑格式"}
                outputValue="EPUB"
                features={readerMode
                    ? [
                          {
                              title: "目录导航",
                              detail: "读取 NAV 与 NCX 章节目录",
                          },
                          {
                              title: "正文预览",
                              detail: "按阅读顺序打开 EPUB 内容",
                          },
                          { title: "本地阅读", detail: "文件不会上传到服务器" },
                      ]
                    : [
                          {
                              title: "文件结构",
                              detail: "浏览并管理 EPUB 内部资源",
                          },
                          {
                              title: "内容编辑",
                              detail: "编辑 XHTML、CSS 与元数据",
                          },
                          {
                              title: "重新导出",
                              detail: "处理完成后生成新的 EPUB",
                          },
                      ]}
                prompt={importHeading}
                hint={readerMode
                    ? "选择 EPUB 后进入原有阅读器"
                    : "选择 EPUB 后进入原有编辑器"}
                actionLabel={importAction}
                accept=".epub,application/epub+zip"
                {busy}
                on:select={pickFile}
                on:files={handleImportFiles}
            />
        </div>
    {:else}
        <main
            class="workspace"
            class:reader-workspace={readerMode}
            class:reader-toc-collapsed={readerMode && readerTocCollapsed}
            class:phone-stack={phoneLayout}
            data-pane={mobilePane}
        >
            {#if phoneLayout}
                <!-- 手机上三栏同时在屏幕上只能各分到 1/3 高，谁都干不了活。
                     收成"一次只看一栏"，用这条页签切换。
                     桌面不渲染这个节点，三个面板同时可见，布局完全不变。 -->
                <div class="phone-pane-bar" role="tablist" aria-label="编辑区切换">
                    <button
                        type="button"
                        role="tab"
                        aria-selected={mobilePane === "tree"}
                        class:active={mobilePane === "tree"}
                        on:click={() => selectPane("tree")}
                        >{readerMode ? "目录" : "文件"}</button
                    >
                    {#if !readerMode}
                        <button
                            type="button"
                            role="tab"
                            aria-selected={mobilePane === "editor"}
                            class:active={mobilePane === "editor"}
                            on:click={showEditorPane}>编辑</button
                        >
                    {/if}
                    <button
                        type="button"
                        role="tab"
                        aria-selected={mobilePane === "preview"}
                        class:active={mobilePane === "preview"}
                        on:click={() => selectPane("preview")}
                        >{readerMode ? "阅读" : "预览"}</button
                    >
                </div>
                <!-- status 一直有值却从来没被渲染过：桌面版里"已导入 xx 个文件"
                     "已替换 3 处"这类反馈只写进变量，用户看不到。手机没有状态栏
                     可以瞄，这里给它一条常驻的 aria-live 细线。 -->
                <div class="phone-status" role="status" aria-live="polite">
                    <span title={status}>{status}</span>
                </div>
            {/if}

            <aside class="left-panel">
                {#if readerMode}
                    <div class="tree-header">
                        <h2>目录</h2>
                        <div class="tree-actions">
                            <button
                                type="button"
                                on:click={pickFile}
                                disabled={busy}
                                title="重新选择 EPUB">换书</button
                            >
                        </div>
                    </div>
                {/if}

                {#if readerMode}
                    <div class="toc-container reader-toc-container">
                        {#if tocTree.length === 0}
                            <p class="panel-empty">未读取到 NAV / NCX 目录。</p>
                        {:else}
                            <div class="toc-list">
                                {#each tocTree as item (item.id)}
                                    <TocNode
                                        {item}
                                        selectedSrc={selectedFile?.path || ""}
                                        onSelect={handleTocSelect}
                                    />
                                {/each}
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div class="tabs">
                        <button
                            type="button"
                            class:active={activeTab === "files"}
                            on:click={() => (activeTab = "files")}>文件</button
                        >
                        <button
                            type="button"
                            class:active={activeTab === "metadata"}
                            on:click={() => (activeTab = "metadata")}
                            >信息</button
                        >
                    </div>

                    {#if activeTab === "files"}
                        {#if phoneLayout}
                            <!-- 一本书动辄几百个资源，手机上文件树只有一屏高，
                                 不筛选就只能靠盲滚。桌面用不上（三栏同屏，
                                 树和编辑器可以对着看），所以只在手机渲染。 -->
                            <div class="phone-tree-filter">
                                <input
                                    type="search"
                                    bind:value={phoneTreeFilter}
                                    placeholder="筛选路径 / 文件名"
                                    autocomplete="off"
                                    autocapitalize="off"
                                    spellcheck="false"
                                    aria-label="筛选文件"
                                />
                                {#if phoneTreeFilter.trim()}
                                    <button
                                        type="button"
                                        on:click={() => (phoneTreeFilter = "")}
                                        aria-label="清空筛选">×</button
                                    >
                                {/if}
                            </div>
                        {/if}
                        <div class="tree-content">
                            {#each visibleFileTree as node (node.path)}
                                <FileTreeItem
                                    {node}
                                    expandedFolders={visibleExpandedFolders}
                                    selectedFile={selectedTreeNode}
                                    {multiSelectedFiles}
                                    {toggleFolder}
                                    selectFile={selectTreeFile}
                                    {getFileIcon}
                                    {getFileDescription}
                                    onFolderAdd={pickFolderResourceImport}
                                    onFolderDrop={dropFilesIntoFolder}
                                    onFileMenu={openFileMenu}
                                />
                            {/each}
                        </div>
                    {:else if activeTab === "metadata"}
                        <form
                            class="metadata-form"
                            on:submit|preventDefault={saveMetadata}
                        >
                            <div class="metadata-top">
                                <div class="metadata-title-fields">
                                    <label>
                                        <span>书名</span>
                                        <input
                                            type="text"
                                            bind:value={metadataDraft.title}
                                            on:input={markMetadataDirty}
                                            autocomplete="off"
                                        />
                                    </label>
                                    <label>
                                        <span>作者</span>
                                        <input
                                            type="text"
                                            bind:value={metadataDraft.creator}
                                            on:input={markMetadataDirty}
                                            autocomplete="off"
                                        />
                                    </label>
                                </div>
                                <div class="metadata-cover" title="封面预览">
                                    {#if coverPreviewUrl}
                                        <img
                                            src={coverPreviewUrl}
                                            alt="封面预览"
                                        />
                                    {:else}
                                        <span>无封面</span>
                                    {/if}
                                </div>
                            </div>
                            <label class="metadata-identifier">
                                <span>标识符</span>
                                <input
                                    type="text"
                                    bind:value={metadataDraft.identifier}
                                    on:input={markMetadataDirty}
                                    autocomplete="off"
                                />
                            </label>
                            <label class="full">
                                <span>简介</span>
                                <textarea
                                    bind:value={metadataDraft.description}
                                    on:input={markMetadataDirty}
                                    rows="5"></textarea>
                            </label>
                            <div class="metadata-meta">
                                <span title={doc.opfPath}
                                    >OPF：{doc.opfPath}</span
                                >
                                <span
                                    >{doc.files.length} 个文件，{textCount} 个可编辑，{imageCount}
                                    张图片</span
                                >
                            </div>
                            <div class="metadata-actions">
                                <button
                                    type="button"
                                    on:click={resetMetadataDraft}
                                    disabled={!metadataDirty || busy}
                                    >还原</button
                                >
                                <button
                                    type="submit"
                                    class="primary"
                                    disabled={!metadataDirty || busy}
                                    >保存元数据</button
                                >
                            </div>
                        </form>
                    {/if}
                {/if}
            </aside>

            {#if !readerMode}
                <section class="editor-panel">
                    <div class="editor-head">
                        <div class="editor-title">
                            <!-- 路径常被省略号截断，悬停给完整路径 -->
                            <strong
                                title={selectedFile?.path ||
                                    selectedImage?.path ||
                                    "未选择文件"}
                                >{selectedFile?.path ||
                                    selectedImage?.path ||
                                    "未选择文件"}</strong
                            >
                        </div>
                        <div class="editor-actions">
                            {#if phoneLayout}
                                <!-- 手机上「暂存 / 换行 / 字号 / 跳转 / 换书」全收进这里：
                                     标题行只有一行高，放五颗按钮会挤成麻将牌 -->
                                <button
                                    type="button"
                                    on:click={() => (phoneMoreOpen = true)}
                                    title="更多操作"
                                    aria-label="更多操作">…</button
                                >
                            {/if}
                            <button
                                type="button"
                                class="primary"
                                on:click={exportEpub}
                                disabled={busy}
                                title="导出 EPUB">导出</button
                            >
                            {#if !phoneLayout}
                                <button
                                    type="button"
                                    class="primary"
                                    on:click={stageCurrentFile}
                                    disabled={!selectedFile || !dirty || busy}
                                    >暂存</button
                                >
                            {/if}
                        </div>
                    </div>

                    {#if phoneLayout && selectedFile && dirty}
                        <!-- 手机没有「暂存」按钮，但未保存这件事必须一直看得见 -->
                        <div class="phone-dirty-hint" role="status"
                            >未暂存 · 点下方「保存」写入书内</div
                        >
                    {/if}

                    {#if selectedFile}
                        <div class="editor-content">
                            <EpubCodeEditor
                                bind:this={epubCodeEditorComponent}
                                doc={editorText}
                                language={getFileLanguage(selectedFile.kind)}
                                onChange={handleEditorChange}
                                onSave={stageCurrentFile}
                                onClick={(line) => {
                                    phoneLine = line;
                                    handleEditorClick(line);
                                }}
                                onSelectionChange={handleEditorSelection}
                                onCursorLine={(line) => (phoneLine = line)}
                                touchInput={touchInput}
                                onOpenSearch={() => {
                                    // 手机上没有常驻查找栏：先切到编辑栏并展开，
                                    // 否则焦点落在隐藏容器里的输入框上，光标不出现
                                    if (phoneLayout) {
                                        selectPane("editor");
                                        phoneFindOpen = true;
                                    }
                                    setTimeout(
                                        () =>
                                            document
                                                .getElementById(
                                                    "epub-search-input-fr",
                                                )
                                                ?.focus(),
                                        phoneLayout ? 120 : 50,
                                    );
                                }}
                            />
                        </div>
                    {:else if selectedImage}
                        <div class="editor-image-preview">
                            {#if imagePreviewUrl}
                                <img
                                    src={imagePreviewUrl}
                                    alt={selectedImage.name}
                                />
                            {:else}
                                <span>正在读取图片...</span>
                            {/if}
                        </div>
                    {:else}
                        <div class="editor-empty">
                            选择 XHTML、CSS、XML、文本或图片资源。
                        </div>
                    {/if}

                    {#if selectedFile && selectedFile.editable && (!phoneLayout || phoneFindOpen)}
                        <div class="find-replace-panel">
                            {#if phoneLayout}
                                <!-- 手机上的查找条是"用完就收"的：它占掉整屏一半，
                                     编辑区只剩三行。桌面保持常驻，一个节点都没删。 -->
                                <div class="phone-find-head">
                                    <span>查找替换</span>
                                    <button
                                        type="button"
                                        on:click={() => (phoneFindOpen = false)}
                                        aria-label="收起查找">收起</button
                                    >
                                </div>
                            {/if}
                            <div class="fr-row">
                                <span class="fr-label">查找:</span>
                                <div class="fr-input-wrapper">
                                    <input
                                        id="epub-search-input-fr"
                                        type="text"
                                        class="fr-input"
                                        bind:value={findPattern}
                                        on:keydown={(e) =>
                                            e.key === "Enter" && findNext()}
                                        on:focus={closeHistoryPanels}
                                        on:click={closeHistoryPanels}
                                        autocomplete="off"
                                    />
                                    <button
                                        type="button"
                                        class="fr-dropdown-btn"
                                        on:click={() => {
                                            showFindHistory = !showFindHistory;
                                            showReplaceHistory = false;
                                        }}
                                        title="显示查找历史">▾</button
                                    >
                                    {#if showFindHistory && findHistory.length > 0}
                                        <div class="fr-history-dropdown">
                                            {#each findHistory as item, i}
                                                <div class="fr-history-item">
                                                    <span
                                                        class="fr-history-text"
                                                        on:click={() =>
                                                            selectFindHistory(
                                                                item,
                                                            )}
                                                        on:keydown={(e) =>
                                                            e.key === "Enter" &&
                                                            selectFindHistory(
                                                                item,
                                                            )}
                                                        role="button"
                                                        tabindex="0"
                                                        >{item}</span
                                                    >
                                                    <button
                                                        type="button"
                                                        class="fr-history-del"
                                                        on:click|stopPropagation={() =>
                                                            removeFromFindHistory(
                                                                i,
                                                            )}
                                                        title="删除"
                                                        >&times;</button
                                                    >
                                                </div>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                                <div class="fr-actions fr-find-actions">
                                    <button
                                        type="button"
                                        class="fr-btn fr-btn-text"
                                        on:click={countMatches}
                                        title="计数">计数</button
                                    >
                                    <button
                                        type="button"
                                        class="fr-btn fr-btn-text"
                                        on:click={findPrev}
                                        title="上一个">上一个</button
                                    >
                                    <button
                                        type="button"
                                        class="fr-btn fr-btn-text"
                                        on:click={findNext}
                                        title="下一个">下一个</button
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
                                        on:focus={closeHistoryPanels}
                                        on:click={closeHistoryPanels}
                                        autocomplete="off"
                                    />
                                    <button
                                        type="button"
                                        class="fr-dropdown-btn"
                                        on:click={() => {
                                            showReplaceHistory =
                                                !showReplaceHistory;
                                            showFindHistory = false;
                                        }}
                                        title="显示替换历史">▾</button
                                    >
                                    {#if showReplaceHistory && replaceHistory.length > 0}
                                        <div class="fr-history-dropdown">
                                            {#each replaceHistory as item, i}
                                                <div class="fr-history-item">
                                                    <span
                                                        class="fr-history-text"
                                                        on:click={() =>
                                                            selectReplaceHistory(
                                                                item,
                                                            )}
                                                        on:keydown={(e) =>
                                                            e.key === "Enter" &&
                                                            selectReplaceHistory(
                                                                item,
                                                            )}
                                                        role="button"
                                                        tabindex="0"
                                                        >{item}</span
                                                    >
                                                    <button
                                                        type="button"
                                                        class="fr-history-del"
                                                        on:click|stopPropagation={() =>
                                                            removeFromReplaceHistory(
                                                                i,
                                                            )}
                                                        title="删除"
                                                        >&times;</button
                                                    >
                                                </div>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                                <div class="fr-actions">
                                    <button
                                        type="button"
                                        class="fr-btn fr-btn-text"
                                        on:click={performReplace}
                                        title="替换">替换</button
                                    >
                                    <button
                                        type="button"
                                        class="fr-btn fr-btn-wide fr-danger"
                                        on:click={performReplaceAll}
                                        title="全部替换">全部替换</button
                                    >
                                </div>
                            </div>
                            <div class="fr-row fr-options">
                                <span class="fr-label">搜索选项:</span>
                                <CustomSelect
                                    className="fr-custom-select fr-select-sm"
                                    value={isRegex ? "regex" : "text"}
                                    options={FIND_MODE_OPTIONS}
                                    on:change={(event) =>
                                        (isRegex = event.detail === "regex")}
                                />
                                <CustomSelect
                                    className="fr-custom-select"
                                    value={searchScope}
                                    options={SEARCH_SCOPE_OPTIONS}
                                    on:change={(event) =>
                                        (searchScope =
                                            event.detail as typeof searchScope)}
                                />
                                <CustomSelect
                                    className="fr-custom-select fr-select-xs"
                                    value={searchDirection}
                                    options={SEARCH_DIRECTION_OPTIONS}
                                    on:change={(event) =>
                                        (searchDirection =
                                            event.detail as typeof searchDirection)}
                                />
                                <label class="fr-checkbox">
                                    <input
                                        type="checkbox"
                                        bind:checked={wrapAround}
                                    />循环
                                </label>
                                <label class="fr-checkbox">
                                    <input
                                        type="checkbox"
                                        bind:checked={textOnly}
                                    />纯文本
                                </label>
                                <span class="fr-message">{searchMessage}</span>
                            </div>
                        </div>
                    {/if}

                    {#if phoneLayout}
                        <!-- 手机动作条：桌面靠 Ctrl+Z / Ctrl+F / Ctrl+S，手机什么都没有。
                             这是 ReaMicro 那套编辑器最值得抄的一处 —— 常用动作常驻在
                             拇指够得着的位置，而不是塞进右上角的菜单里。 -->
                        <div class="phone-actionbar">
                            <button
                                type="button"
                                on:click={() =>
                                    epubCodeEditorComponent?.undoEdit()}
                                disabled={!selectedFile}
                                title="撤销">撤销</button
                            >
                            <button
                                type="button"
                                on:click={() =>
                                    epubCodeEditorComponent?.redoEdit()}
                                disabled={!selectedFile}
                                title="重做">重做</button
                            >
                            <button
                                type="button"
                                class:active={phoneFindOpen}
                                on:click={() => {
                                    phoneFindOpen = !phoneFindOpen;
                                    if (phoneFindOpen) {
                                        setTimeout(
                                            () =>
                                                document
                                                    .getElementById(
                                                        "epub-search-input-fr",
                                                    )
                                                    ?.focus(),
                                            120,
                                        );
                                    }
                                }}
                                disabled={!selectedFile?.editable}
                                title="查找替换">查找</button
                            >
                            <button
                                type="button"
                                on:click={() => {
                                    phoneGotoOpen = true;
                                    phoneGotoValue = String(phoneLine);
                                }}
                                disabled={!selectedFile}
                                title="跳到行">行 {phoneLine}</button
                            >
                            <button
                                type="button"
                                class="primary"
                                on:click={stageCurrentFile}
                                disabled={!selectedFile || !dirty || busy}
                                title="暂存到书内">保存</button
                            >
                        </div>

                        {#if selectedFile?.editable && !phoneFindOpen}
                            <!-- 符号栏：软键盘上打 < > / " 要切两层，
                                 这排键省掉的正是这一步。查找条展开时收起 ——
                                 两排一起会把编辑区挤没 -->
                            <div class="phone-symbolbar" role="toolbar" aria-label="符号">
                                {#each PHONE_SYMBOLS as symbol (symbol)}
                                    <button
                                        type="button"
                                        on:click={() => handlePhoneSymbol(symbol)}
                                        >{phoneSymbolLabel(symbol)}</button
                                    >
                                {/each}
                            </div>
                        {/if}
                    {/if}
                </section>
            {/if}

            <aside class="right-panel">
                <section class="preview-pane">
                    {#if readerMode}
                        <div class="preview-header reader-preview-header">
                            <div class="reader-preview-title">
                                <strong title={currentPreviewTitle}
                                    >{currentPreviewTitle}</strong
                                >
                                <span>{previewNavLabel()}</span>
                            </div>
                            <div class="reader-preview-actions">
                                {#if !phoneLayout}
                                    <!-- 手机上切目录由顶部页签负责，这颗按钮要么
                                         和页签语义重复、要么把另一栏整个隐藏掉 -->
                                    <button
                                        type="button"
                                        class="reader-toc-toggle"
                                        class:active={!readerTocCollapsed}
                                        on:click={toggleReaderToc}
                                        aria-pressed={!readerTocCollapsed}
                                        title={readerTocCollapsed
                                            ? "显示目录"
                                            : "收起目录"}>目录</button
                                    >
                                {/if}
                                <button
                                    type="button"
                                    class="reader-font-btn"
                                    on:click={() => adjustReaderFont(-1)}
                                    disabled={!canAdjustReaderFont(-1) || busy}
                                    title="减小字号">A-</button
                                >
                                <button
                                    type="button"
                                    class="reader-font-btn"
                                    on:click={() => adjustReaderFont(1)}
                                    disabled={!canAdjustReaderFont(1) || busy}
                                    title="增大字号">A+</button
                                >
                                <button
                                    type="button"
                                    on:click={() => previewRelative(-1)}
                                    disabled={!canPreviewRelative(-1) || busy}
                                    >上一章</button
                                >
                                <button
                                    type="button"
                                    on:click={() => previewRelative(1)}
                                    disabled={!canPreviewRelative(1) || busy}
                                    >下一章</button
                                >
                            </div>
                        </div>
                    {:else}
                        <div class="preview-header">
                            <button
                                type="button"
                                class:active={rightTab === "preview"}
                                on:click={() => (rightTab = "preview")}
                                >预览</button
                            >
                            <button
                                type="button"
                                class:active={rightTab === "toc"}
                                on:click={() => (rightTab = "toc")}>目录</button
                            >
                        </div>
                    {/if}
                    <div class="preview-stack">
                        <div
                            class="preview-body"
                            class:dark-paper={resolveUiTheme(
                                appSettings.uiTheme,
                            ) === "dark"}
                            class:hide-scrollbar={!appSettings.previewScrollbar}
                            class:is-hidden={!readerMode &&
                                rightTab !== "preview"}
                            use:trackPreviewWidth
                            bind:this={previewBodyEl}
                        >
                            {#if selectedFile?.kind === "xhtml" && previewHtml}
                                <div
                                    class="mobile-frame"
                                    class:hide-scrollbar={!appSettings.previewScrollbar}
                                    class:reader-frame={readerMode}
                                >
                                    <iframe
                                        bind:this={previewIframeEl}
                                        class="chapter-preview"
                                        title={readerMode
                                            ? "章节阅读预览"
                                            : "章节预览"}
                                        srcdoc={previewHtml}
                                        sandbox="allow-same-origin allow-scripts"
                                        style:height={previewFrameHeight
                                            ? `${previewFrameHeight}px`
                                            : null}
                                        on:load={handlePreviewFrameLoad}
                                    ></iframe>
                                </div>
                            {:else if (!selectedFile && !selectedImage) || selectedFile?.kind === "xhtml"}
                                <div class="preview-empty">
                                    {readerMode
                                        ? "选择目录中的章节后开始阅读。"
                                        : "选择 XHTML 章节后自动预览。"}
                                </div>
                            {/if}
                        </div>
                        {#if !readerMode}
                            <div
                                class="toc-container"
                                class:is-hidden={rightTab !== "toc"}
                            >
                                {#if tocTree.length === 0}
                                    <p class="panel-empty">
                                        未读取到 NAV / NCX 目录。
                                    </p>
                                {:else}
                                    <div class="toc-list">
                                        {#each tocTree as item (item.id)}
                                            <TocNode
                                                {item}
                                                selectedSrc={selectedFile?.path ||
                                                    ""}
                                                onSelect={handleTocSelect}
                                            />
                                        {/each}
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    </div>
                </section>
            </aside>
        </main>
    {/if}

    {#if phoneLayout && phoneMoreOpen}
        <!-- 手机「更多」：桌面标题行右侧那一排按钮的收容处。
             复用 MobileSheet 而不是另写一个遮罩，是为了 Esc / 焦点陷阱 /
             键盘顶起（--ime-height）三件事只有一份实现。 -->
        <MobileSheet
            open={phoneMoreOpen}
            title="更多操作"
            onclose={() => (phoneMoreOpen = false)}
        >
            <div class="phone-more-list">
                <button
                    type="button"
                    class="m-list-row"
                    on:click={() => {
                        phoneMoreOpen = false;
                        pickFile();
                    }}
                    disabled={busy}
                >
                    <span>换一本书</span>
                    <span class="phone-more-hint">重新选择 EPUB</span>
                </button>
                <button
                    type="button"
                    class="m-list-row"
                    on:click={() => {
                        phoneMoreOpen = false;
                        stageCurrentFile();
                    }}
                    disabled={!selectedFile || !dirty || busy}
                >
                    <span>暂存当前文件</span>
                    <span class="phone-more-hint">
                        {dirty ? "有未保存修改" : "没有改动"}
                    </span>
                </button>
                <button
                    type="button"
                    class="m-list-row"
                    on:click={() => {
                        phoneMoreOpen = false;
                        exportEpub();
                    }}
                    disabled={busy}
                >
                    <span>导出 EPUB</span>
                    <span class="phone-more-hint">生成新文件</span>
                </button>
                <button
                    type="button"
                    class="m-list-row"
                    on:click={() => {
                        phoneMoreOpen = false;
                        selectPane("tree");
                        activeTab = "metadata";
                    }}
                >
                    <span>书籍信息</span>
                    <span class="phone-more-hint">书名 / 作者 / 封面</span>
                </button>
                <div class="phone-more-row">
                    <span>自动换行</span>
                    <button
                        type="button"
                        on:click={togglePhoneWrap}
                        disabled={!selectedFile}
                        >切换</button
                    >
                </div>
                <div class="phone-more-row">
                    <span>字号</span>
                    <span class="phone-more-inline">
                        <button
                            type="button"
                            on:click={() => adjustPhoneFont(-1)}
                            disabled={!selectedFile}
                            aria-label="减小字号">A-</button
                        >
                        <button
                            type="button"
                            on:click={() => adjustPhoneFont(1)}
                            disabled={!selectedFile}
                            aria-label="增大字号">A+</button
                        >
                    </span>
                </div>
                <button
                    type="button"
                    class="m-list-row"
                    on:click={() => {
                        phoneMoreOpen = false;
                        selectPane("editor");
                        phoneFindOpen = true;
                    }}
                    disabled={!selectedFile?.editable}
                >
                    <span>查找替换</span>
                    <span class="phone-more-hint">当前文件 / 全部文件</span>
                </button>
            </div>
        </MobileSheet>
    {/if}

    {#if phoneLayout && phoneGotoOpen}
        <MobileSheet
            open={phoneGotoOpen}
            title="跳到行"
            onclose={() => (phoneGotoOpen = false)}
        >
            <!-- 动作按钮放在 body 里而不是用 actions snippet：
                 本文件仍是旧语法（$: / on:click），混进 snippet 容易和
                 编译模式打架，收益只有一点点边距 -->
            <div class="phone-goto">
                <input
                    type="text"
                    inputmode="numeric"
                    bind:value={phoneGotoValue}
                    on:keydown={(e) => e.key === "Enter" && handlePhoneGoto()}
                    placeholder="行号"
                    aria-label="行号"
                />
                <span>共 {epubCodeEditorComponent?.getView()?.state.doc.lines ?? 0} 行</span>
            </div>
            <div class="phone-goto-actions">
                <button type="button" on:click={() => (phoneGotoOpen = false)}
                    >取消</button
                >
                <button
                    type="button"
                    class="primary"
                    on:click={handlePhoneGoto}>跳转</button
                >
            </div>
        </MobileSheet>
    {/if}

    {#if fileMenu}
        <div
            class="file-menu-popover"
            style={`left: ${fileMenu.x}px; top: ${fileMenu.y}px;`}
            role="menu"
            tabindex="-1"
            on:click|stopPropagation
            on:keydown|stopPropagation
        >
            <button
                type="button"
                role="menuitem"
                on:click={() => renameResourcePath(fileMenu?.path || "")}
                disabled={busy}>重命名</button
            >
            <button
                type="button"
                role="menuitem"
                class="danger"
                on:click={() => deleteResourcePath(fileMenu?.path || "")}
                disabled={busy}>删除</button
            >
        </div>
    {/if}
</div>

<style>
    .web-epub-page {
        position: fixed;
        inset: 0;
        height: 100vh;
        /* 不自建色板子集，直接继承 :root 的 design token，避免与全局样式漂移 */
        --font-code: Consolas, "Cascadia Mono", "Microsoft YaHei", monospace;
        background: var(--color-canvas);
        color: var(--color-text);
        font-family: var(--font-ui);
        overflow: hidden;
    }

    .import-shell {
        height: 100%;
        min-height: 0;
        display: grid;
        grid-template-rows: minmax(0, 1fr);
        overflow: hidden;
    }

    .file-input {
        display: none;
    }

    h2,
    p {
        margin: 0;
    }

    h2 {
        font-size: 14px;
        line-height: 1.4;
    }

    button {
        height: var(--control-height);
        padding: 0 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text);
        font: inherit;
        cursor: pointer;
    }

    button:hover:not(:disabled) {
        border-color: var(--color-accent);
    }

    button:disabled {
        cursor: not-allowed;
        opacity: 0.55;
    }

    button.primary {
        border-color: var(--color-accent);
        background: var(--color-accent);
        color: var(--color-accent-contrast);
        font-weight: 800;
    }

    button.danger {
        border-color: color-mix(
            in srgb,
            var(--color-danger-soft) 92%,
            var(--color-danger)
        );
        background: var(--color-danger-soft);
        color: var(--color-danger);
        font-weight: 800;
    }

    button.danger:hover:not(:disabled) {
        border-color: var(--color-danger);
    }

    button.active {
        border-color: var(--color-accent);
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
        font-weight: 800;
    }

    .workspace {
        /* 必须是 100% 而不是 100vh：.web-epub-page 由 app.css 改成
           position:absolute;inset:0;height:auto，它只占 app-frame-body 的
           高度（窗口减掉 41px 标签栏）。写 100vh 会比父容器高 41px，
           预览列跟着被撑长，同一本书 web 版的预览框就比安装版高一截。 */
        height: 100%;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(240px, 280px) minmax(0, 1fr) minmax(
                320px,
                380px
            );
        gap: 0;
        padding: 0;
        box-sizing: border-box;
        overflow: hidden;
    }

    .workspace.reader-workspace {
        grid-template-columns: minmax(260px, 340px) minmax(0, 1fr);
        background: var(--color-canvas);
    }

    .workspace.reader-workspace.reader-toc-collapsed {
        grid-template-columns: minmax(0, 1fr);
    }

    .left-panel,
    .right-panel,
    .editor-panel {
        min-height: 0;
        border: 0;
        border-radius: 0;
        background: var(--color-surface);
        overflow: hidden;
    }

    .left-panel {
        display: grid;
        grid-template-rows: auto minmax(0, 1fr);
        border-right: 1px solid var(--color-border);
    }

    .reader-workspace .left-panel {
        grid-template-rows: auto minmax(0, 1fr);
    }

    .reader-workspace.reader-toc-collapsed .left-panel {
        display: none;
    }

    .right-panel {
        display: grid;
        grid-template-rows: minmax(0, 1fr);
        border-left: 1px solid var(--color-border);
    }

    .reader-workspace .right-panel {
        border-left: 0;
    }

    .tree-header {
        height: 50px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        padding: 0 10px 0 12px;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-soft);
        box-sizing: border-box;
    }

    .tree-actions {
        display: flex;
        align-items: center;
        gap: 4px;
    }

    .tree-actions button {
        height: 26px;
        padding: 0 7px;
        font-size: 12px;
    }

    .tabs {
        /* 顶到左栏最上沿，高度与右侧预览栏头一致，三栏首行对齐 */
        height: 50px;
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 0;
        padding: 0;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-soft);
        box-sizing: border-box;
    }

    /* 文件/信息/预览/目录 四个页签共用一套（非选中态）样式，
       之前左右两栏各写一份，颜色和字重不一致，看着像两种控件 */
    .tabs button,
    .preview-header > button {
        height: 50px;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--color-muted);
        font-weight: 700;
        transition:
            background var(--transition-fast),
            color var(--transition-fast);
    }

    .tabs button:hover,
    .preview-header > button:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .tabs button.active,
    .preview-header > button.active {
        background: var(--color-surface);
        color: var(--color-accent);
        box-shadow: inset 0 -2px 0 var(--color-accent);
    }

    .tree-content,
    .toc-list {
        min-height: 0;
        max-height: none;
        display: grid;
        align-content: start;
        overflow: auto;
        padding: 8px;
    }

    .tree-content {
        padding: 8px;
        overflow-x: hidden;
    }

    .metadata-form {
        min-height: 0;
        overflow: auto;
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 10px;
        padding: 12px;
    }

    .metadata-top {
        grid-column: 1 / -1;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 96px;
        gap: 10px;
        align-items: start;
        min-width: 0;
    }

    .metadata-title-fields {
        display: grid;
        gap: 10px;
        min-width: 0;
    }

    .metadata-form label,
    .metadata-meta {
        min-width: 0;
        display: grid;
        gap: 5px;
    }

    .metadata-form label.full,
    .metadata-form label.metadata-identifier,
    .metadata-meta,
    .metadata-actions {
        grid-column: 1 / -1;
    }

    .metadata-form span {
        color: var(--color-muted);
        font-size: 12px;
    }

    .metadata-form input,
    .metadata-form textarea {
        width: 100%;
        min-width: 0;
        box-sizing: border-box;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text);
        font: inherit;
    }

    .metadata-form input {
        height: var(--control-height);
        padding: 0 9px;
    }

    .metadata-form textarea {
        min-height: 98px;
        padding: 8px 9px;
        resize: vertical;
        line-height: 1.6;
    }

    .metadata-cover {
        width: 96px;
        height: 128px;
        aspect-ratio: 3 / 4;
        display: grid;
        place-items: center;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface-soft);
        color: var(--color-muted);
        font-size: 12px;
        overflow: hidden;
    }

    .metadata-cover img {
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    .metadata-meta {
        gap: 3px;
        overflow-wrap: anywhere;
    }

    .metadata-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
    }

    .editor-panel {
        display: grid;
        grid-template-rows: auto minmax(0, 1fr) auto;
        border-left: 1px solid var(--color-canvas);
        border-right: 1px solid var(--color-canvas);
    }

    .editor-head {
        height: 50px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 0 12px;
        border-bottom: 1px solid var(--color-border);
        /* 与左右两栏的栏头同色同高，三栏顶部连成一条 */
        background: var(--color-surface-soft);
        box-sizing: border-box;
    }

    /* 只作用于标题容器，不能用 .editor-head div ——
       否则右侧 .editor-actions 也会被变成垂直网格，按钮被压扁错位 */
    .editor-title {
        min-width: 0;
        display: grid;
        gap: 0;
    }

    .editor-head strong {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .editor-actions {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        gap: 6px;
    }

    /* 导出/暂存与右侧「预览/目录」同高，三个栏头视觉一致 */
    .editor-actions button {
        height: 26px;
        padding: 0 10px;
        font-size: 12px;
    }

    .editor-content {
        width: 100%;
        height: 100%;
        min-height: 0;
        box-sizing: border-box;
        background: var(--color-surface);
        overflow: hidden;
    }

    .editor-content :global(.epub-code-editor) {
        height: 100%;
    }

    .editor-content :global(.cm-editor) {
        height: 100%;
        background: var(--color-surface);
    }

    .editor-content :global(.cm-scroller) {
        font-family: var(--font-code);
    }

    .editor-content :global(.cm-content) {
        padding: 18px 20px;
        color: var(--color-text);
    }

    .find-replace-panel {
        min-height: 0;
        background: var(--color-hover);
        border-top: 1px solid var(--color-border);
        padding: 6px 10px;
        font-size: 13px;
        flex-shrink: 0;
        overflow: visible;
        box-sizing: border-box;
    }

    .fr-row {
        display: flex;
        align-items: center;
        gap: 4px;
        margin-bottom: 6px;
        overflow: visible;
    }

    .fr-row:last-child {
        margin-bottom: 0;
    }

    .fr-label {
        flex: 0 0 64px;
        font-weight: 600;
        color: var(--color-text-soft);
        text-align: right;
        white-space: nowrap;
    }

    .fr-input-wrapper {
        flex: 1;
        min-width: 0;
        display: flex;
        position: relative;
    }

    .fr-input {
        flex: 1;
        min-width: 0;
        height: 30px;
        padding: 5px 10px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text);
        font:
            13px Consolas,
            "Cascadia Mono",
            "Microsoft YaHei",
            monospace;
        box-sizing: border-box;
    }

    .fr-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-input-wrapper .fr-input {
        border-top-right-radius: 0;
        border-bottom-right-radius: 0;
    }

    .fr-actions {
        width: 164px;
        display: flex;
        justify-content: flex-end;
        gap: 4px;
        flex-shrink: 0;
    }

    .fr-find-actions {
        width: 164px;
    }

    .fr-btn,
    .fr-dropdown-btn {
        height: 30px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-xs);
        background: linear-gradient(
            to bottom,
            var(--color-surface),
            var(--color-border)
        );
        color: var(--color-text);
        cursor: pointer;
        font-size: 12px;
        white-space: nowrap;
    }

    .fr-btn {
        padding: 0 8px;
    }

    .fr-btn-text {
        min-width: 44px;
    }

    .fr-btn-wide {
        min-width: 72px;
    }

    .fr-btn:hover,
    .fr-dropdown-btn:hover {
        background: linear-gradient(
            to bottom,
            var(--color-surface-soft),
            var(--color-border)
        );
        border-color: var(--color-muted);
    }

    .fr-danger {
        border-color: #d99;
        background: linear-gradient(to bottom, var(--color-danger-soft), #fcc);
        color: var(--color-danger);
    }

    .fr-danger:hover {
        border-color: #c66;
        background: linear-gradient(to bottom, #fdd, #faa);
    }

    .fr-dropdown-btn {
        width: 30px;
        flex: 0 0 30px;
        border-left: 0;
        border-top-left-radius: 0;
        border-bottom-left-radius: 0;
        padding: 0;
    }

    .fr-options {
        font-size: 12px;
    }

    .fr-options :global(.fr-custom-select) {
        flex: 0 0 104px;
        width: 104px;
    }

    .fr-options :global(.fr-custom-select.fr-select-sm) {
        flex-basis: 96px;
        width: 96px;
    }

    .fr-options :global(.fr-custom-select.fr-select-xs) {
        flex-basis: 70px;
        width: 70px;
    }

    .fr-options :global(.fr-custom-select .custom-select-trigger) {
        height: 30px;
        min-height: 30px;
        padding: 0 8px;
        border-color: var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text);
        box-shadow: none;
        font-size: 12px;
        font-weight: 600;
    }

    .fr-options :global(.fr-custom-select.open .custom-select-trigger),
    .fr-options
        :global(.fr-custom-select .custom-select-trigger:focus-visible) {
        border-color: var(--color-accent);
        box-shadow: 0 0 0 2px rgba(33, 150, 243, 0.2);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu) {
        top: auto;
        bottom: calc(100% + 4px);
        border-color: var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        box-shadow: var(--shadow-md);
    }

    .fr-options :global(.fr-custom-select .custom-select-menu button) {
        min-height: 28px;
        padding: 5px 8px;
        border-radius: var(--radius-xs);
        font-size: 12px;
        font-weight: 600;
    }

    .fr-checkbox {
        display: flex;
        align-items: center;
        gap: 4px;
        color: var(--color-text-soft);
        cursor: pointer;
        white-space: nowrap;
    }

    .fr-checkbox input {
        margin: 0;
    }

    .fr-message {
        margin-left: auto;
        min-width: 84px;
        color: var(--color-accent);
        font-weight: 600;
        text-align: right;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .fr-history-dropdown {
        position: absolute;
        bottom: 100%;
        left: 0;
        right: 0;
        z-index: 20;
        max-height: 280px;
        margin-bottom: 2px;
        overflow-y: auto;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        box-shadow: 0 4px 8px rgba(15, 23, 42, 0.15);
    }

    .fr-history-item {
        display: flex;
        align-items: center;
        padding: 6px 10px;
        border-bottom: 1px solid #edf1f6;
    }

    .fr-history-item:last-child {
        border-bottom: 0;
    }

    .fr-history-item:hover {
        background: var(--color-surface-soft);
    }

    .fr-history-text {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        cursor: pointer;
        font-family: Consolas, "Cascadia Mono", monospace;
        font-size: 12px;
    }

    .fr-history-del {
        width: 24px;
        height: 24px;
        min-height: 0;
        margin-left: 8px;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--color-muted);
    }

    .fr-history-del:hover {
        background: #fdd;
        color: var(--color-danger);
    }

    .editor-image-preview {
        min-height: 0;
        display: grid;
        place-items: center;
        padding: 18px;
        background: var(--color-surface-soft);
        box-sizing: border-box;
        overflow: auto;
    }

    .editor-image-preview img {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
        border: 1px solid var(--color-border);
        background: var(--color-surface);
    }

    .editor-image-preview span {
        color: var(--color-muted);
        font-size: 13px;
    }

    .preview-pane {
        min-height: 0;
        display: grid;
        grid-template-rows: auto minmax(0, 1fr);
        background: var(--color-hover);
    }

    .preview-header {
        position: relative;
        height: 50px;
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 0;
        padding: 0;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-soft);
        box-sizing: border-box;
    }

    /* 页签样式与左侧 .tabs 共用，见 .tabs button 处的合并规则 */

    .reader-preview-header {
        height: 56px;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        padding: 0 14px 0 18px;
    }

    .reader-preview-title {
        min-width: 0;
        display: grid;
        gap: 2px;
    }

    .reader-preview-title strong {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--color-text);
        font-size: 15px;
    }

    .reader-preview-title span {
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 700;
    }

    .reader-preview-actions {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        flex-wrap: wrap;
        gap: 8px;
    }

    .reader-preview-actions button {
        height: 32px;
        padding: 0 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        color: var(--color-text-soft);
        font-weight: 800;
    }

    .reader-preview-actions button:disabled {
        cursor: not-allowed;
        opacity: 0.45;
    }

    .reader-preview-actions .reader-font-btn {
        min-width: 38px;
        padding: 0 9px;
        font-size: 14px;
    }

    .reader-preview-actions .reader-toc-toggle {
        min-width: 54px;
        padding: 0 10px;
    }

    .reader-preview-actions .reader-toc-toggle.active {
        border-color: var(--color-text-soft);
        background: var(--color-text);
        color: var(--color-surface);
    }

    /* 预览与目录常驻挂载、只切换显隐：
       若用 {#if} 销毁重建，iframe 回来时要重新量高度，
       外层的滚动条会先消失再出现，肉眼就是「闪一下」 */
    .preview-stack {
        min-height: 0;
        display: grid;
        grid-template-rows: minmax(0, 1fr);
        overflow: hidden;
    }

    .preview-stack > .preview-body,
    .preview-stack > .toc-container {
        grid-area: 1 / 1;
        min-width: 0;
        min-height: 0;
    }

    /* 用 visibility 而不是 display:none —— 后者会让 iframe 脱离布局，
       切回来时整篇重新排版，书页高度先短后长，滚动条就闪一下。
       visibility:hidden 保留布局和测量结果，切换纯粹是一次重绘 */
    .preview-stack > .is-hidden {
        visibility: hidden;
        pointer-events: none;
    }

    /* 兜底：隐藏层里的元素若自带 visibility: visible（内联或组件内样式），
       会盖过继承来的 hidden 单独画在预览区上（目录树的展开箭头就踩过）。
       后代一律压住，显示回来时选择器失配，自然恢复 */
    .preview-stack > .is-hidden :global(*) {
        visibility: hidden !important;
    }

    .preview-body {
        min-height: 0;
        display: flex;
        align-items: flex-start;
        justify-content: center;
        padding: 10px;
        background: var(--color-hover);
        /* 滚动交给手机框（.mobile-frame）自己，这里只负责底色和留白；
           否则框的高度上限等于列高，滚动条又画到列边缘去了 */
        overflow: hidden;
        overscroll-behavior: contain;
    }

    /* 阅读模式是宽页，没有手机框的高度上限，仍由预览列整体滚动 */
    .reader-workspace .preview-body {
        overflow-y: auto;
        overflow-x: hidden;
        scrollbar-gutter: stable;
    }

    /* 设置里关掉预览滚动条（默认）时，连槽位一起取消。
       只藏滚动条本体而留着 scrollbar-gutter，书页仍然偏左，
       不会回到预览列正中 */
    .preview-body.hide-scrollbar {
        scrollbar-width: none;
        -ms-overflow-style: none;
        scrollbar-gutter: auto;
    }

    .preview-body.hide-scrollbar::-webkit-scrollbar {
        width: 0;
        height: 0;
        display: none;
    }

    /* 手机框自己滚，滚动条要跟着设置一起藏（两者用同一个开关） */
    .mobile-frame.hide-scrollbar {
        scrollbar-width: none;
        -ms-overflow-style: none;
        scrollbar-gutter: auto;
    }

    .mobile-frame.hide-scrollbar::-webkit-scrollbar {
        width: 0;
        height: 0;
        display: none;
    }

    .reader-workspace .preview-body {
        padding: 18px clamp(16px, 3vw, 42px);
        background: var(--color-canvas);
    }

    .reader-workspace .mobile-frame {
        align-self: flex-start;
    }

    .mobile-frame,
    .preview-empty {
        /* 和安装版一致：手机框按设计值 320px 宽，不带 // 340 的列宽派生。
           原来写 340 时框比安装版宽一圈，同一本书两边预览换行位置不同 */
        width: min(100%, 320px);
        min-height: 0;
        box-sizing: border-box;
    }

    /* 空态仍占满可视高度；有内容时由 iframe 实测高度决定 */
    .preview-empty {
        align-self: stretch;
        height: 100%;
    }

    .mobile-frame {
        display: flex;
        flex-direction: column;
        /* 高度撑满预览列但封顶 700（320 宽对应的 20:9 设计高度），
           与安装版 PreviewPane 的 height:100% + max-height:700px 完全一致。
           不能写死 700px：安装版窗口默认 1200x740，预览列只有 ~585，
           写死 700 会让 web 版在矮窗口下比安装版高出一截，
           同一本书两边看到的页面大小不同。 */
        height: 100%;
        max-height: 700px;
        overflow-y: auto;
        overflow-x: hidden;
        overscroll-behavior: contain;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-lg);
        background: var(--color-surface);
        box-shadow: var(--shadow-md);
    }

    /* 阅读模式是宽页阅读，不是手机：取消高度上限，交回 .preview-body 滚动 */
    .reader-frame {
        width: min(100%, 1080px);
        height: auto;
        max-height: none;
        overflow: visible;
        border-color: var(--color-border);
        box-shadow: 0 10px 24px rgba(15, 23, 42, 0.14);
    }

    /* 书页外框跟着纸色走：iframe 底色已经是深色了，
       外框还留浅色会在书页四周镶一圈白边 */
    .preview-body.dark-paper .mobile-frame,
    .preview-body.dark-paper .preview-empty,
    .preview-body.dark-paper .chapter-preview {
        background: var(--color-canvas, #191e26);
    }

    .chapter-preview {
        width: 100%;
        /* 高度由 JS 实测正文高度写入 inline style。
           测量完成前先铺满预览区可视高度（--preview-h 由 trackPreviewWidth 写入），
           不要写死 min-height：那会让 iframe 先矮后高，外层滚动条闪一下 */
        height: var(--preview-h, 100%);
        /* 手机框是纵向 flex 容器，iframe 默认会被压缩到框高，
           于是正文在 iframe 内部滚、框自己反而不滚（滚动条跑进书页里）。
           inline 的实测高度要能撑开，这里必须禁止收缩 */
        flex-shrink: 0;
        border: 0;
        /* 与安装版一致：iframe 自己也要圆角，只给外框圆角时书页四个角
           仍会露出直角（外框滚动时尤其明显） */
        border-radius: var(--radius-lg);
        box-sizing: border-box;
        background: var(--color-surface);
    }

    .preview-empty {
        display: grid;
        place-items: center;
        border: 1px dashed var(--color-border);
        /* 和 .mobile-frame 同一套圆角/底色：空态与有内容态尺寸外形一致，
           换章节时哪怕真的走了一次空态，也不会闪出一个直角白框 */
        border-radius: var(--radius-lg);
        background: var(--color-surface);
        color: var(--color-muted);
        font-size: 13px;
    }

    .editor-empty {
        display: grid;
        place-items: center;
        color: var(--color-muted);
    }

    .right-panel {
        box-sizing: border-box;
        overflow: hidden;
    }

    .panel-empty {
        color: var(--color-muted);
        font-size: 13px;
        line-height: 1.6;
    }

    .toc-container {
        min-height: 0;
        background: var(--color-surface);
        overflow: auto;
    }

    .toc-list {
        max-height: none;
        padding: 0;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
    }

    .toc-list {
        border: 0;
        border-radius: 0;
    }

    .toc-container .panel-empty {
        padding: 12px;
    }

    .file-menu-popover {
        position: fixed;
        z-index: 30;
        width: 104px;
        display: grid;
        gap: 4px;
        padding: 6px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-xs);
        background: var(--color-surface);
        box-shadow: var(--shadow-pop);
        /* 外框 = 104 内容 + 12 内边距 + 2 边框。box-sizing 在桌面按 content-box，
           手机若被全局重置成 border-box，同一串数字会算出两种宽度，
           定位预算（FILE_MENU_WIDTH）就会对不上。这里写死，两边一致。 */
        box-sizing: content-box;
    }

    /* 手机：两项按钮抬到触控尺寸。桌面的 30px 密度不动。 */
    :global(:root[data-tepub-layout="phone"]) .file-menu-popover button {
        min-height: var(--tap-target);
    }

    .file-menu-popover button {
        width: 100%;
        height: 30px;
        padding: 0 8px;
        text-align: left;
    }

    /* ⚠ 这一段只服务「桌面壳被拖窄」，不是手机布局。
       手机（data-tepub-layout="phone"）走下面那套 phone-stack 规则，特异性
       与顺序都在这段之后，逐条覆盖掉了这里的 220/320/360 三行。
       它真正命中的唯一场景是 Tauri 桌面窗口被拖到 ≤760px：那时 layout 仍
       是 desktop（宽度信号只在 web 下参与判定），三列挤在 700px 里没法用，
       退化成三行叠放比三列各 230px 好。所以别删 —— 也别往里加手机样式，
       横屏手机 844px 不会命中这段，加了等于静默失效。 */
    @media (max-width: 760px) {
        /* 窄屏下标题和两个按钮挤不进 50px，改为自动高度纵向排布 */
        .editor-head {
            height: auto;
            min-height: 50px;
            align-items: stretch;
            flex-direction: column;
            gap: 8px;
            padding: 10px 12px;
        }

        .editor-actions {
            width: 100%;
            justify-content: stretch;
        }

        .editor-actions button {
            flex: 1 1 auto;
        }

        .workspace {
            grid-template-columns: 1fr;
            grid-template-rows: 220px minmax(320px, 1fr) 360px;
        }

        .workspace.reader-workspace {
            grid-template-columns: 1fr;
            grid-template-rows: minmax(220px, 34vh) minmax(420px, 1fr);
        }

        .workspace.reader-workspace.reader-toc-collapsed {
            grid-template-rows: minmax(420px, 1fr);
        }

        .reader-preview-header {
            height: auto;
            min-height: 56px;
            grid-template-columns: minmax(0, 1fr);
            align-content: center;
            padding: 10px 12px;
        }

        .reader-preview-actions {
            justify-content: flex-start;
        }

        .left-panel,
        .editor-panel,
        .right-panel {
            border: 0;
            border-bottom: 1px solid var(--color-border);
        }
    }

    /* 本页根容器在手机上不需要任何覆写：app.css 已经把
       .app-frame-body 里的 .web-epub-page 强制成 position:absolute;inset:0;
       height:auto（带 !important，任何页面级 height 都压不过它）。
       这里以前写的是 height:100dvh —— 一条永远不会生效、还会误导人以为
       手机上是按视口铺的，删掉。 */

    /* 这里同样不能写 100dvh：父容器 .web-epub-page 由 app.css 压成
       absolute;inset:0;height:auto，比整个视口矮一条标签栏。写视口单位
       会让内容比容器高出一截，底部被标签栏吃掉。 */
    :global(:root[data-tepub-layout="phone"]) .import-shell {
        height: 100%;
        grid-template-rows: minmax(0, 1fr);
        background: var(--color-canvas);
        overflow: auto;
    }

    /* ——— 手机：三栏叠成一格 ———

       以前这里是"三行各给一个 dvh"：文件树 24dvh、编辑区 1fr、预览 38dvh。
       390×844 的屏上每栏只剩两三百 px，编辑区显示不到 10 行，勾选个文件还得
       在三个矮窗里上下滚 —— 就是用户说的"单纯拉伸"。

       现在改成：三栏 grid-area 全等于 1/1，同一时刻只有一栏 visible。
       用 visibility 而不是 display:none，是因为 CodeMirror 一旦脱离布局就会
       重新测量，切回来时滚动位置和光标位置都会跳；visibility:hidden 保留
       布局，切换只是重绘（下面 .phone-stack 的规则负责这件事）。 */
    :global(:root[data-tepub-layout="phone"]) .workspace {
        height: 100%;
        grid-template-columns: minmax(0, 1fr);
        /* 行 1 = 页签，行 2 = 状态线，行 3 = 当前栏 */
        grid-template-rows: auto auto minmax(0, 1fr);
        overflow: hidden;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-status {
        grid-row: 2;
        grid-column: 1;
        padding: 4px 12px;
        font-size: 11px;
        color: var(--color-muted);
        background: var(--color-surface-soft);
        border-bottom: 1px solid var(--color-border);
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-pane-bar {
        grid-row: 1;
        grid-column: 1;
        display: grid;
        grid-auto-flow: column;
        grid-auto-columns: 1fr;
        height: 44px;
        padding-top: var(--safe-top);
        box-sizing: content-box;
        border-bottom: 1px solid var(--color-border);
        background: var(--color-surface-soft);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-pane-bar button {
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--color-muted);
        font-weight: 700;
        font-size: 14px;
        height: 44px;
        padding: 0;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-pane-bar button.active {
        background: var(--color-surface);
        color: var(--color-accent);
        box-shadow: inset 0 -2px 0 var(--color-accent);
    }

    /* 三栏全部落在同一格，只由 data-pane 决定谁可见 */
    :global(:root[data-tepub-layout="phone"]) .workspace.phone-stack > .left-panel,
    :global(:root[data-tepub-layout="phone"]) .workspace.phone-stack > .editor-panel,
    :global(:root[data-tepub-layout="phone"]) .workspace.phone-stack > .right-panel {
        grid-area: 3 / 1;
        min-width: 0;
        min-height: 0;
        border: 0;
        visibility: hidden;
    }

    /* 阅读模式没有 editor-panel（{#if !readerMode}），两栏同样叠一层 */
    :global(:root[data-tepub-layout="phone"])
        .workspace.phone-stack.reader-workspace {
        grid-template-rows: auto auto minmax(0, 1fr);
    }

    /* 阅读模式下「目录」是页签，不是折叠开关 —— 页签已经把两栏分开了，
       再叠一层 reader-toc-collapsed 的 display:none 会让点「目录」变成空白。
       这条要压过页面里原有的 .reader-workspace.reader-toc-collapsed .left-panel
       （两条选择器特异性相同，靠顺序取胜，所以必须写在这里而不是上面）。 */
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack.reader-workspace.reader-toc-collapsed
            > .left-panel
    ) {
        display: flex;
    }

    /* 阅读模式的栏头：手机上一行放不下标题 + 字号 + 上/下一章，
       让按钮换行，标题独占一行 */
    :global(:root[data-tepub-layout="phone"]) .reader-preview-header {
        height: auto;
        min-height: 52px;
        grid-template-columns: minmax(0, 1fr);
        gap: 6px;
        padding: 8px 12px;
    }

    :global(:root[data-tepub-layout="phone"]) .reader-preview-actions {
        justify-content: flex-start;
        flex-wrap: nowrap;
        overflow-x: auto;
        gap: 6px;
    }

    :global(:root[data-tepub-layout="phone"]) .reader-preview-actions button {
        flex: 0 0 auto;
        height: 38px;
    }

    :global(:root[data-tepub-layout="phone"])
        .workspace.phone-stack[data-pane="tree"]
        > .left-panel,
    :global(:root[data-tepub-layout="phone"])
        .workspace.phone-stack[data-pane="editor"]
        > .editor-panel,
    :global(:root[data-tepub-layout="phone"])
        .workspace.phone-stack[data-pane="preview"]
        > .right-panel {
        visibility: visible;
    }

    /* 隐藏栏里的后代要一并压住：某些元素自带 visibility:visible（内联或组件
       样式），会盖过继承来的 hidden 单独画出来 —— 叠成一格时那就是"两栏重叠"。
       与下面 .preview-stack > .is-hidden :global(*) 是同一个手法。
       :global(*) 是必须的：FileTreeItem / TocNode 的节点带的是它们自己的
       作用域属性，本组件的选择器选不中。 */
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="tree"]
            > .editor-panel
            *
    ),
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="tree"]
            > .right-panel
            *
    ),
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="editor"]
            > .left-panel
            *
    ),
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="editor"]
            > .right-panel
            *
    ),
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="preview"]
            > .left-panel
            *
    ),
    :global(
        :root[data-tepub-layout="phone"]
            .workspace.phone-stack[data-pane="preview"]
            > .editor-panel
            *
    ) {
        visibility: hidden !important;
    }

    /* 左栏同理：文件树筛选框只在手机出现，多出来的这一行会让写死的
       grid-template-rows 把 1fr 分错行（筛选框吃掉整屏，树本身塌成一行）。 */
    :global(:root[data-tepub-layout="phone"]) .workspace.phone-stack > .left-panel {
        display: flex;
        flex-direction: column;
    }

    :global(:root[data-tepub-layout="phone"]) .left-panel > .tabs,
    :global(:root[data-tepub-layout="phone"]) .left-panel > .tree-header,
    :global(:root[data-tepub-layout="phone"])
        .left-panel
        > .phone-tree-filter {
        flex: 0 0 auto;
    }

    :global(:root[data-tepub-layout="phone"]) .left-panel > .tree-content,
    :global(:root[data-tepub-layout="phone"]) .left-panel > .toc-container,
    :global(:root[data-tepub-layout="phone"]) .left-panel > .metadata-form {
        flex: 1 1 auto;
        min-height: 0;
    }

    /* 编辑栏在手机上的子节点数量是可变的（未暂存提示、查找条、动作条、
       符号栏都按需出现），写死的 grid-template-rows 会把 1fr 分错行 ——
       比如没有未暂存提示时，1fr 落到查找条上，编辑区被压成 0 高。
       改用 flex 列：除编辑区外全部 0 0 auto，由编辑区吃掉剩余高度。 */
    :global(:root[data-tepub-layout="phone"]) .editor-panel {
        display: flex;
        flex-direction: column;
    }

    /* 编辑区与图片预览共用同一格：没选中文本文件时展示的是 .editor-image-preview
       或 .editor-empty，它们同样要吃掉剩余高度，否则动作条会被顶到屏幕中间 */
    :global(:root[data-tepub-layout="phone"]) .editor-content,
    :global(:root[data-tepub-layout="phone"]) .editor-image-preview,
    :global(:root[data-tepub-layout="phone"]) .editor-empty {
        flex: 1 1 auto;
        min-height: 0;
    }

    /* 页面里的 .editor-panel 是 grid-template-rows: auto 1fr auto，
       转成 flex 后原来落在第二行的编辑区不再是"1fr"，上面这条补上；
       但 find-replace-panel 在 DOM 里位于编辑区之后、动作条之前，
       顺序保持不变即可 —— flex 不重排 DOM 顺序。 */

    :global(:root[data-tepub-layout="phone"]) .editor-head,
    :global(:root[data-tepub-layout="phone"]) .find-replace-panel {
        flex: 0 0 auto;
    }

    /* 查找条展开时最多占四成高度，否则编辑区只剩两三行 */
    :global(:root[data-tepub-layout="phone"]) .find-replace-panel {
        max-height: 42%;
        overflow: auto;
        padding-bottom: 8px;
    }

    /* 上面那条 overflow:auto 是有代价的：滚动容器上方溢出的内容拿 scrollTop
       永远滚不到。查找条里正好有两处**向上**弹出的浮层 —— 查找/替换历史
       （bottom:100%）和「范围」下拉（bottom: calc(100% + 4px)）—— 于是手机上
       点 ▾ 只会露一条 30px 的缝，历史条目点不中。手机上统一改成向下弹：
       输入框就在面板顶部，下面的空间归滚动区管，滚一下就能点到。 */
    :global(
        :root[data-tepub-layout="phone"]
            .web-epub-page
            .find-replace-panel
            .fr-history-dropdown
    ) {
        top: 100%;
        bottom: auto;
        margin-top: 2px;
        margin-bottom: 0;
        max-height: 40vh;
    }

    :global(
        :root[data-tepub-layout="phone"]
            .web-epub-page
            .find-replace-panel
            .fr-custom-select
            .custom-select-menu
    ) {
        top: calc(100% + 4px);
        bottom: auto;
    }

    /* 标题行维持一行：手机上再把标题和按钮拆成两行，等于白吃 50px 屏高 */
    :global(:root[data-tepub-layout="phone"]) .editor-head {
        height: 48px;
        min-height: 48px;
        flex-direction: row;
        align-items: center;
        padding: 0 8px;
    }

    :global(:root[data-tepub-layout="phone"]) .editor-actions {
        width: auto;
        flex-wrap: nowrap;
        gap: 4px;
    }

    :global(:root[data-tepub-layout="phone"]) .editor-actions button {
        flex: 0 0 auto;
        height: 34px;
        min-width: 48px;
    }

    /* 手机上「未暂存」必须常显：暂存按钮被收进「更多」，没有这条提示，
       用户会以为改动已经写进书里 */
    :global(:root[data-tepub-layout="phone"]) .phone-dirty-hint {
        flex: 0 0 auto;
        padding: 5px 12px;
        font-size: 11px;
        color: var(--color-warning, #a06412);
        background: color-mix(in srgb, var(--color-warning, #a06412) 12%, transparent);
        border-bottom: 1px solid var(--color-border);
    }

    /* ——— 手机动作条 + 符号栏 ———
       动作条固定在编辑栏底部（不是 fixed，是全屏布局里的一格）：
       fixed 会盖住手机底部导航，也会在键盘升起时错位。 */
    :global(:root[data-tepub-layout="phone"]) .phone-actionbar {
        flex: 0 0 auto;
        display: grid;
        grid-auto-flow: column;
        grid-auto-columns: 1fr;
        gap: 4px;
        padding: 6px 8px;
        border-top: 1px solid var(--color-border);
        background: var(--color-surface);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-actionbar button {
        /* 竖屏有富余高度，五个按钮（撤销/重做/查找/跳到行/保存）都用满
           触控尺寸；横屏那 337px 实在挤，另由 max-height:500px 块压到 36px。 */
        height: var(--tap-target);
        min-width: 0;
        padding: 0 4px;
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-actionbar button.active {
        border-color: var(--color-accent);
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
    }

    /* 符号栏横向滚动：22 个键在 390px 上放不下，硬折行会吃掉两行编辑区 */
    :global(:root[data-tepub-layout="phone"]) .phone-symbolbar {
        flex: 0 0 auto;
        display: flex;
        gap: 2px;
        padding: 4px;
        overflow-x: auto;
        overscroll-behavior-x: contain;
        border-top: 1px solid var(--color-border);
        background: var(--color-surface-soft);
        -webkit-overflow-scrolling: touch;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-symbolbar button {
        flex: 0 0 auto;
        min-width: 40px;
        height: 36px;
        padding: 0 8px;
        font-family: var(--font-code);
        font-size: 14px;
        background: transparent;
        border-color: transparent;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-symbolbar button:active {
        background: var(--color-accent-soft);
        border-color: var(--color-accent);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-find-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        padding-bottom: 6px;
        font-weight: 700;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-find-head button {
        height: 32px;
        font-size: 12px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-list {
        display: flex;
        flex-direction: column;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-list > .m-list-row {
        justify-content: space-between;
        border-bottom: 1px solid var(--color-border);
        font-size: 15px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-hint {
        font-size: 12px;
        color: var(--color-muted);
        flex: 0 0 auto;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        min-height: 52px;
        padding: 10px 16px;
        border-bottom: 1px solid var(--color-border);
        font-size: 15px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-inline {
        display: flex;
        gap: 8px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-more-inline button {
        /* 抽屉里的字号 A- / A+ 是主要操作，38px 高对拇指太小 */
        min-width: 56px;
        height: var(--tap-target);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-goto {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 12px 16px;
        font-size: 13px;
        color: var(--color-muted);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-goto input {
        flex: 1 1 auto;
        min-width: 0;
        height: 44px;
        font-size: 16px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-goto-actions {
        display: grid;
        grid-auto-flow: column;
        grid-auto-columns: 1fr;
        gap: 8px;
        padding: 0 16px 12px;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-goto-actions button {
        height: 44px;
    }

    /* 文件树/目录行：桌面 30px 高、行内还有 24px 的小按钮，手指按不准。
       这里把整行抬到 44px，并把「更多操作」按钮也撑到 40px 命中区。

       ⚠ 整条选择器必须包在 :global() 里，不能只包 :root —— Svelte 会给
       非 global 部分补上本组件的作用域属性，而 .tree-node / .node-content
       是 FileTreeItem.svelte、TocNode.svelte 渲染的，带的是它们自己的作用域
       属性。写成 `:global(...) .tree-content .tree-node` 会静默失效（编译期
       只报一句 unused selector，运行时表现为"手机上点不中文件"）。

       ⚠ 但也不能只写 .tree-content / .toc-list：这两个类名别处也在用
       （epubEditor/FileTreePanel、editor/ChapterSidebar、text-editor、
       mobile/make），包进 :global() 之后就是全应用的裸类名选择器，路由
       CSS 一旦加载就一直留在文档里，会把别的页面的列表行也撑成 44px。
       所以统一锚在本页独有的 .web-epub-page 上。 */
    :global(:root[data-tepub-layout="phone"] .web-epub-page .tree-content) {
        padding: 4px 6px calc(8px + var(--safe-bottom));
        /* 横屏手机（844×390）下 .tree-content 只有 163px 高，grid 的自动行
           会被压成等高（50px 一层），而文件夹的 .children 比它高得多 ——
           OEBPS / Text / META-INF 三层于是互相叠着画，文件名的字直接压在一
           起（竖屏 844 高时内容比容器高，行高反而是内容撑开的，所以只在
           横屏复现）。grid-auto-rows: max-content 让行高只跟内容走，
           超出的部分交给已有的 overflow:auto 滚动。桌面不受影响。 */
        grid-auto-rows: max-content;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-tree-filter {
        position: relative;
        display: flex;
        align-items: center;
        padding: 8px 8px 4px;
        background: var(--color-surface);
    }

    :global(:root[data-tepub-layout="phone"]) .phone-tree-filter input {
        width: 100%;
        height: 44px;
        /* 16px 以下是 iOS Safari 自动放大的触发线，代码里那些 13px 不能照搬 */
        font-size: 16px;
        padding: 0 40px 0 12px;
        box-sizing: border-box;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface-raised);
        color: var(--color-text);
        -webkit-appearance: none;
        appearance: none;
    }

    /* 原生 search 的清除按钮在安卓上长得和桌面不一致，统一由一个按钮负责 */
    :global(:root[data-tepub-layout="phone"]) .phone-tree-filter input::-webkit-search-cancel-button {
        display: none;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-tree-filter button {
        position: absolute;
        right: 10px;
        width: 34px;
        height: 34px;
        min-width: 0;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--color-muted);
        font-size: 18px;
        line-height: 1;
    }

    :global(
        :root[data-tepub-layout="phone"] .web-epub-page .tree-content .tree-node
    ) {
        min-height: 44px;
    }

    :global(
        :root[data-tepub-layout="phone"] .web-epub-page .tree-content .node-label
    ),
    :global(
        :root[data-tepub-layout="phone"] .web-epub-page .tree-content .file-node
    ) {
        min-height: 44px;
        box-sizing: border-box;
    }

    :global(
        :root[data-tepub-layout="phone"] .web-epub-page .tree-content .file-node
    ) {
        padding-left: 12px;
    }

    :global(
        :root[data-tepub-layout="phone"]
            .web-epub-page
            .tree-content
            .node-action
    ) {
        /* 44 是 --tap-target；行本身 44 高，按钮撑满行高就够了，
           不要用 38 —— 手指在「更多操作 / 添加文件」上按偏一格就点到行选择。 */
        width: var(--tap-target);
        height: var(--tap-target);
    }

    /* 目录树（TocNode）同理：层级更深，行高要够 */
    :global(
        :root[data-tepub-layout="phone"]
            .web-epub-page
            .toc-list
            .toc-node
            .node-content
    ) {
        min-height: 44px;
        box-sizing: border-box;
    }

    :global(:root[data-tepub-layout="phone"] .web-epub-page .toc-list) {
        padding-bottom: calc(8px + var(--safe-bottom));
    }

    :global(:root[data-tepub-layout="phone"]) .metadata-form {
        grid-template-columns: 1fr;
        padding-bottom: calc(16px + var(--safe-bottom));
    }

    :global(:root[data-tepub-layout="phone"]) .metadata-top {
        grid-template-columns: minmax(0, 1fr) 86px;
    }

    :global(:root[data-tepub-layout="phone"]) .metadata-cover {
        width: 86px;
        height: 116px;
    }

    :global(:root[data-tepub-layout="phone"]) .fr-row {
        flex-wrap: wrap;
    }

    :global(:root[data-tepub-layout="phone"]) .fr-label {
        flex-basis: 100%;
        text-align: left;
    }

    :global(:root[data-tepub-layout="phone"]) .fr-actions,
    :global(:root[data-tepub-layout="phone"]) .fr-find-actions {
        width: auto;
        flex: 1 1 160px;
    }

    /* ——— 横屏手机：把纵向装饰压薄 ———

       横屏（844×390）时可用高度只有 269px，而手机上常驻的横向条一共六条：
       页签 44 + 状态线 23 + 栏头 50 + 筛选框 56 + 动作条 51 + 符号栏 45
       = 269px —— 编辑区精确地被挤成 0，实测只剩 125px（不到 6 行代码）。
       竖屏（844）没有这个问题，所以只在「矮」的时候收，而不是按宽度判断。

       用 max-height 而不是 max-width：横屏手机宽 844px，任何宽度断点都
       命中不了；而这条规则的用意本来就是「高度不够」。桌面窗口再矮也带
       data-tepub-layout="desktop"，不会命中。 */
    @media (max-height: 500px) {
        :global(:root[data-tepub-layout="phone"]) .phone-pane-bar {
            height: 38px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-pane-bar button {
            height: 38px;
            font-size: 13px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-status {
            padding: 2px 12px;
            font-size: 10px;
        }

        :global(:root[data-tepub-layout="phone"]) .left-panel > .tabs,
        :global(:root[data-tepub-layout="phone"]) .left-panel > .tabs button {
            height: 40px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-tree-filter {
            padding: 4px 8px 2px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-tree-filter input {
            height: 38px;
            /* 全局 :where(...) 那条按 --control-height 给了 min-height: 44px，
               min-height 会压过 height，不显式放开就白写 */
            min-height: 0;
            font-size: 15px;
        }

        :global(:root[data-tepub-layout="phone"]) .editor-head {
            height: 40px;
            min-height: 40px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-actionbar {
            padding: 4px 6px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-actionbar button {
            height: 36px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-symbolbar {
            padding: 2px;
        }

        :global(:root[data-tepub-layout="phone"]) .phone-symbolbar button {
            height: 32px;
            min-width: 36px;
        }
    }
</style>
