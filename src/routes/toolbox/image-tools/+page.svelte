<script lang="ts">
    import { onDestroy, onMount, tick } from "svelte";
    import { platform, type PlatformUnlisten } from "$lib/platform";
    import {
        loadAppSettings,
        type AiProviderConfig,
        type GlobalAppSettings,
    } from "$lib/appSettings";
    import { clientLayout, phoneDevice } from "$lib/clientProfile";
    import { IMAGE_TOOLS_AI_CONFIG_KEY as CONFIG_KEY } from "$lib/storageKeys";
    import { resolveImageCompareFlow } from "$lib/imageCompareLayout";
    import { isInHiddenView, viewUrl } from "$lib/keepAlive/viewState";
    import { tabIdFor } from "$lib/secondaryTabs";
    import {
        registerKeepAliveGuard,
        unregisterKeepAliveGuard,
    } from "$lib/keepAlive/dirtyGuard";
    import { localizeAiProviderName, t } from "$lib/i18n.svelte";

    type AiTarget = "duokan" | "standard" | "banner";
    type ToolMode = "cover" | "header" | "ai";
    type PreviewMode = "compare" | "reference";
    type ImagePurpose = "cover" | "headerSource" | "ai";
    type HeaderDragState = {
        pointerId: number;
        lastX: number;
        lastY: number;
    } | null;

    type AiImageResult = {
        bytes: number[];
        mime: string;
        extension: string;
        fileName: string;
        prompt: string;
        size: string;
    };

    const DEFAULT_COVER_PROMPT =
        "帮我制作作者{author}的小说《{title}》的全屏竖版封面，比例适合电子书阅读器全屏展示，可以参考原书籍封面。保留书籍气质，画面精致、清晰、无水印。";
    const DEFAULT_STANDARD_PROMPT =
        "帮我制作作者{author}的小说《{title}》的标准书籍封面，比例严格为3:4，可以参考原书籍封面。保留书籍气质，画面精致、清晰、无水印，适合 EPUB 封面使用。";
    const DEFAULT_BANNER_PROMPT =
        "帮我制作作者{author}的小说《{title}》的阅微横幅封面，比例严格为2:1，可以参考原书籍封面。画面适合小说阅读应用首页横幅，主体清晰，构图横向展开，精致、清晰、无水印。";
    const HEADER_SAMPLE_LONG_EDGE = 1080;

    const AI_TARGETS: Array<{
        id: AiTarget;
        label: string;
        size: string;
        ratioLabel: string;
    }> = [
        {
            id: "standard",
            label: "标准封面",
            size: "1200x1600",
            ratioLabel: "3:4",
        },
        {
            id: "duokan",
            label: "全屏封面",
            size: "1400x2400",
            ratioLabel: "7:12",
        },
        {
            id: "banner",
            label: "阅微横幅",
            size: "2048x1024",
            ratioLabel: "2:1",
        },
    ];
    const FORMAT_OPTIONS: Array<{
        value: "jpeg" | "png" | "webp";
        label: string;
    }> = [
        { value: "jpeg", label: "JPEG" },
        { value: "png", label: "PNG" },
        { value: "webp", label: "WebP" },
    ];

    let fileInput: HTMLInputElement | null = null;
    let sampleInput: HTMLInputElement | null = null;
    let localCanvas: HTMLCanvasElement | null = null;
    let headerCanvas: HTMLCanvasElement | null = null;
    let fileInputPurpose: ImagePurpose = "cover";
    let coverImage: HTMLImageElement | null = null;
    let coverBytes: Uint8Array | null = null;
    let coverName = "";
    let coverMime = "";
    let coverObjectUrl = "";
    let headerSourceImage: HTMLImageElement | null = null;
    let headerSourceName = "";
    let headerSourceObjectUrl = "";
    let headerSampleImage: HTMLImageElement | null = null;
    let headerSampleName = "";
    let headerSampleObjectUrl = "";
    let aiReferenceImage: HTMLImageElement | null = null;
    let aiReferenceBytes: Uint8Array | null = null;
    let aiReferenceName = "";
    let aiReferenceMime = "";
    let aiReferenceObjectUrl = "";
    let headerScale = 1;
    let headerOffsetX = 0;
    let headerOffsetY = 0;
    let headerDragState: HeaderDragState = null;

    let bgMode: "blur" | "solid" = "blur";
    let solidColor = "#f3f4f6";
    let blur = 10;
    let radius = 10;
    let foregroundSize = 80;
    let offsetX = 0;
    let offsetY = 0;
    let shadow = false;
    let shadowBlur = 20;
    let shadowSpread = 5;
    let shadowColor = "#000000";
    let localWidth = 1400;
    let localHeight = 2400;
    let outputFormat: "jpeg" | "png" | "webp" = "jpeg";
    let outputQuality = 92;
    let showGuides = true;
    let guideColor = "#1478ff";

    /**
     * 三个工作区各自「有东西、但还没导出」。
     *
     * 以前保活守卫直接看 coverImage / generated 这类对象是否非空 ——
     * 它们在保存成功之后也不会清空，于是一旦选过图，这一页就永久否决
     * 手机端的裁剪，3 条上限等于不存在，整张原图的 Uint8Array 常驻内存。
     * 反过来，只导入头图样图时守卫里又漏了 headerSampleImage，页面被裁掉、
     * 刚规范化好的样图直接没了。
     *
     * 现在改成显式记账：载入 / 生成 / 改参数 = 脏，保存成功 = 干净，
     * 清空按钮 = 干净。
     */
    let dirtyCover = false;
    let dirtyHeader = false;
    let dirtyAi = false;
    $: hasUnsavedOutput = dirtyCover || dirtyHeader || dirtyAi;

    function markDirty(kind: "cover" | "header" | "ai") {
        if (kind === "cover") dirtyCover = true;
        else if (kind === "header") dirtyHeader = true;
        else dirtyAi = true;
    }

    function markSaved(kind: "cover" | "header" | "ai") {
        if (kind === "cover") dirtyCover = false;
        else if (kind === "header") dirtyHeader = false;
        else dirtyAi = false;
    }

    /**
     * 保存之后再拖滑杆，产物又和导出那张不一样了 —— 也算未保存。
     *
     * 只在「手里已经有图」时才记脏：否则一进页面这些 $: 会先跑一遍默认值，
     * 让一页什么都没做的工具变成不可淘汰。保存本身不改这些参数，
     * 所以这段不会在 markSaved 之后立刻把标记翻回来。
     */
    $: if (coverImage) {
        void [
            localWidth,
            localHeight,
            radius,
            foregroundSize,
            offsetX,
            offsetY,
            blur,
            bgMode,
            solidColor,
            shadow,
            shadowBlur,
            shadowSpread,
            shadowColor,
            outputFormat,
            outputQuality,
            guideColor,
        ];
        // 经函数赋值：直接写 dirtyCover = true 会被 lint 当成无用赋值
        markDirty("cover");
    }

    $: if (headerSourceImage || headerSampleImage) {
        void [headerScale, headerOffsetX, headerOffsetY];
        markDirty("header");
    }

    let title = "";
    let author = "";
    let appSettings: GlobalAppSettings = loadAppSettings();
    let imageProviders: AiProviderConfig[] = [];
    let selectedImageProvider: AiProviderConfig | undefined;
    let imageProviderId = "";
    let aiTarget: AiTarget = "standard";
    let aiSize = "1200x1600";
    let aiPrompt = DEFAULT_STANDARD_PROMPT;
    let generating = false;
    let generated: AiImageResult | null = null;
    let generatedUrl = "";
    let generatedTarget: AiTarget = "duokan";
    let activeMode: ToolMode = "cover";
    let phoneLayout = false;
    let phoneChrome = false;
    $: phoneLayout = $clientLayout === "phone";
    // 只有带手机外壳的页面才有根布局标题行；窄窗口桌面 Web 虽然也走
    // phone 布局，但没有那条标题行，不能把页内标题一起藏掉。
    $: phoneChrome = $phoneDevice && phoneLayout;
    let previewMode: PreviewMode = "compare";
    let customSelectOpen: "format" | "provider" | null = null;
    let status = "";
    let closingByCode = false;

    $: imageProviders = appSettings.aiProviders.filter(
        (provider) => provider.kind === "image",
    );
    $: if (
        imageProviderId &&
        !imageProviders.some((provider) => provider.id === imageProviderId)
    ) {
        imageProviderId = imageProviders[0]?.id || "";
    }
    $: if (!imageProviderId && imageProviders.length > 0) {
        imageProviderId = imageProviders[0].id;
    }
    $: selectedImageProvider =
        imageProviders.find((provider) => provider.id === imageProviderId) ||
        imageProviders[0];
    $: canGenerate = Boolean(
        !!aiReferenceBytes &&
        !!selectedImageProvider &&
        selectedImageProvider.baseUrl.trim() &&
        selectedImageProvider.apiKey.trim() &&
        selectedImageProvider.model.trim() &&
        title.trim() &&
        aiPrompt.trim(),
    );
    $: currentGeneratedReady = !!generated && generatedTarget === aiTarget;
    $: previewTarget =
        activeMode === "ai"
            ? aiTarget
            : activeMode === "header"
              ? "banner"
              : "duokan";
    $: previewIsBanner = previewTarget === "banner";
    $: showComparePreview = previewMode === "compare";
    $: compareOutputSize =
        activeMode === "cover"
            ? { width: localWidth, height: localHeight }
            : activeMode === "header"
              ? {
                    width: headerSampleImage?.width || 2048,
                    height: headerSampleImage?.height || 1024,
                }
              : parseImageSize(aiSize);
    $: compareFlow = resolveImageCompareFlow(
        compareOutputSize.width,
        compareOutputSize.height,
    );
    $: compareUsesRows = compareFlow === "rows";
    $: activeReferenceImage =
        activeMode === "cover"
            ? coverImage
            : activeMode === "ai"
              ? aiReferenceImage
              : headerSourceImage;
    $: activeReferenceName =
        activeMode === "cover"
            ? coverName
            : activeMode === "ai"
              ? aiReferenceName
              : headerSourceName;
    $: activeReferenceObjectUrl =
        activeMode === "cover"
            ? coverObjectUrl
            : activeMode === "ai"
              ? aiReferenceObjectUrl
              : headerSourceObjectUrl;
    $: previewOutputTitle =
        activeMode === "cover"
            ? t("全屏封面 {width}x{height}", {
                  width: localWidth,
                  height: localHeight,
              })
            : activeMode === "header"
              ? t("头图制作")
              : targetLabel(aiTarget);
    $: selectedFormatLabel =
        FORMAT_OPTIONS.find((option) => option.value === outputFormat)?.label ||
        "JPEG";
    $: selectedProviderLabel = selectedImageProvider
        ? `${selectedImageProvider.name || selectedImageProvider.model} · ${selectedImageProvider.model}`
        : imageProviders.length === 0
          ? t("未配置生图模型")
          : t("选择生图模型");
    $: saveDisabled =
        activeMode === "cover"
            ? !coverImage
            : activeMode === "header"
              ? !headerSourceImage || !headerSampleImage
              : !currentGeneratedReady;

    async function restoreToolboxHome() {
        if (loadAppSettings().closeToolboxOnToolOpen === false) return;
        for (const label of ["toolbox", "main"]) {
            try {
                const win = await platform.getWindowByLabel(label);
                if (!win) continue;
                await win.show();
                await win.setFocus();
                return;
            } catch (error) {
                console.warn(`恢复 ${label} 窗口失败:`, error);
            }
        }
    }

    async function closeWindow() {
        if (closingByCode) return;
        closingByCode = true;
        await restoreToolboxHome();
        await platform.getCurrentWindow().destroy();
    }

    function setMode(mode: ToolMode) {
        activeMode = mode;
        previewMode = "compare";
        tick().then(() => {
            drawLocalPreview();
            drawHeaderPreview();
        });
    }

    function setPreviewMode(mode: PreviewMode) {
        previewMode = mode;
        tick().then(() => {
            drawLocalPreview();
            drawHeaderPreview();
        });
    }

    function toggleCustomSelect(name: "format" | "provider") {
        customSelectOpen = customSelectOpen === name ? null : name;
    }

    function selectOutputFormat(value: "jpeg" | "png" | "webp") {
        outputFormat = value;
        customSelectOpen = null;
    }

    function selectImageProvider(id: string) {
        imageProviderId = id;
        customSelectOpen = null;
    }

    async function imageFromObjectUrl(url: string) {
        const img = new Image();
        await new Promise<void>((resolve, reject) => {
            img.onload = () => resolve();
            img.onerror = () => reject(new Error(t("图片加载失败")));
            img.src = url;
        });
        return img;
    }

    function canvasToObjectUrl(canvas: HTMLCanvasElement, mime = "image/png") {
        return new Promise<string>((resolve, reject) => {
            canvas.toBlob((blob) => {
                if (!blob) {
                    reject(new Error(t("生成图片失败")));
                    return;
                }
                resolve(URL.createObjectURL(blob));
            }, mime);
        });
    }

    async function normalizeHeaderSampleImage(image: HTMLImageElement) {
        const sourceWidth = image.naturalWidth || image.width || 1;
        const sourceHeight = image.naturalHeight || image.height || 1;
        const scale =
            HEADER_SAMPLE_LONG_EDGE / Math.max(sourceWidth, sourceHeight, 1);
        const width = Math.max(1, Math.round(sourceWidth * scale));
        const height = Math.max(1, Math.round(sourceHeight * scale));
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error(t("无法创建头图样图画布"));
        ctx.clearRect(0, 0, width, height);
        ctx.imageSmoothingEnabled = true;
        ctx.imageSmoothingQuality = "high";
        ctx.drawImage(image, 0, 0, width, height);
        const objectUrl = await canvasToObjectUrl(canvas, "image/png");
        return {
            image: await imageFromObjectUrl(objectUrl),
            objectUrl,
            width,
            height,
        };
    }

    function defaultPrompt(target: AiTarget) {
        if (target === "banner") return DEFAULT_BANNER_PROMPT;
        if (target === "standard") return DEFAULT_STANDARD_PROMPT;
        return DEFAULT_COVER_PROMPT;
    }

    function defaultSize(target: AiTarget) {
        return (
            AI_TARGETS.find((item) => item.id === target)?.size || "1400x2400"
        );
    }

    function parseImageSize(value: string) {
        const match = value.trim().match(/^(\d+)\s*[x×]\s*(\d+)$/i);
        if (match) {
            return { width: Number(match[1]), height: Number(match[2]) };
        }
        const fallback = defaultSize(aiTarget).split("x");
        return {
            width: Number(fallback[0]) || 1200,
            height: Number(fallback[1]) || 1600,
        };
    }

    function targetLabel(target: AiTarget) {
        return t(
            AI_TARGETS.find((item) => item.id === target)?.label || "全屏封面",
        );
    }

    function targetRatioLabel(target: AiTarget) {
        return (
            AI_TARGETS.find((item) => item.id === target)?.ratioLabel || "7:12"
        );
    }

    function chooseImage(purpose: ImagePurpose) {
        fileInputPurpose = purpose;
        fileInput?.click();
    }

    function chooseActiveReference() {
        if (activeMode === "header") {
            if (!headerSampleImage) {
                sampleInput?.click();
            } else {
                chooseImage("headerSource");
            }
            return;
        }
        chooseImage(activeMode === "ai" ? "ai" : "cover");
    }

    function loadConfig() {
        try {
            const raw = localStorage.getItem(CONFIG_KEY);
            if (!raw) return;
            const cfg = JSON.parse(raw);
            title = typeof cfg.title === "string" ? cfg.title : "";
            author = typeof cfg.author === "string" ? cfg.author : "";
        } catch (e) {
            console.warn("Failed to load image tool config:", e);
        }
    }

    function saveConfig() {
        localStorage.setItem(
            CONFIG_KEY,
            JSON.stringify({
                title,
                author,
            }),
        );
    }

    function refreshImageProviders() {
        const nextSettings = loadAppSettings();
        const nextProviders = nextSettings.aiProviders.filter(
            (provider) => provider.kind === "image",
        );
        appSettings = nextSettings;
        imageProviderId = nextProviders.some(
            (provider) => provider.id === imageProviderId,
        )
            ? imageProviderId
            : nextProviders[0]?.id || "";
        status =
            nextProviders.length > 0
                ? t("已刷新生图模型")
                : t("未找到已保存的生图模型");
    }

    function detectMime(bytes: Uint8Array, fallback: string) {
        if (
            bytes.length >= 8 &&
            bytes[0] === 0x89 &&
            bytes[1] === 0x50 &&
            bytes[2] === 0x4e &&
            bytes[3] === 0x47
        ) {
            return "image/png";
        }
        if (
            bytes.length >= 12 &&
            bytes[0] === 0x52 &&
            bytes[1] === 0x49 &&
            bytes[8] === 0x57 &&
            bytes[9] === 0x45
        ) {
            return "image/webp";
        }
        if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8) {
            return "image/jpeg";
        }
        return fallback || "image/jpeg";
    }

    async function loadImageFile(
        file: File,
        purpose: ImagePurpose = fileInputPurpose,
    ) {
        if (!file.type.startsWith("image/")) {
            await platform.message(t("请选择图片文件。"), {
                title: t("图片处理"),
                kind: "warning",
            });
            return;
        }
        const bytes = new Uint8Array(await file.arrayBuffer());
        const objectUrl = URL.createObjectURL(file);
        const img = await imageFromObjectUrl(objectUrl);
        const mime = detectMime(bytes, file.type);
        if (purpose === "cover") {
            if (coverObjectUrl) URL.revokeObjectURL(coverObjectUrl);
            coverObjectUrl = objectUrl;
            coverImage = img;
            coverBytes = bytes;
            coverName = file.name;
            coverMime = mime;
        } else if (purpose === "headerSource") {
            if (headerSourceObjectUrl)
                URL.revokeObjectURL(headerSourceObjectUrl);
            headerSourceObjectUrl = objectUrl;
            headerSourceImage = img;
            headerSourceName = file.name;
            resetHeaderTransform();
        } else {
            if (aiReferenceObjectUrl) URL.revokeObjectURL(aiReferenceObjectUrl);
            aiReferenceObjectUrl = objectUrl;
            aiReferenceImage = img;
            aiReferenceBytes = bytes;
            aiReferenceName = file.name;
            aiReferenceMime = mime;
            clearGeneratedImage();
        }
        markDirty(
            purpose === "cover"
                ? "cover"
                : purpose === "headerSource"
                  ? "header"
                  : "ai",
        );
        status =
            purpose === "headerSource"
                ? t("已载入处理图片 {name}", { name: file.name })
                : t("已载入参考图 {name}", { name: file.name });
        await tick();
        drawLocalPreview();
        drawHeaderPreview();
    }

    async function loadHeaderSampleFile(file: File) {
        if (!file.type.startsWith("image/")) {
            await platform.message(t("请选择图片文件。"), {
                title: t("图片处理"),
                kind: "warning",
            });
            return;
        }
        const sourceObjectUrl = URL.createObjectURL(file);
        let normalizedObjectUrl = "";
        try {
            const sourceImage = await imageFromObjectUrl(sourceObjectUrl);
            const normalized = await normalizeHeaderSampleImage(sourceImage);
            normalizedObjectUrl = normalized.objectUrl;
            if (headerSampleObjectUrl)
                URL.revokeObjectURL(headerSampleObjectUrl);
            headerSampleObjectUrl = normalized.objectUrl;
            normalizedObjectUrl = "";
            headerSampleImage = normalized.image;
            headerSampleName = file.name;
            markDirty("header");
            status = t(
                "已载入头图样图 {name}，长边已规范为 {edge}px（{width}x{height}）",
                {
                    name: file.name,
                    edge: HEADER_SAMPLE_LONG_EDGE,
                    width: normalized.width,
                    height: normalized.height,
                },
            );
            resetHeaderTransform();
            await tick();
            drawHeaderPreview();
        } finally {
            URL.revokeObjectURL(sourceObjectUrl);
            if (normalizedObjectUrl) URL.revokeObjectURL(normalizedObjectUrl);
        }
    }

    function onFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        if (file) {
            loadImageFile(file, fileInputPurpose).catch((err) => {
                status = t("图片载入失败");
                platform.message(String(err?.message ?? err), {
                    title: t("图片处理"),
                    kind: "error",
                });
            });
        }
        input.value = "";
    }

    function onSampleFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        if (file) {
            loadHeaderSampleFile(file).catch((err) => {
                status = t("头图样图载入失败");
                platform.message(String(err?.message ?? err), {
                    title: t("图片处理"),
                    kind: "error",
                });
            });
        }
        input.value = "";
    }

    function onDrop(event: DragEvent) {
        event.preventDefault();
        const file = event.dataTransfer?.files?.[0];
        if (file) {
            if (activeMode === "header" && !headerSampleImage) {
                loadHeaderSampleFile(file).catch((err) => {
                    status = t("头图样图载入失败");
                    platform.message(String(err?.message ?? err), {
                        title: t("图片处理"),
                        kind: "error",
                    });
                });
                return;
            }
            const purpose: ImagePurpose =
                activeMode === "header"
                    ? "headerSource"
                    : activeMode === "ai"
                      ? "ai"
                      : "cover";
            loadImageFile(file, purpose).catch((err) => {
                status = t("图片载入失败");
                platform.message(String(err?.message ?? err), {
                    title: t("图片处理"),
                    kind: "error",
                });
            });
        }
    }

    function roundedRect(
        ctx: CanvasRenderingContext2D,
        x: number,
        y: number,
        width: number,
        height: number,
        r: number,
    ) {
        const radius = Math.min(r, width / 2, height / 2);
        ctx.beginPath();
        ctx.moveTo(x + radius, y);
        ctx.lineTo(x + width - radius, y);
        ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
        ctx.lineTo(x + width, y + height - radius);
        ctx.quadraticCurveTo(
            x + width,
            y + height,
            x + width - radius,
            y + height,
        );
        ctx.lineTo(x + radius, y + height);
        ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
        ctx.lineTo(x, y + radius);
        ctx.quadraticCurveTo(x, y, x + radius, y);
        ctx.closePath();
    }

    function drawComposite(
        canvas: HTMLCanvasElement,
        img: HTMLImageElement | null,
        width: number,
        height: number,
        guides: boolean,
    ) {
        const ctx = canvas.getContext("2d");
        if (!img || !ctx) return;

        canvas.width = width;
        canvas.height = height;
        ctx.clearRect(0, 0, width, height);

        if (bgMode === "solid") {
            ctx.fillStyle = solidColor;
            ctx.fillRect(0, 0, width, height);
        } else {
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
                offCtx.filter =
                    blur > 0 ? `blur(${blur * 2 * (width / 1400)}px)` : "none";
                offCtx.drawImage(img, 0, 0, bgW, bgH);
                ctx.drawImage(off, bgX, bgY);
            }
        }

        const fgScale = Math.min(
            (width * (foregroundSize / 100)) / img.width,
            (height * 0.8) / img.height,
        );
        const fgW = img.width * fgScale;
        const fgH = img.height * fgScale;
        const fgX = (width - fgW) / 2 + (width * offsetX) / 100;
        const fgY = (height - fgH) / 2 + (height * offsetY) / 100;
        const scaledRadius = radius * 5 * (width / 1400);

        if (shadow) {
            ctx.save();
            const spread = shadowSpread * (width / 1400);
            roundedRect(
                ctx,
                fgX - spread,
                fgY - spread,
                fgW + spread * 2,
                fgH + spread * 2,
                scaledRadius + spread,
            );
            ctx.filter = `blur(${shadowBlur * (width / 1400)}px)`;
            ctx.fillStyle = `${shadowColor}66`;
            ctx.fill();
            ctx.restore();
        }

        ctx.save();
        roundedRect(ctx, fgX, fgY, fgW, fgH, scaledRadius);
        ctx.clip();
        ctx.drawImage(img, fgX, fgY, fgW, fgH);
        ctx.restore();

        if (guides) {
            ctx.save();
            ctx.globalAlpha = 0.68;
            ctx.strokeStyle = guideColor;
            ctx.lineWidth = Math.max(1, width / 900);
            ctx.setLineDash([8, 8]);
            ctx.beginPath();
            ctx.moveTo(width / 2, 0);
            ctx.lineTo(width / 2, height);
            ctx.moveTo(0, height / 2);
            ctx.lineTo(width, height / 2);
            ctx.stroke();
            ctx.restore();
        }
    }

    function drawLocalPreview() {
        if (!localCanvas || !coverImage) return;
        drawComposite(
            localCanvas,
            coverImage,
            localWidth,
            localHeight,
            showGuides,
        );
    }

    function resetCoverSettings() {
        bgMode = "blur";
        solidColor = "#f3f4f6";
        blur = 10;
        radius = 10;
        foregroundSize = 80;
        offsetX = 0;
        offsetY = 0;
        shadow = false;
        shadowBlur = 20;
        shadowSpread = 5;
        shadowColor = "#000000";
        localWidth = 1400;
        localHeight = 2400;
        outputFormat = "jpeg";
        outputQuality = 92;
        showGuides = true;
        guideColor = "#1478ff";
        drawLocalPreview();
    }

    function clearCoverImage() {
        if (coverObjectUrl) URL.revokeObjectURL(coverObjectUrl);
        coverImage = null;
        coverBytes = null;
        coverName = "";
        coverMime = "";
        coverObjectUrl = "";
        markSaved("cover");
        status = t("已清空封面处理参考图");
    }

    function clearHeaderImages() {
        if (headerSampleObjectUrl) URL.revokeObjectURL(headerSampleObjectUrl);
        if (headerSourceObjectUrl) URL.revokeObjectURL(headerSourceObjectUrl);
        headerSampleImage = null;
        headerSampleName = "";
        headerSampleObjectUrl = "";
        headerSourceImage = null;
        headerSourceName = "";
        headerSourceObjectUrl = "";
        headerScale = 1;
        headerOffsetX = 0;
        headerOffsetY = 0;
        markSaved("header");
        status = t("已清空头图样图和处理图片");
    }

    function clearGeneratedImage() {
        if (generatedUrl) URL.revokeObjectURL(generatedUrl);
        generated = null;
        generatedUrl = "";
        dirtyAi = false;
    }

    function resetAiSettings() {
        aiTarget = "standard";
        aiSize = defaultSize(aiTarget);
        aiPrompt = defaultPrompt(aiTarget);
        generatedTarget = aiTarget;
        status = t("AI 生成参数已重置");
    }

    function clearAiReference() {
        if (aiReferenceObjectUrl) URL.revokeObjectURL(aiReferenceObjectUrl);
        aiReferenceImage = null;
        aiReferenceBytes = null;
        aiReferenceName = "";
        aiReferenceMime = "";
        aiReferenceObjectUrl = "";
        clearGeneratedImage();
        markSaved("ai");
        status = t("已清空 AI 参考图");
    }

    function resetHeaderTransform() {
        if (!headerSourceImage || !headerSampleImage) return;
        headerScale = Math.max(
            headerSampleImage.width / headerSourceImage.width,
            headerSampleImage.height / headerSourceImage.height,
        );
        headerOffsetX = 0;
        headerOffsetY = 0;
        drawHeaderPreview();
    }

    function drawHeaderPreview(
        targetCanvas: HTMLCanvasElement | null = headerCanvas,
        withMask = true,
    ) {
        if (!targetCanvas || !headerSourceImage || !headerSampleImage) return;
        const ctx = targetCanvas.getContext("2d");
        if (!ctx) return;
        const width = headerSampleImage.width;
        const height = headerSampleImage.height;
        targetCanvas.width = width;
        targetCanvas.height = height;
        ctx.clearRect(0, 0, width, height);

        const drawW = headerSourceImage.width * headerScale;
        const drawH = headerSourceImage.height * headerScale;
        const drawX = (width - drawW) / 2 + headerOffsetX;
        const drawY = (height - drawH) / 2 + headerOffsetY;
        ctx.drawImage(headerSourceImage, drawX, drawY, drawW, drawH);

        if (withMask) {
            ctx.save();
            ctx.globalCompositeOperation = "destination-in";
            ctx.drawImage(headerSampleImage, 0, 0, width, height);
            ctx.restore();
        }
    }

    function onHeaderPointerDown(event: PointerEvent) {
        if (!headerSourceImage || !headerSampleImage || !headerCanvas) return;
        (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
        headerDragState = {
            pointerId: event.pointerId,
            lastX: event.clientX,
            lastY: event.clientY,
        };
    }

    function onHeaderPointerMove(event: PointerEvent) {
        if (
            !headerDragState ||
            headerDragState.pointerId !== event.pointerId ||
            !headerCanvas
        )
            return;
        const rect = (
            event.currentTarget as HTMLElement
        ).getBoundingClientRect();
        const unitX = headerCanvas.width / Math.max(1, rect.width);
        const unitY = headerCanvas.height / Math.max(1, rect.height);
        headerOffsetX += (event.clientX - headerDragState.lastX) * unitX;
        headerOffsetY += (event.clientY - headerDragState.lastY) * unitY;
        headerDragState = {
            pointerId: event.pointerId,
            lastX: event.clientX,
            lastY: event.clientY,
        };
        drawHeaderPreview();
    }

    function onHeaderPointerEnd(event: PointerEvent) {
        if (headerDragState?.pointerId === event.pointerId) {
            headerDragState = null;
        }
    }

    function onHeaderWheel(event: WheelEvent) {
        if (!headerSourceImage || !headerSampleImage || !headerCanvas) return;
        event.preventDefault();
        const rect = (
            event.currentTarget as HTMLElement
        ).getBoundingClientRect();
        const canvasX =
            (event.clientX - rect.left) *
            (headerCanvas.width / Math.max(1, rect.width));
        const canvasY =
            (event.clientY - rect.top) *
            (headerCanvas.height / Math.max(1, rect.height));
        const oldScale = headerScale;
        const nextScale = Math.min(
            20,
            Math.max(0.05, headerScale * (event.deltaY < 0 ? 1.08 : 0.925)),
        );
        const oldDrawX =
            (headerCanvas.width - headerSourceImage.width * oldScale) / 2 +
            headerOffsetX;
        const oldDrawY =
            (headerCanvas.height - headerSourceImage.height * oldScale) / 2 +
            headerOffsetY;
        const imageX = (canvasX - oldDrawX) / oldScale;
        const imageY = (canvasY - oldDrawY) / oldScale;
        headerScale = nextScale;
        headerOffsetX =
            canvasX -
            imageX * nextScale -
            (headerCanvas.width - headerSourceImage.width * nextScale) / 2;
        headerOffsetY =
            canvasY -
            imageY * nextScale -
            (headerCanvas.height - headerSourceImage.height * nextScale) / 2;
        drawHeaderPreview();
    }

    function canvasToBytes(
        canvas: HTMLCanvasElement,
        mime: string,
        quality: number,
    ) {
        return new Promise<Uint8Array>((resolve, reject) => {
            canvas.toBlob(
                async (blob) => {
                    if (!blob) {
                        reject(new Error(t("生成图片失败")));
                        return;
                    }
                    resolve(new Uint8Array(await blob.arrayBuffer()));
                },
                mime,
                quality,
            );
        });
    }

    function formatInfo(format: typeof outputFormat) {
        if (format === "png") return { mime: "image/png", ext: "png" };
        if (format === "webp") return { mime: "image/webp", ext: "webp" };
        return { mime: "image/jpeg", ext: "jpg" };
    }

    async function saveBytes(
        bytes: Uint8Array,
        defaultName: string,
        extension: string,
    ): Promise<boolean> {
        const selected = await platform.saveDialog({
            defaultPath: defaultName,
            filters: [
                { name: extension.toUpperCase(), extensions: [extension] },
            ],
        });
        // 用户取消：产物还在手里，页面依然算「有未保存的东西」
        if (!selected) return false;
        await platform.writeFile(selected, bytes);
        status = t("已保存 {path}", { path: selected });
        return true;
    }

    async function saveLocalCover() {
        if (!coverImage) return;
        const canvas = document.createElement("canvas");
        drawComposite(canvas, coverImage, localWidth, localHeight, false);
        const { mime, ext } = formatInfo(outputFormat);
        const quality =
            outputFormat === "png" ? undefined : outputQuality / 100;
        const bytes = await canvasToBytes(canvas, mime, quality ?? 0.92);
        const stem = (title || coverName || "cover").replace(/\.[^.]+$/, "");
        const saved = await saveBytes(
            bytes,
            `${stem}~duokan-${localWidth}x${localHeight}.${ext}`,
            ext,
        );
        if (saved) markSaved("cover");
    }

    async function saveHeaderImage() {
        if (!headerSourceImage || !headerSampleImage) return;
        const canvas = document.createElement("canvas");
        drawHeaderPreview(canvas, true);
        const bytes = await canvasToBytes(canvas, "image/png", 1);
        const stem = (title || headerSourceName || "header").replace(
            /\.[^.]+$/,
            "",
        );
        const saved = await saveBytes(
            bytes,
            `${stem}~header-${headerSampleImage.width}x${headerSampleImage.height}.png`,
            "png",
        );
        if (saved) markSaved("header");
    }

    function setAiTarget(next: AiTarget) {
        const oldDefault = defaultPrompt(aiTarget);
        aiTarget = next;
        aiSize = defaultSize(next);
        previewMode = "compare";
        if (!aiPrompt.trim() || aiPrompt === oldDefault) {
            aiPrompt = defaultPrompt(next);
        }
    }

    function setGeneratedUrl(bytes: Uint8Array, mime: string) {
        if (generatedUrl) URL.revokeObjectURL(generatedUrl);
        generatedUrl = URL.createObjectURL(new Blob([bytes], { type: mime }));
    }

    function normalizeAiBaseUrl(value: string) {
        return value.trim().replace(/\/+$/, "");
    }

    function removeSuffixIgnoreCase(value: string, suffix: string) {
        return value.toLowerCase().endsWith(suffix.toLowerCase())
            ? value.slice(0, -suffix.length)
            : value;
    }

    function browserAiImageGenerationUrl(baseUrl: string) {
        const base = normalizeAiBaseUrl(baseUrl);
        if (!base) throw new Error("Base URL 不能为空");
        const lower = base.toLowerCase();
        if (lower.endsWith("/images/generations")) return base;
        if (lower.endsWith("/chat/completions"))
            return `${removeSuffixIgnoreCase(base, "/chat/completions")}/images/generations`;
        if (lower.endsWith("/v1") || /\/api\/v\d+$/i.test(base))
            return `${base}/images/generations`;
        return `${base}/v1/images/generations`;
    }

    function bytesToBase64(bytes: Uint8Array) {
        let binary = "";
        const chunkSize = 0x8000;
        for (let i = 0; i < bytes.length; i += chunkSize) {
            binary += String.fromCharCode(...bytes.slice(i, i + chunkSize));
        }
        return btoa(binary);
    }

    function bytesToDataUrl(bytes: Uint8Array, mime: string) {
        return `data:${mime || detectMime(bytes, "image/jpeg")};base64,${bytesToBase64(bytes)}`;
    }

    function renderAiPrompt(template: string) {
        const cleanTitle = title.trim() || "当前书籍";
        const cleanAuthor = author.trim();
        let prompt = template.trim();
        if (!cleanAuthor) {
            for (const marker of [
                "作者{author}的",
                "作者{{author}}的",
                "作者 {author} 的",
                "作者 {{author}} 的",
            ]) {
                prompt = prompt.replaceAll(marker, "");
            }
        }
        const replaced = prompt
            .replaceAll("{title}", cleanTitle)
            .replaceAll("{{title}}", cleanTitle)
            .replaceAll("{{text}}", cleanTitle)
            .replaceAll("{author}", cleanAuthor)
            .replaceAll("{{author}}", cleanAuthor);
        return prompt.includes("{title}") ||
            prompt.includes("{{title}}") ||
            prompt.includes("{{text}}") ||
            prompt.includes("{author}") ||
            prompt.includes("{{author}}")
            ? replaced
            : `${replaced}\n\n书籍信息：${cleanAuthor ? `${cleanTitle} / ${cleanAuthor}` : cleanTitle}`;
    }

    function browserImageSizeCandidates(requested: string, target: AiTarget) {
        const fallback =
            target === "banner"
                ? "1536x768"
                : target === "standard"
                  ? "1200x1600"
                  : "1024x1792";
        const secondary =
            target === "banner"
                ? "1280x640"
                : target === "standard"
                  ? "1024x1365"
                  : "1400x2400";
        return Array.from(new Set([requested.trim(), fallback, secondary, ""]));
    }

    function browserAiRequestBodies(
        model: string,
        prompt: string,
        size: string,
        referenceDataUrl: string,
    ) {
        const baseBody = () => {
            const body: Record<string, unknown> = { model, prompt, n: 1 };
            if (size.trim()) body.size = size.trim();
            return body;
        };
        const bodies: Record<string, unknown>[] = [];
        if (referenceDataUrl) {
            for (const variant of [
                "image_array",
                "image_string",
                "image_urls_array",
                "image_array_no_format",
                "image_string_no_format",
                "image_urls_array_no_format",
            ]) {
                const body = baseBody();
                if (variant.startsWith("image_array"))
                    body.image = [referenceDataUrl];
                else if (variant.startsWith("image_string"))
                    body.image = referenceDataUrl;
                else body.image_urls = [referenceDataUrl];
                if (!variant.endsWith("_no_format"))
                    body.response_format = "b64_json";
                bodies.push(body);
            }
        }
        bodies.push({ ...baseBody(), response_format: "b64_json" });
        bodies.push(baseBody());
        const seen = new Set<string>();
        return bodies.filter((body) => {
            const key = JSON.stringify(body);
            if (seen.has(key)) return false;
            seen.add(key);
            return true;
        });
    }

    function decodeBase64Image(value: string) {
        const raw = value.trim().toLowerCase().startsWith("data:image/")
            ? value.split(",").slice(1).join(",").trim()
            : value.trim();
        if (raw.length < 80 || /\s/.test(raw)) return null;
        try {
            const binary = atob(raw);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i += 1)
                bytes[i] = binary.charCodeAt(i);
            return bytes;
        } catch {
            return null;
        }
    }

    function extractBrowserAiImageBytes(value: unknown): Uint8Array | null {
        if (typeof value === "string") return decodeBase64Image(value);
        if (Array.isArray(value)) {
            for (const item of value) {
                const bytes = extractBrowserAiImageBytes(item);
                if (bytes) return bytes;
            }
            return null;
        }
        if (value && typeof value === "object") {
            const record = value as Record<string, unknown>;
            for (const key of [
                "b64_json",
                "base64",
                "image_base64",
                "data",
                "content",
            ]) {
                const bytes = extractBrowserAiImageBytes(record[key]);
                if (bytes) return bytes;
            }
            for (const item of Object.values(record)) {
                const bytes = extractBrowserAiImageBytes(item);
                if (bytes) return bytes;
            }
        }
        return null;
    }

    async function requestBrowserAiImage(body: Record<string, unknown>) {
        if (!selectedImageProvider) throw new Error(t("未选择生图模型"));
        const response = await fetch(
            browserAiImageGenerationUrl(selectedImageProvider.baseUrl),
            {
                method: "POST",
                headers: {
                    "content-type": "application/json",
                    accept: "application/json,image/*",
                    authorization: `Bearer ${selectedImageProvider.apiKey.trim()}`,
                },
                body: JSON.stringify(body),
            },
        );
        const contentType = response.headers.get("content-type") || "";
        const buffer = new Uint8Array(await response.arrayBuffer());
        if (!response.ok) {
            const text = new TextDecoder().decode(buffer);
            let detail = text;
            try {
                const parsed = JSON.parse(text);
                detail =
                    parsed?.error?.message ||
                    parsed?.error ||
                    parsed?.message ||
                    text;
            } catch {
                // Keep raw text.
            }
            throw new Error(
                t("生图接口返回 {status}: {detail}", {
                    status: response.status,
                    detail,
                }),
            );
        }
        if (contentType.startsWith("image/")) return buffer;
        const text = new TextDecoder().decode(buffer);
        const value = JSON.parse(text);
        const bytes = extractBrowserAiImageBytes(value);
        if (!bytes) throw new Error(t("生图接口返回中没有图片数据"));
        return bytes;
    }

    async function generateAiImageInBrowser() {
        if (!selectedImageProvider || !aiReferenceBytes)
            throw new Error(t("缺少生图模型或参考图"));
        const prompt = renderAiPrompt(aiPrompt);
        const referenceDataUrl = bytesToDataUrl(
            aiReferenceBytes,
            aiReferenceMime,
        );
        let lastError: unknown = null;
        for (const size of browserImageSizeCandidates(aiSize, aiTarget)) {
            for (const body of browserAiRequestBodies(
                selectedImageProvider.model.trim(),
                prompt,
                size,
                referenceDataUrl,
            )) {
                try {
                    const bytes = await requestBrowserAiImage(body);
                    const mime = detectMime(bytes, "image/jpeg");
                    const extension =
                        mime === "image/png"
                            ? "png"
                            : mime === "image/webp"
                              ? "webp"
                              : "jpg";
                    return {
                        bytes: Array.from(bytes),
                        mime,
                        extension,
                        fileName: `${(title || aiTarget).replace(/[\\/:*?"<>|]/g, "_")}-${aiTarget}.${extension}`,
                        prompt,
                        size,
                    };
                } catch (error) {
                    lastError = error;
                }
            }
        }
        const message = String(
            (lastError as Error | null)?.message ||
                lastError ||
                t("浏览器直连生图失败"),
        );
        if (/Failed to fetch|NetworkError|Load failed/i.test(message)) {
            throw new Error(
                t(
                    "浏览器直连生图失败，通常是该 API 不允许跨域请求。Docker/正式部署请更新后走后端代理。",
                ),
            );
        }
        throw new Error(message);
    }

    async function generateAiImage() {
        if (!aiReferenceBytes) {
            await platform.message(t("请先选择参考封面。"), {
                title: t("图片处理"),
                kind: "warning",
            });
            return;
        }
        if (!selectedImageProvider) {
            await platform.message(
                t("请先在工具箱设置的 API 配置中新增生图模型。"),
                { title: t("图片处理"), kind: "warning" },
            );
            return;
        }
        if (
            !selectedImageProvider.baseUrl.trim() ||
            !selectedImageProvider.apiKey.trim() ||
            !selectedImageProvider.model.trim()
        ) {
            await platform.message(
                t("选中的生图模型缺少 API 地址、Key 或模型名。"),
                { title: t("图片处理"), kind: "warning" },
            );
            return;
        }
        if (!title.trim()) {
            await platform.message(t("请填写书名。"), {
                title: t("图片处理"),
                kind: "warning",
            });
            return;
        }
        if (!canGenerate) return;
        saveConfig();
        generating = true;
        status = t("正在生成{target}...", { target: targetLabel(aiTarget) });
        try {
            const request = {
                baseUrl: selectedImageProvider.baseUrl,
                apiKey: selectedImageProvider.apiKey,
                model: selectedImageProvider.model,
                target: aiTarget,
                title,
                author,
                prompt: aiPrompt,
                size: aiSize,
                referenceData: Array.from(aiReferenceBytes),
            };
            let result: AiImageResult;
            if (platform.isWeb) {
                try {
                    status = t("正在尝试浏览器直连生图...");
                    result = await generateAiImageInBrowser();
                } catch (error) {
                    status = t("浏览器直连失败，尝试后端代理...");
                    result = await platform.invoke<AiImageResult>(
                        "toolbox_generate_ai_image",
                        { request },
                    );
                }
            } else {
                result = await platform.invoke<AiImageResult>(
                    "toolbox_generate_ai_image",
                    { request },
                );
            }
            generated = result;
            generatedTarget = aiTarget;
            markDirty("ai");
            setGeneratedUrl(new Uint8Array(result.bytes), result.mime);
            status = t("生成完成：{size}", {
                size: result.size || "auto",
            });
        } catch (err) {
            status = t("AI 生成失败");
            await platform.message(String(err), {
                title: t("图片处理"),
                kind: "error",
            });
        } finally {
            generating = false;
        }
    }

    async function saveGenerated() {
        if (!generated) return;
        const saved = await saveBytes(
            new Uint8Array(generated.bytes),
            generated.fileName || `ai-${aiTarget}.${generated.extension}`,
            generated.extension,
        );
        if (saved) markSaved("ai");
    }

    async function saveCurrent() {
        if (activeMode === "cover") {
            await saveLocalCover();
            return;
        }
        if (activeMode === "header") {
            await saveHeaderImage();
            return;
        }
        if (activeMode === "ai" && currentGeneratedReady) {
            await saveGenerated();
        }
    }

    function handleKeydown(event: KeyboardEvent) {
        if (isInHiddenView(event.target)) return;
        if (event.key === "Escape") {
            if (customSelectOpen) {
                customSelectOpen = null;
                return;
            }
            closeWindow();
        }
    }

    /**
     * 选过图（封面 / 头图 / AI 参考图）还没导出，就是有实物在内存里：
     * Uint8Array、ImageBitmap 级的整张原图、生成结果，以及一堆滑杆状态。
     * 手机外壳裁保活页时不能把这页静默卸载 —— 没有草稿，重来一遍要重新选图、
     * 重新调参。
     */
    const ownTabId = tabIdFor(viewUrl().pathname, viewUrl().search);

    onMount(() => {
        registerKeepAliveGuard(ownTabId, () => hasUnsavedOutput);
        appSettings = loadAppSettings();
        imageProviderId =
            appSettings.aiProviders.find(
                (provider) => provider.kind === "image",
            )?.id || "";
        loadConfig();
        drawLocalPreview();
        let unlistenClose: PlatformUnlisten | undefined;
        platform
            .onCurrentWindowCloseRequested(async (event) => {
                if (closingByCode) return;
                event.preventDefault();
                await closeWindow();
            })
            .then((fn) => {
                unlistenClose = fn;
            });
        return () => {
            unlistenClose?.();
        };
    });

    onDestroy(() => {
        unregisterKeepAliveGuard(ownTabId);
        if (coverObjectUrl) URL.revokeObjectURL(coverObjectUrl);
        if (headerSourceObjectUrl) URL.revokeObjectURL(headerSourceObjectUrl);
        if (headerSampleObjectUrl) URL.revokeObjectURL(headerSampleObjectUrl);
        if (aiReferenceObjectUrl) URL.revokeObjectURL(aiReferenceObjectUrl);
        if (generatedUrl) URL.revokeObjectURL(generatedUrl);
    });
</script>

<svelte:window
    on:keydown={handleKeydown}
    on:click={() => (customSelectOpen = null)}
/>

{#snippet phoneModeTabs()}
    <div class="segmented mode-tabs phone-mode-tabs">
        <button
            type="button"
            class:active={activeMode === "cover"}
            on:click={() => setMode("cover")}>封面处理</button
        >
        <button
            type="button"
            class:active={activeMode === "header"}
            on:click={() => setMode("header")}>头图制作</button
        >
        <button
            type="button"
            class:active={activeMode === "ai"}
            on:click={() => setMode("ai")}>AI 生成</button
        >
    </div>
{/snippet}

<main class="image-tool-app">
    <!-- 真机手机已有根布局标题行，三种模式放在最上面。 -->
    {#if phoneChrome}
        {@render phoneModeTabs()}
    {/if}

    <header class="tool-head">
        {#if !phoneChrome}
            <div>
                <h1>封面与横幅</h1>
                <p>{status || "选择封面图后开始处理"}</p>
            </div>
        {/if}
        <div class="tool-head-actions">
            <div class="preview-toggle" aria-label="预览模式">
                <button
                    type="button"
                    class:active={previewMode === "compare"}
                    on:click={() => setPreviewMode("compare")}>对比</button
                >
                <button
                    type="button"
                    class:active={previewMode === "reference"}
                    on:click={() => setPreviewMode("reference")}>预览</button
                >
            </div>
            <button
                class="primary"
                type="button"
                on:click={saveCurrent}
                disabled={saveDisabled}>保存</button
            >
        </div>
    </header>

    <!-- 窄窗口 Web 没有根布局标题行：先保留原标题/说明，再放三列模式行。 -->
    {#if phoneLayout && !phoneChrome}
        {@render phoneModeTabs()}
    {/if}

    <section class="workspace">
        <div class="preview-pane">
            <div
                class="preview-stage"
                class:loaded={activeReferenceImage ||
                    (activeMode === "header" && headerSampleImage) ||
                    (activeMode === "ai" && currentGeneratedReady)}
                class:banner-stage={previewIsBanner}
                role="region"
                aria-label="图片预览区"
                on:dragover|preventDefault
                on:drop={onDrop}
            >
                {#if activeMode === "header"}
                    {#if previewMode === "reference" && headerSampleImage}
                        <button
                            class="reference-thumb"
                            type="button"
                            on:click={() => sampleInput?.click()}
                            title="更换头图样图"
                        >
                            <img src={headerSampleObjectUrl} alt="头图样图" />
                            <span>样图</span>
                        </button>
                    {/if}

                    {#if previewMode === "compare" && headerSourceImage}
                        <div
                            class="split-preview"
                            class:compare-flow-rows={compareUsesRows}
                        >
                            <div class="preview-column">
                                <div class="source-frame">
                                    <img
                                        src={headerSourceObjectUrl}
                                        alt="头图原图"
                                    />
                                </div>
                            </div>
                            <div class="preview-column">
                                <div class="result-frame">
                                    {#if headerSourceImage && headerSampleImage}
                                        <canvas
                                            bind:this={headerCanvas}
                                            class="header-canvas"
                                            aria-label="头图制作对比结果"
                                        ></canvas>
                                    {:else}
                                        <div class="preview-placeholder">
                                            <b>头图制作</b>
                                            <small>选择样图后显示结果</small>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {:else}
                        <div
                            class="header-work-preview header-preview"
                            on:pointerdown={onHeaderPointerDown}
                            on:pointermove={onHeaderPointerMove}
                            on:pointerup={onHeaderPointerEnd}
                            on:pointercancel={onHeaderPointerEnd}
                            on:wheel={onHeaderWheel}
                        >
                            {#if headerSourceImage && headerSampleImage}
                                <canvas
                                    bind:this={headerCanvas}
                                    class="header-canvas"
                                    aria-label="头图制作预览"
                                ></canvas>
                            {:else}
                                <button
                                    class="dropzone inline-dropzone"
                                    type="button"
                                    on:click={chooseActiveReference}
                                >
                                    <span class="drop-copy">
                                        <b
                                            >{!headerSampleImage
                                                ? "选择头图样图"
                                                : "选择处理图片"}</b
                                        >
                                        <small>JPG / PNG / WebP</small>
                                    </span>
                                </button>
                            {/if}
                        </div>
                    {/if}
                {:else if activeReferenceImage}
                    {#if previewMode === "reference"}
                        <button
                            class="reference-thumb"
                            type="button"
                            on:click={chooseActiveReference}
                            title="更换参考图"
                        >
                            <img src={activeReferenceObjectUrl} alt="参考图" />
                            <span>参考图</span>
                        </button>
                    {/if}

                    {#if showComparePreview}
                        <div
                            class="split-preview"
                            class:compare-flow-rows={compareUsesRows}
                        >
                            <div class="preview-column">
                                <div class="source-frame">
                                    <img
                                        src={activeReferenceObjectUrl}
                                        alt="原图"
                                    />
                                </div>
                            </div>
                            <div class="preview-column">
                                <div
                                    class="result-frame"
                                    class:standard-frame={activeMode === "ai" &&
                                        aiTarget === "standard"}
                                >
                                    {#if activeMode === "cover"}
                                        <canvas
                                            bind:this={localCanvas}
                                            class="cover-canvas"
                                            aria-label="全屏封面预览"
                                        ></canvas>
                                    {:else if activeMode === "ai" && currentGeneratedReady && generatedUrl}
                                        <img
                                            src={generatedUrl}
                                            alt="AI 生成结果"
                                        />
                                    {:else}
                                        <div class="preview-placeholder">
                                            <b>{targetLabel(aiTarget)}</b>
                                            <small>生成后显示</small>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {:else if previewIsBanner}
                        <div class="banner-preview">
                            {#if activeMode === "ai" && currentGeneratedReady && generatedUrl}
                                <img src={generatedUrl} alt="AI 生成结果" />
                            {:else}
                                <div class="preview-placeholder">
                                    <b>{previewOutputTitle}</b>
                                    <small>生成后显示横幅</small>
                                </div>
                            {/if}
                        </div>
                    {:else}
                        <div class="single-preview">
                            <div class="preview-column">
                                <div
                                    class="result-frame single-result-frame"
                                    class:standard-frame={activeMode === "ai" &&
                                        aiTarget === "standard"}
                                >
                                    {#if activeMode === "cover"}
                                        <canvas
                                            bind:this={localCanvas}
                                            class="cover-canvas"
                                            aria-label="全屏封面预览"
                                        ></canvas>
                                    {:else if activeMode === "ai" && currentGeneratedReady && generatedUrl}
                                        <img
                                            src={generatedUrl}
                                            alt="AI 生成结果"
                                        />
                                    {:else}
                                        <div class="preview-placeholder">
                                            <b>{targetLabel(aiTarget)}</b>
                                            <small>生成后显示</small>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {/if}
                {:else}
                    <button
                        class="dropzone"
                        type="button"
                        on:click={chooseActiveReference}
                    >
                        <span class="drop-copy">
                            <b>选择参考图</b>
                            <small>JPG / PNG / WebP</small>
                        </span>
                    </button>
                {/if}
            </div>
            <input
                bind:this={fileInput}
                class="file-input"
                type="file"
                accept="image/*"
                on:change={onFileChange}
            />
            <input
                bind:this={sampleInput}
                class="file-input"
                type="file"
                accept="image/*"
                on:change={onSampleFileChange}
            />
            <div class="preview-meta">
                <span
                    >{activeMode === "header"
                        ? `${headerSampleName || "未选择样图"} / ${headerSourceName || "未选择处理图"}`
                        : activeReferenceName || "未选择图片"}</span
                >
                <span
                    >{activeMode === "header" && headerSampleImage
                        ? `${headerSampleImage.width}x${headerSampleImage.height}`
                        : previewIsBanner
                          ? targetRatioLabel(previewTarget)
                          : previewOutputTitle}</span
                >
            </div>
        </div>

        <div class="control-pane">
            <section class="panel unified-panel">
                {#if !phoneLayout}
                    <div class="segmented mode-tabs">
                        <button
                            type="button"
                            class:active={activeMode === "cover"}
                            on:click={() => setMode("cover")}>封面处理</button
                        >
                        <button
                            type="button"
                            class:active={activeMode === "header"}
                            on:click={() => setMode("header")}>头图制作</button
                        >
                        <button
                            type="button"
                            class:active={activeMode === "ai"}
                            on:click={() => setMode("ai")}>AI 生成</button
                        >
                    </div>
                {/if}

                {#if activeMode === "cover"}
                    <div class="mode-section">
                        <div class="panel-head">
                            <h2>全屏封面</h2>
                            <div class="panel-actions">
                                <button
                                    class="ghost"
                                    type="button"
                                    on:click={resetCoverSettings}>重置</button
                                >
                                <button
                                    class="ghost danger-ghost"
                                    type="button"
                                    on:click={clearCoverImage}
                                    disabled={!coverImage}>清空</button
                                >
                            </div>
                        </div>
                        <div class="segmented">
                            <button
                                type="button"
                                class:active={bgMode === "blur"}
                                on:click={() => {
                                    bgMode = "blur";
                                    drawLocalPreview();
                                }}>模糊背景</button
                            >
                            <button
                                type="button"
                                class:active={bgMode === "solid"}
                                on:click={() => {
                                    bgMode = "solid";
                                    drawLocalPreview();
                                }}>纯色背景</button
                            >
                        </div>
                        <div class="form-grid two">
                            <label
                                >宽度<input
                                    type="number"
                                    min="320"
                                    max="6000"
                                    bind:value={localWidth}
                                    on:change={drawLocalPreview}
                                /></label
                            >
                            <label
                                >高度<input
                                    type="number"
                                    min="320"
                                    max="6000"
                                    bind:value={localHeight}
                                    on:change={drawLocalPreview}
                                /></label
                            >
                            <label
                                >前景大小<input
                                    type="range"
                                    min="40"
                                    max="100"
                                    step="1"
                                    bind:value={foregroundSize}
                                    on:input={drawLocalPreview}
                                /></label
                            >
                            <label
                                >圆角<input
                                    type="range"
                                    min="0"
                                    max="100"
                                    step="1"
                                    bind:value={radius}
                                    on:input={drawLocalPreview}
                                /></label
                            >
                            <label
                                >横向偏移<input
                                    type="range"
                                    min="-30"
                                    max="30"
                                    step="1"
                                    bind:value={offsetX}
                                    on:input={drawLocalPreview}
                                /></label
                            >
                            <label
                                >纵向偏移<input
                                    type="range"
                                    min="-30"
                                    max="30"
                                    step="1"
                                    bind:value={offsetY}
                                    on:input={drawLocalPreview}
                                /></label
                            >
                            {#if bgMode === "blur"}
                                <label
                                    >模糊<input
                                        type="range"
                                        min="0"
                                        max="80"
                                        step="1"
                                        bind:value={blur}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                            {:else}
                                <label
                                    >背景色<input
                                        type="color"
                                        bind:value={solidColor}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                            {/if}
                            <label
                                >格式
                                <!-- svelte-ignore a11y-click-events-have-key-events -->
                                <!-- svelte-ignore a11y-no-static-element-interactions -->
                                <div
                                    class="custom-select"
                                    class:open={customSelectOpen === "format"}
                                    on:click|stopPropagation
                                >
                                    <button
                                        class="custom-select-trigger"
                                        type="button"
                                        on:click={() =>
                                            toggleCustomSelect("format")}
                                    >
                                        <span>{selectedFormatLabel}</span>
                                        <span
                                            class="select-caret"
                                            aria-hidden="true"
                                        ></span>
                                    </button>
                                    {#if customSelectOpen === "format"}
                                        <div class="custom-select-menu">
                                            {#each FORMAT_OPTIONS as option}
                                                <button
                                                    type="button"
                                                    class:active={outputFormat ===
                                                        option.value}
                                                    on:click={() =>
                                                        selectOutputFormat(
                                                            option.value,
                                                        )}
                                                    >{option.label}</button
                                                >
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                            </label>
                        </div>
                        <div class="toggles">
                            <label class:checked={showGuides}
                                ><input
                                    type="checkbox"
                                    bind:checked={showGuides}
                                    on:change={drawLocalPreview}
                                /><span>参考线</span></label
                            >
                            <label class:checked={shadow}
                                ><input
                                    type="checkbox"
                                    bind:checked={shadow}
                                    on:change={drawLocalPreview}
                                /><span>阴影</span></label
                            >
                            {#if showGuides}
                                <label
                                    >参考线色<input
                                        type="color"
                                        bind:value={guideColor}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                            {/if}
                            {#if shadow}
                                <label
                                    >阴影色<input
                                        type="color"
                                        bind:value={shadowColor}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                                <label
                                    >阴影模糊<input
                                        type="range"
                                        min="0"
                                        max="60"
                                        bind:value={shadowBlur}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                                <label
                                    >阴影扩散<input
                                        type="range"
                                        min="0"
                                        max="30"
                                        bind:value={shadowSpread}
                                        on:input={drawLocalPreview}
                                    /></label
                                >
                            {/if}
                        </div>
                    </div>
                {:else if activeMode === "header"}
                    <div class="mode-section">
                        <div class="panel-head">
                            <h2>头图制作</h2>
                            <div class="panel-actions">
                                <button
                                    class="ghost"
                                    type="button"
                                    on:click={resetHeaderTransform}
                                    disabled={!headerSourceImage ||
                                        !headerSampleImage}>重置</button
                                >
                                <button
                                    class="ghost danger-ghost"
                                    type="button"
                                    on:click={clearHeaderImages}
                                    disabled={!headerSourceImage &&
                                        !headerSampleImage}>清空</button
                                >
                            </div>
                        </div>
                        <div class="header-file-actions">
                            <button
                                class="ghost"
                                type="button"
                                on:click={() => sampleInput?.click()}
                                >选择头图样图</button
                            >
                            <button
                                class="ghost"
                                type="button"
                                on:click={() => chooseImage("headerSource")}
                                >选择处理图片</button
                            >
                        </div>
                        <div class="header-file-meta">
                            <span title={headerSampleName}
                                >{headerSampleName || "未选择样图"}</span
                            >
                            <span title={headerSourceName}
                                >{headerSourceName || "未选择处理图"}</span
                            >
                        </div>
                        <div class="form-grid two">
                            <label class="wide"
                                >缩放<input
                                    type="range"
                                    min="0.05"
                                    max="20"
                                    step="0.01"
                                    bind:value={headerScale}
                                    on:input={() => drawHeaderPreview()}
                                /></label
                            >
                            <label
                                >横向偏移<input
                                    type="number"
                                    step="1"
                                    bind:value={headerOffsetX}
                                    on:input={() => drawHeaderPreview()}
                                /></label
                            >
                            <label
                                >纵向偏移<input
                                    type="number"
                                    step="1"
                                    bind:value={headerOffsetY}
                                    on:input={() => drawHeaderPreview()}
                                /></label
                            >
                        </div>
                        <p class="header-hint">
                            拖动画布调整位置，滚轮缩放；样图导入时长边规范为
                            1080，保存时按规范后的样图尺寸输出
                            PNG，并保留透明边缘。
                        </p>
                    </div>
                {:else}
                    <div class="mode-section">
                        <div class="panel-head">
                            <h2>AI 生成</h2>
                            <div class="panel-actions">
                                <button
                                    class="ghost"
                                    type="button"
                                    on:click={resetAiSettings}>重置</button
                                >
                                <button
                                    class="ghost danger-ghost"
                                    type="button"
                                    on:click={clearAiReference}
                                    disabled={!aiReferenceImage && !generated}
                                    >清空</button
                                >
                                <button
                                    class="primary"
                                    type="button"
                                    on:click={generateAiImage}
                                    disabled={!canGenerate || generating}
                                    >{generating ? "生成中" : "生成"}</button
                                >
                            </div>
                        </div>
                        <div class="segmented ai-targets">
                            {#each AI_TARGETS as target}
                                <button
                                    type="button"
                                    class:active={aiTarget === target.id}
                                    on:click={() => setAiTarget(target.id)}
                                >
                                    <span>{t(target.label)}</span>
                                    <small>{target.ratioLabel}</small>
                                </button>
                            {/each}
                        </div>
                        <div class="form-grid ai-config">
                            <label
                                >书名<input
                                    bind:value={title}
                                    placeholder="必填"
                                    required
                                /></label
                            >
                            <label
                                >作者<input
                                    bind:value={author}
                                    placeholder="可选"
                                /></label
                            >
                            <label class="wide"
                                >尺寸<input
                                    bind:value={aiSize}
                                    placeholder={defaultSize(aiTarget)}
                                /></label
                            >
                            <label class="wide"
                                >生图模型
                                <!-- svelte-ignore a11y-click-events-have-key-events -->
                                <!-- svelte-ignore a11y-no-static-element-interactions -->
                                <div
                                    class="custom-select"
                                    class:open={customSelectOpen === "provider"}
                                    class:disabled={imageProviders.length === 0}
                                    on:click|stopPropagation
                                >
                                    <button
                                        class="custom-select-trigger"
                                        type="button"
                                        disabled={imageProviders.length === 0}
                                        on:click={() =>
                                            toggleCustomSelect("provider")}
                                    >
                                        <span>{selectedProviderLabel}</span>
                                        <span
                                            class="select-caret"
                                            aria-hidden="true"
                                        ></span>
                                    </button>
                                    {#if customSelectOpen === "provider"}
                                        <div class="custom-select-menu">
                                            {#each imageProviders as provider}
                                                <button
                                                    type="button"
                                                    class:active={imageProviderId ===
                                                        provider.id}
                                                    on:click={() =>
                                                        selectImageProvider(
                                                            provider.id,
                                                        )}
                                                >
                                                    {localizeAiProviderName(
                                                        provider.name,
                                                    ) || provider.model} · {provider.model}
                                                </button>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                            </label>
                            {#if selectedImageProvider}
                                <div class="provider-summary wide">
                                    <span>{selectedImageProvider.baseUrl}</span>
                                    <span
                                        >{selectedImageProvider.apiKey
                                            ? "已保存 Key"
                                            : "未填写 Key"}</span
                                    >
                                </div>
                            {:else}
                                <div class="provider-summary wide">
                                    未配置生图模型，请到工具箱设置 > API
                                    配置新增。
                                </div>
                            {/if}
                            <label class="wide"
                                >提示词<textarea rows="4" bind:value={aiPrompt}
                                ></textarea></label
                            >
                        </div>
                        <div class="actions">
                            <button
                                class="ghost"
                                type="button"
                                on:click={refreshImageProviders}
                                >刷新模型</button
                            >
                            <button
                                class="ghost"
                                type="button"
                                on:click={() => {
                                    aiPrompt = defaultPrompt(aiTarget);
                                }}>恢复提示词</button
                            >
                        </div>
                    </div>
                {/if}
            </section>
        </div>
    </section>
</main>

<style>
    :global(body) {
        margin: 0;
        overflow: hidden;
        background: var(--color-canvas);
    }

    .image-tool-app {
        box-sizing: border-box;
        height: 100vh;
        min-height: 0;
        display: flex;
        flex-direction: column;
        gap: 14px;
        padding: 20px 22px;
        background: var(--color-canvas);
        color: var(--color-text);
        font-family: var(--font-ui);
    }

    .tool-head {
        flex: 0 0 auto;
        display: flex;
        justify-content: space-between;
        gap: 20px;
        align-items: flex-start;
    }

    .tool-head-actions {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
    }

    .tool-head h1 {
        margin: 0;
        font-size: 22px;
        line-height: 1.2;
    }

    .tool-head p {
        margin: 6px 0 0;
        color: var(--color-muted);
        font-size: 13px;
    }

    .preview-toggle {
        display: inline-grid;
        grid-template-columns: repeat(2, minmax(54px, 1fr));
        gap: 0;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        overflow: hidden;
        background: var(--color-surface);
    }

    .preview-toggle button {
        min-height: var(--control-height);
        padding: 7px 10px;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: var(--color-muted);
        font: inherit;
        font-size: 13px;
        font-weight: 800;
    }

    .preview-toggle button + button {
        border-left: 1px solid var(--color-border);
    }

    .preview-toggle button.active {
        background: var(--color-accent);
        color: var(--color-accent-contrast);
    }

    .preview-toggle button:disabled {
        cursor: not-allowed;
        color: color-mix(in srgb, var(--color-muted) 55%, transparent);
        background: color-mix(
            in srgb,
            var(--color-surface-muted) 65%,
            transparent
        );
    }

    .workspace {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(560px, 1.4fr) minmax(380px, 0.6fr);
        gap: 16px;
        overflow: hidden;
    }

    .preview-pane,
    .control-pane,
    .panel {
        min-width: 0;
    }

    .preview-pane {
        display: flex;
        flex-direction: column;
        gap: 8px;
        min-height: 0;
        overflow: hidden;
    }

    .preview-stage {
        position: relative;
        flex: 1;
        min-height: 0;
        width: 100%;
        padding: 18px;
        box-sizing: border-box;
        border: 1px dashed var(--color-border-strong);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        display: flex;
        align-items: center;
        justify-content: center;
        overflow: hidden;
    }

    .preview-stage.loaded {
        border-color: transparent;
        background:
            linear-gradient(
                45deg,
                color-mix(in srgb, var(--color-border) 35%, transparent) 25%,
                transparent 25%
            ),
            linear-gradient(
                -45deg,
                color-mix(in srgb, var(--color-border) 35%, transparent) 25%,
                transparent 25%
            ),
            linear-gradient(
                45deg,
                transparent 75%,
                color-mix(in srgb, var(--color-border) 35%, transparent) 75%
            ),
            linear-gradient(
                -45deg,
                transparent 75%,
                color-mix(in srgb, var(--color-border) 35%, transparent) 75%
            ),
            var(--color-surface);
        background-position:
            0 0,
            0 8px,
            8px -8px,
            -8px 0;
        background-size: 16px 16px;
    }

    .dropzone {
        flex: 1;
        min-height: 0;
        height: 100%;
        width: 100%;
        padding: 18px;
        box-sizing: border-box;
        border: 1px dashed var(--color-border-strong);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        display: flex;
        align-items: center;
        justify-content: center;
        overflow: hidden;
        cursor: pointer;
    }

    .dropzone:hover {
        background: var(--color-hover);
    }

    .drop-copy {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
    }

    .drop-copy b {
        font-size: 16px;
    }

    .drop-copy small,
    .preview-meta {
        color: var(--color-muted);
        font-size: 12px;
    }

    .cover-canvas {
        width: auto;
        height: auto;
        max-width: 100%;
        max-height: 100%;
        display: block;
    }

    .header-canvas {
        max-width: 100%;
        max-height: 100%;
        display: block;
    }

    .reference-thumb {
        position: absolute;
        top: 12px;
        right: 12px;
        z-index: 3;
        width: 88px;
        padding: 6px;
        display: grid;
        gap: 4px;
        justify-items: center;
        background: color-mix(in srgb, var(--color-surface) 92%, transparent);
        box-shadow: var(--shadow-sm);
    }

    .reference-thumb img {
        width: 100%;
        max-height: 96px;
        object-fit: contain;
        border-radius: var(--radius-xs);
        background: var(--color-canvas);
    }

    .reference-thumb span {
        color: var(--color-muted);
        font-size: 11px;
        font-weight: 800;
    }

    .split-preview {
        width: min(100%, 840px);
        height: 100%;
        min-height: 0;
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 18px;
        align-items: stretch;
        box-sizing: border-box;
    }

    .split-preview.compare-flow-rows {
        width: min(100%, 760px);
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: repeat(2, minmax(0, 1fr));
    }

    /* 窄屏会把普通对比框固定为 320px；上下对比时两行只有一半高度，
       再保留 320px 会让两张图叠进对方一行。这里让框严格跟随网格行高。 */
    .split-preview.compare-flow-rows .source-frame,
    .split-preview.compare-flow-rows .result-frame {
        height: 100%;
    }

    .single-preview {
        width: min(100%, 760px);
        height: 100%;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(0, 1fr);
        align-items: stretch;
        justify-items: center;
    }

    .preview-column {
        min-width: 0;
        min-height: 0;
        display: grid;
        grid-template-rows: minmax(0, 1fr);
        gap: 0;
    }

    .source-frame,
    .result-frame,
    .banner-preview,
    .header-work-preview {
        min-width: 0;
        min-height: 0;
        border: 0;
        border-radius: 0;
        background: transparent;
        display: flex;
        align-items: center;
        justify-content: center;
        overflow: hidden;
        box-shadow: none;
    }

    .source-frame,
    .result-frame,
    .header-work-preview {
        width: 100%;
        height: 100%;
        justify-self: center;
    }

    .single-result-frame {
        width: 100%;
    }

    .source-frame img,
    .result-frame img,
    .banner-preview img,
    .header-work-preview canvas {
        max-width: 100%;
        max-height: 100%;
        object-fit: contain;
    }

    .standard-frame {
        aspect-ratio: auto;
    }

    .banner-preview {
        width: min(100%, 760px);
        height: 100%;
        max-height: 430px;
        margin: auto;
    }

    .header-work-preview {
        width: min(100%, 840px);
        margin: auto;
    }

    .header-preview {
        cursor: grab;
        touch-action: none;
        user-select: none;
    }

    .header-preview:active {
        cursor: grabbing;
    }

    .preview-placeholder {
        display: grid;
        gap: 6px;
        justify-items: center;
        color: var(--color-muted);
        text-align: center;
    }

    .preview-placeholder b {
        color: var(--color-text);
        font-size: 14px;
    }

    .file-input {
        display: none;
    }

    .preview-meta {
        flex: 0 0 auto;
        min-height: 28px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
    }

    .preview-meta span:first-child {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .preview-meta span:last-child {
        flex: 0 0 auto;
    }

    .control-pane {
        min-height: 0;
        height: calc(100% - 36px);
        box-sizing: border-box;
        display: flex;
        flex-direction: column;
        gap: 12px;
        align-self: start;
        overflow-y: hidden;
        overflow-x: hidden;
        padding-right: 0;
        padding-bottom: 0;
    }

    .panel {
        flex: 0 0 auto;
        box-sizing: border-box;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        box-shadow: var(--shadow-xs);
        padding: 14px;
    }

    .unified-panel {
        min-height: 0;
        height: 100%;
        overflow-y: auto;
    }

    .unified-panel:has(.toggles) {
        overflow-y: hidden;
    }

    .panel-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 12px;
    }

    .panel-head h2 {
        margin: 0;
        font-size: 15px;
        line-height: 1.3;
    }

    .panel-actions {
        flex: 0 0 auto;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
    }

    .segmented {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 6px;
        margin-bottom: 12px;
    }

    .mode-tabs {
        grid-template-columns: repeat(3, minmax(0, 1fr));
        margin-bottom: 16px;
    }

    .segmented.ai-targets {
        grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .segmented.ai-targets button {
        display: grid;
        gap: 2px;
        justify-items: center;
    }

    .segmented.ai-targets small {
        color: currentColor;
        opacity: 0.72;
        font-size: 11px;
        font-weight: 700;
        line-height: 1.2;
    }

    button {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        color: var(--color-text);
        font: inherit;
        font-size: 13px;
        font-weight: 700;
        cursor: pointer;
    }

    button:disabled {
        cursor: not-allowed;
        opacity: 0.55;
    }

    .segmented button,
    .ghost,
    .primary {
        min-height: var(--control-height);
        padding: 7px 12px;
    }

    .segmented button.active,
    .primary {
        border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
        background: var(--color-accent);
        color: var(--color-accent-contrast);
    }

    .segmented button.active:hover:not(:disabled),
    .preview-toggle button.active:hover:not(:disabled),
    .primary:hover:not(:disabled) {
        border-color: color-mix(
            in srgb,
            var(--color-accent-deep) 48%,
            transparent
        );
        background: var(--color-accent-deep);
        color: var(--color-accent-contrast);
    }

    .danger-ghost {
        color: var(--color-danger);
        background: var(--color-danger-soft);
        border-color: color-mix(in srgb, var(--color-danger) 24%, transparent);
    }

    .mode-section {
        min-width: 0;
    }

    .header-file-actions {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 8px;
        margin-bottom: 10px;
    }

    .header-file-meta {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 8px;
        margin-bottom: 12px;
        color: var(--color-muted);
        font-size: 12px;
    }

    .header-file-meta span {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .header-hint {
        margin: 12px 0 0;
        color: var(--color-muted);
        font-size: 12px;
        line-height: 1.6;
    }

    .ghost:hover:not(:disabled),
    .segmented button:hover:not(:disabled) {
        background: var(--color-hover);
    }

    .danger-ghost:hover:not(:disabled) {
        color: var(--color-danger);
        background: color-mix(in srgb, var(--color-danger-soft) 72%, white);
        border-color: color-mix(in srgb, var(--color-danger) 36%, transparent);
    }

    .form-grid {
        display: grid;
        grid-template-columns: 1fr;
        gap: 10px;
    }

    .form-grid.two {
        grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .form-grid.ai-config {
        grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    label {
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 5px;
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 700;
    }

    input,
    textarea {
        box-sizing: border-box;
        width: 100%;
        min-height: var(--control-height);
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas);
        color: var(--color-text);
        padding: 7px 9px;
        font: inherit;
        font-size: 13px;
    }

    .custom-select {
        position: relative;
        min-width: 0;
        width: 100%;
    }

    .custom-select-trigger {
        box-sizing: border-box;
        width: 100%;
        min-height: var(--control-height);
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        padding: 7px 12px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: color-mix(
            in srgb,
            var(--color-surface) 94%,
            var(--color-accent-quiet)
        );
        color: var(--color-text);
        box-shadow: var(--shadow-xs);
        text-align: left;
    }

    .custom-select-trigger span:first-child {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .custom-select.open .custom-select-trigger,
    .custom-select-trigger:focus-visible {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring), var(--shadow-xs);
    }

    .custom-select.disabled .custom-select-trigger {
        cursor: not-allowed;
        opacity: 0.55;
    }

    .select-caret {
        flex: 0 0 auto;
        width: 0;
        height: 0;
        border-left: 6px solid transparent;
        border-right: 6px solid transparent;
        border-top: 7px solid var(--color-text-soft);
    }

    .custom-select-menu {
        position: absolute;
        z-index: 20;
        top: calc(100% + 6px);
        left: 0;
        right: 0;
        box-sizing: border-box;
        max-height: 220px;
        overflow-y: auto;
        padding: 4px;
        border: 1px solid var(--color-border-strong);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        box-shadow: var(--shadow-pop);
    }

    .custom-select-menu button {
        width: 100%;
        min-height: var(--control-height);
        padding: 7px 10px;
        border: 0;
        border-radius: var(--radius-xs);
        background: transparent;
        color: var(--color-text);
        text-align: left;
        font-size: 13px;
        font-weight: 700;
    }

    .custom-select-menu button:hover,
    .custom-select-menu button:focus-visible {
        outline: none;
        background: var(--color-hover);
        color: var(--color-accent-deep);
    }

    .custom-select-menu button.active {
        background: var(--color-accent);
        color: var(--color-accent-contrast, var(--color-surface));
    }

    input[type="range"] {
        padding: 0;
    }

    input[type="checkbox"] {
        width: 18px;
        min-height: 18px;
        height: 18px;
        flex: 0 0 auto;
        padding: 0;
        accent-color: var(--color-accent);
    }

    input[type="color"] {
        padding: 2px;
    }

    textarea {
        resize: vertical;
        line-height: 1.5;
    }

    .wide {
        grid-column: 1 / -1;
    }

    .provider-summary {
        box-sizing: border-box;
        min-width: 0;
        padding: 8px 10px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface-soft);
        color: var(--color-muted);
        display: flex;
        flex-wrap: wrap;
        gap: 6px 12px;
        font-size: 12px;
        line-height: 1.4;
    }

    .provider-summary span {
        min-width: 0;
        overflow-wrap: anywhere;
    }

    .toggles {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 8px;
        margin-top: 12px;
    }

    .toggles label {
        min-width: 0;
        min-height: 40px;
        box-sizing: border-box;
        flex-direction: row;
        align-items: center;
        justify-content: flex-start;
        gap: 10px;
        padding: 9px 11px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface-soft);
        color: var(--color-text-soft);
        cursor: pointer;
    }

    .toggles label.checked {
        border-color: color-mix(in srgb, var(--color-accent) 34%, transparent);
        background: var(--color-accent-quiet);
        color: var(--color-accent-deep);
    }

    .toggles label:has(input[type="range"]),
    .toggles label:has(input[type="color"]) {
        min-height: auto;
        padding: 0;
        border: 0;
        background: transparent;
        color: var(--color-muted);
        cursor: default;
        display: flex;
        flex-direction: column;
        align-items: stretch;
        gap: 5px;
    }

    .actions {
        margin-top: 10px;
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }

    @media (max-width: 980px) {
        .image-tool-app {
            padding: 14px;
        }

        .workspace {
            grid-template-columns: 1fr;
            overflow: auto;
        }

        .preview-pane {
            min-height: 520px;
        }

        .split-preview {
            padding-right: 92px;
        }

        .control-pane {
            height: auto;
            overflow: visible;
        }

        .unified-panel {
            height: auto;
            overflow: visible;
        }

        .form-grid.two {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }
    }

    @media (max-width: 640px) {
        .tool-head {
            flex-direction: column;
            gap: 10px;
        }

        .tool-head-actions {
            width: 100%;
            justify-content: space-between;
        }

        .form-grid.two {
            grid-template-columns: 1fr;
        }

        .form-grid.ai-config,
        .segmented.ai-targets,
        .mode-tabs {
            grid-template-columns: 1fr;
        }

        .preview-pane {
            min-height: 460px;
        }

        .split-preview {
            padding-right: 0;
            overflow: auto;
        }

        .split-preview.compare-flow-rows {
            grid-template-columns: minmax(0, 1fr);
        }

        .reference-thumb {
            width: 72px;
            top: 10px;
            right: 10px;
        }

        .source-frame,
        .result-frame {
            height: 320px;
        }
    }

    /* 这里原来有一条 :global(:root[data-tepub-layout="phone"] body) { overflow: auto }。
       它是为「整页跟着 body 滚」写的，但手机外壳里滚动发生在 .ka-view，
       body 本身没有溢出；页面被保活卸载后这条规则也仍留在文档里。
       现在滚动统一由外壳负责，页面不再改 body。 */

    :global(:root[data-tepub-layout="phone"]) .image-tool-app {
        min-height: 100dvh;
        height: auto;
        /* 用 token 而不是裸 env()：安卓 WebView 的 env(safe-area-inset-*)
           恒为 0，--safe-top 里带着 --safe-top-min 的 24px 兜底。裸 env()
           特异性相同但注入更晚，会把兜底整个绕过去，状态栏压住第一行。 */
        padding: max(12px, var(--safe-top)) 12px max(12px, var(--safe-bottom));
        overflow: visible;
    }

    :global(:root[data-tepub-layout="phone"]) .tool-head {
        flex-direction: column;
        gap: 10px;
    }

    :global(:root[data-tepub-layout="phone"]) .tool-head-actions {
        width: 100%;
        justify-content: space-between;
    }

    :global(:root[data-tepub-layout="phone"]) .mode-tabs.phone-mode-tabs {
        grid-template-columns: repeat(3, minmax(0, 1fr));
        margin-bottom: 0;
    }

    :global(:root[data-tepub-layout="phone"]) .phone-mode-tabs button {
        min-height: var(--tap-target);
        padding: 8px 6px;
        white-space: nowrap;
    }

    :global(:root[data-tepub-layout="phone"]) .workspace {
        grid-template-columns: 1fr;
        overflow: visible;
    }

    :global(:root[data-tepub-layout="phone"]) .preview-pane {
        min-height: min(460px, 58dvh);
    }

    :global(:root[data-tepub-layout="phone"]) .control-pane,
    :global(:root[data-tepub-layout="phone"]) .unified-panel {
        height: auto;
        overflow: visible;
    }

    :global(:root[data-tepub-layout="phone"]) .form-grid.two,
    :global(:root[data-tepub-layout="phone"]) .form-grid.ai-config,
    :global(:root[data-tepub-layout="phone"]) .segmented.ai-targets,
    :global(:root[data-tepub-layout="phone"]) .mode-tabs {
        grid-template-columns: 1fr;
    }

    :global(:root[data-tepub-layout="phone"]) .split-preview {
        padding-right: 0;
        overflow: auto;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        grid-template-rows: minmax(0, 1fr);
    }

    :global(:root[data-tepub-layout="phone"]) .split-preview.compare-flow-rows {
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: repeat(2, minmax(0, 1fr));
    }

    :global(:root[data-tepub-layout="phone"])
        .split-preview:not(.compare-flow-rows)
        .source-frame,
    :global(:root[data-tepub-layout="phone"])
        .split-preview:not(.compare-flow-rows)
        .result-frame {
        height: 320px;
    }

    :global(:root[data-tepub-layout="phone"]) .single-preview .result-frame {
        height: 320px;
    }
</style>
