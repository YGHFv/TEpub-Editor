<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { platform, type PlatformUnlisten } from "$lib/platform";
  import { loadAppSettings, type AiProviderConfig, type GlobalAppSettings } from "$lib/appSettings";

  type AiTarget = "duokan" | "standard" | "banner";
  type ToolMode = "cover" | "header" | "ai";
  type PreviewMode = "compare" | "reference";
  type ImagePurpose = "cover" | "headerSource" | "ai";
  type HeaderDragState = { pointerId: number; lastX: number; lastY: number } | null;

  type AiImageResult = {
    bytes: number[];
    mime: string;
    extension: string;
    fileName: string;
    prompt: string;
    size: string;
  };

  const CONFIG_KEY = "tepub-image-tools-ai-config";
  const DEFAULT_COVER_PROMPT =
    "帮我制作作者{author}的小说《{title}》的全屏竖版封面，比例适合电子书阅读器全屏展示，可以参考原书籍封面。保留书籍气质，画面精致、清晰、无水印。";
  const DEFAULT_STANDARD_PROMPT =
    "帮我制作作者{author}的小说《{title}》的标准书籍封面，比例严格为3:4，可以参考原书籍封面。保留书籍气质，画面精致、清晰、无水印，适合 EPUB 封面使用。";
  const DEFAULT_BANNER_PROMPT =
    "帮我制作作者{author}的小说《{title}》的阅微横幅封面，比例严格为2:1，可以参考原书籍封面。画面适合小说阅读应用首页横幅，主体清晰，构图横向展开，精致、清晰、无水印。";

  const AI_TARGETS: Array<{ id: AiTarget; label: string; size: string; ratioLabel: string }> = [
    { id: "standard", label: "标准封面", size: "1200x1600", ratioLabel: "3:4" },
    { id: "duokan", label: "全屏封面", size: "1400x2400", ratioLabel: "7:12" },
    { id: "banner", label: "阅微横幅", size: "2048x1024", ratioLabel: "2:1" },
  ];
  const FORMAT_OPTIONS: Array<{ value: "jpeg" | "png" | "webp"; label: string }> = [
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
  let previewMode: PreviewMode = "compare";
  let customSelectOpen: "format" | "provider" | null = null;
  let status = "";
  let closingByCode = false;

  $: imageProviders = appSettings.aiProviders.filter((provider) => provider.kind === "image");
  $: if (imageProviderId && !imageProviders.some((provider) => provider.id === imageProviderId)) {
    imageProviderId = imageProviders[0]?.id || "";
  }
  $: if (!imageProviderId && imageProviders.length > 0) {
    imageProviderId = imageProviders[0].id;
  }
  $: selectedImageProvider = imageProviders.find((provider) => provider.id === imageProviderId) || imageProviders[0];
  $: canGenerate = Boolean(
    !!aiReferenceBytes
    && !!selectedImageProvider
    && selectedImageProvider.baseUrl.trim()
    && selectedImageProvider.apiKey.trim()
    && selectedImageProvider.model.trim()
    && title.trim()
    && aiPrompt.trim(),
  );
  $: currentGeneratedReady = !!generated && generatedTarget === aiTarget;
  $: previewTarget = activeMode === "ai" ? aiTarget : activeMode === "header" ? "banner" : "duokan";
  $: previewIsBanner = previewTarget === "banner";
  $: showComparePreview = previewMode === "compare" && !previewIsBanner;
  $: activeReferenceImage = activeMode === "cover" ? coverImage : activeMode === "ai" ? aiReferenceImage : headerSourceImage;
  $: activeReferenceName = activeMode === "cover" ? coverName : activeMode === "ai" ? aiReferenceName : headerSourceName;
  $: activeReferenceObjectUrl = activeMode === "cover" ? coverObjectUrl : activeMode === "ai" ? aiReferenceObjectUrl : headerSourceObjectUrl;
  $: previewOutputTitle =
    activeMode === "cover"
      ? `全屏封面 ${localWidth}x${localHeight}`
      : activeMode === "header"
        ? "头图制作"
        : targetLabel(aiTarget);
  $: selectedFormatLabel = FORMAT_OPTIONS.find((option) => option.value === outputFormat)?.label || "JPEG";
  $: selectedProviderLabel =
    selectedImageProvider
      ? `${selectedImageProvider.name || selectedImageProvider.model} · ${selectedImageProvider.model}`
      : imageProviders.length === 0
        ? "未配置生图模型"
        : "选择生图模型";
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
      img.onerror = () => reject(new Error("图片加载失败"));
      img.src = url;
    });
    return img;
  }

  function defaultPrompt(target: AiTarget) {
    if (target === "banner") return DEFAULT_BANNER_PROMPT;
    if (target === "standard") return DEFAULT_STANDARD_PROMPT;
    return DEFAULT_COVER_PROMPT;
  }

  function defaultSize(target: AiTarget) {
    return AI_TARGETS.find((item) => item.id === target)?.size || "1400x2400";
  }

  function targetLabel(target: AiTarget) {
    return AI_TARGETS.find((item) => item.id === target)?.label || "全屏封面";
  }

  function targetRatioLabel(target: AiTarget) {
    return AI_TARGETS.find((item) => item.id === target)?.ratioLabel || "7:12";
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
    const nextProviders = nextSettings.aiProviders.filter((provider) => provider.kind === "image");
    appSettings = nextSettings;
    imageProviderId = nextProviders.some((provider) => provider.id === imageProviderId) ? imageProviderId : nextProviders[0]?.id || "";
    status = nextProviders.length > 0 ? "已刷新生图模型" : "未找到已保存的生图模型";
  }

  function detectMime(bytes: Uint8Array, fallback: string) {
    if (bytes.length >= 8 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) {
      return "image/png";
    }
    if (bytes.length >= 12 && bytes[0] === 0x52 && bytes[1] === 0x49 && bytes[8] === 0x57 && bytes[9] === 0x45) {
      return "image/webp";
    }
    if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8) {
      return "image/jpeg";
    }
    return fallback || "image/jpeg";
  }

  async function loadImageFile(file: File, purpose: ImagePurpose = fileInputPurpose) {
    if (!file.type.startsWith("image/")) {
      await platform.message("请选择图片文件。", { title: "图片处理", kind: "warning" });
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
      if (headerSourceObjectUrl) URL.revokeObjectURL(headerSourceObjectUrl);
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
    status = purpose === "headerSource" ? `已载入处理图片 ${file.name}` : `已载入参考图 ${file.name}`;
    await tick();
    drawLocalPreview();
    drawHeaderPreview();
  }

  async function loadHeaderSampleFile(file: File) {
    if (!file.type.startsWith("image/")) {
      await platform.message("请选择图片文件。", { title: "图片处理", kind: "warning" });
      return;
    }
    if (headerSampleObjectUrl) URL.revokeObjectURL(headerSampleObjectUrl);
    headerSampleObjectUrl = URL.createObjectURL(file);
    headerSampleImage = await imageFromObjectUrl(headerSampleObjectUrl);
    headerSampleName = file.name;
    status = `已载入头图样图 ${file.name}`;
    resetHeaderTransform();
    await tick();
    drawHeaderPreview();
  }

  function onFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      loadImageFile(file, fileInputPurpose).catch((err) => {
        status = "图片载入失败";
        platform.message(String(err?.message ?? err), { title: "图片处理", kind: "error" });
      });
    }
    input.value = "";
  }

  function onSampleFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (file) {
      loadHeaderSampleFile(file).catch((err) => {
        status = "头图样图载入失败";
        platform.message(String(err?.message ?? err), { title: "图片处理", kind: "error" });
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
          status = "头图样图载入失败";
          platform.message(String(err?.message ?? err), { title: "图片处理", kind: "error" });
        });
        return;
      }
      const purpose: ImagePurpose = activeMode === "header" ? "headerSource" : activeMode === "ai" ? "ai" : "cover";
      loadImageFile(file, purpose).catch((err) => {
        status = "图片载入失败";
        platform.message(String(err?.message ?? err), { title: "图片处理", kind: "error" });
      });
    }
  }

  function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, r: number) {
    const radius = Math.min(r, width / 2, height / 2);
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

  function drawComposite(canvas: HTMLCanvasElement, img: HTMLImageElement | null, width: number, height: number, guides: boolean) {
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
        offCtx.filter = blur > 0 ? `blur(${blur * 2 * (width / 1400)}px)` : "none";
        offCtx.drawImage(img, 0, 0, bgW, bgH);
        ctx.drawImage(off, bgX, bgY);
      }
    }

    const fgScale = Math.min((width * (foregroundSize / 100)) / img.width, (height * 0.8) / img.height);
    const fgW = img.width * fgScale;
    const fgH = img.height * fgScale;
    const fgX = (width - fgW) / 2 + (width * offsetX) / 100;
    const fgY = (height - fgH) / 2 + (height * offsetY) / 100;
    const scaledRadius = radius * 5 * (width / 1400);

    if (shadow) {
      ctx.save();
      const spread = shadowSpread * (width / 1400);
      roundedRect(ctx, fgX - spread, fgY - spread, fgW + spread * 2, fgH + spread * 2, scaledRadius + spread);
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
    drawComposite(localCanvas, coverImage, localWidth, localHeight, showGuides);
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
    status = "已清空封面处理参考图";
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
    status = "已清空头图样图和处理图片";
  }

  function clearGeneratedImage() {
    if (generatedUrl) URL.revokeObjectURL(generatedUrl);
    generated = null;
    generatedUrl = "";
  }

  function resetAiSettings() {
    aiTarget = "standard";
    aiSize = defaultSize(aiTarget);
    aiPrompt = defaultPrompt(aiTarget);
    generatedTarget = aiTarget;
    status = "AI 生成参数已重置";
  }

  function clearAiReference() {
    if (aiReferenceObjectUrl) URL.revokeObjectURL(aiReferenceObjectUrl);
    aiReferenceImage = null;
    aiReferenceBytes = null;
    aiReferenceName = "";
    aiReferenceMime = "";
    aiReferenceObjectUrl = "";
    clearGeneratedImage();
    status = "已清空 AI 参考图";
  }

  function resetHeaderTransform() {
    if (!headerSourceImage || !headerSampleImage) return;
    headerScale = Math.max(headerSampleImage.width / headerSourceImage.width, headerSampleImage.height / headerSourceImage.height);
    headerOffsetX = 0;
    headerOffsetY = 0;
    drawHeaderPreview();
  }

  function drawHeaderPreview(targetCanvas: HTMLCanvasElement | null = headerCanvas, withMask = true) {
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
    headerDragState = { pointerId: event.pointerId, lastX: event.clientX, lastY: event.clientY };
  }

  function onHeaderPointerMove(event: PointerEvent) {
    if (!headerDragState || headerDragState.pointerId !== event.pointerId || !headerCanvas) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const unitX = headerCanvas.width / Math.max(1, rect.width);
    const unitY = headerCanvas.height / Math.max(1, rect.height);
    headerOffsetX += (event.clientX - headerDragState.lastX) * unitX;
    headerOffsetY += (event.clientY - headerDragState.lastY) * unitY;
    headerDragState = { pointerId: event.pointerId, lastX: event.clientX, lastY: event.clientY };
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
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const canvasX = (event.clientX - rect.left) * (headerCanvas.width / Math.max(1, rect.width));
    const canvasY = (event.clientY - rect.top) * (headerCanvas.height / Math.max(1, rect.height));
    const oldScale = headerScale;
    const nextScale = Math.min(20, Math.max(0.05, headerScale * (event.deltaY < 0 ? 1.08 : 0.925)));
    const oldDrawX = (headerCanvas.width - headerSourceImage.width * oldScale) / 2 + headerOffsetX;
    const oldDrawY = (headerCanvas.height - headerSourceImage.height * oldScale) / 2 + headerOffsetY;
    const imageX = (canvasX - oldDrawX) / oldScale;
    const imageY = (canvasY - oldDrawY) / oldScale;
    headerScale = nextScale;
    headerOffsetX = canvasX - imageX * nextScale - (headerCanvas.width - headerSourceImage.width * nextScale) / 2;
    headerOffsetY = canvasY - imageY * nextScale - (headerCanvas.height - headerSourceImage.height * nextScale) / 2;
    drawHeaderPreview();
  }

  function canvasToBytes(canvas: HTMLCanvasElement, mime: string, quality: number) {
    return new Promise<Uint8Array>((resolve, reject) => {
      canvas.toBlob(
        async (blob) => {
          if (!blob) {
            reject(new Error("生成图片失败"));
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

  async function saveBytes(bytes: Uint8Array, defaultName: string, extension: string) {
    const selected = await platform.saveDialog({
      defaultPath: defaultName,
      filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
    });
    if (!selected) return;
    await platform.writeFile(selected, bytes);
    status = `已保存 ${selected}`;
  }

  async function saveLocalCover() {
    if (!coverImage) return;
    const canvas = document.createElement("canvas");
    drawComposite(canvas, coverImage, localWidth, localHeight, false);
    const { mime, ext } = formatInfo(outputFormat);
    const quality = outputFormat === "png" ? undefined : outputQuality / 100;
    const bytes = await canvasToBytes(canvas, mime, quality ?? 0.92);
    const stem = (title || coverName || "cover").replace(/\.[^.]+$/, "");
    await saveBytes(bytes, `${stem}~duokan-${localWidth}x${localHeight}.${ext}`, ext);
  }

  async function saveHeaderImage() {
    if (!headerSourceImage || !headerSampleImage) return;
    const canvas = document.createElement("canvas");
    drawHeaderPreview(canvas, true);
    const bytes = await canvasToBytes(canvas, "image/png", 1);
    const stem = (title || headerSourceName || "header").replace(/\.[^.]+$/, "");
    await saveBytes(bytes, `${stem}~header-${headerSampleImage.width}x${headerSampleImage.height}.png`, "png");
  }

  function setAiTarget(next: AiTarget) {
    const oldDefault = defaultPrompt(aiTarget);
    aiTarget = next;
    aiSize = defaultSize(next);
    if (!aiPrompt.trim() || aiPrompt === oldDefault) {
      aiPrompt = defaultPrompt(next);
    }
  }

  function setGeneratedUrl(bytes: Uint8Array, mime: string) {
    if (generatedUrl) URL.revokeObjectURL(generatedUrl);
    generatedUrl = URL.createObjectURL(new Blob([bytes], { type: mime }));
  }

  async function generateAiImage() {
    if (!aiReferenceBytes) {
      await platform.message("请先选择参考封面。", { title: "图片处理", kind: "warning" });
      return;
    }
    if (!selectedImageProvider) {
      await platform.message("请先在工具箱设置的 API 配置中新增生图模型。", { title: "图片处理", kind: "warning" });
      return;
    }
    if (!selectedImageProvider.baseUrl.trim() || !selectedImageProvider.apiKey.trim() || !selectedImageProvider.model.trim()) {
      await platform.message("选中的生图模型缺少 API 地址、Key 或模型名。", { title: "图片处理", kind: "warning" });
      return;
    }
    if (!title.trim()) {
      await platform.message("请填写书名。", { title: "图片处理", kind: "warning" });
      return;
    }
    if (!canGenerate) return;
    saveConfig();
    generating = true;
    status = `正在生成${targetLabel(aiTarget)}...`;
    try {
      const result = await platform.invoke<AiImageResult>("toolbox_generate_ai_image", {
        request: {
          baseUrl: selectedImageProvider.baseUrl,
          apiKey: selectedImageProvider.apiKey,
          model: selectedImageProvider.model,
          target: aiTarget,
          title,
          author,
          prompt: aiPrompt,
          size: aiSize,
          referenceData: Array.from(aiReferenceBytes),
        },
      });
      generated = result;
      generatedTarget = aiTarget;
      setGeneratedUrl(new Uint8Array(result.bytes), result.mime);
      status = `生成完成：${result.size || "auto"}`;
    } catch (err) {
      status = "AI 生成失败";
      await platform.message(String(err), { title: "图片处理", kind: "error" });
    } finally {
      generating = false;
    }
  }

  async function saveGenerated() {
    if (!generated) return;
    await saveBytes(new Uint8Array(generated.bytes), generated.fileName || `ai-${aiTarget}.${generated.extension}`, generated.extension);
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
    if (event.key === "Escape") {
      if (customSelectOpen) {
        customSelectOpen = null;
        return;
      }
      closeWindow();
    }
  }

  onMount(() => {
    appSettings = loadAppSettings();
    imageProviderId = appSettings.aiProviders.find((provider) => provider.kind === "image")?.id || "";
    loadConfig();
    drawLocalPreview();
    let unlistenClose: PlatformUnlisten | undefined;
    platform.onCurrentWindowCloseRequested(async (event) => {
      if (closingByCode) return;
      event.preventDefault();
      await closeWindow();
    }).then((fn) => {
      unlistenClose = fn;
    });
    return () => {
      unlistenClose?.();
    };
  });

  onDestroy(() => {
    if (coverObjectUrl) URL.revokeObjectURL(coverObjectUrl);
    if (headerSourceObjectUrl) URL.revokeObjectURL(headerSourceObjectUrl);
    if (headerSampleObjectUrl) URL.revokeObjectURL(headerSampleObjectUrl);
    if (aiReferenceObjectUrl) URL.revokeObjectURL(aiReferenceObjectUrl);
    if (generatedUrl) URL.revokeObjectURL(generatedUrl);
  });
</script>

<svelte:window on:keydown={handleKeydown} on:click={() => (customSelectOpen = null)} />

<main class="image-tool-app">
  <header class="tool-head">
    <div>
      <h1>图片处理</h1>
      <p>{status || "选择封面图后开始处理"}</p>
    </div>
    <div class="tool-head-actions">
      <div class="preview-toggle" aria-label="预览模式">
        <button type="button" class:active={previewMode === "compare"} on:click={() => setPreviewMode("compare")}>对比</button>
        <button type="button" class:active={previewMode === "reference"} on:click={() => setPreviewMode("reference")}>参考</button>
      </div>
      <button class="primary" type="button" on:click={saveCurrent} disabled={saveDisabled}>保存</button>
    </div>
  </header>

  <section class="workspace">
    <div class="preview-pane">
      <div
        class="preview-stage"
        class:loaded={activeReferenceImage || (activeMode === "header" && headerSampleImage) || (activeMode === "ai" && currentGeneratedReady)}
        class:banner-stage={previewIsBanner}
        role="region"
        aria-label="图片预览区"
        on:dragover|preventDefault
        on:drop={onDrop}
      >
        {#if activeMode === "header"}
          {#if previewMode === "reference" && headerSampleImage}
            <button class="reference-thumb" type="button" on:click={() => sampleInput?.click()} title="更换头图样图">
              <img src={headerSampleObjectUrl} alt="头图样图" />
              <span>样图</span>
            </button>
          {/if}

          <div
            class="header-work-preview header-preview"
            on:pointerdown={onHeaderPointerDown}
            on:pointermove={onHeaderPointerMove}
            on:pointerup={onHeaderPointerEnd}
            on:pointercancel={onHeaderPointerEnd}
            on:wheel={onHeaderWheel}
          >
            {#if headerSourceImage && headerSampleImage}
              <canvas bind:this={headerCanvas} class="header-canvas" aria-label="头图制作预览"></canvas>
            {:else}
              <button class="dropzone inline-dropzone" type="button" on:click={chooseActiveReference}>
                <span class="drop-copy">
                  <b>{!headerSampleImage ? "选择头图样图" : "选择处理图片"}</b>
                  <small>JPG / PNG / WebP</small>
                </span>
              </button>
            {/if}
          </div>
        {:else if activeReferenceImage}
          {#if previewMode === "reference"}
            <button class="reference-thumb" type="button" on:click={chooseActiveReference} title="更换参考图">
              <img src={activeReferenceObjectUrl} alt="参考图" />
              <span>参考图</span>
            </button>
          {/if}

          {#if previewIsBanner}
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
          {:else if showComparePreview}
            <div class="split-preview">
              <div class="preview-column">
                <div class="source-frame">
                  <img src={activeReferenceObjectUrl} alt="原图" />
                </div>
              </div>
              <div class="preview-column">
                <div class="result-frame" class:standard-frame={activeMode === "ai" && aiTarget === "standard"}>
                  {#if activeMode === "cover"}
                    <canvas bind:this={localCanvas} class="cover-canvas" aria-label="全屏封面预览"></canvas>
                  {:else if activeMode === "ai" && currentGeneratedReady && generatedUrl}
                    <img src={generatedUrl} alt="AI 生成结果" />
                  {:else}
                    <div class="preview-placeholder">
                      <b>{targetLabel(aiTarget)}</b>
                      <small>生成后显示</small>
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {:else}
            <div class="single-preview">
              <div class="preview-column">
                <div class="result-frame single-result-frame" class:standard-frame={activeMode === "ai" && aiTarget === "standard"}>
                  {#if activeMode === "cover"}
                    <canvas bind:this={localCanvas} class="cover-canvas" aria-label="全屏封面预览"></canvas>
                  {:else if activeMode === "ai" && currentGeneratedReady && generatedUrl}
                    <img src={generatedUrl} alt="AI 生成结果" />
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
          <button class="dropzone" type="button" on:click={chooseActiveReference}>
            <span class="drop-copy">
              <b>选择参考图</b>
              <small>JPG / PNG / WebP</small>
            </span>
          </button>
        {/if}
      </div>
      <input bind:this={fileInput} class="file-input" type="file" accept="image/*" on:change={onFileChange} />
      <input bind:this={sampleInput} class="file-input" type="file" accept="image/*" on:change={onSampleFileChange} />
      <div class="preview-meta">
        <span>{activeMode === "header" ? `${headerSampleName || "未选择样图"} / ${headerSourceName || "未选择处理图"}` : activeReferenceName || "未选择图片"}</span>
        <span>{activeMode === "header" && headerSampleImage ? `${headerSampleImage.width}x${headerSampleImage.height}` : previewIsBanner ? targetRatioLabel(previewTarget) : previewOutputTitle}</span>
      </div>
    </div>

    <div class="control-pane">
      <section class="panel unified-panel">
        <div class="segmented mode-tabs">
          <button type="button" class:active={activeMode === "cover"} on:click={() => setMode("cover")}>封面处理</button>
          <button type="button" class:active={activeMode === "header"} on:click={() => setMode("header")}>头图制作</button>
          <button type="button" class:active={activeMode === "ai"} on:click={() => setMode("ai")}>AI 生成</button>
        </div>

        {#if activeMode === "cover"}
          <div class="mode-section">
            <div class="panel-head">
              <h2>全屏封面</h2>
              <div class="panel-actions">
                <button class="ghost" type="button" on:click={resetCoverSettings}>重置</button>
                <button class="ghost danger-ghost" type="button" on:click={clearCoverImage} disabled={!coverImage}>清空</button>
              </div>
            </div>
            <div class="segmented">
              <button type="button" class:active={bgMode === "blur"} on:click={() => { bgMode = "blur"; drawLocalPreview(); }}>模糊背景</button>
              <button type="button" class:active={bgMode === "solid"} on:click={() => { bgMode = "solid"; drawLocalPreview(); }}>纯色背景</button>
            </div>
            <div class="form-grid two">
              <label>宽度<input type="number" min="320" max="6000" bind:value={localWidth} on:change={drawLocalPreview} /></label>
              <label>高度<input type="number" min="320" max="6000" bind:value={localHeight} on:change={drawLocalPreview} /></label>
              <label>前景大小<input type="range" min="40" max="100" step="1" bind:value={foregroundSize} on:input={drawLocalPreview} /></label>
              <label>圆角<input type="range" min="0" max="100" step="1" bind:value={radius} on:input={drawLocalPreview} /></label>
              <label>横向偏移<input type="range" min="-30" max="30" step="1" bind:value={offsetX} on:input={drawLocalPreview} /></label>
              <label>纵向偏移<input type="range" min="-30" max="30" step="1" bind:value={offsetY} on:input={drawLocalPreview} /></label>
              {#if bgMode === "blur"}
                <label>模糊<input type="range" min="0" max="80" step="1" bind:value={blur} on:input={drawLocalPreview} /></label>
              {:else}
                <label>背景色<input type="color" bind:value={solidColor} on:input={drawLocalPreview} /></label>
              {/if}
              <label>格式
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <!-- svelte-ignore a11y-no-static-element-interactions -->
                <div class="custom-select" class:open={customSelectOpen === "format"} on:click|stopPropagation>
                  <button class="custom-select-trigger" type="button" on:click={() => toggleCustomSelect("format")}>
                    <span>{selectedFormatLabel}</span>
                    <span class="select-caret" aria-hidden="true"></span>
                  </button>
                  {#if customSelectOpen === "format"}
                    <div class="custom-select-menu">
                      {#each FORMAT_OPTIONS as option}
                        <button type="button" class:active={outputFormat === option.value} on:click={() => selectOutputFormat(option.value)}>{option.label}</button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </label>
            </div>
            <div class="toggles">
              <label class:checked={showGuides}><input type="checkbox" bind:checked={showGuides} on:change={drawLocalPreview} /><span>参考线</span></label>
              <label class:checked={shadow}><input type="checkbox" bind:checked={shadow} on:change={drawLocalPreview} /><span>阴影</span></label>
              {#if showGuides}
                <label>参考线色<input type="color" bind:value={guideColor} on:input={drawLocalPreview} /></label>
              {/if}
              {#if shadow}
                <label>阴影色<input type="color" bind:value={shadowColor} on:input={drawLocalPreview} /></label>
                <label>阴影模糊<input type="range" min="0" max="60" bind:value={shadowBlur} on:input={drawLocalPreview} /></label>
                <label>阴影扩散<input type="range" min="0" max="30" bind:value={shadowSpread} on:input={drawLocalPreview} /></label>
              {/if}
            </div>
          </div>
        {:else if activeMode === "header"}
          <div class="mode-section">
            <div class="panel-head">
              <h2>头图制作</h2>
              <div class="panel-actions">
                <button class="ghost" type="button" on:click={resetHeaderTransform} disabled={!headerSourceImage || !headerSampleImage}>重置</button>
                <button class="ghost danger-ghost" type="button" on:click={clearHeaderImages} disabled={!headerSourceImage && !headerSampleImage}>清空</button>
              </div>
            </div>
            <div class="header-file-actions">
              <button class="ghost" type="button" on:click={() => sampleInput?.click()}>选择头图样图</button>
              <button class="ghost" type="button" on:click={() => chooseImage("headerSource")}>选择处理图片</button>
            </div>
            <div class="header-file-meta">
              <span title={headerSampleName}>{headerSampleName || "未选择样图"}</span>
              <span title={headerSourceName}>{headerSourceName || "未选择处理图"}</span>
            </div>
            <div class="form-grid two">
              <label class="wide">缩放<input type="range" min="0.05" max="20" step="0.01" bind:value={headerScale} on:input={() => drawHeaderPreview()} /></label>
              <label>横向偏移<input type="number" step="1" bind:value={headerOffsetX} on:input={() => drawHeaderPreview()} /></label>
              <label>纵向偏移<input type="number" step="1" bind:value={headerOffsetY} on:input={() => drawHeaderPreview()} /></label>
            </div>
            <p class="header-hint">拖动画布调整位置，滚轮缩放；保存时按样图尺寸输出 PNG，并保留样图透明边缘。</p>
          </div>
        {:else}
          <div class="mode-section">
            <div class="panel-head">
              <h2>AI 生成</h2>
              <div class="panel-actions">
                <button class="ghost" type="button" on:click={resetAiSettings}>重置</button>
                <button class="ghost danger-ghost" type="button" on:click={clearAiReference} disabled={!aiReferenceImage && !generated}>清空</button>
                <button class="primary" type="button" on:click={generateAiImage} disabled={!canGenerate || generating}>{generating ? "生成中" : "生成"}</button>
              </div>
            </div>
            <div class="segmented ai-targets">
              {#each AI_TARGETS as target}
                <button type="button" class:active={aiTarget === target.id} on:click={() => setAiTarget(target.id)}>
                  <span>{target.label}</span>
                  <small>{target.ratioLabel}</small>
                </button>
              {/each}
            </div>
            <div class="form-grid ai-config">
              <label>书名<input bind:value={title} placeholder="必填" required /></label>
              <label>作者<input bind:value={author} placeholder="可选" /></label>
              <label class="wide">尺寸<input bind:value={aiSize} placeholder={defaultSize(aiTarget)} /></label>
              <label class="wide">生图模型
                <!-- svelte-ignore a11y-click-events-have-key-events -->
                <!-- svelte-ignore a11y-no-static-element-interactions -->
                <div class="custom-select" class:open={customSelectOpen === "provider"} class:disabled={imageProviders.length === 0} on:click|stopPropagation>
                  <button class="custom-select-trigger" type="button" disabled={imageProviders.length === 0} on:click={() => toggleCustomSelect("provider")}>
                    <span>{selectedProviderLabel}</span>
                    <span class="select-caret" aria-hidden="true"></span>
                  </button>
                  {#if customSelectOpen === "provider"}
                    <div class="custom-select-menu">
                      {#each imageProviders as provider}
                        <button type="button" class:active={imageProviderId === provider.id} on:click={() => selectImageProvider(provider.id)}>
                          {provider.name || provider.model} · {provider.model}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </label>
              {#if selectedImageProvider}
                <div class="provider-summary wide">
                  <span>{selectedImageProvider.baseUrl}</span>
                  <span>{selectedImageProvider.apiKey ? "已保存 Key" : "未填写 Key"}</span>
                </div>
              {:else}
                <div class="provider-summary wide">未配置生图模型，请到工具箱设置 > API 配置新增。</div>
              {/if}
              <label class="wide">提示词<textarea rows="4" bind:value={aiPrompt}></textarea></label>
            </div>
            <div class="actions">
              <button class="ghost" type="button" on:click={refreshImageProviders}>刷新模型</button>
              <button class="ghost" type="button" on:click={() => { aiPrompt = defaultPrompt(aiTarget); }}>恢复提示词</button>
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
    min-height: 34px;
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
    color: white;
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
      linear-gradient(45deg, color-mix(in srgb, var(--color-border) 35%, transparent) 25%, transparent 25%),
      linear-gradient(-45deg, color-mix(in srgb, var(--color-border) 35%, transparent) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, color-mix(in srgb, var(--color-border) 35%, transparent) 75%),
      linear-gradient(-45deg, transparent 75%, color-mix(in srgb, var(--color-border) 35%, transparent) 75%),
      var(--color-surface);
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
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
    min-height: 34px;
    padding: 7px 12px;
  }

  .segmented button.active,
  .primary {
    border-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
    background: var(--color-accent);
    color: white;
  }

  .segmented button.active:hover:not(:disabled),
  .preview-toggle button.active:hover:not(:disabled),
  .primary:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--color-accent-deep) 48%, transparent);
    background: var(--color-accent-deep);
    color: white;
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
    min-height: 34px;
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
    min-height: 34px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-surface) 94%, var(--color-accent-quiet));
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
    min-height: 34px;
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
    color: var(--color-accent-contrast, #fff);
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
      grid-template-columns: 1fr;
      padding-right: 0;
      overflow: auto;
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
</style>
