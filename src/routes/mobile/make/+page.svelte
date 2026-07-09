<script lang="ts">
    import { base } from "$app/paths";
    import { onDestroy, onMount, tick } from "svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import { isWebMobileClient } from "$lib/clientProfile";
    import { platform } from "$lib/platform";
    import {
        cacheBrowserFileStable,
        exportEpubPath,
        offerSystemExport,
        readMobileSelection,
        safeFileName,
        selectionName,
    } from "$lib/mobileFlow";
    import { DEFAULT_TOC_REGEX_RULES, loadAppSettings, saveAppSettings, type TocRegexRule } from "$lib/appSettings";
    import {
        EPUB_HEADER_STYLES,
        EPUB_TITLE_STYLES,
        type EpubStyleModule,
    } from "$lib/epubStyleLibrary";

    interface RegexRule {
        enabled: boolean;
        level: number;
        pattern: string;
    }

    interface RawChapter {
        title: string;
        line_number: number;
        level: number;
        is_meta: boolean;
        word_count: number;
    }

    interface TocItem extends RawChapter {
        id: string;
        depth: number;
        kind: "volume" | "chapter" | "meta";
        volumeKey: string;
        parentId?: string;
        hasChildren: boolean;
    }

    interface MobileMakeEpubResult {
        output_path: string;
        title: string;
        chapter_count: number;
        word_count: number;
    }

    interface CheckItem {
        id: string;
        title: string;
        line: number;
        msg: string;
    }

    interface ReorderPreviewRow {
        id: string;
        line: number;
        kind: "volume" | "chapter" | "meta";
        volumeKey: string;
        original: string;
        replacement: string;
        changed: boolean;
        included: boolean;
        sequenceBroken: boolean;
    }

    interface RenameTitleSheetState {
        open: boolean;
        item: TocItem | null;
        value: string;
    }

    interface ChapterEditSheetState {
        open: boolean;
        item: TocItem | null;
        value: string;
        startLine: number;
        endLine: number;
    }

    type ReorderScope = "all" | "volumes" | "chapters" | "regex";
    type NumberStyle = "arabic" | "chinese";
    type WebMakeStep = "edit" | "style";
    type WebImageSlot = "cover" | "fullCover" | "banner" | "header";
    type WebImageAsset = {
        fileName: string;
        bytes: Uint8Array;
        mime: string;
        objectUrl: string;
    };

    const ruleLevelOptions = [
        { value: "1", label: "卷" },
        { value: "3", label: "章" },
    ];

    const reorderScopeOptions = [
        { value: "all", label: "卷和章" },
        { value: "chapters", label: "仅章节" },
        { value: "volumes", label: "仅卷部" },
        { value: "regex", label: "手动正则" },
    ];

    const numberStyleOptions = [
        { value: "chinese", label: "一二三四" },
        { value: "arabic", label: "1234" },
    ];

    function chevronLabel(open: boolean) {
        return open ? "收起" : "展开";
    }

    const DEFAULT_META_VOLUME_REGEX = "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$";
    const DEFAULT_META_BODY_REGEX = "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$";
    const DEFAULT_VOLUME_REGEX =
        "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*";
    const DEFAULT_CHAPTER_REGEX =
        "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*";
    const WEB_EPUB_FONT_CSS = `@charset "utf-8";`;
    const EPUB_STYLE_LIBRARY_STORAGE_KEY = "tepub-epub-style-library-v1";
    const DEFAULT_WEB_TITLE_STYLE_ID = "title-cinematic-slab";
    const DEFAULT_FULL_COVER_WIDTH = 1400;
    const DEFAULT_FULL_COVER_HEIGHT = 2400;
    const WEB_STYLE_PREVIEW_WIDTH = 360;
    const WEB_STYLE_PREVIEW_HEIGHT = 700;
    const DEFAULT_WEB_HEADER_CSS = `.te-header-figure {
  margin: 0 0 1.35em;
  padding: 0;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`;
    const WEB_EPUB_MAIN_CSS = `@charset "utf-8";

@import url("font.css");

body.te-book-body,
body.te-chapter-page,
body.te-volume-page,
body.te-intro-page {
    padding: 0;
    margin-top: 0;
    margin-bottom: 0;
    margin-left: 1%;
    margin-right: 1%;
    line-height: 130%;
    text-align: justify;
    font-family: "Maintext", "DK-SONGTI", "SimSun", serif;
}

p.te-paragraph {
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "DK-SONGTI", "SimSun", serif;
}

.te-intro-title {
    font-family: "Title", "Microsoft YaHei", sans-serif;
    font-size: 125%;
    color: #00008B;
    margin: 0.3em 0 0.5em 0;
    text-align: left;
    text-indent: 0;
}

.te-intro-title span {
    background-color: transparent;
    padding: 0.4em 2em 0.2em 0.4em;
}

.te-volume-title {
    font-family: "Title", "Microsoft YaHei", serif;
    font-size: 1.2em;
    color: #59bde6;
    font-weight: 600;
    margin: 2em 0 1em 0;
    text-align: center;
    text-indent: 0;
    line-height: 130%;
}

.te-volume-subtitle {
    font-family: "Title", "Microsoft YaHei", serif;
    font-size: 1.2em;
    color: #59bde6;
    margin: 0 0 1em 0;
    text-indent: 0;
    text-align: center;
    line-height: 110%;
}

body.te-volume-page {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: bottom center;
    padding: 1% 1% 5%;
}

body.te-volume-page.te-volume-page--no-image .te-volume-title {
    margin-top: 12.2em;
}

body.te-volume-page.te-volume-page--no-image .te-volume-subtitle {
    margin-bottom: 1.8em;
}

.te-chapter-title {
    font-family: "Microsoft YaHei", sans-serif;
    text-align: center;
    margin: 2em 0 3em 0;
    font-size: 1.2em;
    font-weight: 900;
    color: #c2181e;
}

body.te-chapter-page.te-chapter-page--no-image .te-chapter-title {
    margin-top: 6.5em;
}

.te-chapter-number {
    font-family: "Microsoft YaHei", sans-serif;
    font-weight: 900;
    font-size: 0.8em;
    color: #413245;
    line-height: 130%;
    padding: 0;
    text-align: center;
    background-color: transparent;
}

.te-chapter-name {
    color: #c2181e;
}

body.te-cover-page,
body.te-banner-page {
    margin: 0;
    padding: 0;
    text-align: center;
    background: #ffffff;
}

.te-cover-page-wrap {
    min-height: 98vh;
    display: block;
    line-height: 0;
    text-align: center;
}

.te-cover-page-wrap img {
    width: 100%;
    height: auto;
    max-height: 98vh;
    object-fit: cover;
}

.te-banner-page-wrap {
    padding: 7vh 5% 5vh;
    text-align: center;
    line-height: 0;
}

.te-banner-page-wrap img {
    width: 100%;
    max-width: 100%;
    height: auto;
}

p.te-divider-line {
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0;
    margin: 1em 0;
}`;
    function normalizeMakeRules(list: Array<Partial<TocRegexRule>> | undefined): RegexRule[] {
        const source = Array.isArray(list) && list.length ? list : DEFAULT_TOC_REGEX_RULES;
        return source.map((rule) => ({
            enabled: typeof rule.enabled === "boolean" ? rule.enabled : true,
            level: Number(rule.level) <= 1 ? 1 : 3,
            pattern: String(rule.pattern || "").trim(),
        }));
    }

    function loadMakeRules() {
        return normalizeMakeRules(loadAppSettings().customRegexRules);
    }

    function normalizeSavedEpubStyle(style: EpubStyleModule) {
        if (style.kind === "header") {
            return { ...style } satisfies EpubStyleModule;
        }
        const legacyNumberCss = style.titleCssA || "";
        const legacyNameCss = style.titleCssB || style.titleCssA || "";
        return {
            ...style,
            titleLayout: style.titleLayout || (style.titleNameCss || style.titleCssB ? "split" : "single"),
            titleNumberCss: style.titleNumberCss || legacyNumberCss,
            titleNameCss: style.titleNameCss || legacyNameCss,
        } satisfies EpubStyleModule;
    }

    function loadSavedEpubStyles() {
        if (typeof localStorage === "undefined") return [];
        try {
            const raw = localStorage.getItem(EPUB_STYLE_LIBRARY_STORAGE_KEY);
            if (!raw) return [];
            const parsed = JSON.parse(raw);
            return Array.isArray(parsed?.styles)
                ? parsed.styles
                    .filter((style: Partial<EpubStyleModule>) => style && style.id && style.kind && style.name && style.css)
                    .map(normalizeSavedEpubStyle)
                : [];
        } catch (error) {
            console.warn("读取 EPUB 样式库失败", error);
            return [];
        }
    }

    let rules: RegexRule[] = loadMakeRules();

    let fileInputEl: HTMLInputElement | null = null;
    let coverInputEl: HTMLInputElement | null = null;
    let fullCoverInputEl: HTMLInputElement | null = null;
    let bannerInputEl: HTMLInputElement | null = null;
    let headerInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let content = "";
    let title = "";
    let author = "";
    let coverPath = "";
    let coverName = "";
    let coverPreviewUrl = "";
    let coverAsset: WebImageAsset | null = null;
    let autoFullCoverAsset: WebImageAsset | null = null;
    let fullCoverAsset: WebImageAsset | null = null;
    let bannerAsset: WebImageAsset | null = null;
    let headerAsset: WebImageAsset | null = null;
    let processedHeaderAsset: WebImageAsset | null = null;
    let processedHeaderKey = "";
    let fullCoverManuallySelected = false;
    let uuid = crypto.randomUUID?.() ?? "";
    let uuidAuto = true;
    let desktopMode = false;
    let backHref = platform.isWeb ? withBasePath("/") : withBasePath("/mobile");
    let status = "选择 TXT、MD 或 HTML 文件后预览目录。";
    let busy = false;
    let chapters: RawChapter[] = [];
    let expandedIds = new Set<string>();
    let makeResult: MobileMakeEpubResult | null = null;
    let webEpubBytes: Uint8Array | null = null;
    let exportPath = "";
    let sequenceErrors: CheckItem[] = [];
    let titleErrors: CheckItem[] = [];
    let invalidSequenceIds = new Set<string>();
    let reorderPreviewRows: ReorderPreviewRow[] = [];
    let reorderScope: ReorderScope = "all";
    let reorderRegex = "";
    let reorderPerVolume = false;
    let volumeNumberStyle: NumberStyle = "chinese";
    let chapterNumberStyle: NumberStyle = "arabic";
    let reorderCollapsedVolumeKeys = new Set<string>();
    let metaOpen = true;
    let regexOpen = true;
    let tocOpen = true;
    let checkOpen = true;
    let reorderOpen = false;
    let activeTocId = "";
    let webMakeStep: WebMakeStep = "edit";
    let savedEpubStyles: EpubStyleModule[] = [];
    let selectedHeaderStyleId = "";
    let selectedTitleStyleId = DEFAULT_WEB_TITLE_STYLE_ID;
    let suppressRuleRefresh = false;
    let tocActionTarget: TocItem | null = null;
    let renameTitleSheet: RenameTitleSheetState = {
        open: false,
        item: null,
        value: "",
    };
    let chapterEditSheet: ChapterEditSheetState = {
        open: false,
        item: null,
        value: "",
        startLine: 0,
        endLine: 0,
    };

    $: tocItems = buildToc(chapters);
    $: visibleToc = tocItems.filter((item) => item.kind !== "chapter" || !item.volumeKey || expandedIds.has(item.volumeKey));
    $: visibleReorderRows = reorderPreviewRows.filter(
        (row) => row.kind !== "chapter" || !reorderCollapsedVolumeKeys.has(row.volumeKey),
    );
    $: webHeaderStyles = [...EPUB_HEADER_STYLES, ...savedEpubStyles.filter((style) => style.kind === "header")];
    $: webTitleStyles = [...EPUB_TITLE_STYLES, ...savedEpubStyles.filter((style) => style.kind === "title" && style.target === "chapter-title")];
    $: selectedHeaderStyle = webHeaderStyles.find((style) => style.id === selectedHeaderStyleId) || null;
    $: selectedTitleStyle = webTitleStyles.find((style) => style.id === selectedTitleStyleId)
        || webTitleStyles.find((style) => style.id === DEFAULT_WEB_TITLE_STYLE_ID)
        || webTitleStyles[0]
        || null;
    $: styleHeaderOptions = [
        { value: "", label: "不使用头图样式", meta: "仅应用标题排版" },
        ...webHeaderStyles.map((style) => ({
            value: style.id,
            label: style.name,
            meta: style.sourceKind === "saved" ? "自定义" : "内置",
        })),
    ];
    $: styleTitleOptions = webTitleStyles.map((style) => ({
        value: style.id,
        label: style.name,
        meta: style.sourceKind === "saved" ? "自定义" : "内置",
    }));
    $: desktopStyleWorkflowEnabled = platform.isTauri && desktopMode;
    $: desktopStylePageActive = desktopStyleWorkflowEnabled && webMakeStep === "style";
    $: activeFullCoverAsset = fullCoverAsset || autoFullCoverAsset || coverAsset;
    $: headerProcessKey = [
        headerAsset?.objectUrl || "",
        selectedHeaderStyle?.id || "",
        selectedHeaderStyle?.sampleWidth || "",
        selectedHeaderStyle?.sampleHeight || "",
    ].join("|");
    $: if (platform.isWeb || desktopStyleWorkflowEnabled) {
        void refreshProcessedHeaderFromSelection(headerProcessKey);
    }
    $: activeHeaderPreviewSrc = processedHeaderAsset?.objectUrl || selectedHeaderStyle?.sampleDataUrl || "";
    $: webStylePreviewDoc = buildWebStylePreviewDoc(
        selectedHeaderStyle?.id || "",
        selectedHeaderStyle?.css || "",
        activeHeaderPreviewSrc,
        selectedTitleStyle?.id || "",
        selectedTitleStyle?.css || "",
        resolveSelectedTitleLayout(),
    );
    $: if (tocItems.length) {
        reorderPreviewRows = buildReorderPreviewRows(tocItems);
    }

    function resetResult() {
        makeResult = null;
        webEpubBytes = null;
        exportPath = "";
    }

    function textBaseName(name: string) {
        return selectionName(name).replace(/\.[^.]+$/, "").trim();
    }

    function splitInlineAuthor(value: string) {
        const normalized = value.replace(/\s+/g, " ").trim();
        const match = normalized.match(/^(.*?)\s*(?:作者|Author|By)\s*[:：~]\s*(.+)$/i);
        if (!match) return null;
        const foundTitle = match[1].replace(/^书名\s*[:：]\s*/i, "").trim();
        const foundAuthor = match[2].replace(/[，,；;].*$/, "").trim();
        if (!foundTitle || !foundAuthor) return null;
        return { title: foundTitle, author: foundAuthor };
    }

    function detectTextMetadata(text: string, name: string) {
        const fallbackTitle = textBaseName(name) || "未命名";
        let detectedTitle = fallbackTitle;
        let detectedAuthor = "";

        const fromName = splitInlineAuthor(fallbackTitle);
        if (fromName) {
            detectedTitle = fromName.title;
            detectedAuthor = fromName.author;
        }

        const lines = text
            .replace(/\r\n|\r|\u2028|\u2029/g, "\n")
            .split("\n")
            .map((line) => line.trim())
            .filter(Boolean)
            .slice(0, 8);
        const firstLine = lines[0] || "";
        const secondLine = lines[1] || "";

        const firstInline = splitInlineAuthor(firstLine);
        if (firstInline) {
            detectedTitle = firstInline.title;
            detectedAuthor = firstInline.author;
        } else {
            const bracketMatch = firstLine.match(/《([^》]+)》/);
            const explicitTitle = firstLine.match(/^(?:书名|小说名|Title)\s*[:：]\s*(.+)$/i);
            if (bracketMatch?.[1]) {
                detectedTitle = bracketMatch[1].trim();
            } else if (explicitTitle?.[1]) {
                const titleWithAuthor = splitInlineAuthor(explicitTitle[1]);
                detectedTitle = titleWithAuthor?.title || explicitTitle[1].trim();
                if (titleWithAuthor?.author) detectedAuthor = titleWithAuthor.author;
            } else if (/^(?:作者|Author|By)\s*[:：~]/i.test(secondLine)) {
                detectedTitle = firstLine;
            }
        }

        const authorSource = lines.join("\n");
        const authorMatch = authorSource.match(/(?:^|\n)\s*(?:作者|Author|By)\s*[:：~]\s*([^\n\r]+)/i);
        if (authorMatch?.[1]) {
            detectedAuthor = authorMatch[1].replace(/[，,；;].*$/, "").trim();
        }

        const titleInline = splitInlineAuthor(detectedTitle);
        if (titleInline) {
            detectedTitle = titleInline.title;
            detectedAuthor = detectedAuthor || titleInline.author;
        }

        return {
            title: detectedTitle || fallbackTitle,
            author: detectedAuthor,
        };
    }

    function applyDetectedMetadata(text: string, name: string) {
        const meta = detectTextMetadata(text, name);
        title = meta.title;
        author = meta.author;
    }

    function openPicker() {
        fileInputEl?.click();
    }

    function openCoverPicker() {
        coverInputEl?.click();
    }

    function openFullCoverPicker() {
        fullCoverInputEl?.click();
    }

    function openBannerPicker() {
        bannerInputEl?.click();
    }

    function openHeaderPicker() {
        headerInputEl?.click();
    }

    function detectImageMime(file: File) {
        return file.type && file.type.startsWith("image/") ? file.type : "image/jpeg";
    }

    function imageExtensionFromMime(mime: string, fileName = "") {
        const ext = fileName.match(/\.([a-z0-9]+)$/i)?.[1]?.toLowerCase();
        if (ext && ["jpg", "jpeg", "png", "webp", "gif"].includes(ext)) return ext === "jpeg" ? "jpg" : ext;
        if (mime.includes("png")) return "png";
        if (mime.includes("webp")) return "webp";
        if (mime.includes("gif")) return "gif";
        return "jpg";
    }

    function mediaTypeForExtension(ext: string) {
        if (ext === "png") return "image/png";
        if (ext === "webp") return "image/webp";
        if (ext === "gif") return "image/gif";
        return "image/jpeg";
    }

    function revokeWebImageAsset(asset: WebImageAsset | null) {
        if (asset?.objectUrl) URL.revokeObjectURL(asset.objectUrl);
    }

    async function readWebImageAsset(file: File): Promise<WebImageAsset> {
        return {
            fileName: file.name,
            bytes: new Uint8Array(await file.arrayBuffer()),
            mime: detectImageMime(file),
            objectUrl: URL.createObjectURL(file),
        };
    }

    function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
        ctx.beginPath();
        ctx.moveTo(x + radius, y);
        ctx.lineTo(x + width - radius, y);
        ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
        ctx.lineTo(x + width, y + height - radius);
        ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
        ctx.lineTo(x + radius, y + height);
        ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
        ctx.lineTo(x, y + radius);
        ctx.quadraticCurveTo(x, y, x + radius, y);
        ctx.closePath();
    }

    async function loadImageElement(src: string) {
        const img = new Image();
        img.decoding = "async";
        img.src = src;
        if (img.decode) {
            await img.decode();
        } else {
            await new Promise<void>((resolve, reject) => {
                img.onload = () => resolve();
                img.onerror = () => reject(new Error("image load failed"));
            });
        }
        return img;
    }

    function canvasToBlob(canvas: HTMLCanvasElement, mime: string, quality: number) {
        return new Promise<Blob>((resolve, reject) => {
            canvas.toBlob((blob) => {
                if (blob) resolve(blob);
                else reject(new Error("canvas export failed"));
            }, mime, quality);
        });
    }

    async function buildDefaultFullCoverFromAsset(source: WebImageAsset): Promise<WebImageAsset> {
        const img = await loadImageElement(source.objectUrl);
        const width = DEFAULT_FULL_COVER_WIDTH;
        const height = DEFAULT_FULL_COVER_HEIGHT;
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("canvas unavailable");

        canvas.width = width;
        canvas.height = height;
        ctx.clearRect(0, 0, width, height);

        const bgScale = Math.max(width / img.width, height / img.height);
        const bgW = img.width * bgScale;
        const bgH = img.height * bgScale;
        const bgX = (width - bgW) / 2;
        const bgY = (height - bgH) / 2;
        const off = document.createElement("canvas");
        off.width = Math.ceil(bgW);
        off.height = Math.ceil(bgH);
        const offCtx = off.getContext("2d");
        if (offCtx) {
            offCtx.filter = `blur(${10 * 2 * (width / 1400)}px)`;
            offCtx.drawImage(img, 0, 0, bgW, bgH);
            ctx.drawImage(off, bgX, bgY);
        }

        const foregroundSize = 80;
        const radius = 10;
        const fgScale = Math.min((width * (foregroundSize / 100)) / img.width, (height * 0.8) / img.height);
        const fgW = img.width * fgScale;
        const fgH = img.height * fgScale;
        const fgX = (width - fgW) / 2;
        const fgY = (height - fgH) / 2;
        const scaledRadius = radius * 5 * (width / 1400);

        ctx.save();
        roundedRect(ctx, fgX, fgY, fgW, fgH, scaledRadius);
        ctx.clip();
        ctx.drawImage(img, fgX, fgY, fgW, fgH);
        ctx.restore();

        const blob = await canvasToBlob(canvas, "image/jpeg", 0.92);
        const stem = source.fileName.replace(/\.[^.]+$/, "") || "cover";
        return {
            fileName: `${stem}-full-cover.jpg`,
            bytes: new Uint8Array(await blob.arrayBuffer()),
            mime: "image/jpeg",
            objectUrl: URL.createObjectURL(blob),
        };
    }

    async function buildProcessedHeaderFromAsset(source: WebImageAsset, style: EpubStyleModule | null): Promise<WebImageAsset> {
        const img = await loadImageElement(source.objectUrl);
        const width = Math.max(320, Math.round(style?.sampleWidth || style?.originalSampleWidth || 1920));
        const height = Math.max(180, Math.round(style?.sampleHeight || style?.originalSampleHeight || 1080));
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("canvas unavailable");

        canvas.width = width;
        canvas.height = height;
        ctx.clearRect(0, 0, width, height);

        const scale = Math.max(width / img.width, height / img.height);
        const drawW = img.width * scale;
        const drawH = img.height * scale;
        const drawX = (width - drawW) / 2;
        const drawY = (height - drawH) / 2;
        ctx.drawImage(img, drawX, drawY, drawW, drawH);

        const maskSrc = style?.templateDataUrl || style?.sampleDataUrl || "";
        if (maskSrc) {
            try {
                const mask = await loadImageElement(maskSrc);
                const maskCanvas = document.createElement("canvas");
                const maskCtx = maskCanvas.getContext("2d");
                if (maskCtx) {
                    maskCanvas.width = width;
                    maskCanvas.height = height;
                    maskCtx.clearRect(0, 0, width, height);
                    maskCtx.drawImage(mask, 0, 0, width, height);
                    const data = maskCtx.getImageData(0, 0, width, height).data;
                    let hasTransparentMask = false;
                    for (let i = 3; i < data.length; i += 4) {
                        if (data[i] < 250) {
                            hasTransparentMask = true;
                            break;
                        }
                    }
                    if (hasTransparentMask) {
                        ctx.save();
                        ctx.globalCompositeOperation = "destination-in";
                        ctx.drawImage(maskCanvas, 0, 0);
                        ctx.restore();
                    }
                }
            } catch (error) {
                console.warn("应用章节头图样板蒙版失败", error);
            }
        }

        const blob = await canvasToBlob(canvas, "image/png", 1);
        const stem = source.fileName.replace(/\.[^.]+$/, "") || "chapter-header";
        return {
            fileName: `${stem}-chapter-header.png`,
            bytes: new Uint8Array(await blob.arrayBuffer()),
            mime: "image/png",
            objectUrl: URL.createObjectURL(blob),
        };
    }

    async function refreshProcessedHeaderFromSelection(key: string) {
        if (!headerAsset) {
            if (processedHeaderAsset) {
                revokeWebImageAsset(processedHeaderAsset);
                processedHeaderAsset = null;
            }
            processedHeaderKey = "";
            return;
        }
        if (key && key === processedHeaderKey && processedHeaderAsset) return;
        const activeKey = key;
        try {
            const generated = await buildProcessedHeaderFromAsset(headerAsset, selectedHeaderStyle);
            if (activeKey !== headerProcessKey) {
                revokeWebImageAsset(generated);
                return;
            }
            revokeWebImageAsset(processedHeaderAsset);
            processedHeaderAsset = generated;
            processedHeaderKey = activeKey;
            resetResult();
        } catch (error) {
            console.warn("自动处理章节头图失败", error);
        }
    }

    async function refreshAutoFullCoverFromCover() {
        if ((!platform.isWeb && !desktopStyleWorkflowEnabled) || !coverAsset || fullCoverManuallySelected) return;
        const previous = autoFullCoverAsset;
        try {
            const generated = await buildDefaultFullCoverFromAsset(coverAsset);
            revokeWebImageAsset(previous);
            autoFullCoverAsset = generated;
        } catch (error) {
            console.warn("自动生成全屏封面失败", error);
            revokeWebImageAsset(previous);
            autoFullCoverAsset = null;
        }
    }

    async function setWebImageSlot(slot: WebImageSlot, file: File) {
        const asset = await readWebImageAsset(file);
        if (slot === "cover") {
            const previousAsset = coverAsset;
            const previousPreviewUrl = coverPreviewUrl;
            revokeWebImageAsset(previousAsset);
            if (previousPreviewUrl && previousPreviewUrl !== previousAsset?.objectUrl) {
                URL.revokeObjectURL(previousPreviewUrl);
            }
            coverAsset = asset;
            coverPath = platform.isWeb ? `web-local:${file.name}` : coverPath;
            coverName = file.name;
            coverPreviewUrl = asset.objectUrl;
            await refreshAutoFullCoverFromCover();
        } else if (slot === "fullCover") {
            revokeWebImageAsset(fullCoverAsset);
            fullCoverAsset = asset;
            fullCoverManuallySelected = true;
            revokeWebImageAsset(autoFullCoverAsset);
            autoFullCoverAsset = null;
        } else if (slot === "banner") {
            revokeWebImageAsset(bannerAsset);
            bannerAsset = asset;
        } else {
            revokeWebImageAsset(headerAsset);
            revokeWebImageAsset(processedHeaderAsset);
            headerAsset = asset;
            processedHeaderAsset = null;
            processedHeaderKey = "";
        }
        resetResult();
    }

    function toggleCheckPanel() {
        checkOpen = !checkOpen;
        if (!checkOpen) reorderOpen = true;
    }

    function toggleReorderPanel() {
        reorderOpen = !reorderOpen;
        if (!reorderOpen && !checkOpen) checkOpen = true;
    }

    async function loadSource(sourcePath: string, name = "") {
        try {
            busy = true;
            resetResult();
            selectedName = name || selectionName(sourcePath);
            selectedPath = sourcePath;
            if (!platform.isWeb) {
                content = await platform.invoke<string>("read_text_file", { path: selectedPath });
            }
            applyDetectedMetadata(content, selectedName);
            if (uuidAuto) uuid = crypto.randomUUID?.() ?? uuid;
            status = `已导入 ${selectedName}，正在扫描目录。`;
            await previewToc();
        } catch (err) {
            status = "导入文本失败";
            await platform.message(`导入文本失败：${err}`, { title: "制作 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function onFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        try {
            if (platform.isWeb) {
                busy = true;
                resetResult();
                selectedName = file.name;
                selectedPath = `web-local:${file.name}`;
                content = await withFileText(file);
                applyDetectedMetadata(content, file.name);
                if (uuidAuto) uuid = crypto.randomUUID?.() ?? uuid;
                status = `已导入 ${selectedName}，正在扫描目录。`;
                await previewToc();
                return;
            }
            const cachedPath = await cacheBrowserFileStable(file, "txt");
            await loadSource(cachedPath, file.name);
        } catch (err) {
            status = "导入文本失败";
            await platform.message(`导入文本失败：${err}`, { title: "制作 EPUB", kind: "error" });
        } finally {
            if (platform.isWeb) busy = false;
        }
    }

    async function onCoverChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        try {
            if (platform.isWeb) {
                await setWebImageSlot("cover", file);
            } else {
                const cachedCoverPath = await cacheBrowserFileStable(file, "cover");
                await setWebImageSlot("cover", file);
                coverPath = cachedCoverPath;
                resetResult();
            }
        } catch (err) {
            await platform.message(`封面导入失败：${err}`, { title: "制作 EPUB", kind: "error" });
        }
    }

    async function onFullCoverChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;
        try {
            await setWebImageSlot("fullCover", file);
        } catch (err) {
            await platform.message(`全屏封面导入失败：${err}`, { title: "制作 EPUB", kind: "error" });
        }
    }

    async function onBannerChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;
        try {
            await setWebImageSlot("banner", file);
        } catch (err) {
            await platform.message(`阅微横幅导入失败：${err}`, { title: "制作 EPUB", kind: "error" });
        }
    }

    async function onHeaderChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;
        try {
            await setWebImageSlot("header", file);
        } catch (err) {
            await platform.message(`头图导入失败：${err}`, { title: "制作 EPUB", kind: "error" });
        }
    }

    async function previewToc() {
        if (!content.trim()) return;
        resetResult();
        try {
            busy = true;
            const activeRules = rules.filter((rule) => rule.enabled !== false && rule.pattern.trim());
            chapters = platform.isWeb
                ? scanChaptersInBrowser(content, activeRules)
                : await platform.invoke<RawChapter[]>("mobile_scan_chapters", { content, rules: activeRules });
            chapters = chapters.map((chapter) => ({ ...chapter, title: normalizeCatalogTitle(chapter.title) }));
            expandedIds = new Set(
                buildToc(chapters)
                    .filter((item) => itemKind(item.title, item.level, item.is_meta) === "volume" && item.hasChildren)
                    .map((item) => item.id),
            );
            reorderCollapsedVolumeKeys = new Set();
            status = chapters.length
                ? `已识别 ${chapters.length} 个目录项，可展开预览或调整正则后重扫。`
                : "没有识别到目录，将按单章正文生成。";
            runTocCheck(buildToc(chapters));
        } catch (err) {
            status = "目录扫描失败";
            await platform.message(`目录扫描失败：${err}`, { title: "制作 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function prepareStyleStep() {
        savedEpubStyles = loadSavedEpubStyles();
        if (!webTitleStyles.some((style) => style.id === selectedTitleStyleId)) {
            selectedTitleStyleId = webTitleStyles.find((style) => style.id === DEFAULT_WEB_TITLE_STYLE_ID)?.id || webTitleStyles[0]?.id || "";
        }
        if (coverAsset && !fullCoverManuallySelected && !autoFullCoverAsset) {
            await refreshAutoFullCoverFromCover();
        }
        webMakeStep = "style";
        status = "";
    }

    async function enterWebStyleStep() {
        if (!platform.isWeb) return false;
        await prepareStyleStep();
        return true;
    }

    async function enterDesktopStyleStep() {
        if (!desktopStyleWorkflowEnabled) return false;
        await prepareStyleStep();
        return true;
    }

    function backToWebEditStep() {
        webMakeStep = "edit";
        status = "已返回目录与元数据编辑。";
    }

    async function exportDesktopStyledEpubBytes(fileName: string, bytes: Uint8Array) {
        const outputName = safeFileName(fileName, "epub");
        const selected = await platform.saveDialog({
            defaultPath: outputName,
            filters: [{ name: "EPUB", extensions: ["epub"] }],
        });
        if (!selected) {
            return { message: "已取消导出 EPUB。", path: "" };
        }
        await platform.writeFile(selected, bytes);
        return { message: `已导出 EPUB：${selected}`, path: selected };
    }

    function onWebHeaderStyleChange(styleId: string) {
        selectedHeaderStyleId = styleId;
        const nextHeader = webHeaderStyles.find((style) => style.id === styleId);
        if (nextHeader?.boundTitleStyleId && webTitleStyles.some((style) => style.id === nextHeader.boundTitleStyleId)) {
            selectedTitleStyleId = nextHeader.boundTitleStyleId;
        }
        resetResult();
    }

    function onWebTitleStyleChange(styleId: string) {
        selectedTitleStyleId = styleId;
        resetResult();
    }

    async function makeEpub() {
        if (!selectedPath || !content.trim()) return;
        if (platform.isWeb && webMakeStep !== "style") {
            await enterWebStyleStep();
            return;
        }
        if (desktopStyleWorkflowEnabled && webMakeStep !== "style") {
            await enterDesktopStyleStep();
            return;
        }
        try {
            busy = true;
            if (platform.isWeb) {
                const outputTitle = title.trim() || textBaseName(selectedName) || "book";
                webEpubBytes = await buildSimpleEpubBytes(outputTitle, author.trim(), content, chapters);
                const fileName = safeFileName(outputTitle, "epub");
                makeResult = {
                    output_path: `web-local:${fileName}`,
                    title: outputTitle,
                    chapter_count: Math.max(1, chapters.length),
                    word_count: countWords(content),
                };
                status = await offerSystemExport("", fileName, webEpubBytes);
                exportPath = fileName;
                return;
            }
            if (desktopStyleWorkflowEnabled && webMakeStep === "style") {
                const outputTitle = title.trim() || textBaseName(selectedName) || "book";
                webEpubBytes = await buildSimpleEpubBytes(outputTitle, author.trim(), content, chapters);
                const fileName = safeFileName(outputTitle, "epub");
                makeResult = {
                    output_path: `desktop-local:${fileName}`,
                    title: outputTitle,
                    chapter_count: Math.max(1, chapters.length),
                    word_count: countWords(content),
                };
                const exported = await exportDesktopStyledEpubBytes(fileName, webEpubBytes);
                status = exported.message;
                exportPath = exported.path || "";
                if (exported.path) {
                    makeResult = { ...makeResult, output_path: exported.path };
                }
                return;
            }
            makeResult = await platform.invoke<MobileMakeEpubResult>("mobile_make_epub", {
                sourcePath: selectedPath,
                title: title.trim() || textBaseName(selectedName),
                author: author.trim(),
                coverPath,
                uuid: uuid.trim(),
                rules,
            });
            status = `已生成《${makeResult.title}》，${makeResult.chapter_count} 个目录项，约 ${makeResult.word_count} 字。`;
        } catch (err) {
            status = "制作 EPUB 失败";
            await platform.message(`制作 EPUB 失败：${err}`, { title: "制作 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function exportMadeEpub() {
        if (!makeResult) return;
        try {
            busy = true;
            if (platform.isWeb && webEpubBytes) {
                const fileName = safeFileName(makeResult.title, "epub");
                status = await offerSystemExport("", fileName, webEpubBytes);
                exportPath = fileName;
                return;
            }
            if (desktopStyleWorkflowEnabled && webEpubBytes) {
                const fileName = safeFileName(makeResult.title, "epub");
                const exported = await exportDesktopStyledEpubBytes(fileName, webEpubBytes);
                status = exported.message;
                exportPath = exported.path || "";
                return;
            }
            const result = await exportEpubPath(makeResult.output_path, safeFileName(makeResult.title, "epub"));
            exportPath = result.output_path;
            status = result.message;
        } catch (err) {
            status = "导出 EPUB 失败";
            await platform.message(`导出 EPUB 失败：${err}`, { title: "制作 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function addRule(level: number) {
        rules = [...rules, { enabled: true, level, pattern: "" }];
        saveRulesToSettings();
    }

    function removeRule(index: number) {
        rules = rules.filter((_, i) => i !== index);
        saveRulesToSettings();
    }

    function updateRule(index: number, patch: Partial<RegexRule>) {
        rules = rules.map((item, i) => (i === index ? { ...item, ...patch } : item));
        saveRulesToSettings();
    }

    function saveRulesToSettings() {
        const settings = loadAppSettings();
        settings.customRegexRules = rules.map((rule) => ({
            enabled: rule.enabled !== false,
            level: Number(rule.level) <= 1 ? 1 : 3,
            pattern: String(rule.pattern || ""),
        }));
        suppressRuleRefresh = true;
        saveAppSettings(settings);
        setTimeout(() => (suppressRuleRefresh = false), 0);
    }

    function buildToc(list: RawChapter[]) {
        const out: TocItem[] = [];
        let currentVolume: TocItem | undefined;

        for (const chapter of list) {
            const kind = itemKind(chapter.title, chapter.level, chapter.is_meta);
            const depth = kind === "chapter" && currentVolume ? 1 : 0;
            const item: TocItem = {
                ...chapter,
                id: `${kind}-${chapter.line_number}-${out.length}`,
                depth,
                kind,
                volumeKey: "",
                parentId: undefined,
                hasChildren: false,
            };

            if (kind === "chapter" && currentVolume) {
                item.parentId = currentVolume.id;
                item.volumeKey = currentVolume.id;
                currentVolume.hasChildren = true;
            } else if (kind === "volume") {
                item.volumeKey = item.id;
            }
            out.push(item);
            if (kind === "volume") currentVolume = item;
        }

        return out;
    }

    function itemKind(title: string, level: number, isMeta = false): "volume" | "chapter" | "meta" {
        if (isMeta || isMetaTitle(title)) return "meta";
        if (level <= 1) return "volume";
        return "chapter";
    }

    function isMetaTitle(text: string) {
        return /^(?:内容简介|简介|序(?:章|言)?|前言|楔子|后记|尾声|完本感言|本书相关|(?:新增\s*)?番外)(?:\s|[:：、.．\-—]|$)/.test(text.trim());
    }

    function toggleItem(item: TocItem) {
        if (item.kind !== "volume" || !item.hasChildren) return;
        if (expandedIds.has(item.id)) expandedIds.delete(item.id);
        else expandedIds.add(item.id);
        expandedIds = new Set(expandedIds);
    }

    function normalizedLines() {
        return content.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
    }

    async function saveCurrentText() {
        if (!selectedPath || platform.isWeb) return;
        await platform.invoke("save_text_file", { path: selectedPath, content });
    }

    function openTocActions(item: TocItem, event: Event) {
        event.stopPropagation();
        tocActionTarget = item;
    }

    function closeTocActions() {
        tocActionTarget = null;
    }

    function openRenameTitle(item: TocItem) {
        tocActionTarget = null;
        renameTitleSheet = {
            open: true,
            item,
            value: item.title,
        };
    }

    function closeRenameTitle() {
        renameTitleSheet = {
            open: false,
            item: null,
            value: "",
        };
    }

    async function submitRenameTitle() {
        const item = renameTitleSheet.item;
        const nextTitle = renameTitleSheet.value.trim();
        if (!selectedPath || !item || !nextTitle) return;

        const lines = normalizedLines();
        const lineIndex = item.line_number - 1;
        if (lineIndex < 0 || lineIndex >= lines.length) return;

        const indent = lines[lineIndex].match(/^[\s　]*/)?.[0] ?? "";
        lines[lineIndex] = `${indent}${nextTitle}`;
        content = lines.join("\n");
        await saveCurrentText();
        closeRenameTitle();
        await previewToc();
        status = `已重命名目录标题：${nextTitle}`;
    }

    function chapterBodyRange(item: TocItem) {
        const lines = normalizedLines();
        const nextChapter = chapters
            .filter((chapter) => chapter.line_number > item.line_number)
            .sort((a, b) => a.line_number - b.line_number)[0];
        const startLine = Math.max(0, Math.min(item.line_number - 1, lines.length));
        const endLine = nextChapter ? Math.max(startLine, nextChapter.line_number - 1) : lines.length;
        return { lines, startLine, endLine };
    }

    function openChapterEditor(item: TocItem) {
        tocActionTarget = null;
        const { lines, startLine, endLine } = chapterBodyRange(item);
        chapterEditSheet = {
            open: true,
            item,
            value: lines.slice(startLine, endLine).join("\n"),
            startLine,
            endLine,
        };
    }

    function closeChapterEditor() {
        chapterEditSheet = {
            open: false,
            item: null,
            value: "",
            startLine: 0,
            endLine: 0,
        };
    }

    async function submitChapterEdit() {
        if (!selectedPath || !chapterEditSheet.item) return;
        const lines = normalizedLines();
        const nextLines = [
            ...lines.slice(0, chapterEditSheet.startLine),
            ...chapterEditSheet.value.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n"),
            ...lines.slice(chapterEditSheet.endLine),
        ];
        content = nextLines.join("\n");
        await saveCurrentText();
        const titleText = chapterEditSheet.item.title;
        closeChapterEditor();
        await previewToc();
        status = `已更新章节正文：${titleText}`;
    }

    async function cancelChapterTitle(item: TocItem) {
        if (!selectedPath) return;
        const lines = normalizedLines();
        const lineIndex = item.line_number - 1;
        if (lineIndex < 0 || lineIndex >= lines.length) return;

        const currentLine = lines[lineIndex];
        const indent = currentLine.match(/^[\s　]*/)?.[0] ?? "";
        lines[lineIndex] = `${indent}原章节标题：${item.title}`;
        content = lines.join("\n");
        tocActionTarget = null;
        await saveCurrentText();
        await previewToc();
        status = `已取消本章标题：${item.title}`;
    }

    async function deleteChapterContent(item: TocItem) {
        if (!selectedPath) return;
        const { lines, startLine, endLine } = chapterBodyRange(item);
        if (startLine < 0 || startLine >= lines.length || endLine <= startLine) return;

        content = [...lines.slice(0, startLine), ...lines.slice(endLine)].join("\n");
        tocActionTarget = null;
        await saveCurrentText();
        await previewToc();
        status = `已删除本章标题和内容：${item.title}`;
    }

    async function revealTocItem(id: string) {
        tocOpen = true;
        activeTocId = id;
        const next = new Set(expandedIds);
        let parentId = tocItems.find((item) => item.id === id)?.parentId;
        while (parentId) {
            next.add(parentId);
            parentId = tocItems.find((item) => item.id === parentId)?.parentId;
        }
        expandedIds = next;
        await tick();
        const row = Array.from(document.querySelectorAll<HTMLElement>("[data-toc-id]")).find(
            (element) => element.dataset.tocId === id,
        );
        row?.scrollIntoView({ behavior: "smooth", block: "center", inline: "nearest" });
    }

    function handlePageWheel(event: WheelEvent) {
        if (!desktopMode || !event.deltaY) return;
        const target = event.target as HTMLElement | null;
        const inChapterSheet = Boolean(target?.closest(".chapter-sheet"));
        const scrollBox = target?.closest<HTMLElement>(".toc-list, .check-list, .reorder-preview, .custom-select-menu, .chapter-sheet textarea");
        if (scrollBox && scrollBox.scrollHeight > scrollBox.clientHeight) {
            const goingDown = event.deltaY > 0;
            const canScrollDown = scrollBox.scrollTop + scrollBox.clientHeight < scrollBox.scrollHeight - 1;
            const canScrollUp = scrollBox.scrollTop > 0;
            if ((goingDown && canScrollDown) || (!goingDown && canScrollUp)) return;
        }
        if (inChapterSheet) {
            event.preventDefault();
            return;
        }
        event.preventDefault();
        window.scrollBy({ top: event.deltaY, left: 0, behavior: "auto" });
    }

    function chineseToNum(cn: string) {
        const map: Record<string, number> = { 零: 0, 〇: 0, 一: 1, 二: 2, 两: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9, 十: 10, 百: 100, 千: 1000, 万: 10000 };
        let result = 0;
        let current = 0;
        for (const c of cn) {
            const value = map[c];
            if (value === undefined) return -1;
            if (value >= 10) {
                if (current === 0) current = 1;
                if (value === 10000) {
                    result = (result + current) * value;
                    current = 0;
                } else {
                    result += current * value;
                    current = 0;
                }
            } else {
                current = current * 10 + value;
            }
        }
        return result + current;
    }

    function extractTitleNum(text: string) {
        const chapter = text.match(/第\s*([0-9零一二三四五六七八九十百千万〇两]+)\s*[章回节]/);
        if (chapter) return /^\d+$/.test(chapter[1]) ? Number(chapter[1]) : chineseToNum(chapter[1]);
        const seq = text.match(/^序列\s*([0-9零一二三四五六七八九十百千万〇两]+)(?=\s|[:：、.．\-—]|$)/);
        if (seq) return /^\d+$/.test(seq[1]) ? Number(seq[1]) : chineseToNum(seq[1]);
        const numeric = text.match(/^(\d+)/);
        return numeric ? Number(numeric[1]) : -1;
    }

    function runTocCheck(items = tocItems) {
        sequenceErrors = [];
        titleErrors = [];
        invalidSequenceIds = new Set<string>();
        let lastNum = -1;

        for (const item of items) {
            const kind = itemKind(item.title, item.level, item.is_meta);
            if (kind === "meta") continue;
            if (kind === "volume") {
                lastNum = -1;
                continue;
            }
            const num = extractTitleNum(item.title);
            if (num !== -1) {
                if (lastNum !== -1 && num !== lastNum + 1) {
                    invalidSequenceIds.add(item.id);
                    sequenceErrors.push({
                        id: item.id,
                        title: item.title,
                        line: item.line_number,
                        msg: `(${lastNum}-${num})`,
                    });
                }
                lastNum = num;
            }

            const trimmed = item.title.trim();
            if (/^第\s*[0-9零一二三四五六七八九十百千万〇两]+\s*[章卷回节]\s*$/.test(trimmed) || /^序列\s*[0-9零一二三四五六七八九十百千万〇两]+\s*$/.test(trimmed) || /^\d+$/.test(trimmed)) {
                titleErrors.push({ id: item.id, title: item.title, line: item.line_number, msg: "无标题" });
            }
        }
        reorderPreviewRows = buildReorderPreviewRows(items);
        const hasIssues = sequenceErrors.length + titleErrors.length > 0;
        checkOpen = hasIssues;
        reorderOpen = !hasIssues;
    }

    function titleBody(text: string) {
        const normalized = text.replace(/\u3000/g, " ").replace(/[ \t]+/g, " ").trim();
        const number = "[0-9零〇一二两三四五六七八九十百千万]+";
        const separator = "[:：、.．\\-—]";
        const patterns = [
            new RegExp(`^第\\s*${number}\\s*[卷部章节回节](?:\\s*(?:${separator})\\s*|\\s+)?`, "i"),
            new RegExp(`^卷\\s*${number}(?:\\s*(?:${separator})\\s*|\\s+)?`, "i"),
            new RegExp(`^序列\\s*${number}(?:\\s*(?:${separator})\\s*|\\s+)?`, "i"),
            /^\(?\s*[（(【\[]?\s*\d+\s*[）)】\]]?\s*[：:、.．\-—\s]+/,
            /^\(?\s*[一二三四五六七八九十百千万零〇两]+\s*[：:、.．\-—\s]+/,
            /^\d{1,5}\s*[：:、.．\-—\s]+/,
            /^\d{1,5}(?=[\u4e00-\u9fff])\s*/,
        ];

        let body = normalized;
        for (const re of patterns) {
            if (re.test(body)) {
                body = body.replace(re, "");
                break;
            }
        }

        return body.replace(/^(?:\s|[:：、.．\-—])+/, "").trim();
    }

    function buildReorderPreviewRows(items = tocItems) {
        const rows: ReorderPreviewRow[] = [];
        let volumeIndex = 1;
        let globalChapterIndex = 1;
        let currentVolumeKey = "root";
        let currentVolumeChapterIndex = 1;
        const customRegex = compileReorderRegex();

        for (const item of items) {
            const kind = itemKind(item.title, item.level, item.is_meta);
            if (kind === "volume") {
                currentVolumeKey = item.id;
                currentVolumeChapterIndex = 1;
            }

            if (kind === "meta") {
                rows.push({
                    id: item.id,
                    line: item.line_number,
                    kind: "meta",
                    volumeKey: currentVolumeKey,
                    original: item.title,
                    replacement: item.title,
                    changed: false,
                    included: false,
                    sequenceBroken: false,
                });
                continue;
            }

            const included = shouldReorderItem(item, kind, customRegex);
            const body = titleBody(item.title);
            let replacement = item.title;
            if (included && kind === "volume") {
                replacement = `第${formatNumber(volumeIndex++, volumeNumberStyle)}卷${body ? ` ${body}` : ""}`;
            } else if (included && kind === "chapter") {
                const index = reorderPerVolume ? currentVolumeChapterIndex++ : globalChapterIndex++;
                replacement = `第${formatNumber(index, chapterNumberStyle)}章${body ? ` ${body}` : ""}`;
            }
            rows.push({
                id: item.id,
                line: item.line_number,
                kind,
                volumeKey: kind === "volume" ? item.id : currentVolumeKey,
                original: item.title,
                replacement,
                changed: included && item.title.trim() !== replacement,
                included,
                sequenceBroken: invalidSequenceIds.has(item.id),
            });
        }

        return rows;
    }

    function compileReorderRegex() {
        if (reorderScope !== "regex" || !reorderRegex.trim()) return null;
        try {
            return new RegExp(reorderRegex);
        } catch (_) {
            return null;
        }
    }

    function shouldReorderItem(item: TocItem, kind: "volume" | "chapter", customRegex: RegExp | null) {
        if (isMetaTitle(item.title)) return false;
        if (reorderScope === "all") return kind === "volume" || kind === "chapter";
        if (reorderScope === "volumes") return kind === "volume";
        if (reorderScope === "chapters") return kind === "chapter";
        return customRegex ? customRegex.test(item.title) : false;
    }

    function formatNumber(num: number, style: NumberStyle) {
        if (style === "arabic") return String(num);
        return toChineseNumber(num);
    }

    function toChineseNumber(num: number): string {
        const digits = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
        if (num <= 10) return num === 10 ? "十" : digits[num];
        if (num < 20) return `十${digits[num % 10]}`;
        if (num < 100) {
            const tens = Math.floor(num / 10);
            const ones = num % 10;
            return `${digits[tens]}十${ones ? digits[ones] : ""}`;
        }
        return String(num);
    }

    function toggleReorderVolume(volumeKey: string) {
        if (reorderCollapsedVolumeKeys.has(volumeKey)) reorderCollapsedVolumeKeys.delete(volumeKey);
        else reorderCollapsedVolumeKeys.add(volumeKey);
        reorderCollapsedVolumeKeys = new Set(reorderCollapsedVolumeKeys);
    }

    async function applyReorderToc() {
        if (!content.trim() || !chapters.length) return;
        const lines = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        let changed = 0;

        for (const row of reorderPreviewRows) {
            if (!row.included || row.kind === "meta") continue;
            const lineIndex = row.line - 1;
            if (lineIndex < 0 || lineIndex >= lines.length) continue;
            const indent = lines[lineIndex].match(/^[\s　]*/)?.[0] ?? "";
            if (lines[lineIndex].trim() !== row.replacement) {
                lines[lineIndex] = `${indent}${row.replacement}`;
                changed++;
            }
        }

        content = lines.join("\n");
        await saveCurrentText();
        await previewToc();
        status = changed ? `已重排 ${changed} 个目录标题。` : "目录标题已是当前顺序。";
    }

    function enableManualUuid() {
        uuidAuto = false;
        if (!uuid) uuid = crypto.randomUUID?.() ?? "";
    }

    function isCompactDevice() {
        if (typeof navigator === "undefined") return false;
        return /Android|iPhone|iPad|iPod/i.test(navigator.userAgent || "");
    }

    function withFileText(file: File) {
        return new Promise<string>((resolve, reject) => {
            const reader = new FileReader();
            reader.onload = () => resolve(String(reader.result || ""));
            reader.onerror = () => reject(reader.error || new Error("读取文件失败"));
            reader.readAsText(file);
        });
    }

    function countWords(text: string) {
        return (text.match(/[\u4e00-\u9fff]|[A-Za-z0-9]+/g) || []).length;
    }

    function scanChaptersInBrowser(text: string, scanRules: RegexRule[]): RawChapter[] {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const compiled = scanRules
            .filter((rule) => rule.enabled !== false)
            .map((rule) => {
                try {
                    return { level: rule.level, regex: new RegExp(rule.pattern) };
                } catch {
                    return null;
                }
            })
            .filter((rule): rule is { level: number; regex: RegExp } => Boolean(rule));
        const found: RawChapter[] = [];

        lines.forEach((line, index) => {
            const titleText = line.trim();
            if (!titleText) return;
            const matched = compiled.find((rule) => rule.regex.test(titleText));
            if (!matched) return;
            found.push({
                title: normalizeCatalogTitle(titleText),
                line_number: index + 1,
                level: matched.level,
                is_meta: isMetaTitle(titleText),
                word_count: 0,
            });
        });

        found.forEach((chapter, index) => {
            const start = chapter.line_number - 1;
            const end = index + 1 < found.length ? found[index + 1].line_number - 1 : lines.length;
            chapter.word_count = countWords(lines.slice(start, end).join("\n"));
        });

        return found;
    }

    function escapeXml(value: string) {
        return value
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&apos;");
    }

    function normalizeEpubUuid(value: string) {
        const cleaned = value.trim().replace(/^urn:uuid:/i, "") || crypto.randomUUID?.() || Date.now().toString();
        return `urn:uuid:${cleaned}`;
    }

    function normalizeEpubTitle(value: string) {
        return normalizeCatalogTitle(value.replace(/\r\n|\r|\u2028|\u2029/g, "\n").replace(/[\t ]+/g, " ").replace(/\s*\n\s*/g, " ").trim());
    }

    function normalizeCatalogTitle(value: string) {
        let title = value.trim().replace(/[\s\u3000]*[\u003a\uff1a][\s\u3000]*$/, "");
        const noisyBracketSuffix =
            /[\s\u3000]*[\uff08(【\[][^）)\]】]*(?:求\s*(?:打赏|月票|推荐票|订阅)|打赏|月票|推荐票|加更|补更|爆更)[^）)\]】]*[\uff09)】\]]\s*$/u;
        const noisyPlainSuffix =
            /[\s\u3000]*(?:[，,。.!！、；;:：\-—_]*[\s\u3000]*(?:求\s*(?:打赏|月票|推荐票|订阅)|为.{1,48}?(?:打赏|加更|补更|爆更)|(?:打赏|加更|补更|爆更)(?:\s*\d+\s*\/\s*\d+)?)[!！。.]*)+$/u;

        let previous = "";
        while (title && title !== previous) {
            previous = title;
            title = title.replace(noisyBracketSuffix, "").replace(noisyPlainSuffix, "").trim();
            title = title.replace(/[\s\u3000]*[\u003a\uff1a][\s\u3000]*$/, "").trim();
        }

        return title;
    }

    function cleanTitleRemainder(value: string) {
        return value.replace(/^[\s\u3000]*(?:[\u003a\uff1a\u3001\u3002.．\-—/\\]+)[\s\u3000]*/, "").trim();
    }

    function splitEpubTitle(value: string) {
        const normalized = normalizeEpubTitle(value);
        const strict = normalized.match(
            /^\s*((?:\u7b2c\s*[\u96f6\u3007\u4e00\u4e8c\u4e24\u4e09\u56db\u4e94\u516d\u4e03\u516b\u4e5d\u5341\u767e\u5343\u4e070-9]+\s*[\u5377\u90e8\u7ae0\u8282\u56de])|(?:\u5377\s*[\u96f6\u3007\u4e00\u4e8c\u4e24\u4e09\u56db\u4e94\u516d\u4e03\u516b\u4e5d\u5341\u767e\u5343\u4e070-9]+)|(?:Chapter\s*\d+)|\u7ec8\u7ae0|\u5e8f\u7ae0|\u6954\u5b50)\s*(.*)$/i,
        );
        if (strict?.[1]) {
            const number = strict[1].trim();
            const name = cleanTitleRemainder(strict[2] || "");
            return {
                number,
                name,
                display: `${number}${name ? ` ${name}` : ""}`,
            };
        }
        return { number: "", name: "", display: normalized };
    }

    function isEllipsisParagraph(line: string) {
        const compact = line.replace(/\s+/g, "");
        if (!compact) return false;
        return (/^[…。\.]+$/.test(compact) && (compact.includes("…") || compact.length >= 3));
    }

    function buildTextBody(lines: string[]) {
        const rows = lines.map((line) => line.trim()).filter(Boolean);
        if (!rows.length) return `  <p class="te-paragraph"></p>\n`;
        return rows
            .map((line, index) => {
                const prev = index > 0 ? rows[index - 1] : "";
                const next = index + 1 < rows.length ? rows[index + 1] : "";
                if (isEllipsisParagraph(line) && !isEllipsisParagraph(prev) && !isEllipsisParagraph(next) && index + 1 < rows.length) {
                    return `  <p class="te-divider-line">...</p>`;
                }
                return `  <p class="te-paragraph">${escapeXml(line)}</p>`;
            })
            .join("\n") + "\n";
    }

    function chapterSections(text: string, list: RawChapter[]) {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const headings = list.filter((item) => itemKind(item.title, item.level, item.is_meta) !== "meta" || item.is_meta);
        if (headings.length === 0) {
            return [
                {
                    title: title.trim() || textBaseName(selectedName) || "正文",
                    level: 3,
                    isMeta: false,
                    lines,
                },
            ];
        }
        return headings.map((heading, index) => {
            const titleLine = Math.max(0, heading.line_number - 1);
            const end = index + 1 < headings.length ? Math.max(titleLine + 1, headings[index + 1].line_number - 1) : lines.length;
            return {
                title: normalizeEpubTitle(heading.title),
                level: heading.level,
                isMeta: heading.is_meta,
                lines: lines.slice(titleLine + 1, end),
            };
        });
    }

    function resolveSelectedTitleLayout() {
        return selectedTitleStyle?.titleLayout || (selectedTitleStyle?.titleNameCss || selectedTitleStyle?.titleCssB ? "split" : "single");
    }

    function buildWebStylePreviewDoc(
        headerStyleId: string,
        headerCss: string,
        headerImageSrc: string,
        titleStyleId: string,
        titleCss: string,
        titleLayout: string,
    ) {
        void headerStyleId;
        void titleStyleId;
        const hasHeaderStyle = Boolean(headerImageSrc);
        const activeHeaderCss = hasHeaderStyle ? (headerCss?.trim() || DEFAULT_WEB_HEADER_CSS) : "";
        const titleMarkup = titleLayout === "split"
            ? `<h3 class="te-chapter-title"><span class="te-chapter-number">第十二章</span><span class="te-chapter-name">灯塔来信</span></h3>`
            : `<h3 class="te-chapter-title">第十二章 灯塔来信</h3>`;
        const headerMarkup = hasHeaderStyle
            ? `<figure class="te-header-figure"><img class="te-header-image" src="${escapeXml(headerImageSrc)}" alt="" /></figure>`
            : "";
        const baseCss = `
html, body {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: #eef2f7;
    color: #172033;
    font-family: "Microsoft YaHei", "PingFang SC", sans-serif;
}

body {
    box-sizing: border-box;
}

.te-preview-page {
    --te-preview-width: ${WEB_STYLE_PREVIEW_WIDTH}px;
    --te-preview-height: ${WEB_STYLE_PREVIEW_HEIGHT}px;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    padding: ${hasHeaderStyle ? "0 24px 42px" : "46px 24px 42px"};
    overflow: hidden;
    background: #fffdf8;
    box-shadow: 0 18px 42px rgba(15, 23, 42, 0.18);
}

p.te-paragraph {
    margin: 0 0 0.85em;
    color: #263244;
    font-family: "DK-SONGTI", "SimSun", serif;
    font-size: 16px;
    line-height: 1.75;
    text-align: justify;
    text-indent: 2em;
}
`;
        const spacingCss = hasHeaderStyle
            ? `.te-header-figure { margin-bottom: 1.15em; }
.te-header-figure + .te-chapter-title { margin-top: 0.85em; }
.te-chapter-title { margin-bottom: 2.15em; }`
            : `.te-chapter-title { margin-top: 2.65em; margin-bottom: 2.45em; }`;
        return `<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <style>${baseCss}</style>
  <style>${titleCss}</style>
  <style>${activeHeaderCss}</style>
  <style>${spacingCss}</style>
</head>
<body>
  <main class="te-preview-page">
    ${headerMarkup}
    ${titleMarkup}
    <p class="te-paragraph">夜色沉入城市边缘，风从旧站台吹过，带着潮湿的铁锈味。</p>
    <p class="te-paragraph">她合上手中的书，抬头看见远处灯塔亮起，像一枚缓慢落下的星。</p>
  </main>
</body>
</html>`;
    }

    function buildWebStyleCss(hasHeaderStyle: boolean) {
        const titleCss = selectedTitleStyle?.css?.trim() || "";
        const headerCss = hasHeaderStyle ? (selectedHeaderStyle?.css?.trim() || DEFAULT_WEB_HEADER_CSS) : "";
        const spacingCss = hasHeaderStyle
            ? `body.te-chapter-page--with-header .te-header-figure {
    margin-bottom: 1.15em;
}

body.te-chapter-page--with-header .te-header-figure + .te-chapter-title {
    margin-top: 0.85em;
}

body.te-chapter-page--with-header .te-chapter-title {
    margin-bottom: 2.15em;
}`
            : `body.te-chapter-page--no-header .te-chapter-title {
    margin-top: 2.65em;
    margin-bottom: 2.45em;
}

body.te-chapter-page--no-header p.te-paragraph:first-of-type {
    margin-top: 0;
}`;
        return [WEB_EPUB_MAIN_CSS, titleCss, headerCss, spacingCss]
            .filter((block) => block && block.trim())
            .join("\n\n");
    }

    function buildHeaderFigureXhtml(hasHeaderStyle: boolean, imageSrc: string) {
        if (!hasHeaderStyle) return "";
        return `  <figure class="te-header-figure" aria-label="章节头图">\n    <img class="te-header-image" src="${imageSrc}" alt="" />\n  </figure>\n`;
    }

    function buildChapterXhtml(section: { title: string; level: number; isMeta: boolean; lines: string[] }, hasHeaderStyle: boolean, headerImageSrc: string) {
        const titleParts = splitEpubTitle(section.title);
        const safeDisplayTitle = escapeXml(titleParts.display || section.title || "正文");
        let bodyClass = "te-book-body te-chapter-page";
        let heading = `  <h3 class="te-chapter-title">${safeDisplayTitle}</h3>\n`;

        if (section.isMeta) {
            bodyClass = "te-book-body te-intro-page";
            heading = `  <h1 class="te-intro-title" title="${safeDisplayTitle}"><span><b>${safeDisplayTitle}</b></span></h1>\n`;
        } else if (section.level === 1) {
            bodyClass = "te-book-body te-volume-page te-volume-page--no-image";
            const safeNumber = escapeXml(titleParts.number || titleParts.display || section.title);
            const safeName = escapeXml(titleParts.name || titleParts.display || section.title);
            heading = `  <h1 class="te-volume-title" title="${safeDisplayTitle}">${safeNumber}</h1>\n  <p class="te-volume-subtitle">${safeName}</p>\n`;
        } else {
            bodyClass = `te-book-body te-chapter-page ${hasHeaderStyle ? "te-chapter-page--with-header" : "te-chapter-page--no-header"}`;
        }

        if (!section.isMeta && section.level !== 1 && titleParts.number && resolveSelectedTitleLayout() !== "single") {
            const safeNumber = escapeXml(titleParts.number);
            const safeName = escapeXml(titleParts.name || "");
            heading = `  <h3 class="te-chapter-title"><span class="te-chapter-number">${safeNumber}</span><span class="te-chapter-name">${safeName}</span></h3>\n`;
        }

        const headerFigure = !section.isMeta && section.level !== 1 ? buildHeaderFigureXhtml(hasHeaderStyle, headerImageSrc) : "";

        return `<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>${safeDisplayTitle}</title>
  <link href="../Styles/font.css" type="text/css" rel="stylesheet" />
  <link href="../Styles/main.css" type="text/css" rel="stylesheet" />
</head>
<body class="${bodyClass}">
${headerFigure}${heading}${buildTextBody(section.lines)}</body>
</html>`;
    }

    function buildNcxNavMap(sections: { title: string; level: number; isMeta: boolean; lines: string[] }[]) {
        const navPoints: string[] = [];
        let openVolume = false;
        let maxDepth = 1;

        sections.forEach((section, index) => {
            const kind = itemKind(section.title, section.level, section.isMeta);
            if (kind !== "chapter" && openVolume) {
                navPoints.push("  </navPoint>");
                openVolume = false;
            }

            const depth = kind === "chapter" && openVolume ? 2 : 1;
            maxDepth = Math.max(maxDepth, depth);
            const fileName = `Text/chapter${index}.xhtml`;
            const safeTitle = escapeXml(splitEpubTitle(section.title).display || section.title || `章节 ${index + 1}`);
            navPoints.push(
                `${"  ".repeat(depth)}<navPoint id="navPoint-${index + 1}" playOrder="${index + 1}"><navLabel><text>${safeTitle}</text></navLabel><content src="${fileName}"/>`,
            );

            if (kind === "volume") {
                openVolume = true;
            } else {
                navPoints.push(`${"  ".repeat(depth)}</navPoint>`);
            }
        });

        if (openVolume) {
            navPoints.push("  </navPoint>");
        }

        return { navMap: navPoints.join("\n"), depth: maxDepth };
    }

    function buildImagePageXhtml(pageTitle: string, imageSrc: string, className: string) {
        return `<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>${escapeXml(pageTitle)}</title>
  <link href="../Styles/font.css" type="text/css" rel="stylesheet" />
  <link href="../Styles/main.css" type="text/css" rel="stylesheet" />
</head>
<body class="te-book-body ${className}">
  <div class="${className}-wrap"><img src="${imageSrc}" alt="${escapeXml(pageTitle)}" /></div>
</body>
</html>`;
    }

    function dataUrlToBytes(dataUrl: string) {
        const match = dataUrl.match(/^data:([^;,]+)?(?:;base64)?,(.*)$/);
        if (!match) return null;
        const isBase64 = /;base64,/.test(dataUrl.slice(0, Math.min(dataUrl.length, 80)));
        const raw = isBase64 ? atob(match[2]) : decodeURIComponent(match[2]);
        const bytes = new Uint8Array(raw.length);
        for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
        return {
            mime: match[1] || "image/png",
            bytes,
        };
    }

    async function imageSourceToBytes(src: string) {
        if (!src) return null;
        const data = dataUrlToBytes(src);
        if (data) return data;
        const response = await fetch(src);
        if (!response.ok) return null;
        const mime = response.headers.get("content-type") || "image/png";
        return {
            mime,
            bytes: new Uint8Array(await response.arrayBuffer()),
        };
    }

    async function buildSimpleEpubBytes(bookTitle: string, bookAuthor: string, text: string, list: RawChapter[]) {
        const sections = chapterSections(text, list);
        const files: { name: string; data: Uint8Array }[] = [];
        const encoder = new TextEncoder();
        const addText = (name: string, value: string) => files.push({ name, data: encoder.encode(value) });
        const addBytes = (name: string, data: Uint8Array) => files.push({ name, data });
        let exportHeaderAsset = processedHeaderAsset;
        if (headerAsset && !exportHeaderAsset) {
            exportHeaderAsset = await buildProcessedHeaderFromAsset(headerAsset, selectedHeaderStyle);
            revokeWebImageAsset(processedHeaderAsset);
            processedHeaderAsset = exportHeaderAsset;
            processedHeaderKey = headerProcessKey;
        }
        const fallbackHeaderImage = !exportHeaderAsset && selectedHeaderStyle?.sampleDataUrl
            ? await imageSourceToBytes(selectedHeaderStyle.sampleDataUrl)
            : null;
        const headerImage = exportHeaderAsset
            ? { mime: exportHeaderAsset.mime, bytes: exportHeaderAsset.bytes, fileName: exportHeaderAsset.fileName }
            : fallbackHeaderImage
                ? { ...fallbackHeaderImage, fileName: "chapter-header.png" }
                : null;
        const headerExt = headerImage ? imageExtensionFromMime(headerImage.mime || "image/png", headerImage.fileName) : "png";
        const headerImageName = `Images/chapter-header.${headerExt}`;
        const headerImageSrc = `../${headerImageName}`;
        const hasHeaderStyle = Boolean(headerImage?.bytes);
        const fullUuid = normalizeEpubUuid(uuid);
        const date = new Date().toISOString().slice(0, 10);
        files.push({ name: "mimetype", data: encoder.encode("application/epub+zip") });
        addText(
            "META-INF/container.xml",
            `<?xml version="1.0"?>\n<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">\n  <rootfiles>\n    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>\n  </rootfiles>\n</container>`,
        );
        addText("OEBPS/Styles/font.css", WEB_EPUB_FONT_CSS);
        addText("OEBPS/Styles/main.css", buildWebStyleCss(hasHeaderStyle));

        const manifestItems: string[] = [
            `<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>`,
            `<item id="font-css" href="Styles/font.css" media-type="text/css"/>`,
            `<item id="main-css" href="Styles/main.css" media-type="text/css"/>`,
        ];
        const spineItems: string[] = [];
        const metadataExtras: string[] = [];

        const primaryCoverAsset = coverAsset || activeFullCoverAsset;
        if (primaryCoverAsset) {
            const coverExt = imageExtensionFromMime(primaryCoverAsset.mime, primaryCoverAsset.fileName);
            const coverImageName = `Images/cover.${coverExt}`;
            addBytes(`OEBPS/${coverImageName}`, primaryCoverAsset.bytes);
            addText("OEBPS/Text/cover.xhtml", buildImagePageXhtml("封面", `../${coverImageName}`, "te-cover-page"));
            manifestItems.push(`<item id="cover-image" href="${coverImageName}" media-type="${mediaTypeForExtension(coverExt)}" properties="cover-image"/>`);
            manifestItems.push(`<item id="cover-page" href="Text/cover.xhtml" media-type="application/xhtml+xml"/>`);
            metadataExtras.push(`<meta name="cover" content="cover-image"/>`);
            spineItems.push(`<itemref idref="cover-page"/>`);
        }

        if (coverAsset && activeFullCoverAsset) {
            const slimExt = imageExtensionFromMime(activeFullCoverAsset.mime, activeFullCoverAsset.fileName);
            const slimImageName = `Images/cover~slim.${slimExt}`;
            addBytes(`OEBPS/${slimImageName}`, activeFullCoverAsset.bytes);
            manifestItems.push(`<item id="cover-slim-image" href="${slimImageName}" media-type="${mediaTypeForExtension(slimExt)}"/>`);
        }

        if (primaryCoverAsset && bannerAsset) {
            const bannerExt = imageExtensionFromMime(bannerAsset.mime, bannerAsset.fileName);
            const bannerImageName = `Images/cover~banner.${bannerExt}`;
            addBytes(`OEBPS/${bannerImageName}`, bannerAsset.bytes);
            manifestItems.push(`<item id="cover-banner-image" href="${bannerImageName}" media-type="${mediaTypeForExtension(bannerExt)}"/>`);
        }

        if (hasHeaderStyle && headerImage) {
            addBytes(`OEBPS/${headerImageName}`, headerImage.bytes);
            manifestItems.push(`<item id="chapter-header-image" href="${headerImageName}" media-type="${mediaTypeForExtension(headerExt)}"/>`);
        }

        sections.forEach((section, index) => {
            const fileName = `Text/chapter${index}.xhtml`;
            const id = `chapter${index}`;
            addText(`OEBPS/${fileName}`, buildChapterXhtml(section, hasHeaderStyle, headerImageSrc));
            manifestItems.push(`<item id="${id}" href="${fileName}" media-type="application/xhtml+xml"/>`);
            spineItems.push(`<itemref idref="${id}"/>`);
        });
        const { navMap, depth } = buildNcxNavMap(sections);

        addText(
            "OEBPS/content.opf",
            `<?xml version="1.0" encoding="utf-8"?>\n<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">\n  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">\n    <dc:title>${escapeXml(bookTitle)}</dc:title>\n    <dc:creator>${escapeXml(bookAuthor || "未知作者")}</dc:creator>\n    <dc:language>zh-CN</dc:language>\n    <dc:date>${date}</dc:date>\n    <dc:identifier opf:scheme="UUID" id="BookId">${escapeXml(fullUuid)}</dc:identifier>${metadataExtras.length ? `\n    ${metadataExtras.join("\n    ")}` : ""}\n  </metadata>\n  <manifest>\n    ${manifestItems.join("\n    ")}\n  </manifest>\n  <spine toc="ncx">\n    ${spineItems.join("\n    ")}\n  </spine>\n</package>`,
        );
        addText(
            "OEBPS/toc.ncx",
            `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE ncx PUBLIC "-//NISO//DTD ncx 2005-1//EN" "http://www.daisy.org/z3986/2005/ncx-2005-1.dtd">\n<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">\n  <head>\n    <meta name="dtb:uid" content="${escapeXml(fullUuid)}"/>\n    <meta name="dtb:depth" content="${depth}"/>\n    <meta name="dtb:totalPageCount" content="0"/>\n    <meta name="dtb:maxPageNumber" content="0"/>\n  </head>\n  <docTitle><text>${escapeXml(bookTitle)}</text></docTitle>\n  <navMap>\n${navMap}\n  </navMap>\n</ncx>`,
        );
        return createStoredZip(files);
    }
    function crc32(data: Uint8Array) {
        let crc = -1;
        for (const byte of data) {
            crc ^= byte;
            for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
        }
        return (crc ^ -1) >>> 0;
    }

    function write16(out: number[], value: number) {
        out.push(value & 255, (value >>> 8) & 255);
    }

    function write32(out: number[], value: number) {
        out.push(value & 255, (value >>> 8) & 255, (value >>> 16) & 255, (value >>> 24) & 255);
    }

    function pushBytes(out: number[], data: Uint8Array) {
        for (const byte of data) out.push(byte);
    }

    function createStoredZip(files: { name: string; data: Uint8Array }[]) {
        const encoder = new TextEncoder();
        const out: number[] = [];
        const central: number[] = [];
        for (const file of files) {
            const nameBytes = encoder.encode(file.name);
            const offset = out.length;
            const crc = crc32(file.data);
            write32(out, 0x04034b50);
            write16(out, 20);
            write16(out, 0);
            write16(out, 0);
            write16(out, 0);
            write16(out, 0);
            write32(out, crc);
            write32(out, file.data.length);
            write32(out, file.data.length);
            write16(out, nameBytes.length);
            write16(out, 0);
            pushBytes(out, nameBytes);
            pushBytes(out, file.data);

            write32(central, 0x02014b50);
            write16(central, 20);
            write16(central, 20);
            write16(central, 0);
            write16(central, 0);
            write16(central, 0);
            write16(central, 0);
            write32(central, crc);
            write32(central, file.data.length);
            write32(central, file.data.length);
            write16(central, nameBytes.length);
            write16(central, 0);
            write16(central, 0);
            write16(central, 0);
            write16(central, 0);
            write32(central, 0);
            write32(central, offset);
            pushBytes(central, nameBytes);
        }
        const centralOffset = out.length;
        out.push(...central);
        write32(out, 0x06054b50);
        write16(out, 0);
        write16(out, 0);
        write16(out, files.length);
        write16(out, files.length);
        write32(out, central.length);
        write32(out, centralOffset);
        write16(out, 0);
        return new Uint8Array(out);
    }

    function withBasePath(path: string) {
        return `${base}${path.startsWith("/") ? path : `/${path}`}`;
    }

    onMount(() => {
        const toolboxMakePath = withBasePath("/toolbox/make-epub");
        if (platform.isWeb && window.location.pathname === withBasePath("/mobile/make")) {
            const params = new URLSearchParams(window.location.search);
            params.delete("view");
            const suffix = params.toString();
            window.location.replace(`${toolboxMakePath}${suffix ? `?${suffix}` : ""}`);
            return;
        }
        const params = new URLSearchParams(window.location.search);
        const isToolboxMakeRoute = window.location.pathname === toolboxMakePath;
        const webMobileClient = isWebMobileClient();
        if (isToolboxMakeRoute && params.get("view") === "desktop") {
            params.delete("view");
            const suffix = params.toString();
            window.history.replaceState(null, "", `${toolboxMakePath}${suffix ? `?${suffix}` : ""}`);
        }
        desktopMode =
            (isToolboxMakeRoute && !webMobileClient) ||
            params.get("view") === "desktop" ||
            (platform.isWeb && !webMobileClient && !isCompactDevice());
        backHref = platform.isWeb || desktopMode ? withBasePath("/") : withBasePath("/mobile");
        const refreshRules = () => {
            if (suppressRuleRefresh) return;
            rules = loadMakeRules();
            if (content.trim()) void previewToc();
        };
        window.addEventListener("tepub-settings-updated", refreshRules);
        const selection = readMobileSelection(window.location.search);
        if (selection.path) {
            void loadSource(selection.path, selection.name);
        }
        return () => window.removeEventListener("tepub-settings-updated", refreshRules);
    });

    onDestroy(() => {
        const assetUrls = new Set(
            [
                coverAsset?.objectUrl,
                autoFullCoverAsset?.objectUrl,
                fullCoverAsset?.objectUrl,
                bannerAsset?.objectUrl,
                headerAsset?.objectUrl,
                processedHeaderAsset?.objectUrl,
            ].filter(Boolean) as string[],
        );
        assetUrls.forEach((url) => URL.revokeObjectURL(url));
        if (coverPreviewUrl && !assetUrls.has(coverPreviewUrl)) {
            URL.revokeObjectURL(coverPreviewUrl);
        }
    });
</script>

<svelte:head>
    <title>TEpub-Editor</title>
</svelte:head>

<main
    class="page"
    class:desktop-page={desktopMode}
    class:web-import-page={platform.isWeb && desktopMode && !selectedPath}
    class:desktop-import-page={desktopStyleWorkflowEnabled && !selectedPath}
    class:web-style-mode={platform.isWeb && webMakeStep === "style"}
    class:desktop-style-mode={desktopStylePageActive}
    on:wheel|nonpassive={handlePageWheel}
>
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".txt,.md,.html,.htm" on:change={onFileChange} />
    <input bind:this={coverInputEl} class="file-input" type="file" accept="image/*" on:change={onCoverChange} />
    <input bind:this={fullCoverInputEl} class="file-input" type="file" accept="image/*" on:change={onFullCoverChange} />
    <input bind:this={bannerInputEl} class="file-input" type="file" accept="image/*" on:change={onBannerChange} />
    <input bind:this={headerInputEl} class="file-input" type="file" accept="image/*" on:change={onHeaderChange} />

    {#if !((platform.isWeb && webMakeStep === "style") || desktopStylePageActive)}
        <header class="topbar">
            <a href={backHref} aria-label="返回">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M15 6L9 12L15 18"></path>
                </svg>
            </a>
            <h1>制作 EPUB</h1>
        </header>
    {/if}

    {#if !selectedPath}
        <section
            class="empty-panel"
            class:web-import-panel={platform.isWeb && desktopMode}
            class:desktop-import-panel={desktopStyleWorkflowEnabled}
        >
            <div>
                <h2>选择文本文件开始制作</h2>
                <p>支持 TXT、Markdown 和 HTML 文件，导入后可扫描目录、调整规则并生成 EPUB。</p>
            </div>
            <button type="button" on:click={openPicker} disabled={busy}>选择文本文件</button>
        </section>
    {:else if (platform.isWeb && webMakeStep === "style") || desktopStylePageActive}
        <section class="web-style-page">
            <div class="web-style-grid">
                <div class="web-style-head">
                    <div>
                        <p class="web-style-kicker">EPUB 样式</p>
                        <h2>{title.trim() || textBaseName(selectedName) || "未命名作品"}</h2>
                        <p>{chapters.length ? `${chapters.length} 个目录项` : "未识别目录，将按正文生成"} · {author.trim() || "未填写作者"}</p>
                    </div>
                    <div class="web-style-actions">
                        <button class="secondary" type="button" on:click={backToWebEditStep} disabled={busy}>返回编辑</button>
                        <button type="button" on:click={makeEpub} disabled={busy}>生成 EPUB</button>
                    </div>
                </div>

                <section class="web-style-panel">
                    <div class="panel-title">
                        <h3>封面与横幅</h3>
                        <p>
                            {#if desktopStylePageActive}
                                全屏封面会用当前封面按图片处理默认参数自动生成，也可以单独替换；阅微横幅会作为 cover~banner 写入 EPUB。
                            {:else}
                                全屏封面会用当前封面按图片处理默认参数自动生成，也可以单独替换；阅微横幅会作为封面后的独立横幅页。
                            {/if}
                        </p>
                    </div>
                    <div class="asset-grid">
                        <button class="asset-card full-cover-card" type="button" on:click={openFullCoverPicker}>
                            {#if activeFullCoverAsset}
                                <img src={activeFullCoverAsset.objectUrl} alt="全屏封面" />
                                <span>{fullCoverAsset ? "手动全屏封面" : autoFullCoverAsset ? "自动全屏封面" : "当前封面"}</span>
                            {:else}
                                <span>选择全屏封面</span>
                            {/if}
                        </button>
                        <button class="asset-card banner-card" type="button" on:click={openBannerPicker}>
                            {#if bannerAsset}
                                <img src={bannerAsset.objectUrl} alt="阅微横幅" />
                            {:else}
                                <span>选择阅微横幅</span>
                            {/if}
                        </button>
                        <button class="asset-card header-card" type="button" on:click={openHeaderPicker}>
                            {#if headerAsset}
                                <img src={processedHeaderAsset?.objectUrl || headerAsset.objectUrl} alt="章节头图" />
                                <span>已选择章节头图</span>
                            {:else if selectedHeaderStyle?.sampleDataUrl}
                                <img src={selectedHeaderStyle.sampleDataUrl} alt="头图样式样图" />
                                <span>选择章节头图</span>
                            {:else}
                                <span>选择章节头图</span>
                            {/if}
                        </button>
                    </div>
                </section>

                <section class="web-style-panel">
                    <div class="panel-title">
                        <h3>章节样式</h3>
                        <p>头图样式会使用样式库内的头图样板；标题样式会自动按有无头图调整上下留白。</p>
                    </div>
                    <div class="style-fields">
                        <label>
                            <span>头图样式</span>
                            <CustomSelect
                                className="make-select style-select"
                                value={selectedHeaderStyleId}
                                options={styleHeaderOptions}
                                on:change={(e) => onWebHeaderStyleChange(e.detail)}
                            />
                        </label>
                        <label>
                            <span>标题样式</span>
                            <CustomSelect
                                className="make-select style-select"
                                value={selectedTitleStyle?.id || selectedTitleStyleId}
                                options={styleTitleOptions}
                                placeholder="选择标题样式"
                                on:change={(e) => onWebTitleStyleChange(e.detail)}
                            />
                        </label>
                    </div>
                </section>

                <section class="web-style-panel web-preview-panel">
                    <div class="chapter-preview">
                        <div class="phone-preview-frame">
                            <iframe title="章节样式预览" srcdoc={webStylePreviewDoc}></iframe>
                        </div>
                    </div>
                </section>
            </div>
        </section>
    {:else}
        <div class="make-column left-column">
        <section class="meta" class:expanded={metaOpen} class:collapsed={!metaOpen}>
            <div class="section-head meta-section-head">
                <button class="fold-head" type="button" on:click={() => (metaOpen = !metaOpen)}>
                    <span>图书信息</span>
                    <small>3 项</small>
                    <span class="chevron-shell" aria-hidden="true">
                        <svg class:open={metaOpen} viewBox="0 0 24 24">
                            <path d="M9 6L15 12L9 18"></path>
                        </svg>
                    </span>
                </button>
            </div>
            {#if metaOpen}
                <div class="meta-top">
                    <div class="meta-main">
                        <label class="title-field">
                            <span>书名</span>
                            <input bind:value={title} autocomplete="off" />
                        </label>
                        <label class="author-field">
                            <span>作者</span>
                            <input bind:value={author} autocomplete="off" />
                        </label>
                    </div>
                    <label class="cover-field">
                        <span>封面</span>
                        <button class="cover-box" type="button" on:click={openCoverPicker} aria-label="选择封面">
                            {#if coverPreviewUrl}
                                <img src={coverPreviewUrl} alt={coverName || "封面"} />
                            {:else}
                                <b>选择封面</b>
                            {/if}
                        </button>
                    </label>
                </div>
                <label class="uuid-row">
                    <span>UUID</span>
                    <input bind:value={uuid} readonly={uuidAuto} on:focus={enableManualUuid} autocomplete="off" />
                </label>
            {/if}
        </section>

        <section class="toc-panel" class:expanded={tocOpen} class:collapsed={!tocOpen}>
            <div class="section-head">
                <button class="fold-head" type="button" on:click={() => (tocOpen = !tocOpen)}>
                    <span>目录预览</span>
                    <small>{chapters.length} 项</small>
                    <span class="chevron-shell" aria-hidden="true">
                        <svg class:open={tocOpen} viewBox="0 0 24 24">
                            <path d="M9 6L15 12L9 18"></path>
                        </svg>
                    </span>
                </button>
            </div>
            {#if tocOpen}
                <div class="toc-actions">
                    <button type="button" on:click={previewToc} disabled={busy}>重新扫描并检查</button>
                </div>
                <div class="status">{status}</div>
                {#if visibleToc.length}
                    <div class="toc-list">
                        {#each visibleToc as item}
                            <div
                                class="toc-row"
                                class:volume={item.kind === "volume"}
                                class:sequence-error={invalidSequenceIds.has(item.id)}
                                class:active={activeTocId === item.id}
                                data-toc-id={item.id}
                                style={`--depth:${item.depth}`}
                            >
                                <button class="toc-main" type="button" on:click={() => toggleItem(item)}>
                                    <span class="fold" aria-label={item.kind === "volume" && item.hasChildren ? chevronLabel(expandedIds.has(item.id)) : undefined}>
                                        {#if item.kind === "volume" && item.hasChildren}
                                            <svg class:open={expandedIds.has(item.id)} viewBox="0 0 24 24" aria-hidden="true">
                                                <path d="M9 6L15 12L9 18"></path>
                                            </svg>
                                        {/if}
                                    </span>
                                    <strong>{item.title}</strong>
                                    <small>第 {item.line_number} 行 · {item.word_count} 字</small>
                                </button>
                                <button class="toc-more" type="button" aria-label={`${item.title} 更多操作`} on:click={(event) => openTocActions(item, event)}>
                                    <svg viewBox="0 0 24 24" aria-hidden="true">
                                        <circle cx="12" cy="5" r="1.75"></circle>
                                        <circle cx="12" cy="12" r="1.75"></circle>
                                        <circle cx="12" cy="19" r="1.75"></circle>
                                    </svg>
                                </button>
                            </div>
                        {/each}
                    </div>
                {/if}
            {/if}
        </section>

        <section class="bottom-actions">
            <button type="button" on:click={makeEpub} disabled={busy}>生成 EPUB</button>
            {#if makeResult}
                <button type="button" on:click={exportMadeEpub} disabled={busy}>导出 EPUB</button>
            {/if}
            {#if exportPath}<code>{exportPath}</code>{/if}
        </section>
        </div>

        <div
            class="make-column right-column"
            class:hasIssues={sequenceErrors.length + titleErrors.length > 0}
            class:noIssues={sequenceErrors.length + titleErrors.length === 0}
            class:checkCollapsed={!checkOpen}
            class:reorderExpanded={reorderOpen}
        >
        <section class="regex-panel">
            <div class="section-head">
                <button class="fold-head" type="button" on:click={() => (regexOpen = !regexOpen)}>
                    <span>目录正则</span>
                    <small>{rules.length} 条</small>
                    <span class="chevron-shell" aria-hidden="true">
                        <svg class:open={regexOpen} viewBox="0 0 24 24">
                            <path d="M9 6L15 12L9 18"></path>
                        </svg>
                    </span>
                </button>
            </div>
            {#if regexOpen}
                {#each rules as rule, index}
                    <div class="rule-row">
                        <label class="rule-enabled" title="是否应用该正则">
                            <input
                                type="checkbox"
                                checked={rule.enabled !== false}
                                on:change={(e) => updateRule(index, { enabled: (e.currentTarget as HTMLInputElement).checked })}
                            />
                        </label>
                        <CustomSelect
                            className="make-select compact-select"
                            value={String(rule.level)}
                            options={ruleLevelOptions}
                            on:change={(e) => updateRule(index, { level: Number(e.detail) })}
                        />
                        <input
                            value={rule.pattern}
                            autocomplete="off"
                            on:input={(e) => updateRule(index, { pattern: (e.currentTarget as HTMLInputElement).value })}
                        />
                        <button type="button" on:click={() => removeRule(index)} aria-label="删除正则">×</button>
                    </div>
                {/each}
                <div class="rule-actions">
                    <button type="button" on:click={() => addRule(1)}>添加卷规则</button>
                    <button type="button" on:click={() => addRule(3)}>添加章节规则</button>
                </div>
            {/if}
        </section>

        <section class="check-panel" class:expanded={checkOpen} class:collapsed={!checkOpen}>
            <button class="check-head" type="button" on:click={toggleCheckPanel}>
                <span>目录检查</span>
                <small>{sequenceErrors.length + titleErrors.length} 个问题</small>
                <span class="chevron-shell" aria-hidden="true">
                    <svg class:open={checkOpen} viewBox="0 0 24 24">
                        <path d="M9 6L15 12L9 18"></path>
                    </svg>
                </span>
            </button>
            {#if checkOpen}
                {#if sequenceErrors.length || titleErrors.length}
                    <div class="check-list">
                        {#each sequenceErrors as item}
                            <button type="button" class="check-row" on:click={() => revealTocItem(item.id)}>
                                <strong>序号跳跃</strong>
                                <span>{item.title}</span>
                                <small>第 {item.line} 行 · {item.msg}</small>
                            </button>
                        {/each}
                        {#each titleErrors as item}
                            <button type="button" class="check-row" on:click={() => revealTocItem(item.id)}>
                                <strong>标题缺失</strong>
                                <span>{item.title}</span>
                                <small>第 {item.line} 行 · {item.msg}</small>
                            </button>
                        {/each}
                    </div>
                {:else}
                    <p class="check-empty">当前目录没有发现明显问题。</p>
                {/if}
            {/if}
        </section>

        <section class="reorder-panel" class:expanded={reorderOpen} class:collapsed={!reorderOpen}>
            <button class="check-head" type="button" on:click={toggleReorderPanel}>
                <span>目录重排</span>
                <small>{visibleReorderRows.length} 项预览</small>
                <span class="chevron-shell" aria-hidden="true">
                    <svg class:open={reorderOpen} viewBox="0 0 24 24">
                        <path d="M9 6L15 12L9 18"></path>
                    </svg>
                </span>
            </button>
            {#if reorderOpen}
                <div class="reorder-options">
                    <label>
                        <span>重排范围</span>
                        <CustomSelect
                            className="make-select"
                            value={reorderScope}
                            options={reorderScopeOptions}
                            on:change={(e) => (reorderScope = e.detail as ReorderScope)}
                        />
                    </label>
                    <label class="check-toggle">
                        <input type="checkbox" bind:checked={reorderPerVolume} />
                        <span>每卷章节从第一章开始</span>
                    </label>
                    <label>
                        <span>卷序号</span>
                        <CustomSelect
                            className="make-select"
                            value={volumeNumberStyle}
                            options={numberStyleOptions}
                            on:change={(e) => (volumeNumberStyle = e.detail as NumberStyle)}
                        />
                    </label>
                    <label>
                        <span>章序号</span>
                        <CustomSelect
                            className="make-select"
                            value={chapterNumberStyle}
                            options={[numberStyleOptions[1], numberStyleOptions[0]]}
                            on:change={(e) => (chapterNumberStyle = e.detail as NumberStyle)}
                        />
                    </label>
                    {#if reorderScope === "regex"}
                        <label class="wide">
                            <span>重排正则</span>
                            <input bind:value={reorderRegex} autocomplete="off" placeholder="匹配需要重排的目录标题" />
                        </label>
                    {/if}
                </div>
                <div class="check-actions">
                    <button class="secondary" type="button" on:click={applyReorderToc} disabled={busy || !chapters.length}>应用目录重排</button>
                </div>
                <div class="reorder-preview">
                    <div class="reorder-head">
                        <span>原标题</span>
                        <span>修改后</span>
                    </div>
                    {#each visibleReorderRows as row}
                        <button
                            type="button"
                            class="reorder-row"
                            class:volume={row.kind === "volume"}
                            class:meta={row.kind === "meta"}
                            class:sequence-broken={row.sequenceBroken}
                            on:click={() => (row.kind === "volume" ? toggleReorderVolume(row.id) : revealTocItem(row.id))}
                        >
                            <span>
                                {#if row.kind === "volume"}
                                    <span class="row-chevron" aria-hidden="true">
                                        <svg class:open={!reorderCollapsedVolumeKeys.has(row.id)} viewBox="0 0 24 24">
                                            <path d="M9 6L15 12L9 18"></path>
                                        </svg>
                                    </span>
                                {/if}
                                {row.original}
                            </span>
                            <span class:changed={row.changed}>{row.replacement}</span>
                        </button>
                    {/each}
                </div>
            {/if}
        </section>

        </div>
    {/if}

    {#if tocActionTarget}
        <div class="sheet-backdrop" role="presentation" on:click={closeTocActions}></div>
        <div class="action-sheet" role="dialog" aria-modal="true" aria-labelledby="toc-actions-title">
            <div class="sheet-copy">
                <strong id="toc-actions-title">{tocActionTarget.title}</strong>
                <p>第 {tocActionTarget.line_number} 行</p>
            </div>
            <div class="action-sheet-actions">
                <button type="button" on:click={() => openRenameTitle(tocActionTarget!)}>重命名标题</button>
                <button type="button" on:click={() => openChapterEditor(tocActionTarget!)}>编辑本章文本</button>
                <button class="sheet-danger" type="button" on:click={() => deleteChapterContent(tocActionTarget!)}>删除本章内容</button>
                <button class="sheet-danger" type="button" on:click={() => cancelChapterTitle(tocActionTarget!)}>取消本章标题</button>
                <button class="sheet-cancel" type="button" on:click={closeTocActions}>取消</button>
            </div>
        </div>
    {/if}

    {#if renameTitleSheet.open}
        <div class="sheet-backdrop" role="presentation" on:click={closeRenameTitle}></div>
        <div class="action-sheet" role="dialog" aria-modal="true" aria-labelledby="rename-title">
            <div class="sheet-copy">
                <strong id="rename-title">重命名标题</strong>
                <p>第 {renameTitleSheet.item?.line_number} 行</p>
            </div>
            <label class="sheet-field">
                <span>标题</span>
                <input bind:value={renameTitleSheet.value} autocomplete="off" />
            </label>
            <div class="action-sheet-actions two">
                <button class="sheet-cancel" type="button" on:click={closeRenameTitle}>取消</button>
                <button type="button" on:click={submitRenameTitle} disabled={busy || !renameTitleSheet.value.trim()}>保存</button>
            </div>
        </div>
    {/if}

    {#if chapterEditSheet.open}
        <div class="sheet-backdrop" role="presentation" on:click={closeChapterEditor}></div>
        <div class="chapter-sheet" role="dialog" aria-modal="true" aria-labelledby="edit-chapter-text">
            <div class="sheet-copy">
                <strong id="edit-chapter-text">编辑本章文本</strong>
                <p>{chapterEditSheet.item?.title}</p>
            </div>
            <textarea bind:value={chapterEditSheet.value}></textarea>
            <div class="action-sheet-actions two">
                <button class="sheet-cancel" type="button" on:click={closeChapterEditor}>取消</button>
                <button type="button" on:click={submitChapterEdit} disabled={busy}>保存并重扫</button>
            </div>
        </div>
    {/if}
</main>

<style>
    :global(html),
    :global(body) {
        background: #f4f5f8;
    }

    :global(body) {
        overscroll-behavior: auto;
    }

    .page {
        min-height: 100vh;
        box-sizing: border-box;
        padding: max(10px, env(safe-area-inset-top)) 14px max(44px, env(safe-area-inset-bottom));
        background: #f4f5f8;
        color: #171b24;
    }

    .file-input {
        position: fixed;
        width: 1px;
        height: 1px;
        opacity: 0;
        pointer-events: none;
    }

    .topbar {
        display: grid;
        grid-template-columns: 38px minmax(0, 1fr);
        align-items: center;
        gap: 8px;
        min-height: 52px;
    }

    .topbar a {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        color: inherit;
        text-decoration: none;
        padding: 0;
    }

    .topbar a svg {
        width: 20px;
        height: 20px;
        fill: none;
        stroke: currentColor;
        stroke-width: 2.2;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    h1,
    p {
        margin: 0;
        letter-spacing: 0;
    }

    h1 {
        font-size: 22px;
    }

    .meta,
    .regex-panel,
    .toc-panel,
    .check-panel,
    .reorder-panel,
    .empty-panel,
    .web-style-panel,
    .bottom-actions {
        margin-top: 10px;
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-radius: 8px;
        background: #fff;
        padding: 12px;
    }

    .empty-panel {
        display: grid;
        gap: 14px;
        min-height: 220px;
        align-content: center;
        justify-items: center;
        text-align: center;
    }

    .empty-panel h2 {
        margin: 0;
        font-size: 20px;
        line-height: 1.25;
    }

    .empty-panel p {
        max-width: 520px;
        margin-top: 8px;
        color: #626a78;
        font-size: 14px;
        line-height: 1.6;
    }

    .empty-panel button {
        min-width: 150px;
        padding: 0 18px;
    }

    .web-style-page {
        display: grid;
        gap: 12px;
    }

    .web-style-head {
        display: grid;
        gap: 12px;
        margin-top: 10px;
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-radius: 8px;
        background: #fff;
        padding: 10px 12px;
    }

    .web-style-kicker {
        color: #1677b8;
        font-size: 12px;
        font-weight: 900;
    }

    .web-style-head h2,
    .panel-title h3 {
        margin: 0;
        letter-spacing: 0;
    }

    .web-style-head h2 {
        margin-top: 4px;
        font-size: 18px;
        line-height: 1.25;
    }

    .web-style-head p,
    .panel-title p {
        color: #626a78;
        font-size: 12px;
        line-height: 1.55;
    }

    .web-style-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
    }

    .web-style-actions button {
        min-height: 40px;
        padding: 0 12px;
    }

    .web-style-actions .secondary {
        background: #eef1f6;
        color: #4f5867;
    }

    .web-style-grid {
        display: grid;
        gap: 12px;
    }

    .web-style-panel {
        display: grid;
        gap: 12px;
    }

    .web-preview-panel {
        grid-template-rows: minmax(0, 1fr);
        min-height: 0;
    }

    .panel-title {
        display: grid;
        gap: 5px;
    }

    .panel-title h3 {
        font-size: 16px;
        line-height: 1.3;
    }

    .asset-grid {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
        gap: 10px;
    }

    .asset-card {
        position: relative;
        min-height: 0;
        display: grid;
        place-items: center;
        overflow: hidden;
        border: 1px solid rgba(23, 27, 36, 0.1);
        border-radius: 8px;
        background: #f6f8fb;
        color: #657080;
        padding: 0;
        font-size: 13px;
        font-weight: 900;
    }

    .asset-card span {
        position: relative;
        z-index: 1;
        display: inline-grid;
        place-items: center;
        max-width: calc(100% - 20px);
        min-height: 26px;
        box-sizing: border-box;
        border-radius: 999px;
        background: rgba(255, 255, 255, 0.88);
        color: #4f5867;
        padding: 4px 10px;
        font-size: 12px;
        line-height: 1.2;
    }

    .asset-card img {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .full-cover-card {
        aspect-ratio: 3 / 4;
        grid-row: span 2;
    }

    .banner-card {
        aspect-ratio: 16 / 9;
        align-self: start;
    }

    .header-card {
        aspect-ratio: 16 / 9;
        align-self: start;
    }

    .style-fields {
        display: grid;
        gap: 10px;
    }

    .chapter-preview {
        height: 100%;
        min-height: 0;
        display: grid;
        place-items: stretch center;
        overflow: hidden;
        background: transparent;
    }

    .phone-preview-frame {
        width: min(100%, 360px, calc(100% * 360 / 700));
        max-height: 100%;
        aspect-ratio: 360 / 700;
        overflow: hidden;
        border: 1px solid rgba(23, 27, 36, 0.14);
        border-radius: 6px;
        background: #ffffff;
        box-shadow: 0 18px 42px rgba(15, 23, 42, 0.16);
    }

    .chapter-preview iframe {
        width: 100%;
        height: 100%;
        display: block;
        border: 0;
        background: #eef2f7;
        overflow: hidden;
    }

    .status,
    code {
        color: #747986;
        font-size: 13px;
        line-height: 1.5;
        word-break: break-all;
    }

    button {
        min-height: 36px;
        border: 0;
        border-radius: 8px;
        background: #1677b8;
        color: #fff;
        font-weight: 900;
    }

    button:disabled {
        opacity: 0.6;
    }

    .meta {
        display: grid;
        gap: 10px;
    }

    .meta.collapsed {
        gap: 0;
    }

    .meta-top {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 94px;
        gap: 12px;
        align-items: stretch;
    }

    .meta-main {
        display: grid;
        gap: 10px;
    }

    .cover-field {
        align-content: start;
    }

    .cover-box {
        width: 100%;
        height: 118px;
        min-height: 0;
        display: grid;
        place-items: center;
        overflow: hidden;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        background: #f1f5f8;
        color: #747986;
        padding: 0;
        font-size: 11px;
        font-weight: 900;
    }

    .cover-box b {
        display: block;
        max-width: 4em;
        color: #7b8491;
        font-size: 11px;
        line-height: 1.35;
        text-align: center;
    }

    .cover-box img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    label {
        display: grid;
        gap: 6px;
    }

    label span {
        color: #626a78;
        font-size: 12px;
        font-weight: 800;
    }

    input {
        width: 100%;
        box-sizing: border-box;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        padding: 8px 9px;
        background: #fff;
        color: inherit;
        font: inherit;
    }

    :global(.make-select) {
        --control-height: 36px;
    }

    :global(.make-select .custom-select-trigger) {
        min-height: 36px;
        border-color: rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        background: #fff;
        color: #171b24;
        box-shadow: none;
        font-size: 13px;
        font-weight: 800;
    }

    :global(.make-select .custom-select-menu) {
        max-height: min(360px, calc(100vh - 220px));
        overflow-y: auto;
        overscroll-behavior: contain;
        border-color: rgba(23, 27, 36, 0.14);
        border-radius: 8px;
        background: #fff;
        box-shadow: 0 12px 28px rgba(25, 31, 43, 0.14);
    }

    :global(.make-select .custom-select-menu button) {
        border-radius: 7px;
        font-size: 13px;
        font-weight: 800;
    }

    :global(.compact-select) {
        --control-height: 34px;
    }

    :global(.compact-select .custom-select-trigger) {
        min-height: 34px;
        padding: 5px 10px;
        font-size: 13px;
    }

    .section-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        margin-bottom: 9px;
    }

    .section-head button,
    .rule-actions button {
        min-height: 32px;
        padding: 0 12px;
        background: #e8f2f8;
        color: #1677b8;
    }

    .fold-head,
    .check-head {
        width: 100%;
        min-height: 34px;
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto 20px;
        align-items: center;
        gap: 8px;
        padding: 0;
        background: transparent;
        color: inherit;
        text-align: left;
    }

    .section-head .fold-head {
        background: transparent;
        color: inherit;
        padding: 0;
    }

    .fold-head span,
    .check-head span {
        font-size: 15px;
        font-weight: 900;
    }

    .fold-head small,
    .check-head small {
        color: #747986;
        font-size: 12px;
    }

    .chevron-shell {
        width: 20px;
        height: 20px;
        display: grid;
        place-items: center;
        color: #8d94a0;
    }

    .chevron-shell svg,
    .fold svg,
    .row-chevron svg {
        width: 16px;
        height: 16px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1.9;
        stroke-linecap: round;
        stroke-linejoin: round;
        transition: transform 0.16s ease;
    }

    .chevron-shell svg.open,
    .fold svg.open,
    .row-chevron svg.open {
        transform: rotate(90deg);
    }

    .rule-row {
        display: grid;
        grid-template-columns: 26px 56px minmax(0, 1fr) 32px;
        gap: 6px;
        margin-top: 6px;
        align-items: center;
    }

    .rule-enabled {
        min-height: 34px;
        display: grid;
        place-items: center;
        gap: 0;
    }

    .rule-enabled input {
        width: 16px;
        height: 16px;
        padding: 0;
        accent-color: #1677b8;
    }

    .rule-row button {
        min-height: 34px;
        background: #f2e7e7;
        color: #9b3d4f;
        font-size: 18px;
    }

    .rule-row input {
        min-height: 34px;
        padding: 5px 10px;
        font-size: 13px;
    }

    .rule-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-top: 8px;
    }

    .toc-list {
        display: grid;
        margin-top: 10px;
        position: relative;
    }

    .toc-actions {
        display: grid;
        margin-bottom: 8px;
    }

    .toc-actions button {
        min-height: 32px;
        background: #e8f2f8;
        color: #1677b8;
    }

    .toc-row {
        min-height: 46px;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 34px;
        gap: 6px;
        padding: 0 2px 0 calc(2px + var(--depth) * 18px);
        border-radius: 0;
        border-bottom: 1px solid #eceef2;
        background: transparent;
        color: inherit;
        text-align: left;
    }

    .toc-main {
        min-height: 46px;
        display: grid;
        grid-template-columns: 20px minmax(0, 1fr);
        grid-template-areas:
            "fold title"
            "fold meta";
        gap: 2px 8px;
        padding: 6px 0;
        border-radius: 0;
        background: transparent;
        color: inherit;
        text-align: left;
    }

    .toc-more {
        width: 30px;
        height: 30px;
        min-height: 30px;
        display: grid;
        place-items: center;
        align-self: center;
        border-radius: 8px;
        background: transparent;
        color: #858b96;
        padding: 0;
    }

    .toc-more svg {
        width: 16px;
        height: 16px;
        fill: currentColor;
    }

    .toc-row.volume strong {
        font-weight: 900;
    }

    .toc-row.volume {
        position: sticky;
        top: 0;
        z-index: 4;
        background: #fff;
        box-shadow: 0 1px 0 #eceef2;
    }

    .toc-row.volume.sequence-error {
        background: #fff4f2;
    }

    .toc-row.sequence-error strong {
        color: #b33636;
    }

    .toc-row.sequence-error {
        background: #fff4f2;
    }

    .toc-row.active {
        border-radius: 8px;
        background: #e8f2f8;
        box-shadow: inset 3px 0 0 #1677b8;
    }

    .fold {
        width: 20px;
        height: 20px;
        grid-area: fold;
        display: grid;
        place-items: center;
        align-self: center;
        color: #8d94a0;
    }

    .toc-row strong {
        grid-area: title;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 13px;
    }

    .toc-row small {
        grid-area: meta;
        color: #858b96;
        font-size: 11px;
    }

    .sheet-backdrop {
        position: fixed;
        inset: 0;
        z-index: 30;
        background: rgba(20, 25, 35, 0.34);
        backdrop-filter: blur(10px);
    }

    .action-sheet,
    .chapter-sheet {
        position: fixed;
        left: 14px;
        right: 14px;
        top: 50%;
        transform: translateY(-50%);
        z-index: 31;
        display: grid;
        gap: 14px;
        max-width: 420px;
        margin: 0 auto;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.98);
        box-shadow: 0 18px 40px rgba(25, 31, 43, 0.16);
        padding: 16px;
    }

    .chapter-sheet {
        top: max(18px, env(safe-area-inset-top));
        bottom: max(18px, env(safe-area-inset-bottom));
        transform: none;
        grid-template-rows: auto minmax(0, 1fr) auto;
        max-width: 640px;
    }

    .sheet-copy {
        display: grid;
        gap: 6px;
    }

    .sheet-copy strong {
        font-size: 16px;
        line-height: 1.25;
    }

    .sheet-copy p {
        margin: 0;
        color: #666f7d;
        font-size: 13px;
        line-height: 1.45;
        word-break: break-all;
    }

    .sheet-field {
        display: grid;
        gap: 6px;
    }

    .chapter-sheet textarea {
        width: 100%;
        min-height: 0;
        overflow-y: auto;
        overscroll-behavior: contain;
        resize: none;
        box-sizing: border-box;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 10px;
        background: #fbfcfe;
        color: inherit;
        padding: 10px;
        font: inherit;
        font-size: 14px;
        line-height: 1.65;
    }

    .action-sheet-actions {
        display: grid;
        gap: 8px;
    }

    .action-sheet-actions.two {
        grid-template-columns: 1fr 1fr;
    }

    .action-sheet-actions button {
        min-height: 38px;
        background: #1677b8;
        color: #fff;
        box-shadow: none;
    }

    .action-sheet-actions .sheet-cancel {
        background: #eef1f6;
        color: #4f5867;
    }

    .action-sheet-actions .sheet-danger {
        background: #f4ecee;
        color: #9b3d4f;
    }

    .check-panel,
    .reorder-panel {
        display: grid;
        gap: 10px;
    }

    .check-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
    }

    .reorder-options {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
    }

    .reorder-options .wide {
        grid-column: 1 / -1;
    }

    .check-toggle {
        align-content: end;
        grid-template-columns: 18px minmax(0, 1fr);
        grid-auto-flow: column;
        align-items: center;
        min-height: 54px;
        gap: 8px;
    }

    .check-toggle input {
        width: 16px;
        height: 16px;
        padding: 0;
    }

    .check-toggle span {
        color: #525a68;
        font-size: 12px;
        line-height: 1.3;
    }

    .check-actions button {
        background: #e8f2f8;
        color: #1677b8;
    }

    .check-actions button.secondary {
        background: #edf6f1;
        color: #1f7a5a;
    }

    .reorder-preview {
        display: grid;
        overflow: hidden;
        border: 1px solid #e6e8ee;
        border-radius: 8px;
    }

    .reorder-head,
    .reorder-row {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
        gap: 8px;
        align-items: center;
        min-height: 34px;
        padding: 0 9px;
    }

    .reorder-head {
        background: #f5f6f9;
        color: #747986;
        font-size: 11px;
        font-weight: 900;
    }

    .reorder-row {
        border-radius: 0;
        border-top: 1px solid #edf0f4;
        background: #fff;
        color: inherit;
        text-align: left;
        font-size: 12px;
    }

    .reorder-row.volume {
        background: #fbfcff;
        font-weight: 900;
    }

    .reorder-row.meta {
        color: #7d8490;
    }

    .reorder-row.sequence-broken span:first-child {
        color: #b33636;
        font-weight: 900;
    }

    .reorder-row span {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .row-chevron {
        width: 18px;
        height: 18px;
        display: inline-grid;
        place-items: center;
        margin-right: 2px;
        color: #8d94a0;
        vertical-align: middle;
    }

    .reorder-row .changed {
        color: #1f7a5a;
        font-weight: 900;
    }

    .check-list {
        display: grid;
        gap: 6px;
        align-content: start;
        grid-auto-rows: 44px;
    }

    .check-row {
        height: 44px;
        min-height: 44px;
        display: grid;
        grid-template-columns: 70px minmax(0, 1fr);
        grid-template-areas:
            "type meta"
            "type title";
        gap: 2px 8px;
        padding: 7px 9px;
        border: 1px solid #ece1d6;
        border-radius: 8px;
        background: #fffaf4;
        color: inherit;
        text-align: left;
    }

    .check-row strong {
        grid-area: type;
        align-self: center;
        color: #9b5c1d;
        font-size: 12px;
    }

    .check-row span {
        grid-area: title;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 13px;
        font-weight: 800;
    }

    .check-row small,
    .check-empty {
        grid-area: meta;
        margin: 0;
        color: #858b96;
        font-size: 11px;
        line-height: 1.4;
    }

    .bottom-actions {
        display: grid;
        gap: 10px;
    }

    .make-column {
        display: contents;
    }

    @media (min-width: 720px) {
        .page {
            max-width: 820px;
            margin: 0 auto;
        }
    }

    @media (min-width: 900px) {
        .page.desktop-page {
            width: min(1280px, calc(100vw - 40px));
            max-width: none;
            min-height: 100vh;
            overflow: visible;
            display: grid;
            grid-template-columns: minmax(320px, 1fr) minmax(0, 2fr);
            align-items: start;
            gap: 12px;
            padding: 14px 0 96px;
        }

        .page.desktop-page.web-style-mode,
        .page.desktop-page.desktop-style-mode {
            width: min(1600px, calc(100vw - 40px));
            height: 100vh;
            min-height: 0;
            overflow: hidden;
            display: block;
            padding: 18px 0;
        }

        .page.desktop-page.web-import-page,
        .page.desktop-page.desktop-import-page {
            width: auto;
            min-height: 100vh;
            grid-template-columns: minmax(0, 1fr);
            padding: 36px 128px;
            box-sizing: border-box;
        }

        .desktop-page .make-column {
            min-width: 0;
            display: grid;
            grid-auto-rows: auto;
            align-content: start;
            gap: 12px;
        }

        .desktop-page .left-column {
            grid-column: 1;
        }

        .desktop-page .right-column {
            grid-column: 2;
            min-height: auto;
            grid-template-rows: auto minmax(0, 1fr) auto;
            align-content: stretch;
        }

        .desktop-page .right-column.noIssues,
        .desktop-page .right-column.checkCollapsed {
            grid-template-rows: auto auto minmax(0, 1fr);
        }

        .desktop-page .empty-panel {
            grid-column: 1 / -1;
        }

        .desktop-page .empty-panel.web-import-panel,
        .desktop-page .empty-panel.desktop-import-panel {
            width: min(100%, 1792px);
            min-height: 342px;
            margin: 0 auto;
            align-content: center;
            gap: 16px;
            border: 1px solid rgba(23, 27, 36, 0.08);
            border-radius: 8px;
            padding: 48px 24px;
            box-sizing: border-box;
        }

        .desktop-page .empty-panel.web-import-panel h2,
        .desktop-page .empty-panel.desktop-import-panel h2 {
            font-size: 22px;
            line-height: 1.3;
        }

        .desktop-page .empty-panel.web-import-panel p,
        .desktop-page .empty-panel.desktop-import-panel p {
            margin-top: 0;
            color: #64748b;
        }

        .desktop-page .empty-panel.web-import-panel button,
        .desktop-page .empty-panel.desktop-import-panel button {
            height: 34px;
            min-height: 34px;
            min-width: 0;
            padding: 0 12px;
            border: 1px solid #1677b8;
            border-radius: 6px;
            background: #1677b8;
            color: #ffffff;
            font-weight: 800;
        }

        .desktop-page .topbar {
            display: none;
        }

        .desktop-page .meta,
        .desktop-page .regex-panel,
        .desktop-page .toc-panel,
        .desktop-page .check-panel,
        .desktop-page .reorder-panel,
        .desktop-page .web-style-head,
        .desktop-page .web-style-panel,
        .desktop-page .bottom-actions {
            margin-top: 0;
        }

        .desktop-page .web-style-page {
            grid-column: 1 / -1;
            height: 100%;
            min-height: 0;
            gap: 12px;
        }

        .desktop-page .web-style-head {
            grid-template-columns: minmax(0, 1fr) auto;
            align-items: center;
            margin-top: 0;
        }

        .desktop-page .web-style-actions {
            grid-template-columns: auto auto;
        }

        .desktop-page .web-style-actions button {
            min-width: 104px;
            min-height: 38px;
            padding: 0 14px;
        }

        .desktop-page .web-style-grid {
            height: 100%;
            min-height: 0;
            grid-template-columns: minmax(280px, 0.7fr) minmax(300px, 0.82fr) max-content;
            grid-template-rows: auto auto minmax(0, 1fr);
            align-items: start;
        }

        .desktop-page .web-style-panel {
            min-height: 0;
        }

        .desktop-page .web-style-head {
            grid-column: 1 / 3;
            grid-row: 1;
        }

        .desktop-page .web-style-grid > .web-style-panel:not(.web-preview-panel) {
            align-self: start;
        }

        .desktop-page .web-preview-panel {
            grid-column: 3;
            grid-row: 1 / 4;
            align-self: stretch;
            justify-self: end;
            padding: 12px;
        }

        .desktop-page .asset-grid {
            grid-template-columns: minmax(0, 0.78fr) minmax(0, 1fr);
            align-items: start;
        }

        .desktop-page .chapter-preview {
            height: 100%;
            min-height: 0;
        }

        .desktop-page .phone-preview-frame {
            width: calc((100vh - 62px) * 360 / 700);
            max-width: 100%;
            height: calc(100vh - 62px);
            aspect-ratio: 360 / 700;
        }

        .desktop-page .meta {
            grid-column: 1;
            grid-row: 1;
            grid-template-columns: minmax(0, 1fr) 116px;
            grid-template-areas:
                "head head"
                "title cover"
                "author cover"
                "uuid cover";
            align-items: stretch;
            gap: 8px 10px;
            padding: 10px 12px;
        }

        .desktop-page .meta.collapsed {
            grid-template-columns: minmax(0, 1fr);
            grid-template-areas: "head";
        }

        .desktop-page .meta-section-head {
            grid-area: head;
            margin-bottom: 0;
        }

        .desktop-page .meta-top {
            display: contents;
        }

        .desktop-page .meta-main {
            display: contents;
        }

        .desktop-page .title-field {
            grid-area: title;
        }

        .desktop-page .author-field {
            grid-area: author;
        }

        .desktop-page .cover-field {
            grid-area: cover;
            min-height: 0;
            display: grid;
            grid-template-rows: auto minmax(0, 1fr);
        }

        .desktop-page .meta label {
            gap: 5px;
        }

        .desktop-page .meta label span {
            font-size: 12px;
        }

        .desktop-page .meta input {
            min-height: 36px;
            padding: 6px 10px;
            font-size: 13px;
        }

        .desktop-page .uuid-row {
            grid-area: uuid;
        }

        .desktop-page .cover-box {
            height: auto;
            min-height: 132px;
        }

        .desktop-page .cover-box b {
            max-width: 5em;
        }

        .desktop-page .bottom-actions {
            grid-template-columns: auto auto minmax(0, 1fr);
            align-items: center;
        }

        .desktop-page .make-column > .meta,
        .desktop-page .make-column > .regex-panel,
        .desktop-page .make-column > .toc-panel,
        .desktop-page .make-column > .check-panel,
        .desktop-page .make-column > .reorder-panel,
        .desktop-page .make-column > .bottom-actions {
            grid-column: auto;
            grid-row: auto;
            align-self: start;
        }

        .desktop-page .right-column > .check-panel.expanded {
            align-self: stretch;
            display: grid;
            grid-template-rows: auto minmax(0, 1fr);
        }

        .desktop-page .right-column > .check-panel.collapsed {
            align-self: start;
        }

        .desktop-page .right-column.checkCollapsed > .reorder-panel.expanded,
        .desktop-page .right-column.noIssues > .reorder-panel.expanded {
            align-self: stretch;
            display: grid;
            grid-template-rows: auto auto auto minmax(0, 1fr);
        }

        .desktop-page .right-column > .check-panel.expanded .check-list,
        .desktop-page .right-column.checkCollapsed > .reorder-panel.expanded .reorder-preview,
        .desktop-page .right-column.noIssues > .reorder-panel.expanded .reorder-preview {
            max-height: none;
        }

        .desktop-page .bottom-actions button {
            min-width: 128px;
            padding: 0 18px;
        }

        .desktop-page .toc-list {
            max-height: clamp(300px, calc(100vh - 390px), 560px);
            overflow: auto;
        }

        .desktop-page .toc-row,
        .desktop-page .toc-main {
            min-height: 38px;
        }

        .desktop-page .toc-main {
            padding: 4px 0;
        }

        .desktop-page .toc-row strong {
            font-size: 12px;
        }

        .desktop-page .regex-panel,
        .desktop-page .check-panel,
        .desktop-page .reorder-panel,
        .desktop-page .toc-panel {
            padding: 12px;
        }

        .desktop-page .section-head,
        .desktop-page .toc-list {
            margin-top: 0;
            margin-bottom: 6px;
        }

        .desktop-page .toc-actions {
            margin-bottom: 6px;
        }

        .desktop-page .reorder-options {
            grid-template-columns: repeat(4, minmax(0, 1fr));
            gap: 8px;
            align-items: end;
        }

        .desktop-page .check-toggle {
            min-height: 36px;
        }

        .desktop-page .check-actions {
            grid-template-columns: repeat(2, minmax(0, 140px));
        }

        .desktop-page .reorder-preview {
            min-height: 0;
            max-height: clamp(180px, calc(100vh - 560px), 360px);
            overflow: auto;
        }

        .desktop-page .reorder-head,
        .desktop-page .reorder-row {
            min-height: 30px;
            padding: 0 8px;
        }

        .desktop-page .check-list {
            min-height: 0;
            max-height: clamp(180px, calc(100vh - 520px), 360px);
            grid-auto-rows: 40px;
            overflow: auto;
        }

        .desktop-page .check-row {
            grid-template-columns: 86px 160px minmax(0, 1fr);
            grid-template-areas: "type meta title";
            height: 40px;
            min-height: 40px;
            align-items: center;
        }

        .desktop-page .check-row small {
            justify-self: start;
            white-space: nowrap;
        }
    }
</style>
