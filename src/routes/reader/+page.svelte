<script lang="ts">
    import { onMount, onDestroy, tick } from "svelte";
    import { cleanErrorMessage } from "$lib/toast";
    import { isPhoneLayout } from "$lib/clientProfile";
    import { platform, type PlatformUnlisten } from "$lib/platform";
    import {
        loadAppSettings,
        resolveUiTheme,
        THEME_RESOLVED_EVENT,
    } from "$lib/appSettings";
    import { closeCurrentTab, usesMultiWindow } from "$lib/secondaryTabs";
    import {
        bookTextCache,
        readerCacheKey,
        readReaderCache,
        writeReaderCache,
    } from "$lib/readerCache";
    import { base } from "$app/paths";
    import { goto } from "$app/navigation";
    import {
        currentKeepAliveView,
        onlyWhenActive,
        titleSetter,
        viewUrl,
    } from "$lib/keepAlive/viewState";

    // 保活视图：切走时把整本书的 CSSStyleSheet 从 document 上摘下来，
    // 切回来再挂上。EPUB 里的 `p { }`、`body { }` 是全局选择器，多个页面
    // 同时挂载时会污染别人的排版（尤其是编辑器预览）。
    const keepAliveView = currentKeepAliveView();
    // getContext 只能在组件初始化时读：把本页地址先取出来，onMount 里直接用
    const ownUrl = viewUrl();
    // 保活下 <title> 会被别的页面覆盖，标题统一交给视图宿主登记
    const setWindowTitle = titleSetter();
    import ReaderTocNode from "$lib/ReaderTocNode.svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import {
        parseContainerRootfilePath,
        parseOpfPackage,
        parseNavDocument,
        parseNcxDocument,
        type WebEpubNavItem,
    } from "$lib/webEpub";
    import {
        READER_SETTINGS_KEY,
        readerProgressKey,
        readerBookmarksKey,
    } from "$lib/storageKeys";
    import { t } from "$lib/i18n.svelte";

    // ===== EPUB 数据结构 =====
    interface SpineEntry {
        path: string; // OPF dir 已解析后的相对路径，如 "OEBPS/Text/c1.xhtml"
        title: string; // 来自 nav/ncx 或 <title>，否则空
    }

    // 多级目录节点：解析自 EPUB3 nav 或 EPUB2 NCX 的层级结构。
    // spineIdx = -1 表示该节点只是分组标题（无对应 spine 章节）。
    interface TocNode {
        title: string;
        spineIdx: number;
        children: TocNode[];
    }

    // ===== 状态 =====
    let epubPath = ""; // 当前实际打开（可能已预处理）的路径
    let bookSourcePath = ""; // 用于进度 / 书签的稳定书源路径
    let bookTitle = "";
    let bookAuthor = "";
    // 书名解析出来后同步窗口标题。保活下由视图宿主登记 —— 隐藏页面的
    // <title> 会覆盖当前页的标题，所以不能只靠 svelte:head。
    // 这个文件是传统 `$:` 语法，不能混用 runes。
    $: setWindowTitle(bookTitle ? `${bookTitle} · ${t("阅读")}` : t("阅读"));
    let spine: SpineEntry[] = [];
    let toc: TocNode[] = []; // 多级目录树（顶层节点列表）
    let chapterStartCols: number[] = []; // 每章首列索引
    let totalCols = 0;
    let totalSpreads = 0;
    let currentSpread = 0;
    let scrollChapterIdx = 0;
    let scrollChapterOffset = 0;

    let loading = true;
    let loadingSource = "正在解压 EPUB…";
    let loadingParams: Record<string, unknown> = {};
    $: loadingMsg = t(loadingSource, loadingParams);
    let errorMsg = "";

    function setLoadingMessage(
        source: string,
        params: Record<string, unknown> = {},
    ) {
        loadingSource = source;
        loadingParams = params;
    }
    let unlistenPrepareStage: PlatformUnlisten | undefined;

    type EpubPrepareStageEvent = {
        epubPath: string;
        message: string;
    };

    // 阅读器设置（写入 localStorage）
    type ReadMode = "portrait" | "landscape";
    let readMode: ReadMode = "portrait"; // 兼容旧 CSS data-mode
    type ColumnLayout = "single" | "double";
    let columnLayout: ColumnLayout = "single"; // 平移翻页下的列布局
    type PageMode = "paginated" | "scroll";
    let pageMode: PageMode = "paginated"; // 翻页样式
    // 翻页样式切换期间，避免 Svelte 重渲染把旧页面的滚动位置写回进度。
    let suppressProgressSave = false;
    // 抑制期间用户仍可能滚动或布局被锚回。解除抑制后补存最后位置，
    // 避免中间态落盘，也避免最后一次滚动永远丢失。
    let progressSavePending = false;

    let fontSize = 19; // 文字主体 px
    let lineHeight = 1.85; // 行距倍数
    let paragraphSpacing = 0.6; // 段间距，em
    let pageMarginH = 0.04; // 横向边距占 viewport 的比例（左右）
    let pageMarginV = 0.02; // 纵向边距占 viewport 的比例（上下）
    let userFontFamily = ""; // 用户选择的字体名（空 = 默认）
    let useEpubFonts = true; // EPUB 自带字体启用 / 禁用
    type BodyTextTone = "theme" | "deep" | "black";
    let bodyTextTone: BodyTextTone = "deep";
    let bodyFontWeight = 500;

    // ===== 主题 =====
    type ThemePreset = "paper" | "eye" | "dark" | "sepia" | "snow";
    let themePreset: ThemePreset = "paper";
    let customBgImage = ""; // 用户自定义背景图（dataURL）。优先级高于 themePreset。
    /**
     * 用户是否自己挑过阅读主题。
     * 没挑过时跟随应用深/浅色（深色界面下默认就是暗夜），
     * 挑过之后一律尊重用户选择，不再被应用主题覆盖。
     */
    let themePresetExplicit = false;

    /** 应用主题是深色且用户没指定过阅读主题 → 用暗夜纸色 */
    function applyAppThemeDefault() {
        if (themePresetExplicit) return false;
        const nextDark = resolveUiTheme(loadAppSettings().uiTheme) === "dark";
        const nextPreset: ThemePreset = nextDark ? "dark" : "paper";
        if (themePreset === nextPreset) return false;
        themePreset = nextPreset;
        return true;
    }
    $: bodyTextColor = (() => {
        if (bodyTextTone === "theme") return "var(--rd-text)";
        if (bodyTextTone === "black")
            return themePreset === "dark" ? "#ffffff" : "#111111";
        switch (themePreset) {
            case "eye":
                return "#1f3425";
            case "sepia":
                return "#3f301f";
            case "snow":
                return "#111827";
            case "dark":
                return "#e8e8e8";
            default:
                return "#2f2a22";
        }
    })();

    // ===== 翻页交互开关 =====
    let fullScreenNext = false; // 全屏任意位置点击 = 下一页
    let wheelTurnPage = true; // 鼠标滚轮上下翻页（仅平移模式）
    let arrowTurnPage = true; // 方向键 / 空格翻页

    // ===== 书签 =====
    interface Bookmark {
        ch: number;
        sp: number;
        scrollOffset?: number;
        title: string; // 章节标题
        preview: string; // 该处章节标题或附近文字片段
        time: number; // 时间戳
    }
    let bookmarks: Bookmark[] = [];

    // ===== 工具栏 / 面板 =====
    // 新版工具栏：屏幕底部弹出，5 个 tab：目录 / 书签 / 主题 / 排版 / 设置
    type Panel =
        "" | "menu" | "toc" | "bookmarks" | "theme" | "typography" | "settings";
    let activePanel: Panel = "";
    let footnotePopupHtml = "";
    let toolbarOpen = false; // 工具栏（5 tab 横条）是否显示

    // 派生：root 元素的 inline CSS 变量字符串。
    // 之所以用派生而非直接在模板中写 style="..."{expr}...，是因为 svelte 解析器
    // 在嵌套模板字符串 `'${x}'` 时容易把内部的 `}` 当成表达式结束，造成编译失败。
    $: appStyle =
        `--rd-fontsize:${fontSize}px;` +
        `--rd-lineheight:${lineHeight};` +
        `--rd-paragraph-spacing:${paragraphSpacing}em;` +
        `--rd-user-font:${userFontFamily ? `"${userFontFamily}"` : "inherit"};` +
        `--rd-body-color:${bodyTextColor};` +
        `--rd-body-weight:${bodyFontWeight};` +
        `--rd-bg-image:${customBgImage ? `url("${customBgImage.replace(/"/g, '\\"')}")` : "none"};`;

    // DOM 引用
    let frameEl: HTMLElement;
    let viewportEl: HTMLDivElement;
    // 目录面板容器：打开目录面板时用来自动滚动到当前章节行
    let tocPanelEl: HTMLDivElement | null = null;
    let canvasEl: HTMLDivElement;
    let pageEl: HTMLDivElement;

    // 已渲染的合并 HTML（注入到 pageEl）
    let combinedHtml = "";

    // ===== 增量 hydrate 状态 =====
    // 性能策略：整本书一次性合并几十 MB 文本注入 pageEl 会让浏览器
    //   - innerHTML 同步解析整段 HTML
    //   - 多列布局对超大 DOM 同步计算列分布
    // 这两步都是单线程同步操作，对大书直接卡几分钟没文字。
    // 优化：第一次只把"当前章节附近若干章"渲染为完整 HTML，其他章节用极轻
    //       占位 <section>，浏览器秒级完成首屏布局。后台 idle 逐章替换占位
    //       为完整内容，每次替换后重新 measure 列分布并锚定回当前章节相对位置。
    let chapterFullHtml: string[] = []; // 每章完整 HTML（不变）
    // 每章 body 上的 background 样式（提取自 EPUB 自带 body inline style 中的
    // background-* 属性）。当当前章节有自带背景时，把它复制到 .rd-chapter-bg
    // 全屏背景层，让背景图覆盖整个 frame 而不是只覆盖列宽。
    let chapterBgStyle: string[] = [];
    let renderedSet = new Set<number>(); // 已 hydrate 的章节索引集合
    let hydrateScheduled = false;
    let hydrateId: any = null;
    let destroyed = false;
    const RENDER_RADIUS = 2; // 首屏渲染当前章节前后各 N 章（共 2N+1 章）

    // 时钟
    let nowStr = "";
    let clockTimer: any = null;

    // 旧版 settingsOpen / toolbarVisible 已被新版 activePanel 替代，删除以避免歧义。

    let appEl: HTMLDivElement;

    // 进度
    let progressKey = "";

    // 用户通过 End 跳到书末后，后台增量水合会不断重测总列数。
    // 保持“仍在书末”的意图，直到用户主动离开末页。
    let keepAtBookEnd = false;
    let pinningBookEnd = false;

    // ===== 工具函数 =====
    function pad2(n: number): string {
        return n < 10 ? "0" + n : "" + n;
    }

    function tickClock() {
        const d = new Date();
        nowStr = `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
    }

    // 路径解析：以 base 为锚（含文件名），把 rel 解析为同根相对路径
    function resolveRelative(base: string, rel: string): string {
        if (!rel) return base;
        // 去除 fragment
        const hashIdx = rel.indexOf("#");
        if (hashIdx >= 0) rel = rel.slice(0, hashIdx);
        if (!rel) return base.split("#")[0];
        if (rel.startsWith("/")) return rel.replace(/^\/+/, "");
        const baseParts = base.split("/").slice(0, -1);
        const relParts = rel.split("/");
        for (const p of relParts) {
            if (p === "" || p === ".") continue;
            if (p === "..") baseParts.pop();
            else baseParts.push(p);
        }
        return baseParts.join("/");
    }

    function safeDecode(value: string): string {
        try {
            return decodeURIComponent(value);
        } catch {
            return value;
        }
    }

    function migrateLegacyReaderKeys(processedPath: string) {
        const sourcePath = bookSourcePath || epubPath;
        if (!sourcePath || !processedPath || processedPath === sourcePath)
            return;
        const pairs = [
            [readerProgressKey(processedPath), readerProgressKey(sourcePath)],
            [readerBookmarksKey(processedPath), readerBookmarksKey(sourcePath)],
        ];
        try {
            for (const [legacyKey, stableKey] of pairs) {
                if (!localStorage.getItem(stableKey)) {
                    const legacy = localStorage.getItem(legacyKey);
                    if (legacy !== null)
                        localStorage.setItem(stableKey, legacy);
                }
            }
        } catch {}
    }

    // XML documents can expose inline style declarations as inert attributes.
    // Reassigning cssText in the final HTML document makes them live declarations
    // again, so display:none, text-align, margins, etc. actually take effect.
    function normalizeInlineStyles(root: ParentNode) {
        for (const el of Array.from(root.querySelectorAll("[style]"))) {
            const cssText = el.getAttribute("style") || "";
            if (!cssText.trim()) continue;
            (el as HTMLElement).style.cssText = cssText;
        }
    }

    const adoptedChapterStyleCss = new Set<string>();
    const adoptedChapterStyleSheets = new Set<CSSStyleSheet>();

    // Svelte's @html insertion leaves EPUB style elements inert in this
    // WebView2 build. Constructable stylesheets are applied by the CSS engine
    // and restore the book's font, color and layout rules. Linked CSS is
    // repeated in every chapter, so adopt each distinct stylesheet only once.
    function adoptChapterStyles(root: ParentNode) {
        const pending: {
            node: HTMLStyleElement;
            sheet: CSSStyleSheet;
            css: string;
        }[] = [];
        for (const node of Array.from(
            root.querySelectorAll("style"),
        ) as HTMLStyleElement[]) {
            const css = node.textContent || "";
            if (!css.trim()) continue;
            if (adoptedChapterStyleCss.has(css)) {
                node.remove();
                continue;
            }
            try {
                const sheet = new CSSStyleSheet();
                sheet.replaceSync(css);
                pending.push({ node, sheet, css });
            } catch (error) {
                console.warn("EPUB style adoption failed:", error);
            }
        }
        if (pending.length === 0) return;
        try {
            document.adoptedStyleSheets = [
                ...document.adoptedStyleSheets,
                ...pending.map((entry) => entry.sheet),
            ];
        } catch (error) {
            console.warn("EPUB stylesheet registration failed:", error);
            return;
        }
        for (const entry of pending) {
            adoptedChapterStyleCss.add(entry.css);
            adoptedChapterStyleSheets.add(entry.sheet);
            entry.node.remove();
        }
    }

    /** 把已登记的样式表挂回 document（切回本视图时用） */
    function attachAdoptedChapterStyles() {
        if (adoptedChapterStyleSheets.size === 0) return;
        try {
            document.adoptedStyleSheets = [
                ...document.adoptedStyleSheets,
                ...adoptedChapterStyleSheets,
            ];
        } catch (error) {
            console.warn("EPUB stylesheet re-registration failed:", error);
        }
    }

    /**
     * 把样式表从 document 上摘下来，但**保留 sheet 对象**。
     *
     * 保活模式下本页切走时调用：EPUB 里的 `p { }`、`body { }` 是全局选择器，
     * 不摘掉会污染同时挂载着的其它页面。保留对象是因为章节里的 style 节点
     * 在首次登记时就已经从 DOM 移除了 —— 清掉这一份就再也拼不回来。
     */
    function detachAdoptedChapterStyles() {
        if (adoptedChapterStyleSheets.size === 0) return;
        document.adoptedStyleSheets = document.adoptedStyleSheets.filter(
            (sheet) => !adoptedChapterStyleSheets.has(sheet),
        );
    }

    /** 页面真正销毁时才连对象一起丢掉 */
    function clearAdoptedChapterStyles() {
        detachAdoptedChapterStyles();
        adoptedChapterStyleSheets.clear();
        adoptedChapterStyleCss.clear();
    }

    // 注：之前需要 mimeForExt / bytesToBlobUrl 把 IPC 回来的二进制转 blob，
    // 现在改走 Tauri Asset Protocol 让 webview 自己加载本地文件，已不再需要。

    // 跨命名空间按 localName 取首个元素
    function findFirstByLocalName(
        root: Element | Document,
        localName: string,
    ): Element | null {
        const all = root.getElementsByTagName("*");
        for (let i = 0; i < all.length; i++) {
            if (all[i].localName === localName) return all[i] as Element;
        }
        return null;
    }
    function findAllByLocalName(
        root: Element | Document,
        localName: string,
    ): Element[] {
        const out: Element[] = [];
        const all = root.getElementsByTagName("*");
        for (let i = 0; i < all.length; i++) {
            if (all[i].localName === localName) out.push(all[i] as Element);
        }
        return out;
    }

    // ===== EPUB 解析 =====
    function markOpeningHeaderImage(body: HTMLElement | null) {
        // Intentionally no-op: keep EPUB opening artwork in its original flow.
        // The reader's earlier header-image normalization fought CSS columns and
        // broke books whose chapter art/title/logo are designed as one EPUB block.
        void body;
    }

    async function loadEpub() {
        loading = true;
        errorMsg = "";
        try {
            // 1) 准备文件（处理混淆 / 加密格式）
            setLoadingMessage("正在解压 EPUB…");
            // 源文件身份（修改时间 + 大小），与解压目录一起构成缓存键
            let sourceModifiedMs = 0;
            let sourceSize = 0;
            try {
                const prepared: any = await platform.invoke(
                    "prepare_epub_for_open",
                    { epubPath },
                );
                // EpubPrepareResult 使用 camelCase 序列化；兼容旧版/其他调用端的
                // snake_case 字段，否则伪加密与文件名混淆修复会被静默跳过。
                const preparedPath =
                    prepared?.processedPath || prepared?.processed_path;
                sourceModifiedMs =
                    prepared?.sourceModifiedMs ??
                    prepared?.source_modified_ms ??
                    0;
                sourceSize = prepared?.sourceSize ?? prepared?.source_size ?? 0;
                if (preparedPath) {
                    migrateLegacyReaderKeys(preparedPath);
                    epubPath = preparedPath;
                    loadBookmarks();
                }
            } catch (e) {
                // 非致命，继续尝试
                console.warn("prepare_epub_for_open 失败，继续:", e);
            }

            // 2) 触发解压并建立缓存
            await platform.invoke("extract_epub", { epubPath });

            // 2.5) 立刻拿临时目录绝对路径，准备 asset URL 拼接器和 fetch 辅助
            //
            // 关键性能策略：所有"文本读取"也改走 webview fetch + asset:// 协议，
            // 不再用 read_epub_file_content / read_epub_files_batch IPC。原因：
            //   - 一次 batch 几十 MB JSON 字符串通过 IPC 序列化、传输、JSON.parse、
            //     是单线程同步阻塞，大书直接卡几分钟。loadingMsg 哪怕已经被赋值
            //     "读取样式表"，svelte 也来不及刷帧 —— 用户看到的就是"卡在样式表"。
            //   - fetch 是浏览器原生网络栈，可以并行多个 host 内请求，每个返回小字符串
            //     立即让出主线程，无单次大序列化。N 章 × 100KB 并行 fetch 比单次 N×100KB
            //     IPC batch 快至少一个数量级。
            let tempDirPath = "";
            try {
                tempDirPath = await platform.invoke<string>(
                    "get_epub_temp_dir_path",
                    { epubPath },
                );
            } catch (e) {
                console.warn(
                    "get_epub_temp_dir_path 失败，回退到 IPC 文本读取:",
                    e,
                );
            }
            // 缓存键必须带上解压目录：章节 HTML 里的图片/CSS 已经是 asset URL，
            // 换个解压目录（重启后 temp dir 会变）这些 URL 就全失效了
            const bookKey = readerCacheKey(
                bookSourcePath || epubPath,
                sourceModifiedMs,
                sourceSize,
                tempDirPath,
            );
            // 统一用正斜杠拼路径。Windows 上 convertFileSrc 也接受正斜杠，且这样
            // 生成的 asset URL 路径里仍保留 `/` 作为段分隔符 —— 浏览器在解析 EPUB
            // 自带 CSS 里的 `@import url("fonts.css")` / `@font-face src:url("../Fonts/x.ttf")`
            // / `background:url("../Images/x.jpg")` 时，按 URL 相对路径规则正常退回上级目录。
            // 旧实现把 tempDir 转成反斜杠再 convertFileSrc，整条 path 被 %5C 编码成
            // 单 segment（asset.localhost/C%3A%5C...%5Cfonts.css），相对 url 用 `..`
            // 一退就退到 root，于是出现 GET asset.localhost/Fonts/x.ttf 之类 500，
            // 全屏背景图也因此加载不出来。
            const tempDirSlash = tempDirPath
                .replace(/\\/g, "/")
                .replace(/\/+$/, "");
            // 同步转换器从 platform 异步获取一次（Tauri 端懒加载 convertFileSrc；
            // Web 端为恒等函数）——toAssetUrl 会在 processCss 的同步 replace 回调里
            // 被逐次调用，必须保持同步。
            const convertFileSrc = tempDirPath
                ? await platform.getFileSrcConverter()
                : null;
            function toAssetUrl(absInEpub: string): string {
                if (!tempDirPath || !convertFileSrc) return absInEpub;
                const cleaned = absInEpub
                    .replace(/\\/g, "/")
                    .replace(/^\/+/, "");
                return convertFileSrc(`${tempDirSlash}/${cleaned}`, "asset");
            }

            // 文本读取统一接口：优先内存缓存，其次 fetch，失败回退 IPC
            //
            // 同窗口模式切回阅读器标签时页面会重新挂载，整本书要重新读一遍。
            // 800 章就是 800 多次 asset fetch（实测 2~3 秒），而这批文本在同一
            // 本书、同一个解压目录里不会变 —— 用 bookKey 缓存起来，第二次进来
            // 直接命中内存，只剩 DOMParser 与 CSS 处理的成本。
            const textCache = bookTextCache(bookKey);
            async function fetchText(absInEpub: string): Promise<string> {
                const cached = textCache.get(absInEpub);
                if (cached !== undefined) return cached;
                let text: string | null = null;
                if (tempDirPath) {
                    try {
                        const resp = await fetch(toAssetUrl(absInEpub));
                        if (resp.ok) text = await resp.text();
                        else
                            console.warn(
                                `fetch ${absInEpub} 状态 ${resp.status}，回退 IPC`,
                            );
                    } catch (e) {
                        console.warn(`fetch ${absInEpub} 失败，回退 IPC:`, e);
                    }
                }
                if (text === null) {
                    text = await platform.invoke<string>(
                        "read_epub_file_content",
                        { epubPath, filePath: absInEpub },
                    );
                }
                // 读失败时写空串会把后续重试路径也堵死，成功才缓存
                if (text !== "") textCache.set(absInEpub, text);
                return text;
            }

            // 2.6) 解析成品缓存：整本 parse + 序列化的结果与阅读进度无关，
            //      切标签回来时直接复用 —— 否则 800 章要重跑 800 次 DOMParser
            //      和一遍 CSS 处理，这就是「切回去像重新加载」的主要成本。
            //      排版（列宽/字号/当前页）不走缓存，下面仍然重新测量与恢复。
            const cachedSnapshot = readReaderCache(bookKey);
            if (cachedSnapshot) {
                bookTitle = cachedSnapshot.title;
                bookAuthor = cachedSnapshot.author;
                spine = cachedSnapshot.spine;
                toc = cachedSnapshot.toc;
                chapterFullHtml = cachedSnapshot.chapterFullHtml;
                chapterBgStyle = cachedSnapshot.chapterBgStyle;
                setLoadingMessage("恢复上次的排版…");
            } else {
                // 3) 读取 container.xml -> OPF 路径（解析与 webEpub.ts 共用同一实现）
                setLoadingMessage("解析目录…");
                let containerXml = "";
                try {
                    containerXml = await fetchText("META-INF/container.xml");
                } catch (e) {
                    throw new Error(
                        t("找不到 META-INF/container.xml：{error}", {
                            error: String(e),
                        }),
                    );
                }
                const opfPath = parseContainerRootfilePath(containerXml);

                // 4) 读取并解析 OPF
                const opfXml = await fetchText(opfPath);
                const pkg = parseOpfPackage(opfXml, opfPath);
                bookTitle =
                    (pkg.metadata.title || "").trim() || t("未命名");
                bookAuthor = (pkg.metadata.creator || "").trim();

                const spinePaths: string[] = pkg.spine
                    .map((s) => s.manifest?.fullPath)
                    .filter((h): h is string => !!h);

                // 5) 读取 NCX / Nav 用于章节标题 + 多级目录树
                //
                // 两个产物：
                //   titleByPath: 每个 spine 文件的最佳章节标题，用于 spine[i].title
                //   toc:         多级目录树，每个节点带一个 spineIdx（或 -1）
                //                用于"目录"面板的 ReaderTocNode 递归渲染
                const titleByPath = new Map<string, string>();

                // 把 href 解析到 spine 索引。nav/ncx 里的 href 可能带 #fragment
                // （章节内部锚点），但我们粒度只到 spine 文件 —— 丢掉 fragment 后再匹配。
                function hrefToSpineIdx(absHref: string): number {
                    const clean = absHref.split("#")[0];
                    for (let i = 0; i < spinePaths.length; i++) {
                        if (spinePaths[i] === clean) return i;
                    }
                    return -1;
                }

                // webEpub.ts 的层级 nav/ncx 解析结果 -> 本页 TocNode（附带填 titleByPath）
                const mapNavItems = (items: WebEpubNavItem[]): TocNode[] =>
                    items.map((item) => {
                        const abs = item.fullPath
                            ? item.fullPath.split("#")[0]
                            : "";
                        if (abs && item.label && !titleByPath.has(abs)) {
                            titleByPath.set(abs, item.label);
                        }
                        return {
                            title: item.label || t("(未命名)"),
                            spineIdx: item.fullPath
                                ? hrefToSpineIdx(item.fullPath)
                                : -1,
                            children: item.children
                                ? mapNavItems(item.children)
                                : [],
                        };
                    });

                // EPUB3 nav
                const navEntry = pkg.manifest.find((m) =>
                    m.properties.split(/\s+/).includes("nav"),
                );
                if (navEntry) {
                    try {
                        const navHtml = await fetchText(navEntry.fullPath);
                        toc = mapNavItems(
                            parseNavDocument(navHtml, navEntry.fullPath),
                        );
                    } catch {}
                }

                // EPUB2 NCX (media-type == application/x-dtbncx+xml 或扩展名 .ncx)
                if (toc.length === 0) {
                    const ncxEntry = pkg.manifest.find(
                        (m) =>
                            m.mediaType === "application/x-dtbncx+xml" ||
                            m.fullPath.endsWith(".ncx"),
                    );
                    if (ncxEntry) {
                        try {
                            const ncxXml = await fetchText(ncxEntry.fullPath);
                            toc = mapNavItems(
                                parseNcxDocument(ncxXml, ncxEntry.fullPath),
                            );
                        } catch {}
                    }
                }
                // 最终兜底：没有 nav 也没有 ncx —— 每个 spine 文件就是一个顶层目录项。
                if (toc.length === 0) {
                    toc = spinePaths.map((p, i) => ({
                        title:
                            titleByPath.get(p) ||
                            t("第 {index} 节", { index: i + 1 }),
                        spineIdx: i,
                        children: [],
                    }));
                }

                // 6) 形成 spine
                spine = spinePaths.map((p) => ({
                    path: p,
                    title: titleByPath.get(p) || "",
                }));
                if (spine.length === 0)
                    throw new Error(t("EPUB spine 为空，无法显示"));

                // 7) 并行 fetch 所有章节文本（不再用 batch IPC）
                //
                // 之前 read_epub_files_batch 一次返回整本书所有 spine 文本（几百 KB
                // 到几十 MB JSON），单次 IPC 序列化 + JSON.parse 同步阻塞主线程几秒
                // 到几分钟 —— 这才是用户看到 "读取样式表"（实际是上一步还没让出
                // 主线程）卡几分钟的真正源头。
                // 改为 fetch 之后：浏览器并行多路加载，每个返回小字符串立即让出主线程，
                // loadingMsg 也能实时刷帧让用户看到进度。
                let loadedCh = 0;
                const totalCh = spine.length;
                setLoadingMessage("读取章节 0/{total}…", {
                    total: totalCh,
                });
                const chapterTextEntries = await Promise.all(
                    spine.map(async (s) => {
                        try {
                            const text = await fetchText(s.path);
                            return [s.path, text] as const;
                        } catch (e) {
                            console.warn(`读取章节失败 ${s.path}:`, e);
                            return [s.path, ""] as const;
                        } finally {
                            loadedCh++;
                            // 每完成一个章节都更新一次 loadingMsg，配合 await 让 UI 能刷帧
                            setLoadingMessage(
                                "读取章节 {current}/{total}…",
                                {
                                    current: loadedCh,
                                    total: totalCh,
                                },
                            );
                        }
                    }),
                );
                const chapterTexts: Record<string, string> =
                    Object.fromEntries(chapterTextEntries);

                // 8) 解析章节并收集所有图片 / 外部 CSS 引用（一次性，不发起 IPC）
                setLoadingMessage("解析章节…");
                const parsedDocs: (Document | null)[] = new Array(
                    spine.length,
                ).fill(null);
                type ImgRef = {
                    el: Element;
                    abs: string;
                    isSvg: boolean;
                    useXlink: boolean;
                };
                const allImgRefs: ImgRef[] = [];
                const imagePathSet = new Set<string>();
                // EPUB 自带 CSS（如 @font-face 字体定义）的引用
                type CssLinkRef = {
                    el: Element;
                    chapterIdx: number;
                    abs: string;
                };
                const cssLinkRefs: CssLinkRef[] = [];
                const cssPathSet = new Set<string>();

                for (let i = 0; i < spine.length; i++) {
                    const raw = chapterTexts[spine[i].path];
                    if (!raw) continue;
                    let doc: Document;
                    try {
                        doc = new DOMParser().parseFromString(
                            raw,
                            "application/xhtml+xml",
                        );
                        if (doc.querySelector("parsererror"))
                            throw new Error("xhtml parse error");
                    } catch {
                        doc = new DOMParser().parseFromString(raw, "text/html");
                    }
                    // 移除 script（安全）；保留 style 标签和 inline style 让 EPUB 自带的
                    // 颜色 / 对齐 / 字体规则生效。link[rel=stylesheet] 不直接移除，下面会
                    // IPC 读其内容并替换为 style 标签，连带把 url() 字体文件转成 blob URL。
                    doc.querySelectorAll("script").forEach((n) => n.remove());
                    normalizeInlineStyles(doc);

                    Array.from(
                        doc.querySelectorAll('link[rel="stylesheet"]'),
                    ).forEach((link) => {
                        const href = link.getAttribute("href") || "";
                        if (
                            !href ||
                            /^https?:/i.test(href) ||
                            href.startsWith("data:")
                        ) {
                            link.remove();
                            return;
                        }
                        const abs = resolveRelative(spine[i].path, href);
                        cssLinkRefs.push({ el: link, chapterIdx: i, abs });
                        cssPathSet.add(abs);
                    });

                    // 收集 <img>
                    Array.from(doc.querySelectorAll("img")).forEach((img) => {
                        const src = img.getAttribute("src");
                        img.removeAttribute("width");
                        img.removeAttribute("height");
                        if (
                            !src ||
                            src.startsWith("data:") ||
                            /^https?:/i.test(src)
                        )
                            return;
                        const abs = resolveRelative(spine[i].path, src);
                        imagePathSet.add(abs);
                        allImgRefs.push({
                            el: img,
                            abs,
                            isSvg: false,
                            useXlink: false,
                        });
                    });
                    // 收集 SVG <image>
                    Array.from(doc.getElementsByTagName("image")).forEach(
                        (img) => {
                            const xlink = img.getAttributeNS(
                                "http://www.w3.org/1999/xlink",
                                "href",
                            );
                            const href =
                                img.getAttribute("href") || xlink || "";
                            if (
                                !href ||
                                href.startsWith("data:") ||
                                /^https?:/i.test(href)
                            )
                                return;
                            const abs = resolveRelative(spine[i].path, href);
                            imagePathSet.add(abs);
                            allImgRefs.push({
                                el: img,
                                abs,
                                isSvg: true,
                                useXlink: xlink !== null,
                            });
                        },
                    );
                    parsedDocs[i] = doc;
                }

                // ===== 9) CSS 文件读取与资源 URL 替换 =====
                //
                // tempDirPath / toAssetUrl / fetchText 已在第 2.5 步定义并被前面的
                // container/OPF/Nav/NCX/spine 使用，这里直接复用。
                const URL_RE = /url\s*\(\s*(['"]?)([^'")]+?)\1\s*\)/g;
                // @import 两种语法：@import url("x.css") 或 @import "x.css"。带可选 media list。
                const IMPORT_RE =
                    /@import\s+(?:url\s*\(\s*(['"]?)([^'")]+?)\1\s*\)|(['"])([^'"]+?)\3)\s*[^;]*;/g;

                // —— CSS 文件并行 fetch（不再用 batch IPC，避免一次返回所有 CSS 字符串）
                const cssPaths = Array.from(cssPathSet);
                const cssContent = new Map<string, string>();
                if (cssPaths.length > 0) {
                    let loadedCss = 0;
                    setLoadingMessage("读取样式表 0/{total}…", {
                        total: cssPaths.length,
                    });
                    const cssEntries = await Promise.all(
                        cssPaths.map(async (p) => {
                            try {
                                return [p, await fetchText(p)] as const;
                            } catch (e) {
                                console.warn(`读取 CSS 失败 ${p}:`, e);
                                return [p, ""] as const;
                            } finally {
                                loadedCss++;
                                setLoadingMessage(
                                    "读取样式表 {current}/{total}…",
                                    {
                                        current: loadedCss,
                                        total: cssPaths.length,
                                    },
                                );
                            }
                        }),
                    );
                    for (const [p, content] of cssEntries)
                        cssContent.set(p, content);
                }

                // —— 递归拉平所有 CSS 的 @import：被 @import 引用的子样式表也提前 fetch 进来
                // 进 cssContent。否则浏览器加载主 CSS 后会自己去 asset:// 拉子表，子表里
                // url(../Fonts/x.ttf) 走浏览器 URL 相对解析。Tauri 在 Windows 上的 asset
                // URL 是 `https://asset.localhost/{encodeURIComponent(path)}` —— 整条路径
                // 被 %2F / %5C 编码成单 segment，浏览器相对解析时 `..` 会一退到 root，
                // 实际请求变成 `asset.localhost/Fonts/x.ttf` → 500（找不到文件）。
                // 解决：所有 @import 提前 fetch 入内存，processCss 时把 @import 整句替换
                // 为对应已处理（含 url() 改写、body→.epub-body）的内容内联进来。
                // 浏览器拿到的最终样式里没有 @import，绝对不会再触发相对解析问题。
                async function ensureCssLoaded(absPath: string): Promise<void> {
                    if (cssContent.has(absPath)) return;
                    try {
                        cssContent.set(absPath, await fetchText(absPath));
                    } catch (e) {
                        console.warn(
                            `@import 子样式表 fetch 失败 ${absPath}:`,
                            e,
                        );
                        cssContent.set(absPath, "");
                    }
                }
                // BFS：对每个已知 CSS 扫 @import，把新发现的 abs 路径加进 fetch 队列
                {
                    const seen = new Set<string>(cssContent.keys());
                    // 起点除已加载的 link CSS 外，还要把章节内嵌 style 块里的 @import 算上 ——
                    // 它们指向的子表同样要预拉，否则 processCss 内联时拿不到内容。
                    // 用一个临时 Map 暂存内嵌 style 的"虚拟 base 路径 → 文本"以参与 BFS。
                    const inlineStyleSources: { base: string; txt: string }[] =
                        [];
                    for (let i = 0; i < spine.length; i++) {
                        const doc = parsedDocs[i];
                        if (!doc) continue;
                        Array.from(doc.querySelectorAll("style")).forEach(
                            (s) => {
                                const t = s.textContent || "";
                                if (t)
                                    inlineStyleSources.push({
                                        base: spine[i].path,
                                        txt: t,
                                    });
                            },
                        );
                    }
                    // 把内嵌 style 的 @import 直接加到第一轮 frontier
                    const initialFromInline: string[] = [];
                    for (const src of inlineStyleSources) {
                        IMPORT_RE.lastIndex = 0;
                        let m: RegExpExecArray | null;
                        while ((m = IMPORT_RE.exec(src.txt)) !== null) {
                            const rel = (m[2] || m[4] || "").trim();
                            if (
                                !rel ||
                                rel.startsWith("data:") ||
                                /^https?:/i.test(rel)
                            )
                                continue;
                            const abs = resolveRelative(src.base, rel);
                            if (!seen.has(abs)) {
                                seen.add(abs);
                                initialFromInline.push(abs);
                            }
                        }
                    }
                    if (initialFromInline.length > 0) {
                        await Promise.all(
                            initialFromInline.map(ensureCssLoaded),
                        );
                    }
                    let frontier = [
                        ...Array.from(cssContent.keys()),
                        ...initialFromInline,
                    ];
                    while (frontier.length > 0) {
                        const newAbs: string[] = [];
                        for (const cur of frontier) {
                            const txt = cssContent.get(cur) || "";
                            IMPORT_RE.lastIndex = 0;
                            let m: RegExpExecArray | null;
                            while ((m = IMPORT_RE.exec(txt)) !== null) {
                                const rel = (m[2] || m[4] || "").trim();
                                if (
                                    !rel ||
                                    rel.startsWith("data:") ||
                                    /^https?:/i.test(rel)
                                )
                                    continue;
                                const abs = resolveRelative(cur, rel);
                                if (!seen.has(abs)) {
                                    seen.add(abs);
                                    newAbs.push(abs);
                                }
                            }
                        }
                        if (newAbs.length === 0) break;
                        await Promise.all(newAbs.map(ensureCssLoaded));
                        frontier = newAbs;
                    }
                }

                // —— 把 EPUB 章节里的图片 src 直接指向 asset URL
                // 这一步完全本地，无 IPC，对 1000 张图也只是 setAttribute 调用，毫秒级。
                for (const ref of allImgRefs) {
                    const url = toAssetUrl(ref.abs);
                    if (ref.isSvg) {
                        if (ref.useXlink) {
                            ref.el.setAttributeNS(
                                "http://www.w3.org/1999/xlink",
                                "href",
                                url,
                            );
                        }
                        ref.el.setAttribute("href", url);
                    } else {
                        ref.el.setAttribute("src", url);
                    }
                }

                // —— 第四步：处理 CSS 文本（url() 改写为 asset URL，body/html 改写为 .epub-body）
                //
                // 后断言除常见的 \s , { : . [ # 外，还要包括 ) —— 处理 :where(html, body)
                // 这种 body 紧跟 ) 的写法。漏掉它会让整条规则 body 选择器没替换、规则失效。
                // body / html 选择器同时映射到 .epub-body 与 .rd-chapter 两类元素：
                // - .epub-body 是真正的内容容器（文字 / 图片 / inline style），承担"局部样式"
                // - .rd-chapter 是 section 容器，被 CSS column-fragmented 时每页都画一份背景
                // 用户 CSS 中 `body.full { background:url(); ... }` 经替换后写到这两个选择器
                // 上 → .rd-chapter.full 拿到 background 即可在每列重复绘制实现"全屏背景"。
                const BODY_RE = /(^|[\s,>+~(])(?:body|html)(?=[\s,{:.\[#)])/g;
                const BODY_REPLACE = "$1.epub-body, $1.rd-chapter";
                // processedCssCache 记忆每个 CSS 已经处理（含递归内联）的最终文本。
                // 同一文件可能被多个 link / @import 引用，避免重复处理。
                const processedCssCache = new Map<string, string>();
                function processCss(
                    css: string,
                    cssPath: string,
                    visiting: Set<string>,
                ): string {
                    // 1) 先把 @import 整句替换为已处理的子表内容（递归内联）
                    //    用 replace + 函数回调一次扫完，避免在循环里改字符串导致 lastIndex 错位
                    const inlined = css.replace(
                        IMPORT_RE,
                        (match, _q1, u1, _q2, u2) => {
                            const rel = (u1 || u2 || "").trim();
                            if (
                                !rel ||
                                rel.startsWith("data:") ||
                                /^https?:/i.test(rel)
                            )
                                return match;
                            const abs = resolveRelative(cssPath, rel);
                            if (visiting.has(abs)) return ""; // 循环引用：丢掉本次 @import
                            const sub = cssContent.get(abs);
                            if (sub === undefined) return ""; // 没拉到内容也丢掉，避免浏览器再去 fetch
                            if (processedCssCache.has(abs))
                                return processedCssCache.get(abs)!;
                            const next = new Set(visiting);
                            next.add(abs);
                            const out = processCss(sub, abs, next);
                            processedCssCache.set(abs, out);
                            return out;
                        },
                    );
                    // 2) 在已无 @import 的文本上做 url() 与 body/html 改写
                    return inlined
                        .replace(URL_RE, (match, _q, url) => {
                            const trimmed = url.trim();
                            if (
                                !trimmed ||
                                trimmed.startsWith("data:") ||
                                /^https?:/i.test(trimmed) ||
                                trimmed.startsWith("#")
                            ) {
                                return match;
                            }
                            const abs = resolveRelative(cssPath, trimmed);
                            return `url("${toAssetUrl(abs)}")`;
                        })
                        .replace(BODY_RE, BODY_REPLACE);
                }
                for (const ref of cssLinkRefs) {
                    let css = "";
                    if (cssContent.has(ref.abs)) {
                        if (processedCssCache.has(ref.abs)) {
                            css = processedCssCache.get(ref.abs)!;
                        } else {
                            css = processCss(
                                cssContent.get(ref.abs)!,
                                ref.abs,
                                new Set([ref.abs]),
                            );
                            processedCssCache.set(ref.abs, css);
                        }
                    }
                    const ownerDoc = ref.el.ownerDocument || document;
                    const styleEl = ownerDoc.createElement("style");
                    styleEl.textContent = css;
                    ref.el.parentNode?.replaceChild(styleEl, ref.el);
                }

                // 同样处理章节内嵌 style 块：走 processCss（含 @import 内联、url() 改写、
                // body→.epub-body）。某些 EPUB 在章节里直接写 inline style 块带 @import "../Styles/x.css"，
                // 必须把子表也内联进来，避免浏览器自己 fetch 时被 asset URL 单 segment
                // 路径绊倒（详见上面 ensureCssLoaded 注释）。
                for (let i = 0; i < spine.length; i++) {
                    const doc = parsedDocs[i];
                    if (!doc) continue;
                    const chapterPath = spine[i].path;
                    Array.from(doc.querySelectorAll("style")).forEach(
                        (styleEl) => {
                            const txt = styleEl.textContent || "";
                            if (!txt) return;
                            styleEl.textContent = processCss(
                                txt,
                                chapterPath,
                                new Set([chapterPath]),
                            );
                        },
                    );
                }

                // 11) 序列化各章节为完整 HTML（仅生成数组，不立刻拼接）
                // 关键修复：head 里的 style 标签（含 link 替换来的）必须显式提取注入到
                // section 内 —— body.innerHTML 不会包含 head 的内容，否则 EPUB 自带的字体
                // 定义、颜色、对齐等 CSS 全部失效（用户看到"样式直接全没了"）。
                //
                // 同样关键：body 自身的 class / style / id 也要复制到 .epub-body 包装 div，
                // 否则 epub 通过 `body.fullpage` 这类 class，或 `<body style="background:url()">`
                // 这种 inline 写法设的背景全部丢失（用户看到"自带背景无法显示"）。
                // body inline style 里的 url() 也要走 url 替换（与 processCss 同样规则），
                // 否则 asset 协议拿不到文件。
                setLoadingMessage("渲染…");
                chapterFullHtml = new Array(spine.length);
                // 转义 HTML 属性值用，避免 class/style 里的特殊字符破坏 outerHTML 注入
                const escapeAttr = (s: string) =>
                    s
                        .replace(/&/g, "&amp;")
                        .replace(/"/g, "&quot;")
                        .replace(/</g, "&lt;")
                        .replace(/>/g, "&gt;");
                // 从 body inline style 中只提取 "background-image 相关" 属性（image / repeat /
                // position / size / attachment / origin / clip）。
                //
                // 关键：不再提取 background-color 与 plain `background:` 的颜色部分。
                // 旧实现把 `background: white` 也搬到 chapterBgStyle，再被 .rd-chapter-bg 全屏
                // div 渲染成 inset:0 的白色块 —— 直接遮住 .rd-app 的主题色（用户看到的就是
                // "主题没了"、"全屏背景白成一片"）。新实现只让 EPUB 自带的背景图通过全屏层
                // 显示，bg-color 由阅读器主题接管。shorthand `background: white url(x.png)` 也
                // 只取 url() 部分。
                const extractBgStyle = (styleStr: string): string => {
                    if (!styleStr) return "";
                    const decls = styleStr
                        .split(";")
                        .map((s) => s.trim())
                        .filter(Boolean);
                    const out: string[] = [];
                    const IMAGE_LONGHAND =
                        /^background-(image|repeat|position|size|attachment|origin|clip)\s*:/i;
                    for (const d of decls) {
                        if (IMAGE_LONGHAND.test(d)) {
                            out.push(d);
                            continue;
                        }
                        if (/^background\s*:/i.test(d)) {
                            // 简写：只挑 url() 与 size/repeat/position 关键词，忽略颜色词
                            const val = d.slice(d.indexOf(":") + 1);
                            const urlM = val.match(/url\([^)]*\)/i);
                            if (urlM) out.push(`background-image: ${urlM[0]}`);
                            const repeatM = val.match(
                                /\b(no-repeat|repeat-x|repeat-y|repeat|space|round)\b/i,
                            );
                            if (repeatM)
                                out.push(`background-repeat: ${repeatM[1]}`);
                            const sizeM = val.match(
                                /\/\s*(cover|contain|auto|\d+(?:\.\d+)?(?:px|em|rem|%|vh|vw)(?:\s+\d+(?:\.\d+)?(?:px|em|rem|%|vh|vw))?)/i,
                            );
                            if (sizeM) out.push(`background-size: ${sizeM[1]}`);
                        }
                    }
                    return out.join("; ");
                };
                // 重置 chapterBgStyle，与 chapterFullHtml 对齐
                chapterBgStyle = new Array(spine.length).fill("");
                for (let i = 0; i < spine.length; i++) {
                    const doc = parsedDocs[i];
                    if (!doc) {
                        chapterFullHtml[i] =
                            `<section class="rd-chapter" data-idx="${i}"><p style="color:#b66;">[章节加载失败：${spine[i].path}]</p></section>`;
                        continue;
                    }
                    const head = doc.head;
                    const headStyles = head
                        ? Array.from(head.querySelectorAll("style"))
                              .map((s) => s.outerHTML)
                              .join("")
                        : "";
                    const body = doc.body || doc.querySelector("body");
                    if (body instanceof HTMLElement)
                        markOpeningHeaderImage(body);
                    const html = body
                        ? body.innerHTML
                        : doc.documentElement.innerHTML;
                    // 把 body 的 class / style / id 转移到 .epub-body div
                    const chapterPath = spine[i].path;
                    const bodyClass = body?.getAttribute("class") || "";
                    let bodyStyle = body?.getAttribute("style") || "";
                    if (body && bodyStyle) {
                        // Ensure the CSS declaration block is live before serializing it.
                        // XML-parsed inline styles otherwise survive as inert attributes,
                        // so styles such as display:none and background URLs are lost.
                        body.style.cssText = bodyStyle;
                        bodyStyle = body.style.cssText;
                    }
                    if (bodyStyle) {
                        bodyStyle = bodyStyle.replace(
                            URL_RE,
                            (match, _q, url) => {
                                const trimmed = url.trim();
                                if (
                                    !trimmed ||
                                    trimmed.startsWith("data:") ||
                                    /^https?:/i.test(trimmed) ||
                                    trimmed.startsWith("#")
                                ) {
                                    return match;
                                }
                                const abs = resolveRelative(
                                    chapterPath,
                                    trimmed,
                                );
                                return `url("${toAssetUrl(abs)}")`;
                            },
                        );
                    }
                    const bodyId = body?.getAttribute("id") || "";
                    // EPUB body 自带的 background-* 属性单独提取出来，存到 chapterBgStyle，
                    // 后面用全屏背景层 .rd-chapter-bg 渲染（覆盖整个 viewport，不只列宽）。
                    // 不再把 background 写到 section.rd-chapter 的 style 上 —— 否则 column-
                    // fragmented 会把背景在每列重复绘制，与全屏层冲突，且无法贴窗口边。
                    const sectionClass = bodyClass
                        ? `rd-chapter ${bodyClass}`
                        : "rd-chapter";
                    const sectionBgStyle = extractBgStyle(bodyStyle);
                    const epubBodyClass = bodyClass
                        ? `epub-body ${bodyClass}`
                        : "epub-body";
                    // .epub-body 也清掉 background-* 部分（避免在白色卡片背后重复绘制）。
                    // 其他属性保留（color、margin、padding 等）。
                    const bodyStyleNoBg = bodyStyle;
                    const styleAttr = bodyStyleNoBg
                        ? ` style="${escapeAttr(bodyStyleNoBg)}"`
                        : "";
                    const idAttr = bodyId ? ` id="${escapeAttr(bodyId)}"` : "";
                    // 把 body 的 background 样式存到 chapterBgStyle，供全屏背景层使用：
                    // 当前章节的背景图会被复制到 .rd-chapter-bg 全屏 div，覆盖整个 viewport，
                    // 而不只是列宽内的 section.rd-chapter 区域。
                    chapterBgStyle[i] = sectionBgStyle;
                    chapterFullHtml[i] =
                        `<section class="${escapeAttr(sectionClass)}" data-idx="${i}">${headStyles}<div class="${escapeAttr(epubBodyClass)}"${idAttr}${styleAttr}>${html}</div></section>`;
                }

                // 解析产物存进缓存，下次切回这个标签直接命中
                writeReaderCache(bookKey, {
                    spine,
                    toc,
                    title: bookTitle,
                    author: bookAuthor,
                    chapterFullHtml,
                    chapterBgStyle,
                });
            }

            // 12) 进度恢复（兼容旧格式：纯 currentSpread 数字 / 新格式：{ch,sp}）
            progressKey = readerProgressKey(bookSourcePath || epubPath);
            let initialChIdx = 0;
            let initialSpInCh = 0;
            let initialScrollOffset = 0;
            try {
                const raw = localStorage.getItem(progressKey);
                if (raw) {
                    if (raw.trim().startsWith("{")) {
                        const parsed = JSON.parse(raw);
                        if (typeof parsed.ch === "number") {
                            initialChIdx = Math.max(
                                0,
                                Math.min(spine.length - 1, parsed.ch),
                            );
                        }
                        if (typeof parsed.sp === "number" && parsed.sp >= 0) {
                            initialSpInCh = parsed.sp;
                        }
                        if (
                            parsed.mode === "scroll" &&
                            typeof parsed.scrollOffset === "number"
                        ) {
                            initialScrollOffset = Math.max(
                                0,
                                parsed.scrollOffset,
                            );
                        }
                    }
                    // 旧格式（纯 currentSpread 数字）由于无法精确还原章节，进新版后从 0 开始
                }
            } catch {}

            // 13) 第一阶段：以 initialChIdx 为中心 ±RENDER_RADIUS 章用完整 HTML，
            //     其他章节用极轻占位（仅一行文字 + 章节标题）。
            //     占位让浏览器首次列布局只在 ~2N+1 章 + 几百行占位中完成 ——
            //     从"几分钟"压缩到"几百毫秒"。
            renderedSet = new Set<number>();
            function chapterPlaceholder(i: number): string {
                const title = (
                    spine[i].title ||
                    t("第 {index} 节", { index: i + 1 })
                ).replace(
                    /[<>&]/g,
                    (c) => ({ "<": "&lt;", ">": "&gt;", "&": "&amp;" })[c]!,
                );
                return `<section class="rd-chapter rd-pending" data-idx="${i}"><div class="rd-pending-title">${title}</div></section>`;
            }
            const initialParts: string[] = new Array(spine.length);
            for (let i = 0; i < spine.length; i++) {
                if (Math.abs(i - initialChIdx) <= RENDER_RADIUS) {
                    initialParts[i] = chapterFullHtml[i];
                    renderedSet.add(i);
                } else {
                    initialParts[i] = chapterPlaceholder(i);
                }
            }
            combinedHtml = initialParts.join("\n");

            loading = false;
            await tick();
            // @html creates a fresh HTML subtree; XML inline declarations must be
            // rebound after that insertion, not only in the parsed XML document.
            if (pageEl) normalizeInlineStyles(pageEl);
            if (pageEl) adoptChapterStyles(pageEl);
            requestAnimationFrame(() => {
                applyBasicLayout();
                // 同步 measure 列布局，让 chapterStartCols 立刻可用
                recomputeLayout(true);
                // 初始 hydrate 时已经渲染的章节也要从 computed style 提取背景图
                // （否则 EPUB CSS 文件里的 body.full { background:url() } 这种规则
                // 设的全屏背景在首屏不显示，要等用户翻页 hydrate 别的章节时才被
                // extractChapterBgFromCss 触发更新）。
                for (const i of renderedSet) {
                    const sec = pageEl?.querySelector<HTMLElement>(
                        `section.rd-chapter[data-idx="${i}"]`,
                    );
                    if (sec) extractChapterBgFromCss(i, sec);
                }
                restoreInitialPosition();
                // 启动后台增量 hydrate（idle 时段执行，不抢占用户阅读）
                scheduleHydrate();
            });

            function restoreInitialPosition() {
                // 锚定到 initialChIdx 的 initialSpInCh 处
                const meta = (pageEl as any).__rd;
                if (pageMode === "scroll") {
                    const sec = pageEl?.querySelector<HTMLElement>(
                        `section.rd-chapter[data-idx="${initialChIdx}"]`,
                    );
                    if (sec && viewportEl) {
                        const sectionTop =
                            sec.getBoundingClientRect().top -
                            viewportEl.getBoundingClientRect().top +
                            viewportEl.scrollTop;
                        viewportEl.scrollTop = Math.max(
                            0,
                            sectionTop + initialScrollOffset,
                        );
                        syncCurrentSpreadForScroll({
                            ch: initialChIdx,
                            offset: initialScrollOffset,
                        });
                    }
                    return;
                }
                if (meta && chapterStartCols.length > initialChIdx) {
                    const startCol = chapterStartCols[initialChIdx] || 0;
                    const targetCol = startCol + initialSpInCh * meta.numCols;
                    currentSpread = Math.floor(targetCol / meta.numCols);
                    if (currentSpread >= totalSpreads)
                        currentSpread = Math.max(0, totalSpreads - 1);
                }
                applyTransform();
            }
        } catch (e: any) {
            console.error(e);
            // 手机形态才剥掉「Error: 」前缀（和 toastError 同一口径）；
            // 桌面阅读器的报错文案保持原样，不做本轮之外的改动。
            errorMsg = isPhoneLayout()
                ? cleanErrorMessage(String(e))
                : String(e);
            loading = false;
        }
    }

    // 同步计算给定 spread 对应的章节索引（不依赖 svelte derived，避免在
    // 同一函数内连续修改 currentSpread 后立刻读 derived 拿到旧值的问题）。
    function chapterIdxForSpread(spread: number): number {
        if (chapterStartCols.length < 2) return 0;
        const meta = (pageEl as any).__rd;
        const numCols = meta?.numCols ?? 1;
        const leftCol = spread * numCols;
        let lo = 0,
            hi = chapterStartCols.length - 2;
        while (lo < hi) {
            const mid = (lo + hi + 1) >> 1;
            if (chapterStartCols[mid] <= leftCol) lo = mid;
            else hi = mid - 1;
        }
        return lo;
    }

    function getScrollReadingPosition(): { ch: number; offset: number } {
        const sections =
            pageEl?.querySelectorAll<HTMLElement>("section.rd-chapter") || [];
        if (sections.length === 0) {
            return {
                ch: Math.max(0, Math.min(spine.length - 1, scrollChapterIdx)),
                offset: scrollChapterOffset,
            };
        }
        const scrollTop = viewportEl?.scrollTop ?? 0;
        let ch = 0;
        let found = false;
        for (let i = sections.length - 1; i >= 0; i--) {
            // A section's own offsetTop includes the page's top padding, while
            // scrollTop is measured in the viewport's scroll coordinate space.
            // Compare viewport-relative positions so the top margin cannot make
            // every later chapter look invisible.
            const sectionTop =
                sections[i].getBoundingClientRect().top -
                viewportTop() +
                scrollTop;
            if (sectionTop <= scrollTop + 1 && sections[i].offsetHeight > 0) {
                ch = i;
                found = true;
                break;
            }
        }
        if (!found) {
            // Zero-height placeholders can share offsetTop with the next real
            // chapter. Prefer the chapter that actually occupies the viewport.
            for (let i = 0; i < sections.length; i++) {
                const rect = sections[i].getBoundingClientRect();
                if (rect.height > 0 && rect.bottom > viewportTop() + 1) {
                    ch = i;
                    break;
                }
            }
        }
        const sectionTop =
            sections[ch].getBoundingClientRect().top -
            viewportTop() +
            scrollTop;
        return {
            ch,
            offset: Math.max(0, scrollTop - sectionTop),
        };
    }

    function viewportTop(): number {
        return viewportEl?.getBoundingClientRect().top ?? 0;
    }

    // 滚动模式的 currentSpread 是“虚拟页号”。所有依赖章节标题 / 进度 /
    // 书签的派生状态都从它读取，因此必须随真实 scrollTop 同步，不能只在
    // 模式切换时更新一次。
    function syncCurrentSpreadForScroll(position = getScrollReadingPosition()) {
        scrollChapterIdx = position.ch;
        scrollChapterOffset = position.offset;
        // 滚动模式的原生滚动位置不叫 scrollLeft；因此“是否在章首”和目录高亮
        // 必须以真实像素偏移为准，不能沿用分页模式的 virtual spread 列号。
        if (pageMode === "scroll") return currentSpread;
        if (chapterStartCols.length < 2) return currentSpread;
        const meta = (pageEl as any).__rd;
        const pageH = Math.max(1, meta?.pageH || viewportEl?.clientHeight || 1);
        const start = chapterStartCols[position.ch] ?? 0;
        const next = chapterStartCols[position.ch + 1] ?? totalSpreads;
        const pagesIntoChapter = Math.floor(position.offset / pageH);
        const maxPageInChapter = Math.max(0, next - start - 1);
        const spread = Math.max(
            0,
            Math.min(
                Math.max(0, totalSpreads - 1),
                start + Math.min(pagesIntoChapter, maxPageInChapter),
            ),
        );
        if (currentSpread !== spread) currentSpread = spread;
        return spread;
    }

    function scrollToChapterOffset(ch: number, offset: number) {
        const section = pageEl?.querySelector<HTMLElement>(
            `section.rd-chapter[data-idx="${ch}"]`,
        );
        if (!section || !viewportEl) return;
        // In scroll mode the page is laid out as a normal block, so offsetTop
        // is relative to the padded page, not to the scroll container. Convert
        // through viewport coordinates before assigning scrollTop.
        const sectionTop =
            section.getBoundingClientRect().top -
            viewportEl.getBoundingClientRect().top +
            viewportEl.scrollTop;
        const target = Math.max(0, sectionTop + offset);
        viewportEl.scrollTop = target;
        scrollChapterIdx = ch;
        scrollChapterOffset = Math.max(0, viewportEl.scrollTop - sectionTop);
        syncCurrentSpreadForScroll({
            ch: scrollChapterIdx,
            offset: scrollChapterOffset,
        });
    }

    type ReadingAnchor = {
        mode: PageMode;
        ch: number;
        colInCh: number;
        scrollOffset: number;
    };

    function captureReadingAnchor(): ReadingAnchor {
        const meta = (pageEl as any).__rd;
        if (meta?.mode === "scroll") {
            const position = getScrollReadingPosition();
            return {
                mode: "scroll",
                ch: position.ch,
                colInCh: 0,
                scrollOffset: position.offset,
            };
        }
        const numCols = meta?.numCols ?? 1;
        const curLeftCol = currentSpread * numCols;
        const ch = chapterIdxForSpread(currentSpread);
        return {
            mode: "paginated",
            ch,
            colInCh: Math.max(0, curLeftCol - (chapterStartCols[ch] || 0)),
            scrollOffset: 0,
        };
    }

    function keepBookEndAfterReflow() {
        if (!keepAtBookEnd) return false;
        if (pageMode === "scroll") {
            snapScrollToBookEnd();
            return true;
        }
        currentSpread = Math.max(0, totalSpreads - 1);
        applyTransform();
        return true;
    }

    function snapScrollToBookEnd() {
        if (!viewportEl) return;
        const lastIdx = spine.length - 1;
        const lastSection = pageEl?.querySelector<HTMLElement>(
            `section.rd-chapter[data-idx="${lastIdx}"]`,
        );
        let sectionBottom = viewportEl.scrollHeight;
        if (lastSection) {
            const range = document.createRange();
            range.selectNodeContents(lastSection);
            sectionBottom =
                range.getBoundingClientRect().bottom -
                viewportEl.getBoundingClientRect().top +
                viewportEl.scrollTop;
        }
        pinningBookEnd = true;
        viewportEl.scrollTop = Math.max(
            0,
            Math.min(
                sectionBottom - viewportEl.clientHeight,
                viewportEl.scrollHeight - viewportEl.clientHeight,
            ),
        );
        syncCurrentSpreadForScroll();
        saveProgress();
        requestAnimationFrame(() => {
            pinningBookEnd = false;
        });
    }

    function restoreReadingAnchor(anchor: ReadingAnchor) {
        if (keepBookEndAfterReflow()) return;
        if (anchor.mode === "scroll" || pageMode === "scroll") {
            scrollToChapterOffset(anchor.ch, anchor.scrollOffset);
            return;
        }
        const meta = (pageEl as any).__rd;
        if (!meta) return;
        const newCol = (chapterStartCols[anchor.ch] || 0) + anchor.colInCh;
        currentSpread = Math.max(
            0,
            Math.min(totalSpreads - 1, Math.floor(newCol / meta.numCols)),
        );
        applyTransform();
    }

    function navigateEpubLink(rawHref: string) {
        const href = rawHref.trim();
        if (!href || /^javascript:/i.test(href)) {
            return;
        }
        if (/^(?:https?:|mailto:|tel:)/i.test(href)) {
            void platform.openExternal(href);
            return;
        }

        const hashAt = href.indexOf("#");
        const pathPart = hashAt >= 0 ? href.slice(0, hashAt) : href;
        const fragment = hashAt >= 0 ? href.slice(hashAt + 1) : "";
        const currentIdx =
            pageMode === "scroll"
                ? getScrollReadingPosition().ch
                : chapterIdxForSpread(currentSpread);
        const targetPath =
            pathPart && pathPart !== "#"
                ? resolveRelative(spine[currentIdx]?.path || "", pathPart)
                : spine[currentIdx]?.path || "";
        const targetIdx = spine.findIndex((entry) => entry.path === targetPath);
        if (targetIdx < 0) return;

        if (!renderedSet.has(targetIdx)) {
            const anchor = captureReadingAnchor();
            if (hydrateChapterDom(targetIdx)) {
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }
        }
        const navigateWhenReady = (attempt = 0) => {
            if (!renderedSet.has(targetIdx) && attempt < 60) {
                window.setTimeout(() => navigateWhenReady(attempt + 1), 50);
                return;
            }
            const section = pageEl?.querySelector<HTMLElement>(
                `section.rd-chapter[data-idx="${targetIdx}"]`,
            );
            if (!section) return;

            let target: HTMLElement | null = null;
            if (fragment) {
                const decoded = safeDecode(fragment);
                target =
                    Array.from(
                        section.querySelectorAll<HTMLElement>("[id]"),
                    ).find((el) => el.id === decoded) || null;
            }

            if (pageMode === "scroll") {
                const offset =
                    target && target !== section
                        ? target.getBoundingClientRect().top -
                          section.getBoundingClientRect().top
                        : 0;
                scrollToChapterOffset(targetIdx, Math.max(0, offset));
                return;
            }

            const meta = (pageEl as any).__rd;
            if (!meta) return;
            const sectionLeft = section.getBoundingClientRect().left;
            const pageLeft = pageEl.getBoundingClientRect().left;
            const sectionCol = Math.max(
                0,
                Math.round((sectionLeft - pageLeft) / meta.stride),
            );
            const targetX = target
                ? target.getBoundingClientRect().left - pageLeft
                : 0;
            const targetCol = target
                ? Math.max(0, Math.floor(targetX / meta.stride))
                : sectionCol;
            currentSpread = Math.floor(
                Math.max(sectionCol, targetCol) / meta.numCols,
            );
            if (currentSpread >= totalSpreads)
                currentSpread = Math.max(0, totalSpreads - 1);
            applyTransform();
        };
        navigateWhenReady();
    }

    // ===== 增量 hydrate =====
    // 把一个占位章节同步替换为完整 HTML，重新 measure 列布局，并把 currentSpread
    // 锚定回用户原本所在章节的相对位置（这样列分布变化不会让画面跳到错误的页）。

    /**
     * 从已 hydrate 的章节中提取实际 background-image（包含 EPUB CSS 文件中
     * `body.full { background:url() }` 这种规则的效果）。
     *
     * 实现思路：BODY_REPLACE 把 EPUB body 选择器改写成 `.epub-body, .rd-chapter`。
     * .rd-chapter 上我们用 !important 强制 background-image: none（避免列内重复绘制），
     * 但 .epub-body 仍然保留原始 background-image。所以这里从 .epub-body 上读
     * computed background-image / size / position / repeat，组装成 inline style，
     * 写到 chapterBgStyle[idx]，用于全屏背景层 .rd-chapter-bg。
     *
     * 关键：不读 background-color。EPUB 几乎一律 `body { background: white }`，
     * 把 white 提取到全屏背景层会把整窗口刷成白色，遮住阅读器主题（用户反馈
     * 的"全屏背景没了 / 主题没了"就是这个原因）。颜色由 .rd-app 的主题色负责。
     *
     * 第二步：提取完成后，用 inline !important 把 .epub-body 的 background 强制
     * 透明 —— EPUB CSS body 选择器经 BODY_REPLACE 落在 .epub-body 上的 bg-color
     * 也借此让位给主题色。inline style 在 EPUB style 之上，能稳赢 cascade。
     */
    function extractChapterBgFromCss(idx: number, sectionEl: HTMLElement) {
        // Keep EPUB body backgrounds in the document flow, like the editor preview.
        // The old reader copied them to a fullscreen fixed layer and then cleared
        // .epub-body, which made decorative chapter art bleed over later pages.
        void idx;
        void sectionEl;
    }

    function hydrateChapter(idx: number): boolean {
        if (idx < 0 || idx >= chapterFullHtml.length) return false;
        if (renderedSet.has(idx)) return false;

        const anchor = captureReadingAnchor();

        if (!hydrateChapterDom(idx)) return false;

        // 重 measure
        recomputeLayout(true);
        // 锚回当前位置：必须始终调用 applyTransform 恢复 scrollLeft。
        //
        // 关键：recomputeLayout → applyBasicLayout 内部有 `viewportEl.scrollLeft = 0`，
        // 每次 hydrate 都会把视口滚回起点。如果不调 applyTransform，画面就会
        // 闪回第一页（用户翻页后下一帧 hydrate 触发，立刻被拽回开头）。
        // 之前为了避免"抖动"加的 if (nextSpread !== currentSpread) 优化在这里反噬，
        // 必须去掉 —— 同帧内多次 scrollLeft 写入浏览器只 paint 最后一次，无抖动。
        restoreReadingAnchor(anchor);
        return true;
    }

    // 仅替换 DOM、不做 measure/anchor。给后台 batch hydrate 用 —— 每批结束后
    // 再统一 recomputeLayout，省掉每章一次 100ms+ 的 sync 列布局测量。
    function hydrateChapterDom(idx: number): boolean {
        if (idx < 0 || idx >= chapterFullHtml.length) return false;
        if (renderedSet.has(idx)) return false;
        const placeholder = pageEl?.querySelector(
            `section.rd-pending[data-idx="${idx}"]`,
        );
        if (!placeholder) return false;
        const tmp = document.createElement("div");
        tmp.innerHTML = chapterFullHtml[idx];
        const newSection = tmp.firstElementChild;
        if (!newSection) return false;
        placeholder.replaceWith(newSection);
        normalizeInlineStyles(newSection);
        adoptChapterStyles(newSection);
        renderedSet.add(idx);
        extractChapterBgFromCss(idx, newSection as HTMLElement);
        return true;
    }

    // 后台 idle 队列：每个 idle slice 内尽可能多地把"距离当前章节最近的"未 hydrate
    // 章节替换为完整 DOM，slice 末尾才做一次 recomputeLayout —— 不再每章一次。
    //
    // 旧实现每章一次 sync recomputeLayout（100~300 ms / 次，整本 100 章累计 10s+
    // 的主线程占用），是用户感受到"翻页还是有点卡顿"的真正源头：哪怕翻页本身用
    // scrollLeft 已经很轻，后台 hydrate 总在抢主线程。
    // 新实现：先纯 DOM 替换（每章 ~1ms），一个 idle slice 凑够 BATCH_SIZE 章或
    // deadline 用完再 measure，开销摊薄数十倍。
    const HYDRATE_BATCH_SIZE = 3;
    const USER_NAV_HYDRATE_PAUSE_MS = 260;
    let lastPageTurnAt = 0;
    function markPageTurn() {
        lastPageTurnAt = Date.now();
    }
    function scheduleHydrate() {
        if (hydrateScheduled) return;
        hydrateScheduled = true;
        const ric =
            typeof requestIdleCallback === "function"
                ? requestIdleCallback
                : (cb: any) =>
                      setTimeout(
                          () =>
                              cb({
                                  timeRemaining: () => 50,
                                  didTimeout: false,
                              }),
                          50,
                      );

        function tick(deadline: any) {
            hydrateId = null;
            if (destroyed) {
                hydrateScheduled = false;
                return;
            }
            if (renderedSet.size >= chapterFullHtml.length) {
                hydrateScheduled = false;
                return;
            }
            // 保活：切走的标签不再在后台抢主线程 —— 隐藏页面照样会执行
            // idle 回调，继续 hydrate 就是在和用户当前看的页面抢帧。
            // 切回来时 onActivationChange 会重新调度。
            if (keepAliveView && !keepAliveView.isActive()) {
                hydrateScheduled = false;
                return;
            }
            if (Date.now() - lastPageTurnAt < USER_NAV_HYDRATE_PAUSE_MS) {
                hydrateId = ric(tick);
                return;
            }

            // 锚定信息：在 DOM 改动前记录 user 当前所在章节内列偏移；批量 hydrate 完
            // 再用同一锚点把 currentSpread 拉回（章节膨胀会让其他章节列号整体后移，
            // 用户原本在的"章节内列"必须保持，否则画面会跳）。
            const anchor = captureReadingAnchor();

            let hydrated = 0;
            // 每批至多 BATCH_SIZE 章，且尊重 idle deadline：留 5ms 余量做 measure。
            while (
                hydrated < HYDRATE_BATCH_SIZE &&
                deadline.timeRemaining() > 5
            ) {
                const cur =
                    pageMode === "scroll"
                        ? currentChapterIdx
                        : chapterIdxForSpread(currentSpread);
                let nextIdx = -1,
                    bestDist = Infinity;
                for (let i = 0; i < chapterFullHtml.length; i++) {
                    if (renderedSet.has(i)) continue;
                    const dist = Math.abs(i - cur);
                    if (dist < bestDist) {
                        bestDist = dist;
                        nextIdx = i;
                    }
                }
                if (nextIdx < 0) break;
                if (!hydrateChapterDom(nextIdx)) break;
                hydrated++;
            }

            if (hydrated > 0) {
                // 整批一次 recompute + anchor
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }

            if (renderedSet.size < chapterFullHtml.length) {
                hydrateId = ric(tick);
            } else {
                hydrateScheduled = false;
            }
        }
        hydrateId = ric(tick);
    }

    // 翻页前确保即将进入的章节已 hydrate，避免用户翻到占位。
    //
    // 性能：hydrate 内部要做同步 recomputeLayout（measure sentinel + 测算
    // 章节起点）—— pageEl 总宽几十万像素时这一步耗时可达上百 ms，发生在
    // 用户翻页帧里就会感受到"翻一下要等一下"。
    // 优化：debounce —— 连续快翻时不每帧都 hydrate；停 80ms 后再做。
    // 另外用 setTimeout 保证 hydrate 始终在浏览器把 scrollLeft 那一帧 paint
    // 完之后才执行，翻页动效不会被 hydrate 拖住。
    //
    // v3 改动：把 pendingChapter 单值替换为 Set —— 用户连点 next 时 ensureChapterReady
    // 会被分别请求 ch+1, ch+2, ch+3...；旧实现只保最后一个（ch+3），其他章节
    // 留在占位上，用户翻回时还要再等 hydrate。新实现 80ms 内累积所有请求，
    // 一次 hydrateChapterDom 全部 + 一次 recomputeLayout 锚定 —— 既不浪费
    // 测量，又把所有用户走过的章节都喂上去。
    let ensureTimer: any = null;
    let pendingChapters = new Set<number>();
    function ensureChapterReady(chIdx: number) {
        if (chIdx < 0 || chIdx >= chapterFullHtml.length) return;
        if (renderedSet.has(chIdx)) return;
        pendingChapters.add(chIdx);
        if (ensureTimer) clearTimeout(ensureTimer);
        ensureTimer = setTimeout(() => {
            ensureTimer = null;
            const ids = [...pendingChapters];
            pendingChapters.clear();
            // 锚定信息：批量 hydrate 会让 chapterStartCols 整体后移，要按"原章节内
            // 列偏移"恢复 currentSpread（与 hydrateChapter / scheduleHydrate 同思路）。
            const anchor = captureReadingAnchor();
            let any = false;
            for (const idx of ids) {
                if (!renderedSet.has(idx) && hydrateChapterDom(idx)) any = true;
            }
            if (any) {
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }
        }, 140);
    }

    // ===== 分页计算 =====
    // 多列布局必须给容器一个大于实际所需的可用内联宽度（让内容向右流出新列）。
    // 改用 viewport.scrollLeft 翻页后，pageEl 自身不再被提升为 GPU layer，
    // 长宽度不会再触发软件渲染回退；因此可以一次到位 reflow，不用迭代探测。
    // lastTotalCols 命中时直接用缓存值 + 缓冲，避免不必要的大宽度 reflow。
    const COLS_MAX = 2000; // 单次 reflow 上限，覆盖几乎所有 EPUB
    const COLS_BUFFER = 8; // 缓存命中时多留 8 列，吸收字号/边距小幅变化

    let lastTotalCols = 0; // 上次成功测得的总列数

    function setCanvasWidth(
        cols: number,
        layout?: {
            numCols: number;
            pageInnerW: number;
            columnGap: number;
        },
    ) {
        if (!canvasEl || !pageEl) return;
        const meta = (pageEl as any).__rd;
        const numCols = layout?.numCols ?? meta?.numCols ?? 1;
        const pageInnerW =
            layout?.pageInnerW ?? meta?.pageInnerW ?? canvasEl.clientWidth;
        const columnGap = layout?.columnGap ?? meta?.columnGap ?? 0;
        const safeCols = Math.max(
            cols,
            numCols,
            lastTotalCols + COLS_BUFFER,
            24,
        );
        canvasEl.style.width =
            safeCols * pageInnerW +
            Math.max(0, safeCols - 1) * columnGap +
            "px";
    }

    // 拆分为两步：applyBasicLayout（同步、毫秒级）让用户首屏立即可见，
    // measurePageMetrics（异步、可慢）后台测总列数 / 章节起点。
    // recomputeLayout(measureNow=true) 同步走完整链路（用于交互时 setMode/changeFont）；
    // recomputeLayout(false) 让首屏先出来，下一帧再测页码。

    // 顶部状态栏（章节名 / 本章剩余页数）独立占用的高度。
    // viewport 上下都至少留出这段空间，避免状态栏与正文重叠（之前两者都是
    // absolute，叠层时正文从上方"穿透"显示）。这个值同时是默认窗口高度
    // 比 8:5 严格高出的那部分（窗高 = 宽 * 8/5 + STATUS_BAR_H * 2）。
    const STATUS_BAR_H = 32;

    function applyBasicLayout(): {
        tryCols: number;
        pageInnerW: number;
        columnGap: number;
        stride: number;
        numCols: number;
        pageH: number;
    } | null {
        if (!frameEl || !pageEl || !viewportEl) return null;
        const numCols = readMode === "landscape" ? 2 : 1;
        const fullW = frameEl.clientWidth;
        const fullH = frameEl.clientHeight;

        // 单页 / 每页 实际比例 = 窗口可用区域比例（去除两侧 minSideMargin）
        const minSideMargin = Math.max(16, Math.round(fullW * pageMarginH));
        // 最小垂直 margin 至少要 STATUS_BAR_H，确保 viewport 顶部留出状态栏空间，
        // 状态栏 absolute 在 frame top: 8px 处，落在 viewport 顶部上方的留白区。
        const minVerticalMargin = Math.max(
            STATUS_BAR_H,
            Math.round(fullH * Math.max(0.02, pageMarginV)),
        );
        const visibleColumnGap = numCols === 2 ? minSideMargin : 0;
        const columnGap = numCols === 2 ? minSideMargin : minSideMargin * 4;

        const usableW = Math.max(1, fullW - 2 * minSideMargin);
        const pageInnerW =
            numCols === 2
                ? Math.max(1, Math.floor((usableW - visibleColumnGap) / 2))
                : usableW;
        // 单列模式下，文本可用区域保持 5:8（宽:高）= 8:5（高:宽）的比例，
        // 与默认窗口比例一致；如果 fullH 不够，则受窗口高度约束（取较小值）。
        const availableH = Math.max(1, fullH - 2 * minVerticalMargin);
        const targetPageH =
            numCols === 1 ? Math.round((pageInnerW * 8) / 5) : availableH;
        const pageH = Math.max(1, Math.min(availableH, targetPageH));
        const totalW = numCols * pageInnerW + visibleColumnGap;

        const stride = pageInnerW + columnGap;

        // 列内容在 frame 中的水平居中量：列总宽通常 < frame 宽，
        // 居中后两侧露出的空隙等于 (fullW - totalW) / 2，
        // 这部分用 viewport 的左右 padding 实现 —— viewport 仍然全屏覆盖，
        // 只是 padding 区不渲染列内容（列从 padding 内侧开始流）。
        // 这样 viewport 自己（包括它的背景层）能填满整窗，
        // 头图通过更大负 margin 也能扩到 frame 边。
        const horizPad = Math.max(0, Math.round((fullW - totalW) / 2));
        const vertPad = minVerticalMargin;

        // === 公共样式变量 ===
        pageEl.style.setProperty("--page-w", pageInnerW + "px");
        pageEl.style.setProperty("--page-h", pageH + "px");
        // 把当前页面的左右 / 上下边距（像素）暴露给 CSS，供头图、全屏背景
        // 等"突破列宽限制贴窗口边"的样式使用。
        // 单列时 horizPad = minSideMargin；双列时 horizPad 较小（gap 占了一部分）。
        // 头图想贴到整个窗口边时用 horizPad（实际 viewport 内边距），不用 minSideMargin。
        pageEl.style.setProperty("--page-margin-h", horizPad + "px");
        pageEl.style.setProperty("--page-margin-v", vertPad + "px");
        pageEl.style.setProperty("--col-gap", columnGap + "px");
        pageEl.style.setProperty("--font-size", fontSize + "px");
        pageEl.style.setProperty("--line-height", String(lineHeight));
        pageEl.style.setProperty(
            "--paragraph-spacing",
            paragraphSpacing + "em",
        );

        // === 滚动模式：viewport 占满 frame，pageEl 单列纵向流 ===
        if (pageMode === "scroll") {
            // viewport 占满 frame：左右用 padding 留出阅读边距，但背景层（viewport
            // 自己的 background / EPUB 章节的 body background）能填满整窗
            viewportEl.style.width = fullW + "px";
            viewportEl.style.height = fullH + "px";
            viewportEl.style.padding = `${vertPad}px ${horizPad}px`;
            viewportEl.style.boxSizing = "border-box";
            viewportEl.style.overflowY = "auto";
            viewportEl.style.overflowX = "hidden";
            // 100% resolves against viewport's content box (after padding and
            // the vertical scrollbar), so text is never clipped on the right.
            canvasEl.style.width = "100%";
            canvasEl.style.height = "auto";
            pageEl.style.width = "100%";
            pageEl.style.height = "auto";
            pageEl.style.transform = "";
            viewportEl.scrollLeft = 0;
            // 滚动模式必须回到普通块级流。这里不能写 auto：内联的
            // column-width:auto 会与页面样式里的 column-count:1 组合，WebView2
            // 仍可能按可用高度生成大量纵向分片，表现为滚动页高度数百万像素。
            // 清掉平移模式留下的内联列属性，让滚动模式 CSS 完整接管。
            pageEl.style.removeProperty("column-width");
            pageEl.style.removeProperty("column-count");
            pageEl.style.removeProperty("column-gap");
            pageEl.style.removeProperty("column-fill");
            pageEl.style.height = "auto";
            const scrollPageW = Math.max(
                1,
                viewportEl.clientWidth - horizPad * 2,
            );
            (pageEl as any).__rd = {
                pageInnerW: scrollPageW,
                columnGap: 0,
                stride: scrollPageW,
                numCols: 1,
                pageH,
                mode: "scroll",
            };
            // 滚动模式下 chapterStartCols / totalCols 不再以列定义；为兼容现有派生只放 1 项
            return {
                tryCols: 1,
                pageInnerW: scrollPageW,
                columnGap: 0,
                stride: scrollPageW,
                numCols: 1,
                pageH,
            };
        }

        // === 平移模式：viewport 全屏 + padding，列从 padding 内侧开始流 ===
        //
        // 之前 viewport.width = totalW，由 .rd-frame flex 居中 —— 这导致 viewport
        // 之外两侧露出 frame 的 --rd-bg 米色背景，而 EPUB 章节的 body 背景 /
        // 自定义背景图都到不了 viewport 之外，用户看到的就是"两侧米色条"。
        // 现在改成：viewport 占满整个 frame，左右用 padding = horizPad 留阅读边距。
        // viewport 的 scrollLeft 仍然能横向翻页，因为 pageEl 总宽 > viewport 内容区宽。
        viewportEl.style.width = fullW + "px";
        viewportEl.style.height = fullH + "px";
        viewportEl.style.padding = `${vertPad}px ${horizPad}px`;
        viewportEl.style.boxSizing = "border-box";
        viewportEl.style.overflowY = "hidden";
        viewportEl.style.overflowX = "hidden";

        pageEl.style.transform = "";
        viewportEl.scrollLeft = 0;
        // 滚动模式会清掉列属性；切回平移翻页时必须显式恢复，否则页面会按
        // 普通块级文档重排成超宽超长的一张画布，左右翻页与页码全部失效。
        pageEl.style.columnWidth = pageInnerW + "px";
        pageEl.style.columnCount = String(numCols);
        pageEl.style.columnGap = columnGap + "px";
        pageEl.style.columnFill = "auto";
        pageEl.style.height = pageH + "px";
        // Reader diagnostics may inject temporary style nodes. They must not
        // survive a mode switch and override the column layout.
        pageEl
            .querySelectorAll<HTMLStyleElement>("style[data-rd-probe]")
            .forEach((node) => node.remove());

        // Keep the multicol container at one viewport row width. Chromium lets
        // column fragments overflow to the right while the element itself stays
        // narrow. Giving this element a huge explicit width makes every section
        // take that full width and prevents column fragmentation.
        pageEl.style.width =
            numCols * pageInnerW + Math.max(0, numCols - 1) * columnGap + "px";
        // The page element must stay exactly one row wide for Chromium to
        // fragment columns correctly. The canvas carries the full scroll width.
        const estimatedCols = Math.max(
            1,
            Math.ceil(combinedHtml.length / Math.max(120, pageInnerW * 1.2)),
        );

        (pageEl as any).__rd = {
            pageInnerW,
            columnGap,
            stride,
            numCols,
            pageH,
            mode: "paginated",
            horizPad,
            vertPad,
            fullW,
            fullH,
            tryCols: estimatedCols,
        };
        // The canvas must use the newly computed layout. Reading __rd before the
        // assignment above kept the previous spread's dimensions and could clip
        // the final page (especially after changing column count or margins).
        setCanvasWidth(estimatedCols, {
            numCols,
            pageInnerW,
            columnGap,
        });

        return {
            tryCols: (pageEl as any).__rd.tryCols,
            pageInnerW,
            columnGap,
            stride,
            numCols,
            pageH,
        };
    }

    function normalizeOpeningHeaderImages() {
        if (!pageEl) return;
        pageEl
            .querySelectorAll<HTMLElement>(
                ".rd-header-only, .rd-after-header-only",
            )
            .forEach((el) => {
                el.classList.remove("rd-header-only", "rd-after-header-only");
            });
        pageEl
            .querySelectorAll<HTMLElement>(".rd-opening-header-spacer")
            .forEach((el) => el.remove());
    }

    function measurePageMetrics(initial: {
        tryCols: number;
        pageInnerW: number;
        columnGap: number;
        stride: number;
        numCols: number;
    }) {
        if (!pageEl) return;

        // 滚动模式：列布局不存在，按"每章节占视口高度的整数倍"估算章节起点（用 spread 近似）。
        // chapterStartCols 在滚动模式下用作"章节边界对应的虚拟列号"，仅用于派生 currentChapterIdx。
        if (pageMode === "scroll") {
            const meta = (pageEl as any).__rd;
            const ph = meta?.pageH ?? viewportEl.clientHeight;
            const sections =
                pageEl.querySelectorAll<HTMLElement>("section.rd-chapter");
            chapterStartCols = [];
            // 每个 section 的 offsetTop / pageH 作为虚拟列号
            sections.forEach((sec) => {
                const col = Math.max(
                    0,
                    Math.round(sec.offsetTop / Math.max(1, ph)),
                );
                chapterStartCols.push(col);
            });
            const total = Math.max(
                1,
                Math.ceil(
                    (canvasEl?.scrollHeight || pageEl.scrollHeight) /
                        Math.max(1, ph),
                ),
            );
            chapterStartCols.push(total);
            totalCols = total;
            totalSpreads = total;
            return;
        }

        const { pageInnerW, columnGap, stride, numCols } = initial;
        let tryCols = initial.tryCols;

        let sentinel = pageEl.querySelector<HTMLElement>(".rd-end-sentinel");
        if (!sentinel) {
            sentinel = document.createElement("span");
            sentinel.className = "rd-end-sentinel";
            sentinel.textContent = "​";
        }
        pageEl.appendChild(sentinel);
        void pageEl.offsetWidth;

        const pr = pageEl.getBoundingClientRect();
        const sentX = sentinel.getBoundingClientRect().left - pr.left;
        const sentCol = Math.max(0, Math.floor(sentX / stride));

        totalCols = Math.max(1, sentCol + 1);
        totalSpreads = Math.ceil(totalCols / numCols);
        setCanvasWidth(totalCols);

        chapterStartCols = [];
        const sections =
            pageEl.querySelectorAll<HTMLElement>("section.rd-chapter");
        sections.forEach((sec) => {
            const r = sec.getBoundingClientRect();
            const x = r.left - pr.left;
            const col = Math.max(0, Math.round(x / stride));
            chapterStartCols.push(col);
        });
        chapterStartCols.push(totalCols);

        lastTotalCols = totalCols;
        void tryCols;
    }

    function recomputeLayout(measureNow = true) {
        // 重新测量会短暂把 scrollLeft / scrollTop 归零；若这时正好触发滚动
        // 监听，会把旧位置或章节开头误写成阅读进度。
        const wasSuppressed = suppressProgressSave;
        suppressProgressSave = true;
        const initial = applyBasicLayout();
        if (!initial) {
            suppressProgressSave = wasSuppressed;
            flushPendingProgressSave();
            return;
        }
        normalizeOpeningHeaderImages();
        if (measureNow) {
            measurePageMetrics(initial);
            queueMicrotask(() => {
                suppressProgressSave = wasSuppressed;
                flushPendingProgressSave();
            });
        } else {
            // 让 paint 先把首屏画出来，下一帧再做昂贵的 sentinel 测量
            requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                    try {
                        measurePageMetrics(initial);
                        if (currentSpread >= totalSpreads)
                            currentSpread = Math.max(0, totalSpreads - 1);
                        applyTransform();
                    } finally {
                        suppressProgressSave = wasSuppressed;
                        flushPendingProgressSave();
                    }
                });
            });
        }
    }

    function flushPendingProgressSave() {
        if (suppressProgressSave || !progressSavePending) return;
        progressSavePending = false;
        saveProgress();
    }

    function applyTransform() {
        if (!pageEl || !viewportEl) return;
        const meta = (pageEl as any).__rd;
        if (!meta) return;
        if (meta.mode === "scroll") {
            // 滚动模式：不需要操纵 scrollLeft；保存当前 scrollTop 作进度
            if (!suppressProgressSave) saveProgress();
            return;
        }
        const { pageInnerW, columnGap, numCols } = meta;
        const stride = pageInnerW + columnGap;
        const offset = currentSpread * numCols * stride;
        // 改用 viewport.scrollLeft 替代 pageEl.transform：
        // transform 在长元素上会触发整层提升为 composited layer，超 GPU 纹理上限时
        // 退化为软件渲染 / 图层切片，翻页极卡。scrollLeft 走浏览器原生的瓦片
        // 渲染 + 视口剔除路径，长容器横向翻页能保持丝滑。
        const maxScrollLeft = Math.max(
            0,
            viewportEl.scrollWidth - viewportEl.clientWidth,
        );
        viewportEl.scrollLeft = Math.min(offset, maxScrollLeft);
        if (!suppressProgressSave) saveProgress();
    }

    function saveProgress() {
        if (!progressKey) return;
        if (suppressProgressSave) {
            progressSavePending = true;
            return;
        }
        try {
            const meta = (pageEl as any).__rd;
            if (meta?.mode === "scroll") {
                // 滚动模式：保存 scrollTop 对应的章节内偏移
                const { ch, offset } = getScrollReadingPosition();
                syncCurrentSpreadForScroll({ ch, offset });
                localStorage.setItem(
                    progressKey,
                    JSON.stringify({
                        ch,
                        sp: 0,
                        scrollOffset: offset,
                        mode: "scroll",
                    }),
                );
                return;
            }
            const numCols = meta?.numCols ?? 1;
            const curLeftCol = currentSpread * numCols;
            const ch = chapterIdxForSpread(currentSpread);
            const chStart = chapterStartCols[ch] || 0;
            const colInCh = Math.max(0, curLeftCol - chStart);
            const sp = Math.floor(colInCh / numCols);
            localStorage.setItem(progressKey, JSON.stringify({ ch, sp }));
        } catch {}
    }

    // ===== 导航 =====
    // 翻页要做到"零抖动"：先立刻 applyTransform 让画面到位，再下一帧做可能
    // 触发 recompute / 锚定调整的 hydrate。否则同一帧内若 hydrate 触发
    // recomputeLayout，currentSpread 会被锚回到不同的列号，applyTransform
    // 第二次又改 scrollLeft，浏览器在两个 scrollLeft 之间渲染一帧 = 抖动。
    function nextSpread() {
        keepAtBookEnd = false;
        if (currentSpread + 1 < totalSpreads) {
            markPageTurn();
            currentSpread += 1;
            applyTransform();
            requestAnimationFrame(() => {
                const ch = chapterIdxForSpread(currentSpread);
                ensureChapterReady(ch);
                ensureChapterReady(ch + 1);
            });
        }
    }
    function prevSpread() {
        keepAtBookEnd = false;
        if (currentSpread > 0) {
            markPageTurn();
            currentSpread -= 1;
            applyTransform();
            requestAnimationFrame(() => {
                const ch = chapterIdxForSpread(currentSpread);
                ensureChapterReady(ch);
                ensureChapterReady(ch - 1);
            });
        }
    }
    function goToSpread(idx: number) {
        keepAtBookEnd = false;
        markPageTurn();
        currentSpread = Math.max(0, Math.min(totalSpreads - 1, idx));
        ensureChapterReady(chapterIdxForSpread(currentSpread));
        applyTransform();
    }
    function goToChapter(idx: number) {
        if (idx < 0 || idx >= spine.length) return;
        keepAtBookEnd = false;
        // 占位章节只有标题行；直接按占位测量会落到错误列。先同步 hydrate
        // 并重排，再按最新的 chapterStartCols 定位。
        if (!renderedSet.has(idx)) {
            const anchor = captureReadingAnchor();
            if (hydrateChapterDom(idx)) {
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }
        }
        if (pageMode === "scroll") {
            scrollToChapterOffset(idx, 0);
            return;
        }
        const meta = (pageEl as any).__rd;
        if (!meta || chapterStartCols.length <= idx) {
            currentSpread = 0;
        } else {
            const col = chapterStartCols[idx];
            currentSpread = Math.floor(col / meta.numCols);
            if (currentSpread >= totalSpreads)
                currentSpread = Math.max(0, totalSpreads - 1);
        }
        applyTransform();
    }
    function goToEnd() {
        if (spine.length === 0) return;
        markPageTurn();
        const lastIdx = spine.length - 1;
        if (!renderedSet.has(lastIdx)) {
            const anchor = captureReadingAnchor();
            if (hydrateChapterDom(lastIdx)) {
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }
        }
        if (pageMode === "scroll") {
            // Scroll containers can have extra scrollable space below the
            // chapter's last line (padding, hydration placeholders and CSS
            // margins). Seeking by "chapter height - viewport" can therefore
            // stop short and hide the actual end of the book.
            keepAtBookEnd = true;
            snapScrollToBookEnd();
            return;
        }
        keepAtBookEnd = true;
        currentSpread = Math.max(0, totalSpreads - 1);
        applyTransform();
    }

    // ===== 模式切换 =====
    function setMode(m: ReadMode) {
        if (m === readMode) return;
        const anchor = captureReadingAnchor();
        readMode = m;
        columnLayout = m === "landscape" ? "double" : "single";
        persistSettings();
        requestAnimationFrame(() => {
            recomputeLayout();
            restoreReadingAnchor(anchor);
        });
    }
    function setColumnLayout(c: ColumnLayout) {
        setMode(c === "double" ? "landscape" : "portrait");
    }

    // 翻页样式：平移 (paginated) / 滚动 (scroll) 之间切换
    function setPageMode(m: PageMode) {
        if (m === pageMode) return;
        keepAtBookEnd = false;
        const anchor = captureReadingAnchor();
        const section = pageEl?.querySelector<HTMLElement>(
            `section.rd-chapter[data-idx="${anchor.ch}"]`,
        );
        const chapterProgress =
            anchor.mode === "scroll"
                ? Math.max(
                      0,
                      Math.min(
                          1,
                          anchor.scrollOffset /
                              Math.max(1, section?.scrollHeight ?? 1),
                      ),
                  )
                : (() => {
                      const chapterSpan =
                          (chapterStartCols[anchor.ch + 1] ?? totalCols) -
                          (chapterStartCols[anchor.ch] || 0);
                      return chapterSpan > 0
                          ? Math.max(
                                0,
                                Math.min(1, anchor.colInCh / chapterSpan),
                            )
                          : 0;
                  })();
        suppressProgressSave = true;
        pageMode = m;
        persistSettings();
        requestAnimationFrame(() => {
            // recomputeLayout measures synchronously in both modes; only read
            // chapterStartCols/totalCols after that call has completed.
            recomputeLayout();
            const newMeta = (pageEl as any).__rd;
            if (!newMeta) {
                suppressProgressSave = false;
                flushPendingProgressSave();
                return;
            }
            if (pageMode === "paginated") {
                const start = chapterStartCols[anchor.ch] || 0;
                const end = chapterStartCols[anchor.ch + 1] ?? totalCols;
                const newCol =
                    anchor.mode === "scroll"
                        ? Math.round(start + chapterProgress * (end - start))
                        : start + anchor.colInCh;
                currentSpread = Math.floor(newCol / newMeta.numCols);
                if (currentSpread >= totalSpreads)
                    currentSpread = Math.max(0, totalSpreads - 1);
                applyTransform();
            } else {
                const newSection = pageEl?.querySelector<HTMLElement>(
                    `section.rd-chapter[data-idx="${anchor.ch}"]`,
                );
                const verticalOffset =
                    anchor.mode === "scroll"
                        ? anchor.scrollOffset
                        : chapterProgress *
                          Math.max(
                              0,
                              (newSection?.scrollHeight ?? 1) -
                                  viewportEl.clientHeight * 0.5,
                          );
                scrollToChapterOffset(anchor.ch, verticalOffset);
            }
            suppressProgressSave = false;
            saveProgress();
            progressSavePending = false;
        });
    }

    // 滚动模式下的"下一屏 / 上一屏"
    function scrollNext() {
        if (!viewportEl) return;
        keepAtBookEnd = false;
        markPageTurn();
        viewportEl.scrollTop = Math.min(
            viewportEl.scrollTop + viewportEl.clientHeight - 40,
            viewportEl.scrollHeight - viewportEl.clientHeight,
        );
        syncCurrentSpreadForScroll();
        saveProgress();
    }
    function scrollPrev() {
        if (!viewportEl) return;
        keepAtBookEnd = false;
        markPageTurn();
        viewportEl.scrollTop = Math.max(
            0,
            viewportEl.scrollTop - viewportEl.clientHeight + 40,
        );
        syncCurrentSpreadForScroll();
        saveProgress();
    }

    // ===== 字号 / 行距 / 段距 / 边距 / 字体（统一 reflow + 锚回章节进度） =====
    function applyTypographyChange(mutate: () => void) {
        const anchor = captureReadingAnchor();

        mutate();
        persistSettings();

        requestAnimationFrame(() => {
            recomputeLayout();
            restoreReadingAnchor(anchor);
        });
    }
    function changeFont(delta: number) {
        const next = Math.max(12, Math.min(36, fontSize + delta));
        if (next === fontSize) return;
        applyTypographyChange(() => {
            fontSize = next;
        });
    }
    function changeLineHeight(delta: number) {
        const next = Math.max(
            1.2,
            Math.min(2.6, +(lineHeight + delta).toFixed(2)),
        );
        if (next === lineHeight) return;
        applyTypographyChange(() => {
            lineHeight = next;
        });
    }
    function changeParagraphSpacing(delta: number) {
        const next = Math.max(
            0,
            Math.min(2.5, +(paragraphSpacing + delta).toFixed(2)),
        );
        if (next === paragraphSpacing) return;
        applyTypographyChange(() => {
            paragraphSpacing = next;
        });
    }
    function changeMarginH(delta: number) {
        const next = Math.max(
            0.0,
            Math.min(0.18, +(pageMarginH + delta).toFixed(3)),
        );
        if (next === pageMarginH) return;
        applyTypographyChange(() => {
            pageMarginH = next;
        });
    }
    function changeMarginV(delta: number) {
        const next = Math.max(
            0.0,
            Math.min(0.1, +(pageMarginV + delta).toFixed(3)),
        );
        if (next === pageMarginV) return;
        applyTypographyChange(() => {
            pageMarginV = next;
        });
    }
    function setUserFontFamily(family: string) {
        if (family === userFontFamily) return;
        applyTypographyChange(() => {
            userFontFamily = family;
        });
    }
    function setBodyTextTone(tone: BodyTextTone) {
        if (tone === bodyTextTone) return;
        bodyTextTone = tone;
        persistSettings();
    }
    function setBodyFontWeight(weight: number) {
        const next = Math.max(350, Math.min(700, weight));
        if (next === bodyFontWeight) return;
        bodyFontWeight = next;
        persistSettings();
    }
    function toggleEpubFonts() {
        applyTypographyChange(() => {
            useEpubFonts = !useEpubFonts;
        });
    }

    // ===== 主题 =====
    function setTheme(p: ThemePreset) {
        themePreset = p;
        themePresetExplicit = true;
        customBgImage = ""; // 切换到内置主题就清掉自定义图
        persistSettings();
    }
    async function pickCustomBg() {
        return new Promise<void>((resolve) => {
            const input = document.createElement("input");
            input.type = "file";
            input.accept = "image/*";
            input.onchange = () => {
                const f = input.files?.[0];
                if (!f) {
                    resolve();
                    return;
                }
                const fr = new FileReader();
                fr.onload = () => {
                    customBgImage = String(fr.result || "");
                    persistSettings();
                    resolve();
                };
                fr.readAsDataURL(f);
            };
            input.click();
        });
    }
    function clearCustomBg() {
        customBgImage = "";
        persistSettings();
    }

    function persistSettings() {
        try {
            localStorage.setItem(
                READER_SETTINGS_KEY,
                JSON.stringify({
                    readMode,
                    columnLayout,
                    pageMode,
                    fontSize,
                    lineHeight,
                    paragraphSpacing,
                    pageMarginH,
                    pageMarginV,
                    userFontFamily,
                    useEpubFonts,
                    bodyTextTone,
                    bodyFontWeight,
                    themePreset,
                    themePresetExplicit,
                    customBgImage,
                    fullScreenNext,
                    wheelTurnPage,
                    arrowTurnPage,
                }),
            );
        } catch {}
    }
    function loadSettings() {
        try {
            const raw = localStorage.getItem(READER_SETTINGS_KEY);
            if (!raw) return;
            const s = JSON.parse(raw);
            if (s.readMode === "portrait" || s.readMode === "landscape")
                readMode = s.readMode;
            if (s.columnLayout === "single" || s.columnLayout === "double")
                columnLayout = s.columnLayout;
            if (s.pageMode === "paginated" || s.pageMode === "scroll")
                pageMode = s.pageMode;
            if (typeof s.fontSize === "number") fontSize = s.fontSize;
            if (typeof s.lineHeight === "number") lineHeight = s.lineHeight;
            if (typeof s.paragraphSpacing === "number")
                paragraphSpacing = s.paragraphSpacing;
            if (typeof s.pageMarginH === "number") pageMarginH = s.pageMarginH;
            if (typeof s.pageMarginV === "number") pageMarginV = s.pageMarginV;
            if (typeof s.userFontFamily === "string")
                userFontFamily = s.userFontFamily;
            if (typeof s.useEpubFonts === "boolean")
                useEpubFonts = s.useEpubFonts;
            if (
                s.bodyTextTone === "theme" ||
                s.bodyTextTone === "deep" ||
                s.bodyTextTone === "black"
            )
                bodyTextTone = s.bodyTextTone;
            if (typeof s.bodyFontWeight === "number")
                bodyFontWeight = Math.max(350, Math.min(700, s.bodyFontWeight));
            if (
                s.themePreset === "paper" ||
                s.themePreset === "eye" ||
                s.themePreset === "dark" ||
                s.themePreset === "sepia" ||
                s.themePreset === "snow"
            ) {
                themePreset = s.themePreset;
                // 只有"确实点过 chip"才算用户选择。老版本没存这个字段，
                // 里面的 themePreset 是默认值而不是选择，必须当作未选择，
                // 否则升级后深色界面打开书仍是米黄纸，这次的跟随主题等于没做
                themePresetExplicit = s.themePresetExplicit === true;
            }
            if (typeof s.customBgImage === "string")
                customBgImage = s.customBgImage;
            if (typeof s.fullScreenNext === "boolean")
                fullScreenNext = s.fullScreenNext;
            if (typeof s.wheelTurnPage === "boolean")
                wheelTurnPage = s.wheelTurnPage;
            if (typeof s.arrowTurnPage === "boolean")
                arrowTurnPage = s.arrowTurnPage;
            // 同步 readMode <-> columnLayout（旧版只有 readMode）
            if (s.columnLayout) {
                readMode = columnLayout === "double" ? "landscape" : "portrait";
            } else if (s.readMode) {
                columnLayout = readMode === "landscape" ? "double" : "single";
            }
        } catch {}
    }

    // 书签持久化（按 epubPath 独立，不与 settings 共享）
    function bookmarkKey(): string {
        return readerBookmarksKey(bookSourcePath || epubPath);
    }
    function loadBookmarks() {
        try {
            const raw = localStorage.getItem(bookmarkKey());
            if (!raw) return;
            const arr = JSON.parse(raw);
            if (Array.isArray(arr))
                bookmarks = arr.filter(
                    (b) =>
                        b &&
                        typeof b.ch === "number" &&
                        typeof b.sp === "number" &&
                        (b.scrollOffset === undefined ||
                            typeof b.scrollOffset === "number"),
                );
        } catch {}
    }
    function saveBookmarks() {
        try {
            localStorage.setItem(bookmarkKey(), JSON.stringify(bookmarks));
        } catch {}
    }
    function addBookmark() {
        const scrollPosition =
            pageMode === "scroll" ? getScrollReadingPosition() : null;
        const ch = scrollPosition?.ch ?? chapterIdxForSpread(currentSpread);
        const meta = (pageEl as any).__rd;
        const numCols = meta?.numCols ?? 1;
        const curLeftCol = currentSpread * numCols;
        const sp = scrollPosition
            ? Math.floor(
                  scrollPosition.offset /
                      Math.max(1, viewportEl?.clientHeight ?? 1),
              )
            : Math.floor((curLeftCol - (chapterStartCols[ch] || 0)) / numCols);
        // 防止同一位置重复添加
        if (
            bookmarks.some(
                (b) =>
                    b.ch === ch &&
                    (scrollPosition
                        ? Math.abs(
                              (b.scrollOffset ??
                                  b.sp * viewportEl.clientHeight) -
                                  scrollPosition.offset,
                          ) < 120
                        : b.sp === sp),
            )
        )
            return;
        const title = spine[ch]?.title || `第 ${ch + 1} 章`;
        // preview 取当前可见列内的文本片段（前 60 字）
        let preview = "";
        try {
            const sec = pageEl?.querySelector<HTMLElement>(
                `section.rd-chapter[data-idx="${ch}"]`,
            );
            if (sec) {
                const clone = sec.cloneNode(true) as HTMLElement;
                clone
                    .querySelectorAll(
                        "style, script, .rd-opening-header-spacer, .rd-end-sentinel",
                    )
                    .forEach((node) => node.remove());
                clone
                    .querySelectorAll<HTMLElement>("[style]")
                    .forEach((node) => {
                        if (getComputedStyle(node).display === "none")
                            node.remove();
                    });
                const txt = (clone.textContent || "")
                    .trim()
                    .replace(/\s+/g, " ");
                preview = txt.slice(0, 60);
            }
        } catch {}
        bookmarks = [
            {
                ch,
                sp,
                ...(scrollPosition
                    ? { scrollOffset: scrollPosition.offset }
                    : {}),
                title,
                preview,
                time: Date.now(),
            },
            ...bookmarks,
        ];
        saveBookmarks();
    }
    function removeBookmark(idx: number) {
        bookmarks = bookmarks.filter((_, i) => i !== idx);
        saveBookmarks();
    }
    function gotoBookmark(b: Bookmark) {
        if (b.ch < 0 || b.ch >= spine.length) return;
        keepAtBookEnd = false;
        if (!renderedSet.has(b.ch)) {
            const anchor = captureReadingAnchor();
            if (hydrateChapterDom(b.ch)) {
                recomputeLayout(true);
                restoreReadingAnchor(anchor);
            }
        }
        if (pageMode === "scroll") {
            scrollToChapterOffset(
                b.ch,
                b.scrollOffset ??
                    b.sp * Math.max(1, (pageEl as any).__rd?.pageH ?? 0),
            );
            activePanel = "";
            toolbarOpen = false;
            return;
        }
        const meta = (pageEl as any).__rd;
        const numCols = meta?.numCols ?? 1;
        const startCol = chapterStartCols[b.ch] || 0;
        const targetCol = startCol + b.sp * numCols;
        currentSpread = Math.floor(targetCol / numCols);
        if (currentSpread >= totalSpreads)
            currentSpread = Math.max(0, totalSpreads - 1);
        applyTransform();
        activePanel = "";
        toolbarOpen = false;
    }

    // ===== 派生：当前页码 / 章节进度 =====
    $: numColsView =
        pageMode === "paginated" && readMode === "landscape" ? 2 : 1;
    $: leftPageIdx = currentSpread * numColsView; // 左侧页索引（0-based）
    $: displayPageNum =
        pageMode === "scroll"
            ? Math.min(spine.length, currentChapterIdx + 1)
            : Math.min(totalCols, leftPageIdx + 1);
    $: currentChapterIdx = (() => {
        if (pageMode === "scroll")
            return Math.max(0, Math.min(spine.length - 1, scrollChapterIdx));
        if (chapterStartCols.length < 2) return 0;
        let lo = 0,
            hi = chapterStartCols.length - 2;
        while (lo < hi) {
            const mid = (lo + hi + 1) >> 1;
            if (chapterStartCols[mid] <= leftPageIdx) lo = mid;
            else hi = mid - 1;
        }
        return lo;
    })();
    $: chapterRemaining = (() => {
        const next = chapterStartCols[currentChapterIdx + 1] ?? totalCols;
        return Math.max(0, next - leftPageIdx - numColsView);
    })();

    // 当前章节标题（用于左上角显示）。第一章常常是"标题页 / 封面"，按用户要求不显示。
    $: chapterTitle = (() => {
        if (currentChapterIdx <= 0) return ""; // 标题页 / 封面：不显示
        return (
            spine[currentChapterIdx]?.title || `第 ${currentChapterIdx + 1} 章`
        );
    })();

    // 当前左侧页是否是章节的第一页 —— 用来决定顶部"章节名 / 本章剩余页"是否显示。
    // 章节首页（页 = 章节起始列）不显示，让头图 / 章节标题尽情贴顶展示；
    // 章节中后页才显示状态栏文字。
    $: isChapterFirstPage =
        pageMode === "scroll"
            ? scrollChapterOffset < 8
            : leftPageIdx === (chapterStartCols[currentChapterIdx] ?? 0);

    // 当前章节的全屏背景样式：从 chapterBgStyle 取（已在 loadEpub 中提取自 EPUB
    // body inline style 的 background-* 部分）。空字符串表示该章节没有自带背景，
    // 此时不渲染 .rd-chapter-bg div，背景由 .rd-app::before 自定义图层 / --rd-bg
    // 主题色显示。
    $: currentChapterBg = chapterBgStyle[currentChapterIdx] || "";

    // 阅读进度百分比（页码尚未计算完成时用，右下角 fallback 显示）
    $: progressPercent = (() => {
        if (!spine.length) return 0;
        if (pageMode === "scroll") {
            const section = pageEl?.querySelector<HTMLElement>(
                `section.rd-chapter[data-idx="${currentChapterIdx}"]`,
            );
            const inCh = section
                ? Math.max(
                      0,
                      Math.min(
                          1,
                          scrollChapterOffset /
                              Math.max(1, section.offsetHeight),
                      ),
                  )
                : 0;
            return Math.min(
                100,
                Math.max(0, ((currentChapterIdx + inCh) / spine.length) * 100),
            );
        }
        // 用 章节数 + 章节内 spread 估算一个粗略百分比
        const ch = currentChapterIdx;
        const sp = leftPageIdx - (chapterStartCols[ch] || 0);
        const chSpan =
            (chapterStartCols[ch + 1] ?? totalCols) -
            (chapterStartCols[ch] || 0);
        const inCh = chSpan > 0 ? sp / chSpan : 0;
        return Math.min(100, Math.max(0, ((ch + inCh) / spine.length) * 100));
    })();

    // 当前位置是否已加书签
    // 注意：bind:this 元素 (pageEl) 在组件首次渲染前是 undefined，但 Svelte 5 把
    // `$:` 编译成 legacy_pre_effect，会在 mount 之前先跑一次。直接 (pageEl as any).__rd
    // 会抛 TypeError 让整个组件渲染失败 → 白屏。这里必须用可选链兜底。
    $: isBookmarked = (() => {
        if (!pageEl) return false;
        const scrollPosition =
            pageMode === "scroll" ? getScrollReadingPosition() : null;
        const ch = scrollPosition?.ch ?? chapterIdxForSpread(currentSpread);
        const meta = (pageEl as any)?.__rd;
        const numCols = meta?.numCols ?? 1;
        const curLeftCol = currentSpread * numCols;
        const sp = scrollPosition
            ? Math.floor(
                  scrollPosition.offset /
                      Math.max(1, viewportEl?.clientHeight ?? 1),
              )
            : Math.floor((curLeftCol - (chapterStartCols[ch] || 0)) / numCols);
        return bookmarks.some(
            (b) =>
                b.ch === ch &&
                (scrollPosition && b.scrollOffset !== undefined
                    ? Math.abs(b.scrollOffset - scrollPosition.offset) < 120
                    : b.sp === sp),
        );
    })();

    // ===== 事件 =====
    // 同窗口保活：别的标签的页面也一直挂着，它们的 window 监听器不会因为
    // 隐藏而失效。不判当前视图的话，隐藏的阅读器会把可见编辑器的方向键
    // 拿去翻页。
    const activeOnly = onlyWhenActive();
    function onKeydown(e: KeyboardEvent) {
        if (!activeOnly()) return;
        // 焦点在阅读区域时，按键由 onFrameKeydown 处理；两个监听器都执行
        // 会造成一次 ArrowRight 前进两页。
        if (e.defaultPrevented) return;
        if (activePanel) return;
        if (!arrowTurnPage) {
            // 仍允许 Esc 关闭
            if (e.key === "Escape") back();
            return;
        }
        if (e.key === "ArrowLeft" || e.key === "PageUp") {
            e.preventDefault();
            if (e.key === "ArrowLeft" && pageMode === "scroll") return;
            pageMode === "scroll" ? scrollPrev() : prevSpread();
        } else if (
            e.key === "ArrowRight" ||
            e.key === "PageDown" ||
            e.key === " "
        ) {
            e.preventDefault();
            if (e.key === "ArrowRight" && pageMode === "scroll") return;
            pageMode === "scroll" ? scrollNext() : nextSpread();
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            pageMode === "scroll" ? scrollPrev() : prevSpread();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            pageMode === "scroll" ? scrollNext() : nextSpread();
        } else if (e.key === "Home") {
            e.preventDefault();
            if (pageMode === "scroll" && viewportEl) viewportEl.scrollTop = 0;
            else goToSpread(0);
        } else if (e.key === "End") {
            e.preventDefault();
            goToEnd();
        } else if (e.key === "Escape") {
            back();
        } else if (e.key === "+" || e.key === "=") {
            e.preventDefault();
            changeFont(+1);
        } else if (e.key === "-") {
            e.preventDefault();
            changeFont(-1);
        }
    }

    // 滚轮翻页（仅平移模式有意义；滚动模式让浏览器原生处理）
    let wheelAccum = 0;
    let wheelTimer: any = null;
    function onWheel(e: WheelEvent) {
        if (!activeOnly()) return;
        if (pageMode !== "paginated") return; // 滚动模式：不拦截，浏览器自然滚动
        if (!wheelTurnPage) return;
        if (activePanel) return;
        e.preventDefault();
        wheelAccum += e.deltaY;
        // 简单防抖：累积超过阈值才触发，避免触摸板细滚导致疯狂翻页
        const THRESHOLD = 50;
        if (Math.abs(wheelAccum) >= THRESHOLD) {
            if (wheelAccum > 0) nextSpread();
            else prevSpread();
            wheelAccum = 0;
        }
        if (wheelTimer) clearTimeout(wheelTimer);
        wheelTimer = setTimeout(() => {
            wheelAccum = 0;
        }, 200);
    }

    function onFrameClick(e: MouseEvent) {
        const target = e.target as HTMLElement | null;
        const anchor = target?.closest<HTMLAnchorElement>("a") || null;
        if (anchor) {
            e.preventDefault();
            const epubType = anchor.getAttribute("epub:type") || "";
            const isNoteref =
                anchor.getAttribute("role") === "doc-noteref" ||
                anchor.classList.contains("duokan-footnote") ||
                epubType.split(/\s+/).includes("noteref");
            const href = anchor.getAttribute("href") || "";
            if (isNoteref && href.startsWith("#")) {
                const fragmentId = safeDecode(href.slice(1));
                const noteChapter =
                    pageMode === "scroll"
                        ? currentChapterIdx
                        : chapterIdxForSpread(currentSpread);
                const currentSection = pageEl?.querySelector<HTMLElement>(
                    `section.rd-chapter[data-idx="${noteChapter}"]`,
                );
                const note =
                    Array.from(
                        currentSection?.querySelectorAll<HTMLElement>("[id]") ||
                            [],
                    ).find((el) => el.id === fragmentId) ||
                    pageEl?.querySelector<HTMLElement>(
                        `#${CSS.escape(fragmentId)}`,
                    );
                if (note) {
                    e.stopPropagation();
                    footnotePopupHtml = note.innerHTML;
                    activePanel = "";
                    return;
                }
            }
            if (href) {
                e.stopPropagation();
                navigateEpubLink(href);
                return;
            }
        }

        // 工具栏 / 面板已开 → 任意点击关闭
        if (activePanel) {
            activePanel = "";
            return;
        }

        // 全屏下一页：点击任意位置都翻下一页（中部仍可唤起工具栏？让交互一致：设此模式后必须用工具栏按钮唤起，不再用中间区）
        if (fullScreenNext) {
            if (pageMode === "scroll") scrollNext();
            else nextSpread();
            return;
        }

        // 默认：左 1/3 上一页 / 右 1/3 下一页 / 中间 1/3 切换工具栏
        const rect = frameEl.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const w = rect.width;
        if (x < w * 0.33) {
            if (pageMode === "scroll") scrollPrev();
            else prevSpread();
        } else if (x > w * 0.67) {
            if (pageMode === "scroll") scrollNext();
            else nextSpread();
        } else {
            // 切换底部工具栏
            activePanel = activePanel ? "" : "menu";
        }
    }

    function onFootnotePopupClick(e: MouseEvent) {
        const target = e.target as HTMLElement | null;
        const backLink = target?.closest<HTMLAnchorElement>(
            "a.duokan-footnote-back",
        );
        if (!backLink) return;
        e.preventDefault();
        e.stopPropagation();
        footnotePopupHtml = "";
    }

    function onFrameKeydown(event: KeyboardEvent) {
        if (event.target !== event.currentTarget) return;
        // Arrow keys bubble to the window listener exactly once. Space is kept
        // local to toggle the toolbar instead of turning the page.
        if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            event.stopPropagation();
            activePanel = activePanel ? "" : "menu";
        }
    }

    // 隐藏期间窗口尺寸变了：布局没重算，切回来必须补一次，否则列宽还是旧的
    let layoutDirtyWhileHidden = false;

    function onResize() {
        if (loading) return;
        // 保活：隐藏的页面也占着真实尺寸，改窗口大小同样会变宽高。但没必要
        // 在后台重排整本书，记个脏标记，切回来时补算。
        if (keepAliveView && !keepAliveView.isActive()) {
            layoutDirtyWhileHidden = true;
            return;
        }
        const anchor = captureReadingAnchor();
        requestAnimationFrame(() => {
            recomputeLayout();
            restoreReadingAnchor(anchor);
        });
    }

    async function back() {
        // 同窗口模式（默认）：没有独立阅读器窗口可关。先摘掉自己的标签，
        // 再跳到上一个标签 —— 用 history.back() 会把已关闭的页留在标签栏里
        if (!usesMultiWindow()) {
            const remaining = closeCurrentTab();
            // 用 goto 而不是 location.href：后者会整页重载，标签栏闪一下白，
            // 而且 href 已经带 base 前缀，再拼一次 base 会把路径拼坏
            await goto(remaining?.href || `${base}/`);
            return;
        }
        // 桌面：关闭当前 Tauri 窗口（失败时回退浏览器后退）；Web：直接浏览器后退。
        if (platform.isTauri) {
            try {
                await platform.closeCurrentWindow();
                return;
            } catch {}
        }
        window.history.back();
    }

    // ===== 生命周期 =====
    onMount(() => {
        loadSettings();
        // 没存过（或没显式选过）阅读主题时，跟随应用深/浅色：
        // 深色界面打开书不该直接怼一张米黄纸
        applyAppThemeDefault();
        const onAppThemeChanged = () => {
            if (applyAppThemeDefault()) persistSettings();
        };
        window.addEventListener("tepub-settings-updated", onAppThemeChanged);
        window.addEventListener(THEME_RESOLVED_EVENT, onAppThemeChanged);
        window.addEventListener("storage", onAppThemeChanged);
        const stopActivation = keepAliveView?.onActivationChange((active) => {
            if (active) {
                // 隐藏期间样式表被摘掉了，切回来挂回去（sheet 对象一直留着）
                attachAdoptedChapterStyles();
                scheduleHydrate();
                // 隐藏期间改过窗口尺寸：补算一次列宽，再按锚点回到原位置
                if (layoutDirtyWhileHidden) {
                    layoutDirtyWhileHidden = false;
                    requestAnimationFrame(() => {
                        if (destroyed || loading) return;
                        const anchor = captureReadingAnchor();
                        recomputeLayout();
                        restoreReadingAnchor(anchor);
                    });
                }
                return;
            }
            detachAdoptedChapterStyles();
            // 隐藏前把阅读进度落盘：用户可能在切走后就关掉窗口
            if (pageMode === "scroll" && !loading) saveProgress();
        });
        onDestroy(() => {
            stopActivation?.();
            window.removeEventListener(
                "tepub-settings-updated",
                onAppThemeChanged,
            );
            window.removeEventListener(
                THEME_RESOLVED_EVENT,
                onAppThemeChanged,
            );
            window.removeEventListener("storage", onAppThemeChanged);
        });
        // 保活模式下读本视图自己的地址：全局 page.url 会随别的标签跳转而变
        epubPath = ownUrl.searchParams.get("file") || "";
        if (!epubPath) {
            errorMsg = t("未提供 EPUB 文件路径");
            loading = false;
            return;
        }
        // URLSearchParams has already decoded the query value. Decoding it a
        // second time would corrupt real filenames containing sequences such
        // as "%20" or "%E5".
        bookSourcePath = epubPath;
        loadBookmarks();

        tickClock();
        clockTimer = setInterval(tickClock, 30 * 1000);

        window.addEventListener("keydown", onKeydown);
        window.addEventListener("resize", onResize);
        window.addEventListener("wheel", onWheel, { passive: false });

        platform
            .listen<EpubPrepareStageEvent>("epub-prepare-stage", (event) => {
                if (!loading) return;
                if (
                    event.payload.epubPath &&
                    event.payload.epubPath !== epubPath
                )
                    return;
                setLoadingMessage(event.payload.message);
            })
            .then((fn) => {
                unlistenPrepareStage = fn;
            })
            .finally(() => {
                loadEpub();
            });
    });

    onDestroy(() => {
        destroyed = true;
        clearAdoptedChapterStyles();
        if (pageMode === "scroll" && !loading) saveProgress();
        if (clockTimer) clearInterval(clockTimer);
        if (scrollSaveTimer) clearTimeout(scrollSaveTimer);
        scrollSaveTimer = null;
        if (ensureTimer) clearTimeout(ensureTimer);
        if (hydrateId !== null) {
            if (typeof cancelIdleCallback === "function")
                cancelIdleCallback(hydrateId);
            else clearTimeout(hydrateId);
            hydrateId = null;
        }
        window.removeEventListener("keydown", onKeydown);
        window.removeEventListener("resize", onResize);
        window.removeEventListener("wheel", onWheel as any);
        unlistenPrepareStage?.();
    });

    function noop() {}

    // 滚动模式下，监听 viewport 滚动以保存进度（节流）
    let scrollSaveTimer: any = null;
    function onViewportScroll() {
        if (destroyed) return;
        if (pageMode !== "scroll") {
            // Do not infer "user left the book end" from programmatic
            // scrollLeft changes. applyTransform() clamps to the same
            // maxScrollLeft repeatedly, and those writes also fire scroll
            // events. Only explicit navigation clears keepAtBookEnd.
            return;
        }
        if (keepAtBookEnd && !pinningBookEnd) keepAtBookEnd = false;
        if (suppressProgressSave) {
            progressSavePending = true;
            return;
        }
        syncCurrentSpreadForScroll();
        // Do not reset the debounce on every scroll event. Background hydration
        // triggers a layout/scroll event roughly once per idle batch, so a
        // sliding 300ms timer can postpone the save for the entire hydration
        // run. Keep the first timer and let it write the latest position.
        if (!scrollSaveTimer) {
            scrollSaveTimer = setTimeout(() => {
                scrollSaveTimer = null;
                saveProgress();
            }, 300);
        }
    }

    // ===== 模板辅助函数（避免在模板里写复杂表达式让 Svelte 5 parser 出错）=====
    function gotoChapterAndClose(i: number) {
        goToChapter(i);
        activePanel = "";
    }
    function tabClick(name: Panel) {
        activePanel = activePanel === name ? "" : name;
        // 切到目录面板时，等面板渲染完再把当前章节行滚到视野中部，
        // 省得用户自己翻长目录找"我在哪"。
        if (activePanel === "toc") {
            tick().then(() => {
                const el =
                    tocPanelEl?.querySelector<HTMLElement>(
                        '[data-current="1"]',
                    );
                if (el && tocPanelEl) {
                    const rect = el.getBoundingClientRect();
                    const pRect = tocPanelEl.getBoundingClientRect();
                    // scrollIntoView({block:"center"}) 在 flex 容器里有时无效，手动算
                    tocPanelEl.scrollTop +=
                        rect.top - pRect.top - (pRect.height - rect.height) / 2;
                }
            });
        }
    }
    function setEpubFonts(enable: boolean) {
        if (enable !== useEpubFonts) toggleEpubFonts();
    }
    function formatBookmarkTime(t: number): string {
        const d = new Date(t);
        const pad = (n: number) => (n < 10 ? "0" + n : "" + n);
        return `${d.getMonth() + 1}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
    }
</script>

<svelte:head>
    <!-- 保活时这条会被别的页面覆盖，真正的标题登记在视图宿主里（见脚本） -->
    <title>{bookTitle ? `${bookTitle} · ${t("阅读")}` : t("阅读")}</title>
</svelte:head>

<div
    class="rd-app"
    data-mode={readMode}
    data-page-mode={pageMode}
    data-theme={customBgImage ? "custom" : themePreset}
    bind:this={appEl}
    style={appStyle}
    class:no-epub-fonts={!useEpubFonts}
    class:fullscreen-next={fullScreenNext}
>
    <!-- ===== 主阅读区 ===== -->
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
        class="rd-frame"
        bind:this={frameEl}
        role="application"
        aria-label={t("EPUB 阅读区域")}
        tabindex="0"
        on:click={onFrameClick}
        on:keydown={onFrameKeydown}
    >
        {#if loading}
            <div class="rd-loading">{loadingMsg}</div>
        {:else if errorMsg}
            <div class="rd-error">
                <p>{t("加载失败")}</p>
                <pre>{errorMsg}</pre>
            </div>
        {/if}
        <!--
      全屏章节背景层：当前章节有自带 body background 时（如 .full 章节有
      background-image:url），这里复制相同的 background-* 属性，让背景图
      填满整个 frame（不只列宽），不再被 viewport padding 区的米色 paper
      主题色泄露。位于 viewport 之下、frame 之内，pointer-events: none 不挡点击。
      章节没自带背景时 style 为空字符串，div 仍然存在但不可见 —— 这样切换
      章节时 div 不会被销毁重建，只是 background-image 切换，无闪烁。
    -->
        {#if currentChapterBg}
            <div
                class="rd-chapter-bg"
                style={currentChapterBg}
                aria-hidden="true"
            ></div>
        {/if}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
            class="rd-viewport"
            bind:this={viewportEl}
            on:scroll={onViewportScroll}
        >
            <div class="rd-canvas" bind:this={canvasEl}>
                <div class="rd-page" bind:this={pageEl}>
                    {@html combinedHtml}
                </div>
            </div>
        </div>
    </div>

    {#if footnotePopupHtml}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
            class="rd-footnote-backdrop"
            on:click={() => (footnotePopupHtml = "")}
        >
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <!-- svelte-ignore a11y-no-static-element-interactions -->
            <div
                class="rd-footnote-popup"
                on:click|stopPropagation={onFootnotePopupClick}
            >
                <button
                    class="rd-footnote-close"
                    type="button"
                    aria-label={t("关闭注释")}
                    on:click={() => (footnotePopupHtml = "")}>×</button
                >
                {@html footnotePopupHtml}
            </div>
        </div>
    {/if}

    <!-- ===== 4 个角的常驻信息（始终显示，不随工具栏开关） ===== -->
    <!--
    顶部章节名 + 剩余页：仅在"非章节第一页"显示。章节首页留空让头图 / 章节
    标题贴顶，避免状态栏文字盖住章节封面图。
  -->
    {#if !loading && !errorMsg}
        {#if chapterTitle && !isChapterFirstPage}
            <div class="rd-corner rd-corner-tl" title={chapterTitle}>
                {chapterTitle}
            </div>
        {/if}
        {#if pageMode === "paginated" && totalCols > 0 && !isChapterFirstPage}
            <div class="rd-corner rd-corner-tr">
                {t("本章剩余 {count} 页", { count: chapterRemaining })}
            </div>
        {/if}
        <div class="rd-corner rd-corner-bl">{nowStr}</div>
        <div class="rd-corner rd-corner-br">
            {#if pageMode === "scroll"}
                {progressPercent.toFixed(1)}%
            {:else if totalCols > 0}
                {displayPageNum} / {totalCols}
            {:else}
                {t("约 {percent}%", {
                    percent: progressPercent.toFixed(0),
                })}
            {/if}
        </div>
    {/if}

    <!-- ===== 返回按钮已移除：保留 back() 函数供 Esc 键调用 ===== -->

    <!-- ===== 底部工具栏：5 tab + 当前面板内容 ===== -->
    {#if !loading && !errorMsg}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
            class="rd-tools"
            class:open={!!activePanel}
            on:click|stopPropagation={noop}
        >
            <!-- 当前面板（位于 tab 条上方） -->
            {#if activePanel === "toc"}
                <div class="rd-panel rd-panel-toc" bind:this={tocPanelEl}>
                    <div class="rd-panel-title">{t("目录")}</div>
                    <div class="rd-toc-list">
                        {#each toc as node}
                            <ReaderTocNode
                                {node}
                                {currentChapterIdx}
                                level={0}
                                onSelect={gotoChapterAndClose}
                            />
                        {/each}
                    </div>
                </div>
            {:else if activePanel === "bookmarks"}
                <div class="rd-panel rd-panel-bookmarks">
                    <div class="rd-panel-title">
                        {t("书签")}
                        <button
                            class="rd-mini-btn"
                            class:active={isBookmarked}
                            on:click={() =>
                                isBookmarked ? null : addBookmark()}
                            disabled={isBookmarked}
                            >{t(
                                isBookmarked ? "已添加" : "+ 添加当前位置",
                            )}</button
                        >
                    </div>
                    {#if bookmarks.length === 0}
                        <div class="rd-empty">
                            {t('暂无书签。点击上方"+ 添加当前位置"。')}
                        </div>
                    {:else}
                        <div class="rd-bookmarks-list">
                            {#each bookmarks as b, i}
                                <div class="rd-bookmark-row">
                                    <button
                                        class="rd-bookmark-main"
                                        on:click={() => gotoBookmark(b)}
                                    >
                                        <div class="rd-bookmark-title">
                                            {b.title}
                                        </div>
                                        {#if b.preview}
                                            <div class="rd-bookmark-preview">
                                                {b.preview}
                                            </div>
                                        {/if}
                                        <div class="rd-bookmark-time">
                                            {formatBookmarkTime(b.time)}
                                        </div>
                                    </button>
                                    <button
                                        class="rd-mini-btn rd-bookmark-del"
                                        on:click={() => removeBookmark(i)}
                                        title={t("删除")}>×</button
                                    >
                                </div>
                            {/each}
                        </div>
                    {/if}
                </div>
            {:else if activePanel === "theme"}
                <div class="rd-panel rd-panel-theme">
                    <div class="rd-panel-title">{t("主题")}</div>
                    <div class="rd-theme-grid">
                        <button
                            class="rd-theme-chip rd-theme-paper"
                            class:active={themePreset === "paper" &&
                                !customBgImage}
                            on:click={() => setTheme("paper")}
                            ><span>{t("纸张")}</span></button
                        >
                        <button
                            class="rd-theme-chip rd-theme-eye"
                            class:active={themePreset === "eye" &&
                                !customBgImage}
                            on:click={() => setTheme("eye")}
                            ><span>{t("护眼")}</span></button
                        >
                        <button
                            class="rd-theme-chip rd-theme-sepia"
                            class:active={themePreset === "sepia" &&
                                !customBgImage}
                            on:click={() => setTheme("sepia")}
                            ><span>{t("羊皮")}</span></button
                        >
                        <button
                            class="rd-theme-chip rd-theme-snow"
                            class:active={themePreset === "snow" &&
                                !customBgImage}
                            on:click={() => setTheme("snow")}
                            ><span>{t("雪白")}</span></button
                        >
                        <button
                            class="rd-theme-chip rd-theme-dark"
                            class:active={themePreset === "dark" &&
                                !customBgImage}
                            on:click={() => setTheme("dark")}
                            ><span>{t("暗夜")}</span></button
                        >
                    </div>
                    <div class="rd-row">
                        <button class="rd-secondary" on:click={pickCustomBg}
                            >{t("更换背景图…")}</button
                        >
                        <button
                            class="rd-secondary"
                            disabled={!customBgImage}
                            on:click={clearCustomBg}>{t("恢复默认")}</button
                        >
                    </div>
                    {#if customBgImage}
                        <div class="rd-row rd-tip">
                            {t("已应用自定义背景图")}
                        </div>
                    {/if}
                </div>
            {:else if activePanel === "typography"}
                <div class="rd-panel rd-panel-typography">
                    <div class="rd-panel-title">{t("排版")}</div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("字号")}</span>
                        <div class="rd-segctrl">
                            <button
                                on:click={() => changeFont(-1)}
                                disabled={fontSize <= 12}>A−</button
                            >
                            <span class="rd-font-val">{fontSize}</span>
                            <button
                                on:click={() => changeFont(+1)}
                                disabled={fontSize >= 36}>A+</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("行距")}</span>
                        <div class="rd-segctrl">
                            <button
                                on:click={() => changeLineHeight(-0.1)}
                                disabled={lineHeight <= 1.2}>−</button
                            >
                            <span class="rd-font-val"
                                >{lineHeight.toFixed(2)}</span
                            >
                            <button
                                on:click={() => changeLineHeight(+0.1)}
                                disabled={lineHeight >= 2.6}>+</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("段距")}</span>
                        <div class="rd-segctrl">
                            <button
                                on:click={() => changeParagraphSpacing(-0.1)}
                                disabled={paragraphSpacing <= 0}>−</button
                            >
                            <span class="rd-font-val"
                                >{paragraphSpacing.toFixed(2)}</span
                            >
                            <button
                                on:click={() => changeParagraphSpacing(+0.1)}
                                disabled={paragraphSpacing >= 2.5}>+</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("字色")}</span>
                        <div class="rd-segctrl">
                            <button
                                class:active={bodyTextTone === "theme"}
                                on:click={() => setBodyTextTone("theme")}
                                >{t("主题")}</button
                            >
                            <button
                                class:active={bodyTextTone === "deep"}
                                on:click={() => setBodyTextTone("deep")}
                                >{t("加深")}</button
                            >
                            <button
                                class:active={bodyTextTone === "black"}
                                on:click={() => setBodyTextTone("black")}
                                >{t("黑字")}</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("字重")}</span>
                        <div class="rd-segctrl">
                            <button
                                class:active={bodyFontWeight === 400}
                                on:click={() => setBodyFontWeight(400)}
                                >{t("常规")}</button
                            >
                            <button
                                class:active={bodyFontWeight === 500}
                                on:click={() => setBodyFontWeight(500)}
                                >{t("适中")}</button
                            >
                            <button
                                class:active={bodyFontWeight === 600}
                                on:click={() => setBodyFontWeight(600)}
                                >{t("加粗")}</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("左右")}</span>
                        <div class="rd-segctrl">
                            <button
                                on:click={() => changeMarginH(-0.01)}
                                disabled={pageMarginH <= 0}>−</button
                            >
                            <span class="rd-font-val"
                                >{Math.round(pageMarginH * 100)}%</span
                            >
                            <button
                                on:click={() => changeMarginH(+0.01)}
                                disabled={pageMarginH >= 0.18}>+</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("上下")}</span>
                        <div class="rd-segctrl">
                            <button
                                on:click={() => changeMarginV(-0.01)}
                                disabled={pageMarginV <= 0}>−</button
                            >
                            <span class="rd-font-val"
                                >{Math.round(pageMarginV * 100)}%</span
                            >
                            <button
                                on:click={() => changeMarginV(+0.01)}
                                disabled={pageMarginV >= 0.1}>+</button
                            >
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("字体")}</span>
                        <div class="rd-segctrl rd-font-picker">
                            <CustomSelect
                                value={userFontFamily}
                                options={[
                                    { value: "", label: t("默认（衬线）") },
                                    {
                                        value: "Source Han Sans SC",
                                        label: t("思源黑体"),
                                    },
                                    {
                                        value: "Microsoft YaHei",
                                        label: t("微软雅黑"),
                                    },
                                    {
                                        value: "PingFang SC",
                                        label: t("苹方"),
                                    },
                                    { value: "SimSun", label: t("宋体") },
                                    { value: "KaiTi", label: t("楷体") },
                                    {
                                        value: "FangSong",
                                        label: t("仿宋"),
                                    },
                                    {
                                        value: "Source Han Serif SC",
                                        label: t("思源宋体"),
                                    },
                                ]}
                                ariaLabel={t("阅读字体")}
                                on:change={(event) =>
                                    setUserFontFamily(event.detail)}
                            />
                        </div>
                    </div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("EPUB 字体")}</span>
                        <div class="rd-segctrl">
                            <button
                                class:active={useEpubFonts}
                                on:click={() => setEpubFonts(true)}
                                >{t("启用")}</button
                            >
                            <button
                                class:active={!useEpubFonts}
                                on:click={() => setEpubFonts(false)}
                                >{t("禁用")}</button
                            >
                        </div>
                    </div>
                </div>
            {:else if activePanel === "settings"}
                <div class="rd-panel rd-panel-settings">
                    <div class="rd-panel-title">{t("设置")}</div>

                    <div class="rd-set-row">
                        <span class="rd-set-label">{t("翻页样式")}</span>
                        <div class="rd-segctrl">
                            <button
                                class:active={pageMode === "paginated"}
                                on:click={() => setPageMode("paginated")}
                                >{t("平移翻页")}</button
                            >
                            <button
                                class:active={pageMode === "scroll"}
                                on:click={() => setPageMode("scroll")}
                                >{t("滚动翻页")}</button
                            >
                        </div>
                    </div>

                    {#if pageMode === "paginated"}
                        <div class="rd-set-row">
                            <span class="rd-set-label">{t("列布局")}</span>
                            <div class="rd-segctrl">
                                <button
                                    class:active={columnLayout === "single"}
                                    on:click={() => setColumnLayout("single")}
                                    >{t("单列")}</button
                                >
                                <button
                                    class:active={columnLayout === "double"}
                                    on:click={() => setColumnLayout("double")}
                                    >{t("双列")}</button
                                >
                            </div>
                        </div>
                    {/if}

                    <div class="rd-set-row rd-toggle-row">
                        <span class="rd-set-label">{t("全屏下一页")}</span>
                        <label class="rd-switch">
                            <input
                                type="checkbox"
                                bind:checked={fullScreenNext}
                                on:change={persistSettings}
                            />
                            <span class="rd-switch-slider"></span>
                        </label>
                    </div>

                    <div class="rd-set-row rd-toggle-row">
                        <span class="rd-set-label">{t("滚轮翻页")}</span>
                        <label class="rd-switch">
                            <input
                                type="checkbox"
                                bind:checked={wheelTurnPage}
                                on:change={persistSettings}
                            />
                            <span class="rd-switch-slider"></span>
                        </label>
                    </div>

                    <div class="rd-set-row rd-toggle-row">
                        <span class="rd-set-label">{t("方向键翻页")}</span>
                        <label class="rd-switch">
                            <input
                                type="checkbox"
                                bind:checked={arrowTurnPage}
                                on:change={persistSettings}
                            />
                            <span class="rd-switch-slider"></span>
                        </label>
                    </div>
                </div>
            {/if}

            <!-- 5 tab 横条 -->
            <div class="rd-tabs">
                <button
                    class:active={activePanel === "toc"}
                    on:click={() => tabClick("toc")}
                >
                    <span class="rd-tab-icon">≡</span><span class="rd-tab-label"
                        >{t("目录")}</span
                    >
                </button>
                <button
                    class:active={activePanel === "bookmarks"}
                    on:click={() => tabClick("bookmarks")}
                >
                    <span class="rd-tab-icon">❑</span><span class="rd-tab-label"
                        >{t("书签")}</span
                    >
                </button>
                <button
                    class:active={activePanel === "theme"}
                    on:click={() => tabClick("theme")}
                >
                    <span class="rd-tab-icon">◐</span><span class="rd-tab-label"
                        >{t("主题")}</span
                    >
                </button>
                <button
                    class:active={activePanel === "typography"}
                    on:click={() => tabClick("typography")}
                >
                    <span class="rd-tab-icon">A</span><span class="rd-tab-label"
                        >{t("排版")}</span
                    >
                </button>
                <button
                    class:active={activePanel === "settings"}
                    on:click={() => tabClick("settings")}
                >
                    <span class="rd-tab-icon">⚙</span><span class="rd-tab-label"
                        >{t("设置")}</span
                    >
                </button>
            </div>
        </div>
    {/if}
</div>

<style>
    /* ================================================================
     主题（CSS 变量）：默认 paper；其他通过 [data-theme=...] 切换。
     用户自定义背景图通过 --rd-bg-image 注入到 .rd-app::before 层。
     ================================================================ */
    .rd-app {
        --rd-bg: #e9e3d3;
        --rd-bg-soft: #efe9da;
        --rd-text: #443f37;
        --rd-text-soft: #8a8576;
        --rd-accent: #7c5a3a;
        --rd-border: rgba(124, 90, 58, 0.18);
        --rd-tools-bg: rgba(233, 227, 211, 0.96);
    }
    .rd-app[data-theme="eye"] {
        --rd-bg: #cce8cf;
        --rd-bg-soft: #d8eedb;
        --rd-text: #2f4633;
        --rd-text-soft: #678a6c;
        --rd-accent: #3f7a4f;
        --rd-border: rgba(63, 122, 79, 0.22);
        --rd-tools-bg: rgba(204, 232, 207, 0.96);
    }
    .rd-app[data-theme="sepia"] {
        --rd-bg: #f4ecd8;
        --rd-bg-soft: #fbf5e3;
        --rd-text: #5c4a32;
        --rd-text-soft: #8a7755;
        --rd-accent: #b07c4c;
        --rd-border: rgba(176, 124, 76, 0.22);
        --rd-tools-bg: rgba(244, 236, 216, 0.96);
    }
    .rd-app[data-theme="snow"] {
        --rd-bg: #ffffff;
        --rd-bg-soft: #f3f4f6;
        --rd-text: #1f2937;
        --rd-text-soft: #6b7280;
        --rd-accent: #2563eb;
        --rd-border: rgba(37, 99, 235, 0.18);
        --rd-tools-bg: var(--color-surface);
    }
    /* 暗夜主题的底色对齐应用深色画布（--color-canvas 深色值 #191e26）。
       原先用 #1c1c1c，从墨蓝的界面跳进阅读器会像换了个应用 */
    .rd-app[data-theme="dark"] {
        --rd-bg: #191e26;
        --rd-bg-soft: #1e2430;
        --rd-text: #e2e8f0;
        --rd-text-soft: #93a1b0;
        --rd-accent: #c69658;
        --rd-border: rgba(198, 150, 88, 0.24);
        --rd-tools-bg: rgba(20, 24, 29, 0.96);
    }
    .rd-app {
        position: fixed;
        inset: 0;
        background: var(--rd-bg);
        color: var(--rd-text);
        overflow: hidden;
        font-family:
            var(--rd-user-font, "Source Han Serif SC"), "Noto Serif SC",
            "Source Han Serif CN", "Songti SC", "STSong", "FangSong", "SimSun",
            serif;
    }
    .rd-app::before {
        /* 自定义背景图层（用户手动选的图）。位于 frame 之下，覆盖整窗口。
       仅在用户设置了 --rd-bg-image 时才显示，否则透明。 */
        content: "";
        position: absolute;
        inset: 0;
        background-image: var(--rd-bg-image, none);
        background-size: cover;
        background-position: center;
        background-repeat: no-repeat;
        pointer-events: none;
        z-index: 0;
    }

    /* 主阅读区（位于背景层之上）。
     注意：以前 frame 用 flex 居中 viewport，导致 viewport 宽度只等于列宽 + 边距，
     viewport 之外的区域露出 .rd-app 的米色 --rd-bg；EPUB 章节自带的 body 背景图、
     用户头图都到不了那里。现在 viewport 由 JS 设为占满整个 frame（用 padding
     留阅读边距），背景图能填满整窗，头图也能负 margin 到窗口边。
     frame 不再做内部居中，只作为定位锚点。 */
    .rd-frame {
        position: absolute;
        inset: 0;
        overflow: hidden;
        cursor: pointer;
        user-select: none;
        -webkit-user-select: none;
        -webkit-touch-callout: none;
        z-index: 1;
    }
    .rd-viewport {
        position: absolute;
        inset: 0;
        overflow: hidden; /* 平移模式 */
        box-sizing: border-box;
        /*
      告知浏览器 scroll-position 会频繁变化，提前提升合成层 + 准备瓦片缓存。
      避免每次翻页时浏览器从头判定优化策略导致前几次翻页明显比后几次卡。
      不加 contain: paint —— 对 columnar 超长元素反而触发剪裁开销，让翻页变慢。
    */
        will-change: scroll-position;
    }

    /*
    全屏章节背景层：absolute 占满 frame，位于 viewport 之下、frame 之内。
    inline style 由 currentChapterBg 注入（来自 EPUB body inline style 的
    background-* 属性，已经过 url 改写为 asset:// 协议）。
    background-size: cover 让任意尺寸的背景图都铺满 frame，不留米色 paper
    主题色边带。pointer-events: none 不挡点击。
  */
    .rd-chapter-bg {
        position: absolute;
        inset: 0;
        background-size: cover;
        background-position: center;
        background-repeat: no-repeat;
        pointer-events: none;
        z-index: 0;
    }
    .rd-footnote-backdrop {
        position: fixed;
        inset: 0;
        z-index: 10000;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 24px;
        box-sizing: border-box;
        background: rgba(15, 23, 42, 0.76);
    }
    .rd-footnote-popup {
        position: relative;
        width: min(92vw, 760px);
        max-height: 88vh;
        overflow: auto;
        padding: 18px;
        box-sizing: border-box;
        border-radius: var(--radius-md);
        background: var(--rd-paper, #fffdf8);
        color: var(--rd-text, #222);
        box-shadow: var(--shadow-md);
    }
    .rd-footnote-popup :global(img) {
        display: block;
        max-width: 100%;
        max-height: calc(88vh - 48px);
        width: auto;
        height: auto;
        margin: 0 auto;
    }
    .rd-footnote-popup :global(figure) {
        margin: 0;
    }
    .rd-footnote-close {
        position: absolute;
        top: 8px;
        right: 8px;
        z-index: 1;
        width: 32px;
        height: 32px;
        border: 0;
        border-radius: 50%;
        background: rgba(15, 23, 42, 0.86);
        color: #fff;
        font-size: 22px;
        line-height: 32px;
        cursor: pointer;
    }
    .rd-app[data-page-mode="scroll"] .rd-viewport {
        overflow-y: auto;
        overflow-x: hidden;
        /* 滚动条样式 */
        scrollbar-width: thin;
        scrollbar-color: var(--rd-text-soft) transparent;
    }
    .rd-app[data-page-mode="scroll"] .rd-viewport::-webkit-scrollbar {
        width: 6px;
    }
    .rd-app[data-page-mode="scroll"] .rd-viewport::-webkit-scrollbar-thumb {
        background: var(--rd-text-soft);
        border-radius: var(--radius-xs);
    }

    .rd-canvas {
        min-width: 100%;
        min-height: 100%;
    }
    .rd-page {
        column-width: var(--page-w, 100%);
        column-gap: var(--col-gap, 0px);
        column-fill: auto;
        height: var(--page-h, 100%);
        font-size: var(--rd-fontsize, 19px);
        line-height: var(--rd-lineheight, 1.85);
        color: var(--rd-body-color, var(--rd-text));
        font-weight: var(--rd-body-weight, 500);
    }
    .rd-app[data-page-mode="scroll"] .rd-page {
        column-width: auto;
        column-count: 1;
        height: auto;
    }
    .rd-app[data-page-mode="scroll"] .rd-page :global(.epub-body) {
        /* Chromium can leave a chapter's wrapper clamped to one page-height
           inside a one-column multicol container. The content then overflows
           the box, so scroll anchors and the final chapter use stale heights. */
        display: block;
        height: auto;
        min-height: 0;
        max-height: none;
    }
    .rd-app[data-page-mode="scroll"] .rd-canvas {
        width: 100% !important;
        height: auto;
    }

    .rd-page :global(.rd-end-sentinel) {
        display: inline-block;
        width: 0;
        height: 0;
        overflow: hidden;
        pointer-events: none;
    }

    .rd-loading,
    .rd-error {
        position: absolute;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        color: var(--rd-text-soft);
        gap: 12px;
        pointer-events: none;
        z-index: 2;
    }
    .rd-error pre {
        max-width: 640px;
        background: rgba(0, 0, 0, 0.04);
        padding: 8px 12px;
        border-radius: var(--radius-xs);
        font-size: 12px;
        white-space: pre-wrap;
        word-break: break-all;
    }

    /* ===== 章节版式（@html 注入内容受 :global 影响） ===== */
    .rd-page :global(section.rd-chapter) {
        break-before: column;
        /*
      让 section 至少占满一页：column-fragmented 时一致显示所需空间。
      box-sizing 保证 padding 不破坏列宽。
      重要：EPUB CSS `body { background: white }` 经 BODY_REPLACE 也会落到
      .rd-chapter 上。这里强制 section background 透明，让阅读器主题色
      透出，同时避免章节背景在 CSS columns 的每列重复绘制。
      `background` 简写会把 image / color / size / repeat / position /
      attachment 一并重置为 initial（none / transparent / auto / repeat /
      0% 0% / scroll），代替原来零散的 image:none + attachment:scroll。
    */
        min-height: var(--page-h, 100%);
        box-sizing: border-box;
        background: transparent !important;
    }
    .rd-app[data-page-mode="scroll"] .rd-page :global(section.rd-chapter) {
        /* 滚动模式下章节按块布局，章间小间距即可 */
        break-before: auto;
        margin-bottom: 1.6em;
    }
    .rd-page :global(section.rd-chapter:first-child) {
        break-before: avoid;
    }
    /* 用户排版：段间距 */
    .rd-page :global(.epub-body) {
        font-family: var(--rd-user-font, inherit);
        color: var(--rd-body-color, var(--rd-text));
        font-weight: var(--rd-body-weight, 500);
        background-color: transparent !important;
    }
    /*
    section.rd-chapter 已 min-height: page-h。
    .epub-body 不再设 min-height，避免双层最小高度造成空白；它的
    background-color 透明化后，带 alpha 的章节装饰图会露出阅读器主题色。
  */
    .rd-page :global(.epub-body p) {
        margin-top: 0;
        margin-bottom: var(--rd-paragraph-spacing, 0.6em);
    }
    .rd-app[data-page-mode="paginated"] .rd-page :global(.epub-body *) {
        /*
      EPUB page-break hints are written for native EPUB engines. Inside our CSS
      columns they can create empty columns that look like blank pages, so the
      reader keeps chapter boundaries on section.rd-chapter and neutralizes
      forced breaks inside chapter content.
    */
        break-before: auto !important;
        page-break-before: auto !important;
        break-after: auto !important;
        page-break-after: auto !important;
    }
    /* EPUB 自带字体禁用：强制覆盖章节内所有元素 font-family */
    .rd-app.no-epub-fonts .rd-page :global(*) {
        font-family: var(--rd-user-font, inherit) !important;
    }

    /* 占位章节：尽量小，让首屏列布局极快完成 */
    .rd-page :global(section.rd-chapter.rd-pending) {
        min-height: 1.4em;
    }
    .rd-page :global(section.rd-chapter.rd-pending .rd-pending-title) {
        color: var(--rd-text-soft);
        font-size: 0.85em;
        opacity: 0.55;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    /*
    章节首段顶部清零：只清最外两层，不再像上版那样穿透 5 层。
    深层穿透会把用户 CSS 里特意留白的容器（如 .copyright 卡片 margin: 10%）
    也抹掉，反而破坏 epub 自带排版。两层已经足够盖住常见的 p/h1 第一子元素
    margin-top 造成的"顶部空一行"问题。
  */
    .rd-page :global(section.rd-chapter > .epub-body > *:first-child) {
        margin-top: 0 !important;
        padding-top: 0 !important;
    }
    .rd-page :global(img),
    .rd-page :global(svg),
    .rd-page :global(image) {
        max-width: var(--page-w, 100%) !important;
        max-height: var(--page-h, 100%) !important;
        object-fit: contain;
    }
    /*
    EPUBs such as 山河稷 mark their chapter artwork with .header_image and
    `page-break-before: always`. In the reader's CSS-column pagination that can
    isolate the artwork on its own page. Keep the EPUB's normal visual order,
    but neutralize the forced break and cap only this header-art block so the
    chapter title/text can continue below it.
  */
    .rd-page :global(.epub-body .header_image) {
        break-before: auto !important;
        page-break-before: auto !important;
        break-after: auto !important;
        page-break-after: auto !important;
        background-color: transparent !important;
        line-height: 0;
    }
    .rd-page :global(.epub-body .header_image img),
    .rd-page :global(.epub-body .header_image svg),
    .rd-page :global(.epub-body .header_image image) {
        display: block;
        width: 100% !important;
        height: min(42vh, calc(var(--page-h, 100vh) * 0.42)) !important;
        max-width: 100% !important;
        max-height: min(42vh, calc(var(--page-h, 100vh) * 0.42)) !important;
        object-fit: contain;
        object-position: top center;
        background-color: transparent !important;
    }
    .rd-page :global(.epub-body .header_image + .head),
    .rd-page :global(.epub-body .header_image + h1),
    .rd-page :global(.epub-body .header_image + h2),
    .rd-page :global(.epub-body .header_image + h3) {
        break-before: auto !important;
        page-break-before: auto !important;
        background-color: transparent !important;
    }
    .rd-page :global(.epub-body .logo) {
        background-color: transparent !important;
    }
    /*
    "章节头图"特殊版式：当章节 section 第一个可视子元素（含一层 wrapper）
    是图片时，让其上左右贴满整个 frame 窗口宽度 —— 视觉上像 LibraryPreview
    里的封面预览，而不是被 max-height/object-fit 缩成中部居中的小图。

    "贴窗口边"实现（v2，2026/05）：
      viewport 现在已被 JS 改为全屏占满 frame，左右用 padding=horizPad 留阅读边距。
      `--page-margin-h` 现在等于 horizPad（即 viewport 真正的内边距量），不是
      旧的 minSideMargin。所以：
        width  = 列宽 + 2*horizPad  → 等于 frame 全宽
        margin-left = -horizPad     → 图整体左移到 frame 左边
        margin-top  = -vertPad      → 图顶到 frame 上边
      负 margin 让图视觉位置落入 viewport 的 padding 区（不被 overflow:hidden
      剪），实现"贴窗口边"。
      `--page-margin-h/v` 由 applyBasicLayout 实时计算并 set 到 pageEl。
      user-select 关闭让它像封面那样不可选。
  */
    .rd-page :global(table) {
        max-width: var(--page-w, 100%);
    }

    /* ================================================================
     4 个角的常驻信息 overlay
     ================================================================ */
    .rd-corner {
        position: absolute;
        z-index: 10;
        color: var(--rd-text-soft);
        font-size: 12px;
        padding: 6px 12px;
        pointer-events: none;
        max-width: 50%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-variant-numeric: tabular-nums;
        letter-spacing: 0.02em;
        opacity: 0.85;
    }
    .rd-corner-tl {
        top: 8px;
        left: 12px;
        text-align: left;
    }
    .rd-corner-tr {
        top: 8px;
        right: 12px;
        text-align: right;
    }
    .rd-corner-bl {
        bottom: 8px;
        left: 12px;
        text-align: left;
    }
    .rd-corner-br {
        bottom: 8px;
        right: 12px;
        text-align: right;
    }

    /* ================================================================
     底部工具栏（5 tab 横条 + 上方面板）
     ================================================================ */
    .rd-tools {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        z-index: 20;
        display: flex;
        flex-direction: column;
        /*
      之前用半透明 + backdrop-filter blur(6px) 实现"磨砂"效果，但工具栏
      底下垫着整本书的 DOM（几千~几万节点），每帧浏览器都要对工具栏
      下方做高斯模糊 —— 唤起/点击都因此肉眼可见地卡。改用纯不透明
      背景：视觉上仍是浮层效果，但不再触发 backdrop-filter 重渲染，
      点击和动画都瞬时响应。
    */
        background: var(--rd-bg);
        color: var(--rd-text);
        border-top: 1px solid var(--rd-border);
        /*
      v3（2026/05）— 性能修：原 box-shadow `0 -4px 16px` 在 transform 动画
      帧里会把工具栏上方 16px 区域反复重绘，而那 16px 正盖在巨大的 .rd-page
      （几十万 px 宽）之上 —— 用户感受到的"工具栏巨卡"就是这块阴影模糊
      的代价。改成只在工具栏自身的 layer 内部加一条细线（border-top）+ 极小
      偏移阴影，画面变化区域不再扩散到外层 paint。
      contain: layout style — 把 toolbar 的 reflow / 样式失效隔离在自己的
      子树里，下方 .rd-page 不会被 toolbar 状态切换牵连重排。
    */
        box-shadow: 0 -1px 0 var(--rd-border);
        contain: layout style;
        transition: transform 0.15s ease-out;
        /* 让 transform 走 GPU 合成层，避免落入软件路径；显式 translate3d 启动 */
        will-change: transform;
        transform: translate3d(0, 100%, 0);
    }
    .rd-tools.open {
        transform: translate3d(0, 0, 0);
    }
    /* 工具栏未打开时仍要露出 5 tab 条供唤起？不 —— 用户要"唤起的工具栏"，
     即默认隐藏，点击中部唤起。所以默认完全隐藏到屏幕外。 */
    .rd-tools:not(.open) {
        pointer-events: none;
    }
    /* 但需要一个"中部点击唤起"的视觉提示？暂不加，保持极简。 */

    .rd-tabs {
        display: flex;
        border-top: 1px solid var(--rd-border);
    }
    .rd-tabs button {
        flex: 1;
        background: transparent;
        border: 0;
        color: var(--rd-text-soft);
        padding: 10px 0;
        cursor: pointer;
        font-size: 12px;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2px;
        transition:
            color 0.15s,
            background 0.15s;
    }
    .rd-tabs button:hover {
        color: var(--rd-accent);
        background: rgba(0, 0, 0, 0.03);
    }
    .rd-tabs button.active {
        color: var(--rd-accent);
    }
    .rd-tabs button.active::after {
        content: "";
        position: absolute;
        width: 24px;
        height: 2px;
        background: var(--rd-accent);
        border-radius: var(--radius-xs);
        margin-top: 36px;
    }
    .rd-tab-icon {
        font-size: 16px;
        line-height: 1;
    }
    .rd-tab-label {
        font-size: 11px;
    }

    /* 面板（每个 tab 唤起的内容区域） */
    .rd-panel {
        padding: 14px 18px 12px;
        max-height: 50vh;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 10px;
    }
    .rd-panel-title {
        font-size: 13px;
        color: var(--rd-text-soft);
        margin-bottom: 4px;
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .rd-set-row {
        display: flex;
        align-items: center;
        gap: 12px;
    }
    .rd-toggle-row {
        justify-content: space-between;
    }
    .rd-set-label {
        width: 80px;
        color: var(--rd-text-soft);
        font-size: 13px;
        flex-shrink: 0;
    }
    .rd-segctrl {
        flex: 1;
        display: flex;
        gap: 6px;
        align-items: center;
    }
    .rd-segctrl button {
        flex: 1;
        background: transparent;
        border: 1px solid var(--rd-border);
        color: var(--rd-text);
        padding: 6px 10px;
        border-radius: var(--radius-xs);
        cursor: pointer;
        font-size: 13px;
        transition:
            background 0.15s,
            color 0.15s,
            border-color 0.15s;
    }
    .rd-segctrl button:hover {
        background: rgba(0, 0, 0, 0.04);
    }
    .rd-segctrl button.active {
        background: var(--rd-accent);
        color: #fff;
        border-color: var(--rd-accent);
    }
    .rd-segctrl button:disabled {
        opacity: 0.4;
        cursor: not-allowed;
    }
    .rd-font-val {
        min-width: 48px;
        text-align: center;
        font-variant-numeric: tabular-nums;
    }
    .rd-font-picker :global(.custom-select) {
        flex: 1;
        min-width: 0;
    }
    .rd-font-picker :global(.custom-select-trigger) {
        background: var(--rd-bg-soft);
        color: var(--rd-text);
        border: 1px solid var(--rd-border);
        font-size: 13px;
    }

    .rd-row {
        display: flex;
        gap: 8px;
        align-items: center;
    }
    .rd-tip {
        color: var(--rd-text-soft);
        font-size: 12px;
    }
    .rd-secondary {
        background: var(--rd-bg-soft);
        border: 1px solid var(--rd-border);
        color: var(--rd-text);
        padding: 6px 12px;
        border-radius: var(--radius-xs);
        cursor: pointer;
        font-size: 13px;
        flex: 1;
    }
    .rd-secondary:hover {
        background: rgba(0, 0, 0, 0.06);
    }
    .rd-secondary:disabled {
        opacity: 0.45;
        cursor: not-allowed;
    }

    /* 目录（多级，由 ReaderTocNode 递归渲染；此处只管外层容器布局） */
    .rd-toc-list {
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    /* 主题色板 */
    .rd-theme-grid {
        display: grid;
        grid-template-columns: repeat(5, 1fr);
        gap: 8px;
    }
    .rd-theme-chip {
        height: 60px;
        border: 2px solid transparent;
        border-radius: var(--radius-sm);
        cursor: pointer;
        color: inherit;
        font-size: 12px;
        display: flex;
        align-items: flex-end;
        justify-content: center;
        padding-bottom: 6px;
        transition:
            border-color 0.15s,
            transform 0.15s;
    }
    .rd-theme-chip:hover {
        transform: translateY(-2px);
    }
    .rd-theme-chip.active {
        border-color: var(--rd-accent);
    }
    .rd-theme-paper {
        background: #e9e3d3;
        color: #443f37;
    }
    .rd-theme-eye {
        background: #cce8cf;
        color: #2f4633;
    }
    .rd-theme-sepia {
        background: #f4ecd8;
        color: #5c4a32;
    }
    .rd-theme-snow {
        background: #ffffff;
        color: #1f2937;
        border-color: #e5e7eb;
    }
    .rd-theme-dark {
        background: #1c1c1c;
        color: #d4d4d4;
    }

    /* 书签 */
    .rd-empty {
        color: var(--rd-text-soft);
        font-size: 13px;
        text-align: center;
        padding: 20px 0;
    }
    .rd-bookmarks-list {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }
    .rd-bookmark-row {
        display: flex;
        align-items: stretch;
        gap: 6px;
        border: 1px solid var(--rd-border);
        border-radius: var(--radius-xs);
        overflow: hidden;
        background: rgba(0, 0, 0, 0.02);
    }
    .rd-bookmark-main {
        flex: 1;
        background: transparent;
        border: 0;
        cursor: pointer;
        text-align: left;
        padding: 8px 10px;
        color: var(--rd-text);
        overflow: hidden;
    }
    .rd-bookmark-main:hover {
        background: rgba(0, 0, 0, 0.04);
    }
    .rd-bookmark-title {
        font-size: 13px;
        font-weight: 600;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .rd-bookmark-preview {
        font-size: 12px;
        color: var(--rd-text-soft);
        margin-top: 2px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .rd-bookmark-time {
        font-size: 11px;
        color: var(--rd-text-soft);
        opacity: 0.7;
        margin-top: 2px;
    }
    .rd-mini-btn {
        background: transparent;
        border: 1px solid var(--rd-border);
        color: var(--rd-text-soft);
        padding: 4px 10px;
        border-radius: var(--radius-xs);
        cursor: pointer;
        font-size: 12px;
    }
    .rd-mini-btn:hover {
        background: rgba(0, 0, 0, 0.04);
        color: var(--rd-accent);
    }
    .rd-mini-btn:disabled {
        opacity: 0.45;
        cursor: not-allowed;
    }
    .rd-mini-btn.active {
        color: var(--rd-accent);
        border-color: var(--rd-accent);
    }
    .rd-bookmark-del {
        border: 0;
        border-left: 1px solid var(--rd-border);
        border-radius: 0;
        padding: 0 14px;
        font-size: 16px;
    }

    /* 开关 */
    .rd-switch {
        position: relative;
        display: inline-block;
        width: 40px;
        height: 22px;
        flex-shrink: 0;
    }
    .rd-switch input {
        display: none;
    }
    .rd-switch-slider {
        position: absolute;
        inset: 0;
        background: var(--rd-border);
        border-radius: var(--radius-lg);
        transition: background 0.15s;
        cursor: pointer;
    }
    .rd-switch-slider::before {
        content: "";
        position: absolute;
        width: 18px;
        height: 18px;
        left: 2px;
        top: 2px;
        background: #fff;
        border-radius: 50%;
        box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
        transition: transform 0.15s;
    }
    .rd-switch input:checked + .rd-switch-slider {
        background: var(--rd-accent);
    }
    .rd-switch input:checked + .rd-switch-slider::before {
        transform: translateX(18px);
    }

    /* ================================================================
     手机
     ================================================================
     阅读器的分页/滚动本身是量 frameEl 的真实尺寸算的（applyBasicLayout），
     所以尺寸自适应不用改一行 JS：frame 一变窄，单列、行宽、每页高度都跟着
     重算。这里只补三件 CSS 解决不了就会出错的事：

     1. 安卓 edge-to-edge / iPhone 刘海：阅读器上面没有页头，章节名就在状态栏
        底下，必须补安全区；
     2. 底部工具栏的 5 个页签：桌面 10px 内边距 + 12px 字，总高约 44px 刚好
        压线，手机上再压一行就点不准了；
     3. 面板最大高度：50vh 在横屏手机上只有 190px，排版面板有七八行设置，
        必须让它自己能滚。
     ================================================================ */

    :global(:root[data-tepub-layout="phone"]) .rd-corner {
        /* 顶部两个角要压在状态栏下面，底部两个角在手机外壳里已经由底部导航
           让开了位置，不需要再加 safe-bottom（加了会双重留白） */
        font-size: 11px;
        padding: 4px 10px;
        max-width: 60%;
    }

    :global(:root[data-tepub-layout="phone"]) .rd-corner-tl,
    :global(:root[data-tepub-layout="phone"]) .rd-corner-tr {
        top: calc(6px + var(--safe-top, 0px));
    }

    :global(:root[data-tepub-layout="phone"]) .rd-tabs button {
        min-height: 52px;
        padding: 6px 0;
        font-size: 11px;
    }

    /* 激活下划线原来靠 margin-top:36px 硬顶，行高一变就会飘到文字中间 */
    :global(:root[data-tepub-layout="phone"]) .rd-tabs button.active::after {
        margin-top: 0;
        bottom: 4px;
    }

    :global(:root[data-tepub-layout="phone"]) .rd-tabs button {
        position: relative;
    }

    :global(:root[data-tepub-layout="phone"]) .rd-panel {
        max-height: 52vh;
        overflow: auto;
        overscroll-behavior: contain;
        -webkit-overflow-scrolling: touch;
    }

    /* 设置行里的 ± 按钮：桌面 28px 左右，手指按不准相邻的那个 */
    :global(:root[data-tepub-layout="phone"]) .rd-panel button,
    :global(:root[data-tepub-layout="phone"]) .rd-segctrl button {
        min-height: 40px;
        min-width: 40px;
    }

    :global(:root[data-tepub-layout="phone"]) .rd-set-label {
        min-width: 44px;
    }

    /* 目录/书签行：手机上要整行点得中，不能被两行小字挤成 30px。
       .toc-row 由 ReaderTocNode.svelte 渲染，带的是那个组件的作用域属性，
       所以整条包进 :global() 并锚在 .rd-toc-list 上。 */
    :global(
        :root[data-tepub-layout="phone"] .rd-toc-list .toc-row
    ),
    :global(:root[data-tepub-layout="phone"]) .rd-bookmark-main {
        min-height: 44px;
    }

    /* 脚注弹层：手机上一屏就那么高，24px 边距 + 88vh 会把关闭按钮顶出屏幕 */
    :global(:root[data-tepub-layout="phone"]) .rd-footnote-backdrop {
        padding: calc(12px + var(--safe-top, 0px)) 12px
            calc(12px + var(--safe-bottom, 0px));
    }

    :global(:root[data-tepub-layout="phone"]) .rd-footnote-popup {
        max-height: calc(100% - 8px);
    }

    :global(:root[data-tepub-layout="phone"]) .rd-footnote-close {
        width: var(--tap-target);
        height: var(--tap-target);
        line-height: var(--tap-target);
        font-size: 24px;
    }
</style>
