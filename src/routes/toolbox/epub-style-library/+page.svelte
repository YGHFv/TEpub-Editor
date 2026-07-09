<script lang="ts">
  import { onMount } from "svelte";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import {
    EPUB_HEADER_STYLES,
    EPUB_HEADER_PREVIEW_TITLE_CSS,
    EPUB_STYLE_INTERFACE,
    EPUB_STYLE_INTERFACE_NOTES,
    EPUB_TITLE_STYLES,
    type EpubStyleKind,
    type EpubStyleModule,
    type EpubTitleLayout,
  } from "$lib/epubStyleLibrary";

  type ViewMode = EpubStyleKind | "interface";
  type PanelMode = "browse" | "create";
  type TitleInputMode = "parts" | "full";
  type HeaderDraft = {
    fileName: string;
    sampleDataUrl: string;
    width: number;
    height: number;
    originalWidth: number;
    originalHeight: number;
  };

  const STORAGE_KEY = "tepub-epub-style-library-v1";
  const HEADER_TEMPLATE_LONG_EDGE = 1080;
  const DEFAULT_HEADER_TITLE_STYLE_ID = "title-purple-red-emphasis";
  const FIXED_HEADER_PREVIEW_URL = new URL("../../../lib/assets/epub-style-library/fixed-preview-header.png", import.meta.url).href;
  const SAFE_CLASS_NAMES = new Set([
    "te-book-body",
    "te-chapter-page",
    "te-chapter-page--no-image",
    "te-volume-page",
    "te-volume-page--no-image",
    "te-intro-page",
    "te-header-figure",
    "te-header-image",
    "te-header-caption",
    "te-intro-title",
    "te-volume-title",
    "te-volume-subtitle",
    "te-chapter-title",
    "te-chapter-number",
    "te-chapter-name",
    "te-paragraph",
    "te-divider-line",
  ]);
  const TITLE_STYLE_CLASS_NAMES = new Set([
    "te-book-body",
    "te-chapter-page",
    "te-chapter-page--no-image",
    "te-chapter-title",
    "te-chapter-number",
    "te-chapter-name",
  ]);

  let viewMode: ViewMode = "header";
  let panelMode: PanelMode = "browse";
  let selectedStyleId = EPUB_HEADER_STYLES[0]?.id || "";
  let savedStyles: EpubStyleModule[] = [];
  let copyMessage = "";

  let headerInput: HTMLInputElement | null = null;
  let headerName = "";
  let headerTitleStyleId = DEFAULT_HEADER_TITLE_STYLE_ID;
  let headerDraft: HeaderDraft | null = null;
  let headerMessage = "";

  let titleName = "";
  let titleInputMode: TitleInputMode = "parts";
  let titleLayout: EpubTitleLayout = "split";
  let titleNumberCss = "font-family: \"llf\", \"黑体\", sans-serif;\nfont-weight: 900;\nfont-size: 0.8em;\ncolor: #413245;\nline-height: 1.3;";
  let titleNameCss = "font-family: \"llf\", \"黑体\", sans-serif;\nfont-size: 1.2em;\nfont-weight: 900;\ncolor: #c2181e;";
  let titleFullCss = `.te-chapter-title {
  font-family: "llf", "黑体", sans-serif;
  text-align: center;
  font-weight: 900;
  font-size: 0.8em;
  margin: 1em 0 3em;
  color: #413245;
  line-height: 1.3;
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-chapter-number {
  display: block;
  color: #413245;
}

.te-chapter-name {
  display: block;
  font-family: "llf", "黑体", sans-serif;
  font-size: 1.2em;
  font-weight: 900;
  color: #c2181e;
}`;
  let titleMessage = "";

  $: headerStyles = [...EPUB_HEADER_STYLES, ...savedStyles.filter((style) => style.kind === "header")];
  $: titleStyles = [...EPUB_TITLE_STYLES, ...savedStyles.filter((style) => style.kind === "title" && style.target === "chapter-title")];
  $: currentStyles = viewMode === "title" ? titleStyles : headerStyles;
  $: if (viewMode !== "interface" && panelMode === "browse" && !currentStyles.some((style) => style.id === selectedStyleId)) {
    selectedStyleId = currentStyles[0]?.id || "";
  }
  $: selectedStyle = currentStyles.find((style) => style.id === selectedStyleId) || currentStyles[0];
  $: titleCssError = validateTitleCssOnly();
  $: titleStyleOptions = titleStyles.map((style) => ({
    value: style.id,
    label: style.name,
    meta: style.sourceKind === "saved" ? "自定义" : "内置",
  }));
  $: selectedPreviewDoc = (
    selectedStyle,
    titleStyles,
    savedStyles,
    selectedStyle ? buildPreviewDoc(selectedStyle) : ""
  );
  $: draftStyle = (
    viewMode,
    headerDraft,
    headerName,
    headerTitleStyleId,
    titleCssError,
    titleName,
    titleInputMode,
    titleLayout,
    titleNumberCss,
    titleNameCss,
    titleFullCss,
    buildDraftStyle()
  );
  $: draftPreviewDoc = (
    draftStyle,
    titleStyles,
    draftStyle ? buildPreviewDoc(draftStyle) : ""
  );
  $: interfaceRows = Object.entries(EPUB_STYLE_INTERFACE).map(([slot, selector]) => ({
    slot,
    selector,
    note: EPUB_STYLE_INTERFACE_NOTES[slot as keyof typeof EPUB_STYLE_INTERFACE] || "",
  }));

  onMount(() => {
    savedStyles = loadSavedStyles();
  });

  function setMode(mode: ViewMode) {
    viewMode = mode;
    panelMode = "browse";
    if (mode === "header") selectedStyleId = headerStyles[0]?.id || EPUB_HEADER_STYLES[0]?.id || "";
    if (mode === "title") selectedStyleId = titleStyles[0]?.id || EPUB_TITLE_STYLES[0]?.id || "";
  }

  function startCreate(kind: EpubStyleKind) {
    viewMode = kind;
    panelMode = "create";
    copyMessage = "";
    if (kind === "header" && !titleStyles.some((style) => style.id === headerTitleStyleId)) {
      headerTitleStyleId = titleStyles[0]?.id || "";
    }
  }

  function loadSavedStyles() {
    if (typeof localStorage === "undefined") return [];
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      return Array.isArray(parsed?.styles)
        ? parsed.styles
          .filter((style: Partial<EpubStyleModule>) => style && style.id && style.kind && style.name && style.css)
          .map(normalizeSavedStyle)
        : [];
    } catch (error) {
      console.warn("读取 EPUB 样式库失败:", error);
      return [];
    }
  }

  function normalizeSavedStyle(style: EpubStyleModule) {
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

  function persistSavedStyles(nextStyles = savedStyles) {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, styles: nextStyles }));
  }

  function makeStyleId(prefix: string) {
    return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  }

  function resolveTitleStyle(id: string | undefined) {
    return titleStyles.find((style) => style.id === id)
      || titleStyles.find((style) => style.id === DEFAULT_HEADER_TITLE_STYLE_ID)
      || titleStyles[0]
      || EPUB_TITLE_STYLES[0];
  }

  function resolveHeaderTitleStyle(style: EpubStyleModule) {
    return resolveTitleStyle(style.boundTitleStyleId);
  }

  function buildPreviewDoc(style: EpubStyleModule) {
    const boundTitleStyle = style.kind === "header" ? resolveHeaderTitleStyle(style) : null;
    const hasHeaderPreview = style.kind === "header";
    const previewHtml = style.kind === "header"
      ? headerPreviewHtml(style.sampleDataUrl || "", resolveTitleLayout(boundTitleStyle))
      : (style.previewHtml || fallbackPreviewHtml(style));
    const previewCss = style.kind === "header"
      ? `${boundTitleStyle?.css || EPUB_HEADER_PREVIEW_TITLE_CSS}\n\n${style.css}`
      : style.css;
    const spacingCss = hasHeaderPreview
      ? `.te-header-figure {
        margin-bottom: 1.15em;
      }

      .te-header-figure + .te-chapter-title {
        margin-top: 0.85em;
      }

      .te-chapter-title {
        margin-bottom: 2.15em;
      }`
      : `.te-chapter-title {
        margin-top: 2.65em;
        margin-bottom: 2.45em;
      }

      p.te-paragraph:first-of-type {
        margin-top: 0;
      }`;
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
        width: 100%;
        height: 100%;
        box-sizing: border-box;
        padding: ${hasHeaderPreview ? "0 24px 42px" : "46px 24px 42px"};
        overflow: hidden;
        background: #fffdf8;
      }

      .te-preview-header-page {
        padding-top: 0;
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

      .te-header-placeholder {
        min-height: 190px;
        background:
          linear-gradient(135deg, rgba(14, 116, 144, 0.7), rgba(15, 23, 42, 0.85)),
          radial-gradient(circle at 22% 28%, rgba(255, 251, 235, 0.85), transparent 24%),
          linear-gradient(150deg, #93c5fd, #fda4af);
      }
    `;
    return `<!doctype html>
      <html>
        <head>
          <meta charset="utf-8" />
          <style>${baseCss}</style>
          <style>${previewCss}</style>
          <style>${spacingCss}</style>
        </head>
        <body>${previewHtml}</body>
      </html>`;
  }

  function resolveTitleLayout(style: EpubStyleModule | null | undefined): EpubTitleLayout {
    if (style?.titleLayout) return style.titleLayout;
    return style?.titleNameCss || style?.titleCssB ? "split" : "single";
  }

  function fallbackPreviewHtml(style: EpubStyleModule) {
    if (style.kind === "header") {
      return headerPreviewHtml(style.sampleDataUrl || "");
    }
    return titlePreviewHtml(resolveTitleLayout(style));
  }

  function sampleParagraphs() {
    return `
      <p class="te-paragraph">夜色沉入城市边缘，风从旧站台吹过，带着潮湿的铁锈味。</p>
      <p class="te-paragraph">她合上手中的书，抬头看见远处灯塔亮起，像一枚缓慢落下的星。</p>
    `;
  }

  function titleMarkup(mode: EpubTitleLayout) {
    return mode === "split"
      ? `<h3 class="te-chapter-title"><span class="te-chapter-number">第十二章</span><span class="te-chapter-name">灯塔来信</span></h3>`
      : `<h3 class="te-chapter-title">第十二章 灯塔来信</h3>`;
  }

  function headerPreviewHtml(dataUrl: string, titleMode: EpubTitleLayout = "split") {
    const image = dataUrl
      ? `<img class="te-header-image" src="${dataUrl}" alt="" />`
      : `<div class="te-header-image te-header-placeholder"></div>`;
    return `
      <main class="te-preview-page te-preview-header-page">
        <figure class="te-header-figure" aria-label="头图预览">
          ${image}
        </figure>
        ${titleMarkup(titleMode)}
        ${sampleParagraphs()}
      </main>
    `;
  }

  function titlePreviewHtml(mode: EpubTitleLayout) {
    return `<main class="te-preview-page">${titleMarkup(mode)}${sampleParagraphs()}</main>`;
  }

  function headerBaseCss() {
    return `.te-header-figure {
  margin: 0 -1.5em 1.6em;
  padding: 0;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  duokan-bleed: lefttopright;
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
  }

  function normalizeDeclarations(css: string) {
    const trimmed = css.trim();
    if (!trimmed) return "";
    return /;\s*$/.test(trimmed) ? trimmed : `${trimmed};`;
  }

  function buildTitlePartsCss() {
    const numberCss = normalizeDeclarations(titleNumberCss);
    const nameCss = normalizeDeclarations(titleNameCss);
    if (titleLayout === "split") {
      return `.te-chapter-title {
  margin: 2em 0 3em;
  text-align: center;
  text-indent: 0;
}

.te-chapter-number {
  display: block;
  ${numberCss}
}

.te-chapter-name {
  display: block;
  ${nameCss}
}`;
    }
    return `.te-chapter-title {
  margin: 2em 0 3em;
  text-align: center;
  text-indent: 0;
  ${nameCss}
}`;
  }

  function buildDraftStyle(): EpubStyleModule | null {
    if (viewMode === "header") {
      if (!headerDraft) return null;
      return {
        id: "header-draft",
        kind: "header",
        target: "header-image",
        sourceKind: "saved",
        name: headerName.trim() || "未命名头图样式",
        description: `样板长边已规范为 ${HEADER_TEMPLATE_LONG_EDGE}px，保存生成后的透明样板图，尺寸 ${headerDraft.width}x${headerDraft.height}。`,
        selectors: [
          "body.te-chapter-page",
          EPUB_STYLE_INTERFACE.headerFigure,
          EPUB_STYLE_INTERFACE.headerImage,
          EPUB_STYLE_INTERFACE.headerCaption,
        ],
        usage: "保存后制作 EPUB 可用该透明样板生成贴边章节头图。",
        css: headerBaseCss(),
        previewHtml: headerPreviewHtml(headerDraft.sampleDataUrl, resolveTitleLayout(resolveTitleStyle(headerTitleStyleId))),
        sampleDataUrl: headerDraft.sampleDataUrl,
        sampleWidth: headerDraft.width,
        sampleHeight: headerDraft.height,
        originalSampleWidth: headerDraft.originalWidth,
        originalSampleHeight: headerDraft.originalHeight,
        boundTitleStyleId: headerTitleStyleId || undefined,
      };
    }
    if (viewMode === "title") {
      if (titleCssError) return null;
      const css = titleInputMode === "full" ? titleFullCss.trim() : buildTitlePartsCss();
      const split = titleLayout === "split" || /\.te-chapter-(?:number|name)\b/.test(css);
      return {
        id: "title-draft",
        kind: "title",
        target: "chapter-title",
        sourceKind: "saved",
        name: titleName.trim() || "未命名标题样式",
        description: titleInputMode === "full" ? "自定义完整 CSS 定义。" : (split ? "number/name 双行标题样式。" : "name 单行标题样式。"),
        selectors: split
          ? [EPUB_STYLE_INTERFACE.chapterTitle, EPUB_STYLE_INTERFACE.chapterNumber, EPUB_STYLE_INTERFACE.chapterName]
          : [EPUB_STYLE_INTERFACE.chapterTitle],
        usage: titleInputMode === "full" ? "保存完整 CSS，制作时按命名接口自动应用。" : "保存 number/name 样式内容，制作时自动套到标题结构。",
        css,
        previewHtml: titlePreviewHtml(split ? "split" : "single"),
        titleLayout: split ? "split" : "single",
        titleNumberCss: titleInputMode === "parts" && split ? titleNumberCss.trim() : "",
        titleNameCss: titleInputMode === "parts" ? titleNameCss.trim() : "",
      };
    }
    return null;
  }

  function validateUnsafeCss(css: string) {
    if (/<\/?\s*script/i.test(css)) return "CSS 中不能包含 script。";
    if (/@import/i.test(css)) return "CSS 中暂不允许 @import。";
    if (/javascript\s*:/i.test(css)) return "CSS 中不能包含 javascript: URL。";
    return "";
  }

  function validateDeclarationBlock(label: string, css: string, required: boolean) {
    const trimmed = css.trim();
    if (!trimmed) return required ? `${label} 不能为空。` : "";
    const unsafe = validateUnsafeCss(trimmed);
    if (unsafe) return unsafe;
    if (/[{}]/.test(trimmed)) return `${label} 只填写样式内容，不要写选择器或大括号。`;
    return "";
  }

  function validateFullCss(css: string) {
    const trimmed = css.trim();
    if (!trimmed) return "完整 CSS 不能为空。";
    const unsafe = validateUnsafeCss(trimmed);
    if (unsafe) return unsafe;
    if (!/\.[A-Za-z_][\w-]*/.test(trimmed)) return "完整 CSS 至少要包含一个标准 class 选择器。";
    const classNames = Array.from(trimmed.matchAll(/\.([A-Za-z_][\w-]*)/g)).map((match) => match[1]);
    const invalid = classNames.filter((className) => !SAFE_CLASS_NAMES.has(className));
    if (invalid.length) return `存在不符合命名规范的 class：${Array.from(new Set(invalid)).join(", ")}`;
    const offTarget = classNames.filter((className) => !TITLE_STYLE_CLASS_NAMES.has(className));
    if (offTarget.length) return `标题样式暂只允许章节标题接口：${Array.from(new Set(offTarget)).join(", ")} 将放到单独样式列表处理。`;
    return "";
  }

  function validateTitleCssOnly() {
    if (viewMode !== "title") return "";
    if (titleInputMode === "full") return validateFullCss(titleFullCss);
    if (titleLayout === "split") {
      return validateDeclarationBlock("number 样式", titleNumberCss, true) || validateDeclarationBlock("name 样式", titleNameCss, true);
    }
    return validateDeclarationBlock("name 样式", titleNameCss, true);
  }

  function validateTitleForSave() {
    if (!titleName.trim()) return "请填写样式名称。";
    return validateTitleCssOnly();
  }

  function readFileAsDataUrl(file: File) {
    return new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ""));
      reader.onerror = () => reject(reader.error || new Error("读取图片失败"));
      reader.readAsDataURL(file);
    });
  }

  function imageFromDataUrl(dataUrl: string) {
    return new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error("图片加载失败"));
      image.src = dataUrl;
    });
  }

  function drawImageCover(ctx: CanvasRenderingContext2D, image: HTMLImageElement, width: number, height: number) {
    const imageWidth = image.naturalWidth || image.width || 1;
    const imageHeight = image.naturalHeight || image.height || 1;
    const scale = Math.max(width / imageWidth, height / imageHeight);
    const drawWidth = imageWidth * scale;
    const drawHeight = imageHeight * scale;
    const drawX = (width - drawWidth) / 2;
    const drawY = (height - drawHeight) / 2;
    ctx.drawImage(image, drawX, drawY, drawWidth, drawHeight);
  }

  async function importHeaderSample(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      headerMessage = "请选择图片样板。";
      input.value = "";
      return;
    }
    try {
      const dataUrl = await readFileAsDataUrl(file);
      const image = await imageFromDataUrl(dataUrl);
      const originalWidth = image.naturalWidth || image.width;
      const originalHeight = image.naturalHeight || image.height;
      const longEdge = Math.max(originalWidth, originalHeight, 1);
      const scale = HEADER_TEMPLATE_LONG_EDGE / longEdge;
      const width = Math.max(1, Math.round(originalWidth * scale));
      const height = Math.max(1, Math.round(originalHeight * scale));

      const templateCanvas = document.createElement("canvas");
      templateCanvas.width = width;
      templateCanvas.height = height;
      const templateCtx = templateCanvas.getContext("2d");
      if (!templateCtx) throw new Error("无法创建画布。");
      templateCtx.imageSmoothingEnabled = true;
      templateCtx.imageSmoothingQuality = "high";
      templateCtx.drawImage(image, 0, 0, width, height);

      const defaultCanvas = document.createElement("canvas");
      defaultCanvas.width = width;
      defaultCanvas.height = height;
      const defaultCtx = defaultCanvas.getContext("2d");
      if (!defaultCtx) throw new Error("无法创建默认头图画布。");
      const fixedHeaderImage = await imageFromDataUrl(FIXED_HEADER_PREVIEW_URL);
      defaultCtx.imageSmoothingEnabled = true;
      defaultCtx.imageSmoothingQuality = "high";
      drawImageCover(defaultCtx, fixedHeaderImage, width, height);
      defaultCtx.save();
      defaultCtx.globalCompositeOperation = "destination-in";
      defaultCtx.drawImage(templateCanvas, 0, 0);
      defaultCtx.restore();

      headerDraft = {
        fileName: file.name,
        sampleDataUrl: defaultCanvas.toDataURL("image/png"),
        width,
        height,
        originalWidth,
        originalHeight,
      };
      if (!headerName.trim()) headerName = file.name.replace(/\.[^.]+$/, "");
      headerMessage = `已按长边 ${HEADER_TEMPLATE_LONG_EDGE}px 生成遮罩预览：${width}x${height}（原图 ${originalWidth}x${originalHeight}）。`;
    } catch (error) {
      headerDraft = null;
      headerMessage = `导入失败：${String(error)}`;
    } finally {
      input.value = "";
    }
  }

  function saveHeaderStyle() {
    if (!headerName.trim()) {
      headerMessage = "请填写样式名称。";
      return;
    }
    if (!headerDraft) {
      headerMessage = "请先导入样板图片。";
      return;
    }
    const style: EpubStyleModule = {
      ...buildDraftStyle()!,
      id: makeStyleId("header"),
      name: headerName.trim(),
      sourceKind: "saved",
    };
    const nextStyles = [style, ...savedStyles];
    savedStyles = nextStyles;
    persistSavedStyles(nextStyles);
    selectedStyleId = style.id;
    panelMode = "browse";
    headerMessage = "已保存到样式库。";
  }

  function updateSelectedHeaderTitleBinding(titleStyleId: string) {
    if (!selectedStyle || selectedStyle.kind !== "header" || selectedStyle.sourceKind !== "saved") return;
    const titleStyle = resolveTitleStyle(titleStyleId);
    const nextStyles = savedStyles.map((style) => {
      if (style.id !== selectedStyle.id) return style;
      return {
        ...style,
        boundTitleStyleId: titleStyle?.id,
        previewHtml: headerPreviewHtml(style.sampleDataUrl || "", resolveTitleLayout(titleStyle)),
      };
    });
    savedStyles = nextStyles;
    persistSavedStyles(nextStyles);
  }

  function saveTitleStyle() {
    const error = validateTitleForSave();
    if (error) {
      titleMessage = error;
      return;
    }
    const draft = buildDraftStyle();
    if (!draft) {
      titleMessage = "请先确认样式预览无误。";
      return;
    }
    const style: EpubStyleModule = {
      ...draft,
      id: makeStyleId("title"),
      name: titleName.trim(),
      sourceKind: "saved",
    };
    const nextStyles = [style, ...savedStyles];
    savedStyles = nextStyles;
    persistSavedStyles(nextStyles);
    selectedStyleId = style.id;
    panelMode = "browse";
    titleMessage = "已保存到样式库。";
  }

  function deleteSelectedStyle() {
    if (!selectedStyle || selectedStyle.sourceKind !== "saved") return;
    const nextStyles = savedStyles.filter((style) => style.id !== selectedStyle.id);
    savedStyles = nextStyles;
    persistSavedStyles(nextStyles);
    selectedStyleId = currentStyles.find((style) => style.sourceKind !== "saved")?.id || currentStyles[0]?.id || "";
  }

  async function copyCss() {
    if (!selectedStyle) return;
    try {
      await navigator.clipboard.writeText(selectedStyle.css);
      copyMessage = "已复制 CSS";
    } catch (error) {
      copyMessage = `复制失败：${String(error)}`;
    }
    window.setTimeout(() => {
      copyMessage = "";
    }, 1800);
  }
</script>

<svelte:head>
  <title>EPUB 样式库 - TEpub Editor</title>
</svelte:head>

<div class="style-library-page">
  <main class="library-workspace">
    <aside class="style-sidebar">
      <div class="mode-tabs" aria-label="样式分类">
        <button type="button" class:active={viewMode === "header"} on:click={() => setMode("header")}>头图样式</button>
        <button type="button" class:active={viewMode === "title"} on:click={() => setMode("title")}>标题样式</button>
        <button type="button" class:active={viewMode === "interface"} on:click={() => setMode("interface")}>接口</button>
      </div>

      {#if viewMode === "interface"}
        <div class="interface-panel">
          <h2>标准命名接口</h2>
          <p>完整 CSS 只能使用这些 `te-*` 命名，制作逻辑后续按接口自动套用。</p>
          <div class="interface-list">
            {#each interfaceRows as row}
              <div>
                <strong>{row.slot}</strong>
                <code>{row.selector}</code>
                <small>{row.note}</small>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <div class="sidebar-actions">
          <button type="button" class:active={panelMode === "browse"} on:click={() => (panelMode = "browse")}>浏览</button>
          <button type="button" class:active={panelMode === "create"} on:click={() => startCreate(viewMode as EpubStyleKind)}>新增</button>
        </div>
        <div class="style-list">
          {#each currentStyles as style}
            <button
              type="button"
              class="style-row"
              class:active={panelMode === "browse" && selectedStyleId === style.id}
              on:click={() => {
                panelMode = "browse";
                selectedStyleId = style.id;
              }}
            >
              <span>{style.name}</span>
              <small>{style.sourceKind === "saved" ? "自定义" : "内置"} · {style.description}</small>
            </button>
          {/each}
        </div>
      {/if}
    </aside>

    <section class="preview-pane">
      {#if viewMode === "interface"}
        <div class="interface-preview">
          <h2>后续接入方式</h2>
          <p>头图样式保存生成后的透明样板图；标题样式保存自动生成 CSS 或完整 CSS。制作 EPUB 时只需要按标准 class 输出结构。</p>
          <pre>{JSON.stringify(EPUB_STYLE_INTERFACE, null, 2)}</pre>
        </div>
      {:else if panelMode === "create" && viewMode === "header"}
        <div class="preview-head">
          <div>
            <h2>新增头图样式</h2>
            <p>样板图片导入后长边统一调整为 1080，并生成贴边默认头图预览。</p>
          </div>
          <button type="button" on:click={saveHeaderStyle}>确认保存</button>
        </div>
        <div class="create-grid">
          <form class="create-form" on:submit|preventDefault={saveHeaderStyle}>
            <label>
              <span>样式名称</span>
              <input type="text" bind:value={headerName} placeholder="例如：卷首横幅模板" />
            </label>
            <label>
              <span>绑定标题样式</span>
              <CustomSelect
                className="style-binding-select"
                value={headerTitleStyleId}
                options={titleStyleOptions}
                on:change={(event) => (headerTitleStyleId = event.detail)}
              />
            </label>
            <div class="file-row">
              <input bind:this={headerInput} class="file-input" type="file" accept="image/*" on:change={importHeaderSample} />
              <button type="button" on:click={() => headerInput?.click()}>导入样板图片</button>
              <span>{headerDraft?.fileName || "未导入"}</span>
            </div>
            {#if headerDraft}
              <div class="meta-grid">
                <span>原始尺寸</span><strong>{headerDraft.originalWidth}x{headerDraft.originalHeight}</strong>
                <span>保存尺寸</span><strong>{headerDraft.width}x{headerDraft.height}</strong>
              </div>
            {/if}
            {#if headerMessage}
              <p class="form-message">{headerMessage}</p>
            {/if}
          </form>
          <div class="live-preview">
            {#if draftPreviewDoc}
              <iframe title="头图样式预览" srcdoc={draftPreviewDoc}></iframe>
            {:else}
              <div class="preview-empty">导入样板后在这里预览贴边显示的默认头图。</div>
            {/if}
          </div>
        </div>
      {:else if panelMode === "create" && viewMode === "title"}
        <div class="preview-head">
          <div>
            <h2>新增标题样式</h2>
            <p>分段模式只填 number/name 样式内容；全部定义模式必须使用标准 `te-*` 命名。</p>
          </div>
          <button type="button" on:click={saveTitleStyle} disabled={!!titleCssError}>确认保存</button>
        </div>
        <div class="create-grid">
          <form class="create-form title-form" on:submit|preventDefault={saveTitleStyle}>
            <label>
              <span>样式名称</span>
              <input type="text" bind:value={titleName} placeholder="例如：红色双行章题" />
            </label>
            <div class="segmented">
              <button type="button" class:active={titleInputMode === "parts"} on:click={() => (titleInputMode = "parts")}>分段样式</button>
              <button type="button" class:active={titleInputMode === "full"} on:click={() => (titleInputMode = "full")}>全部定义</button>
            </div>
            <div class="segmented">
              <button type="button" class:active={titleLayout === "split"} on:click={() => (titleLayout = "split")}>双行</button>
              <button type="button" class:active={titleLayout === "single"} on:click={() => (titleLayout = "single")}>单行</button>
            </div>
            {#if titleInputMode === "parts"}
              {#if titleLayout === "split"}
                <label>
                  <span>number 样式</span>
                  <textarea rows="6" bind:value={titleNumberCss} placeholder="章节序号样式，只写 CSS 声明"></textarea>
                </label>
              {/if}
              <label>
                <span>name 样式</span>
                <textarea rows="6" bind:value={titleNameCss} placeholder="章节名样式，只写 CSS 声明"></textarea>
              </label>
            {:else}
              <label>
                <span>完整 CSS</span>
                <textarea rows="15" bind:value={titleFullCss} placeholder=".te-chapter-title &#123; ... &#125;"></textarea>
              </label>
            {/if}
            {#if titleCssError || titleMessage}
              <p class:error={!!titleCssError} class="form-message">{titleCssError || titleMessage}</p>
            {/if}
          </form>
          <div class="live-preview">
            {#if draftPreviewDoc}
              <iframe title="标题样式预览" srcdoc={draftPreviewDoc}></iframe>
            {:else}
              <div class="preview-empty">修正 CSS 后在这里预览标题样式。</div>
            {/if}
          </div>
        </div>
      {:else if selectedStyle}
        <div class="preview-head">
          <div>
            <h2>{selectedStyle.name}</h2>
            <p>{selectedStyle.usage}</p>
          </div>
          <div class="preview-actions">
            {#if selectedStyle.sourceKind === "saved"}
              <button type="button" class="danger" on:click={deleteSelectedStyle}>删除</button>
            {/if}
            <button type="button" on:click={copyCss}>复制 CSS</button>
          </div>
        </div>

        <div class="preview-grid">
          <div class="live-preview">
            <iframe title={`${selectedStyle.name} 预览`} srcdoc={selectedPreviewDoc}></iframe>
          </div>
          <div class="style-detail">
            <section>
              <h3>接口选择器</h3>
              <div class="selector-list">
                {#each selectedStyle.selectors as selector}
                  <code>{selector}</code>
                {/each}
              </div>
            </section>
            {#if selectedStyle.kind === "header" && selectedStyle.sampleWidth && selectedStyle.sampleHeight}
              <section>
                <h3>头图样板</h3>
                <div class="meta-grid compact">
                  <span>保存尺寸</span><strong>{selectedStyle.sampleWidth}x{selectedStyle.sampleHeight}</strong>
                  <span>原始尺寸</span><strong>{selectedStyle.originalSampleWidth || "-"}x{selectedStyle.originalSampleHeight || "-"}</strong>
                </div>
              </section>
            {/if}
            {#if selectedStyle.kind === "header"}
              <section>
                <h3>绑定标题样式</h3>
                {#if selectedStyle.sourceKind === "saved"}
                  <CustomSelect
                    className="style-binding-select detail-binding-select"
                    value={resolveHeaderTitleStyle(selectedStyle)?.id || ""}
                    options={titleStyleOptions}
                    on:change={(event) => updateSelectedHeaderTitleBinding(event.detail)}
                  />
                {:else}
                  <div class="readonly-binding">{resolveHeaderTitleStyle(selectedStyle)?.name || "默认标题样式"}</div>
                {/if}
              </section>
            {/if}
            <section>
              <h3>CSS</h3>
              <pre>{selectedStyle.css}</pre>
            </section>
            {#if copyMessage}
              <p class="copy-message">{copyMessage}</p>
            {/if}
          </div>
        </div>
      {/if}
    </section>
  </main>
</div>

<style>
  .style-library-page {
    min-height: 100vh;
    height: 100vh;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    overflow: hidden;
    background: #eef2f7;
    color: #172033;
  }

  .preview-head h2,
  .preview-head p,
  .interface-panel h2,
  .interface-panel p,
  .interface-preview h2,
  .interface-preview p {
    margin: 0;
  }

  .preview-head p,
  .interface-panel p,
  .interface-preview p {
    color: #64748b;
    font-size: 13px;
    line-height: 1.55;
  }

  .library-workspace {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(260px, 320px) minmax(0, 1fr);
    overflow: hidden;
  }

  .style-sidebar {
    min-height: 0;
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    border-right: 1px solid #d8e0eb;
    background: #ffffff;
    overflow: hidden;
  }

  .mode-tabs,
  .sidebar-actions,
  .segmented {
    display: grid;
    gap: 6px;
  }

  .mode-tabs {
    grid-template-columns: repeat(3, 1fr);
    padding: 12px;
    border-bottom: 1px solid #e2e8f0;
  }

  .sidebar-actions {
    grid-template-columns: repeat(2, 1fr);
    padding: 10px 12px;
    border-bottom: 1px solid #e2e8f0;
  }

  .segmented {
    grid-template-columns: repeat(2, 1fr);
  }

  button {
    height: 34px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    color: #334155;
    font-weight: 800;
    cursor: pointer;
  }

  button:hover {
    border-color: #1677b8;
    color: #0f5d91;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.48;
  }

  button.active,
  .style-row.active {
    border-color: #1677b8;
    background: #e8f2f8;
    color: #0f5d91;
  }

  button.danger {
    border-color: #fecaca;
    color: #b91c1c;
  }

  .style-list {
    min-height: 0;
    max-height: 100%;
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 12px 12px 28px;
    overflow: auto;
    overscroll-behavior: contain;
  }

  .style-row {
    height: auto;
    min-height: 72px;
    display: grid;
    gap: 5px;
    padding: 10px;
    text-align: left;
  }

  .style-row span {
    font-size: 15px;
  }

  .style-row small {
    color: #64748b;
    font-size: 12px;
    line-height: 1.45;
    font-weight: 700;
  }

  .preview-pane {
    min-width: 0;
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    overflow: hidden;
  }

  .preview-head {
    min-height: 72px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 18px;
    border-bottom: 1px solid #d8e0eb;
    background: #ffffff;
    box-sizing: border-box;
  }

  .preview-head h2,
  .interface-preview h2,
  .interface-panel h2 {
    font-size: 18px;
  }

  .preview-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preview-grid,
  .create-grid {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(360px, 48%) minmax(0, 1fr);
    overflow: hidden;
  }

  .create-grid {
    grid-template-columns: minmax(320px, 420px) minmax(0, 1fr);
  }

  .live-preview {
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 18px;
    background: #e5ebf3;
    overflow: hidden;
  }

  .live-preview iframe {
    width: min(100%, 360px, calc((100vh - 142px) * 360 / 700));
    height: auto;
    aspect-ratio: 360 / 700;
    display: block;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    box-shadow: 0 18px 42px rgba(15, 23, 42, 0.16);
    overflow: hidden;
  }

  .preview-empty {
    width: min(100%, 420px);
    min-height: 240px;
    display: grid;
    place-items: center;
    padding: 18px;
    border: 1px dashed #94a3b8;
    border-radius: 4px;
    background: #ffffff;
    color: #64748b;
    font-weight: 800;
    text-align: center;
    box-sizing: border-box;
  }

  .style-detail,
  .create-form {
    min-width: 0;
    min-height: 0;
    display: grid;
    gap: 12px;
    padding: 18px;
    background: #f8fafc;
    overflow: auto;
  }

  .style-detail {
    align-content: start;
    overflow: auto;
  }

  .create-form {
    align-content: start;
    border-right: 1px solid #d8e0eb;
  }

  .create-form label {
    display: grid;
    gap: 6px;
    color: #334155;
    font-size: 13px;
    font-weight: 800;
  }

  .create-form input,
  .create-form textarea {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    color: #172033;
    font: inherit;
  }

  .create-form input {
    height: 36px;
    padding: 0 10px;
  }

  :global(.style-binding-select) {
    width: min(100%, 360px);
    --control-height: 38px;
    --radius-sm: 4px;
    --radius-xs: 3px;
    --color-border: #cbd5e1;
    --color-border-strong: #b8c5d6;
    --color-surface: #ffffff;
    --color-text: #172033;
    --color-text-soft: #334155;
    --color-muted: #64748b;
    --color-accent: #1677b8;
    --color-accent-deep: #0f5d91;
    --color-accent-quiet: #e8f2f8;
    --color-hover: #f1f7fb;
    --focus-ring: 0 0 0 2px rgba(22, 119, 184, 0.18);
    --shadow-xs: 0 1px 2px rgba(15, 23, 42, 0.04);
    --shadow-pop: 0 12px 28px rgba(15, 23, 42, 0.16);
  }

  :global(.detail-binding-select) {
    max-width: 360px;
  }

  .readonly-binding {
    width: min(100%, 360px);
    box-sizing: border-box;
    padding: 9px 10px;
    border: 1px solid #d8e0eb;
    border-radius: 4px;
    background: #ffffff;
    color: #475569;
    font-size: 13px;
    font-weight: 800;
  }

  .create-form textarea {
    padding: 10px;
    resize: vertical;
    font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
    font-size: 12px;
    line-height: 1.55;
  }

  .file-input {
    display: none;
  }

  .file-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
  }

  .file-row span {
    min-width: 0;
    overflow: hidden;
    color: #64748b;
    font-size: 12px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 7px 10px;
    padding: 10px;
    border: 1px solid #d8e0eb;
    border-radius: 4px;
    background: #ffffff;
    font-size: 12px;
  }

  .meta-grid.compact {
    max-width: 360px;
  }

  .meta-grid span {
    color: #64748b;
    font-weight: 800;
  }

  .meta-grid strong {
    color: #172033;
  }

  .form-message,
  .copy-message {
    margin: 0;
    color: #1677b8;
    font-weight: 800;
  }

  .form-message.error {
    color: #b91c1c;
  }

  .style-detail section {
    min-width: 0;
  }

  .style-detail h3 {
    margin: 0 0 8px;
    font-size: 14px;
  }

  .selector-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  code,
  pre {
    font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
  }

  code {
    padding: 4px 7px;
    border: 1px solid #d8e0eb;
    border-radius: 4px;
    background: #ffffff;
    color: #0f5d91;
    font-size: 12px;
  }

  pre {
    margin: 0;
    padding: 12px;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
    border: 1px solid #d8e0eb;
    border-radius: 4px;
    background: #111827;
    color: #e5e7eb;
    font-size: 12px;
    line-height: 1.55;
    overflow: auto;
    white-space: pre-wrap;
  }

  .interface-panel,
  .interface-preview {
    min-height: 0;
    display: grid;
    align-content: start;
    gap: 12px;
    padding: 16px;
    overflow: auto;
  }

  .interface-preview {
    grid-template-rows: auto auto minmax(0, 1fr);
  }

  .interface-list {
    display: grid;
    gap: 8px;
  }

  .interface-list div {
    display: grid;
    gap: 5px;
    padding: 10px;
    border: 1px solid #d8e0eb;
    border-radius: 4px;
    background: #f8fafc;
  }

  .interface-list strong {
    font-size: 13px;
  }

  .interface-list small {
    color: #64748b;
    font-size: 12px;
    line-height: 1.45;
    font-weight: 700;
  }

  @media (max-width: 860px) {
    .library-workspace {
      grid-template-columns: 1fr;
      overflow: auto;
    }

    .style-sidebar,
    .preview-pane {
      overflow: visible;
    }

    .preview-grid,
    .create-grid {
      grid-template-columns: 1fr;
      overflow: visible;
    }

    .create-form {
      border-right: 0;
      border-bottom: 1px solid #d8e0eb;
    }

    .live-preview iframe {
      width: min(100%, 360px, calc((100vh - 160px) * 360 / 700));
      height: auto;
    }
  }

  :global(:root[data-tepub-client="web-mobile"]) .style-library-page {
    min-height: 100dvh;
    height: 100dvh;
  }
</style>
