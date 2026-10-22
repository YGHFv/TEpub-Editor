<script lang="ts">
    import { onMount } from "svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import ToolImportPage from "$lib/ToolImportPage.svelte";
    import { isInHiddenView } from "$lib/keepAlive/viewState";
    import {
        EPUB_ILLUSTRATION_STYLES,
        type EpubStyleModule,
    } from "$lib/epubStyleLibrary";
    import { loadWebEpub, type WebEpubDocument } from "$lib/webEpub";
    import {
        embedWebEpubRemoteImages,
        scanWebEpubRemoteImages,
        type DownloadedRemoteImage,
        type RemoteImageEmbedResult,
        type RemoteImageReference,
    } from "$lib/webEpubRemoteImages";
    import {
        downloadBrowserBlob,
        validateBrowserFiles,
    } from "$lib/webFileWorkflow";
    import { platform } from "$lib/platform";
    import { EPUB_STYLE_LIBRARY_KEY as STYLE_STORAGE_KEY } from "$lib/storageKeys";
    import { t } from "$lib/i18n.svelte";

    type DesktopDownloadResponse = {
        dataBase64: string;
        contentType: string;
        finalUrl: string;
    };

    const DEFAULT_STYLE_ID = "illustration-centered-caption";
    const MAX_BROWSER_IMAGE_BYTES = 32 * 1024 * 1024;
    const MAX_CANVAS_PIXELS = 36_000_000;

    let fileInput: HTMLInputElement | null = null;
    let sourceFile: File | null = null;
    let scannedDocument: WebEpubDocument | null = null;
    let workingDocument: WebEpubDocument | null = null;
    let references: RemoteImageReference[] = [];
    let selectedUrls = new Set<string>();
    let styles: EpubStyleModule[] = EPUB_ILLUSTRATION_STYLES.filter(
        (style) => style.target === "illustration",
    );
    let selectedStyleId = DEFAULT_STYLE_ID;
    let busy = false;
    let progressText = "";
    let errorText = "";
    let result: RemoteImageEmbedResult | null = null;
    let showLogModal = false;

    $: selectedStyle =
        styles.find((style) => style.id === selectedStyleId) || styles[0];
    $: styleOptions = styles.map((style) => ({
        value: style.id,
        label: style.name,
        meta: t(style.sourceKind === "saved" ? "我的样式" : "内置"),
    }));
    $: selectedCount = selectedUrls.size;
    $: totalOccurrences = references.reduce(
        (sum, item) => sum + item.occurrences,
        0,
    );
    $: totalChapters = new Set(references.flatMap((item) => item.filePaths))
        .size;
    $: allSelected =
        references.length > 0 && selectedCount === references.length;
    $: previewDocument = selectedStyle
        ? buildPreviewDocument(selectedStyle)
        : "";
    $: hasLogs = Boolean(errorText || result?.failedImages.length);

    onMount(() => {
        styles = loadIllustrationStyles();
        if (!styles.some((style) => style.id === selectedStyleId))
            selectedStyleId = styles[0]?.id || "";
    });

    function loadIllustrationStyles() {
        let saved: EpubStyleModule[] = [];
        try {
            const parsed = JSON.parse(
                localStorage.getItem(STYLE_STORAGE_KEY) || "null",
            );
            if (Array.isArray(parsed?.styles)) {
                saved = parsed.styles.filter(
                    (style: Partial<EpubStyleModule>) =>
                        style?.kind === "illustration" &&
                        style.target === "illustration" &&
                        typeof style.id === "string" &&
                        typeof style.name === "string" &&
                        typeof style.css === "string",
                );
            }
        } catch (error) {
            console.warn("读取自定义插图样式失败:", error);
        }
        const builtIn = EPUB_ILLUSTRATION_STYLES.filter(
            (style) => style.target === "illustration",
        ).map((style) => ({ ...style, sourceKind: "built-in" as const }));
        const ids = new Set(builtIn.map((style) => style.id));
        return [
            ...builtIn,
            ...saved
                .filter((style) => !ids.has(style.id))
                .map((style) => ({ ...style, sourceKind: "saved" as const })),
        ];
    }

    function buildPreviewDocument(style: EpubStyleModule) {
        return `<!doctype html><html><head><meta charset="utf-8" /><style>
      html,body{box-sizing:border-box;width:100%;height:100%;margin:0;overflow:hidden;background:#fbf7ec;color:#172434;font-family:"Microsoft YaHei",sans-serif}
      body{padding:9px}.te-preview-page{box-sizing:border-box;width:142.857%;max-width:none;margin:0;padding:18px 26px;overflow:hidden;background:#ffffff;box-shadow:none;transform:scale(.7);transform-origin:top left}
      .te-paragraph{margin:.65em 0;line-height:1.65;text-indent:2em;text-align:justify;font-size:13px}
      ${style.css}
    </style></head><body>${style.previewHtml || style.markup || ""}</body></html>`;
    }

    function openPicker() {
        if (!busy) fileInput?.click();
    }

    function handleFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const files = Array.from(input.files || []);
        // 取消对话框同样会触发 change，files 为空。把它当成"选错文件"会让
        // 页面上莫名其妙多出一条「请选择扩展名为 .epub 的文件」红字。
        if (!files.length) return;
        void importFiles(files);
    }

    function handleImportFiles(event: CustomEvent<File[]>) {
        void importFiles(event.detail);
    }

    async function importFiles(files: File[]) {
        const validation = validateBrowserFiles(files, {
            extensions: ["epub"],
            mimeTypes: ["application/epub+zip"],
            multiple: false,
        });
        const file = validation.accepted[0];
        if (!file) {
            errorText = t("请选择扩展名为 .epub 的文件。");
            return;
        }
        busy = true;
        errorText = "";
        progressText = t("正在扫描章节中的在线图片…");
        result = null;
        try {
            const doc = await loadWebEpub(file);
            const found = await scanWebEpubRemoteImages(doc);
            sourceFile = file;
            scannedDocument = doc;
            workingDocument = null;
            references = found;
            selectedUrls = new Set(found.map((item) => item.url));
            progressText = found.length
                ? t("发现 {urls} 个在线图片地址，共 {references} 处引用。", {
                      urls: found.length,
                      references: found.reduce(
                          (sum, item) => sum + item.occurrences,
                          0,
                      ),
                  })
                : t("没有发现 HTTP/HTTPS 在线图片。");
        } catch (error) {
            sourceFile = null;
            scannedDocument = null;
            workingDocument = null;
            references = [];
            selectedUrls = new Set();
            errorText = t("读取 EPUB 失败：{error}", {
                error: error instanceof Error ? error.message : String(error),
            });
        } finally {
            busy = false;
            if (fileInput) fileInput.value = "";
        }
    }

    function toggleUrl(url: string) {
        const next = new Set(selectedUrls);
        if (next.has(url)) next.delete(url);
        else next.add(url);
        selectedUrls = next;
        result = null;
        errorText = "";
        showLogModal = false;
    }

    function toggleAll() {
        selectedUrls = allSelected
            ? new Set()
            : new Set(references.map((item) => item.url));
        result = null;
        errorText = "";
        showLogModal = false;
    }

    function decodeBase64(value: string) {
        const binary = atob(value);
        const bytes = new Uint8Array(binary.length);
        for (let index = 0; index < binary.length; index += 1)
            bytes[index] = binary.charCodeAt(index);
        return bytes;
    }

    function errorMessage(error: unknown) {
        return error instanceof Error ? error.message : String(error);
    }

    function hasSupportedImageSignature(bytes: Uint8Array) {
        if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff)
            return true;
        if (
            bytes[0] === 0x89 &&
            bytes[1] === 0x50 &&
            bytes[2] === 0x4e &&
            bytes[3] === 0x47
        )
            return true;
        if (String.fromCharCode(...bytes.slice(0, 4)) === "GIF8") return true;
        return (
            String.fromCharCode(...bytes.slice(0, 4)) === "RIFF" &&
            String.fromCharCode(...bytes.slice(8, 12)) === "WEBP"
        );
    }

    async function downloadWithBrowserFetch(
        url: string,
    ): Promise<DownloadedRemoteImage> {
        const controller = new AbortController();
        const timeout = window.setTimeout(() => controller.abort(), 30_000);
        try {
            const response = await fetch(url, {
                redirect: "follow",
                credentials: "omit",
                mode: "cors",
                signal: controller.signal,
            });
            if (!response.ok)
                throw new Error(
                    t("图片服务器返回 {status} {statusText}", {
                        status: response.status,
                        statusText: response.statusText,
                    }),
                );
            const declaredLength = Number(
                response.headers.get("content-length") || 0,
            );
            if (declaredLength > MAX_BROWSER_IMAGE_BYTES)
                throw new Error(t("图片超过 32 MB 限制"));
            const bytes = new Uint8Array(await response.arrayBuffer());
            if (bytes.byteLength > MAX_BROWSER_IMAGE_BYTES)
                throw new Error(t("图片超过 32 MB 限制"));
            if (!hasSupportedImageSignature(bytes))
                throw new Error(t("响应不是支持的 JPG、PNG、WebP 或 GIF 图片"));
            return {
                bytes,
                mediaType: response.headers.get("content-type") || "",
                finalUrl: response.url || url,
            };
        } finally {
            window.clearTimeout(timeout);
        }
    }

    function canvasOutputMediaType(url: string) {
        const extension =
            new URL(url).pathname.split(".").pop()?.toLowerCase() || "";
        if (extension === "gif")
            throw new Error(t("GIF 不能通过 Canvas 转换，否则会丢失动画"));
        if (extension === "jpg" || extension === "jpeg") return "image/jpeg";
        if (extension === "webp") return "image/webp";
        return "image/png";
    }

    function loadCorsImage(url: string) {
        return new Promise<HTMLImageElement>((resolve, reject) => {
            const image = new Image();
            const timeout = window.setTimeout(() => {
                image.src = "";
                reject(new Error(t("图片元素加载超时")));
            }, 30_000);
            image.crossOrigin = "anonymous";
            image.referrerPolicy = "strict-origin-when-cross-origin";
            image.decoding = "async";
            image.onload = () => {
                window.clearTimeout(timeout);
                resolve(image);
            };
            image.onerror = () => {
                window.clearTimeout(timeout);
                reject(new Error(t("图片服务器未允许匿名跨域图片读取")));
            };
            image.src = url;
        });
    }

    async function downloadWithCanvas(
        url: string,
    ): Promise<DownloadedRemoteImage> {
        const outputMediaType = canvasOutputMediaType(url);
        const image = await loadCorsImage(url);
        const width = image.naturalWidth;
        const height = image.naturalHeight;
        if (!width || !height) throw new Error(t("图片没有有效尺寸"));
        if (width * height > MAX_CANVAS_PIXELS)
            throw new Error(t("图片像素尺寸过大，无法在浏览器中安全转换"));
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d");
        if (!context) throw new Error(t("浏览器无法创建图片转换画布"));
        context.drawImage(image, 0, 0, width, height);
        const blob = await new Promise<Blob>((resolve, reject) => {
            try {
                canvas.toBlob(
                    (value) =>
                        value
                            ? resolve(value)
                            : reject(
                                  new Error(t("浏览器无法导出转换后的图片")),
                              ),
                    outputMediaType,
                    outputMediaType === "image/jpeg" ||
                        outputMediaType === "image/webp"
                        ? 0.94
                        : undefined,
                );
            } catch (error) {
                reject(
                    new Error(
                        t("跨域图片画布不可读取：{error}", {
                            error: errorMessage(error),
                        }),
                    ),
                );
            }
        });
        if (blob.size > MAX_BROWSER_IMAGE_BYTES)
            throw new Error(t("转换后的图片超过 32 MB 限制"));
        return {
            bytes: new Uint8Array(await blob.arrayBuffer()),
            mediaType: blob.type || outputMediaType,
            finalUrl: url,
        };
    }

    async function downloadWithWebProxy(
        url: string,
    ): Promise<DownloadedRemoteImage> {
        const response = await platform.invoke<DesktopDownloadResponse>(
            "download_remote_image",
            { url },
        );
        return {
            bytes: decodeBase64(response.dataBase64),
            mediaType: response.contentType,
            finalUrl: response.finalUrl,
        };
    }

    async function downloadImage(url: string): Promise<DownloadedRemoteImage> {
        if (platform.isTauri) return downloadWithWebProxy(url);
        const failures: string[] = [];
        try {
            return await downloadWithBrowserFetch(url);
        } catch (error) {
            failures.push(
                t("前端直读：{error}", { error: errorMessage(error) }),
            );
        }
        progressText = t("浏览器直读受限，正在尝试前端图片解码转换…");
        try {
            return await downloadWithCanvas(url);
        } catch (error) {
            failures.push(
                t("前端图片转换：{error}", { error: errorMessage(error) }),
            );
        }
        progressText = t("纯前端通道均受跨域限制，正在使用服务端保底下载…");
        try {
            return await downloadWithWebProxy(url);
        } catch (error) {
            failures.push(
                t("服务端保底：{error}", { error: errorMessage(error) }),
            );
        }
        throw new Error(failures.join("；"));
    }

    async function processImages() {
        if (
            !sourceFile ||
            !scannedDocument ||
            !selectedStyle ||
            !selectedUrls.size ||
            busy
        )
            return;
        busy = true;
        result = null;
        errorText = "";
        progressText = t("准备下载在线图片…");
        try {
            // 每次都从原文件重新载入，避免重试时重复写入资源与 manifest。
            const cleanDocument = await loadWebEpub(sourceFile);
            workingDocument = cleanDocument;
            result = await embedWebEpubRemoteImages(
                cleanDocument,
                [...selectedUrls],
                selectedStyle,
                downloadImage,
                (progress) => {
                    progressText = t(progress.message);
                },
            );
            progressText = result.failedImages.length
                ? t(
                      "处理完成：下载 {downloaded} 张，{failed} 张自动重试后仍失败。",
                      {
                          downloaded: result.downloadedImages,
                          failed: result.failedImages.length,
                      },
                  )
                : t(
                      "处理完成：下载 {downloaded} 张，替换 {replaced} 处引用。",
                      {
                          downloaded: result.downloadedImages,
                          replaced: result.replacedOccurrences,
                      },
                  );
        } catch (error) {
            errorText = t("处理失败：{error}", {
                error: error instanceof Error ? error.message : String(error),
            });
            progressText = t("处理失败，请点击“查看日志”了解详情。");
        } finally {
            busy = false;
        }
    }

    async function retryFailedImages() {
        if (
            !result?.failedImages.length ||
            !workingDocument ||
            !selectedStyle ||
            busy
        )
            return;
        const previous = result;
        const failedUrls = previous.failedImages.map((item) => item.url);
        busy = true;
        errorText = "";
        progressText = t("正在重试 {count} 张失败图片…", {
            count: failedUrls.length,
        });
        try {
            const retried = await embedWebEpubRemoteImages(
                workingDocument,
                failedUrls,
                selectedStyle,
                downloadImage,
                (progress) => {
                    progressText = t(progress.message);
                },
            );
            result = {
                ...retried,
                downloadedImages:
                    previous.downloadedImages + retried.downloadedImages,
                replacedOccurrences:
                    previous.replacedOccurrences + retried.replacedOccurrences,
                changedFiles: previous.changedFiles + retried.changedFiles,
            };
            progressText = result.failedImages.length
                ? t("手动重试完成：仍有 {count} 张失败，可再次重试。", {
                      count: result.failedImages.length,
                  })
                : t(
                      "重试成功：累计下载 {downloaded} 张，替换 {replaced} 处引用。",
                      {
                          downloaded: result.downloadedImages,
                          replaced: result.replacedOccurrences,
                      },
                  );
        } catch (error) {
            errorText = t("重试失败：{error}", {
                error: error instanceof Error ? error.message : String(error),
            });
            progressText = t("手动重试未完成，请点击“查看日志”了解详情。");
        } finally {
            busy = false;
        }
    }

    function downloadResult() {
        if (result) downloadBrowserBlob(result.blob, result.outputName);
    }

    function retryFromLog() {
        showLogModal = false;
        void retryFailedImages();
    }

    function reset() {
        sourceFile = null;
        scannedDocument = null;
        workingDocument = null;
        references = [];
        selectedUrls = new Set();
        result = null;
        progressText = "";
        errorText = "";
        showLogModal = false;
    }

    function shortPath(path: string) {
        return path.split("/").pop() || path;
    }
</script>

<svelte:window
    on:keydown={(event) => {
        if (isInHiddenView(event.target)) return;
        if (event.key === "Escape") showLogModal = false;
    }}
/>

<svelte:head>
    <title>EPUB 在线图片下载 - TEpub Editor</title>
    <meta
        name="description"
        content="下载 EPUB 章节中的在线图片，写入资源清单并替换为本地引用。"
    />
</svelte:head>

<input
    bind:this={fileInput}
    class="file-input"
    type="file"
    accept=".epub,application/epub+zip"
    on:change={handleFileChange}
/>

{#if !sourceFile}
    <ToolImportPage
        mark="IMG↓"
        kicker="REMOTE IMAGE EMBEDDER"
        title="EPUB 在线图片下载"
        description="扫描正文中的在线插图，下载到 EPUB 内部并自动更新 XHTML 与 OPF 资源清单。"
        privacy={platform.isTauri
            ? "图片由桌面端直接下载，EPUB 不会上传服务器。"
            : "EPUB 在浏览器本地处理；部分图片站点可能因跨域限制无法下载。"}
        outputLabel="默认插图样式"
        outputValue="居中图注插图"
        features={[
            {
                title: "识别两种标签",
                detail: "支持真实 img 标签和转义为文字的 &lt;img&gt;",
            },
            {
                title: "资源自动入包",
                detail: "图片写入 Images/online，并登记到 OPF manifest",
            },
            {
                title: "样式可选择",
                detail: "默认居中图注插图，也可使用样式库中的普通插图样式",
            },
        ]}
        prompt="选择或拖入 EPUB 文件"
        hint="扫描 HTTP/HTTPS 图片引用，不修改原文件"
        actionLabel="选择 EPUB 文件"
        accept=".epub,application/epub+zip"
        {busy}
        {errorText}
        on:select={openPicker}
        on:files={handleImportFiles}
    />
{:else}
    <main class="download-workspace">
        <header class="workspace-head">
            <div class="header-copy">
                <span class="eyebrow">REMOTE IMAGE EMBEDDER</span>
                <h1>EPUB 在线图片下载</h1>
                <p title={sourceFile.name}>{sourceFile.name}</p>
                <small class="header-status" aria-live="polite">
                    {#if busy}<span class="spinner"></span>{/if}
                    <span>{progressText}</span>
                </small>
            </div>
            <div class="header-actions">
                <button
                    class="secondary"
                    type="button"
                    disabled={busy}
                    on:click={reset}>重新选择</button
                >
                {#if hasLogs}
                    <button
                        class="secondary log-button"
                        type="button"
                        disabled={busy}
                        on:click={() => {
                            showLogModal = true;
                        }}
                    >
                        查看日志{result?.failedImages.length
                            ? `（${result.failedImages.length}）`
                            : ""}
                    </button>
                {/if}
                {#if result && result.downloadedImages > 0}
                    <button
                        class="primary header-download"
                        type="button"
                        disabled={busy}
                        on:click={downloadResult}>下载处理后的 EPUB</button
                    >
                {:else if !result}
                    <button
                        class="primary header-download"
                        type="button"
                        disabled={busy || selectedCount === 0}
                        on:click={processImages}
                    >
                        {busy
                            ? "正在下载并写入…"
                            : `下载并嵌入 ${selectedCount} 张图片`}
                    </button>
                {/if}
            </div>
        </header>

        <section class="metrics" aria-label="扫描结果">
            <article>
                <span>在线地址</span><strong>{references.length}</strong>
            </article>
            <article>
                <span>引用次数</span><strong>{totalOccurrences}</strong>
            </article>
            <article>
                <span>涉及章节</span><strong>{totalChapters}</strong>
            </article>
            <article>
                <span>已选择</span><strong>{selectedCount}</strong>
            </article>
        </section>

        {#if references.length}
            <div class="content-grid">
                <section class="reference-panel">
                    <div class="panel-head">
                        <div>
                            <h2>在线图片</h2>
                            <p>同一地址只下载一次，但会替换全部出现位置。</p>
                        </div>
                        <button
                            class="text-button"
                            type="button"
                            disabled={busy}
                            on:click={toggleAll}
                            >{allSelected ? "取消全选" : "全部选择"}</button
                        >
                    </div>
                    <div class="reference-list">
                        {#each references as item, index}
                            <label
                                class:selected={selectedUrls.has(item.url)}
                                class="reference-row"
                            >
                                <input
                                    type="checkbox"
                                    checked={selectedUrls.has(item.url)}
                                    disabled={busy}
                                    on:change={() => toggleUrl(item.url)}
                                />
                                <span class="index"
                                    >{String(index + 1).padStart(2, "0")}</span
                                >
                                <span class="reference-copy">
                                    <strong title={item.url}>{item.url}</strong>
                                    <small>
                                        {item.occurrences} 处引用 · {item.filePaths
                                            .map(shortPath)
                                            .join("、")}
                                        {#if item.encodedOccurrences}<b
                                                >{item.encodedOccurrences} 处为转义标签</b
                                            >{/if}
                                    </small>
                                </span>
                            </label>
                        {/each}
                    </div>
                </section>

                <aside class="style-panel">
                    <div class="panel-head">
                        <div>
                            <h2>插图样式</h2>
                            <p>应用于独占一段的在线图片。</p>
                        </div>
                    </div>
                    <CustomSelect
                        className="style-select"
                        value={selectedStyleId}
                        options={styleOptions}
                        disabled={busy}
                        ariaLabel="选择插图样式"
                        on:change={(event) => {
                            selectedStyleId = event.detail;
                            result = null;
                            errorText = "";
                            showLogModal = false;
                        }}
                    />
                    {#if selectedStyle}
                        <div class="style-copy">
                            <strong>{t(selectedStyle.name)}</strong>
                            <p>{t(selectedStyle.description)}</p>
                        </div>
                        <iframe
                            title={t("{name} 预览", {
                                name: t(selectedStyle.name),
                            })}
                            srcdoc={previewDocument}
                        ></iframe>
                    {/if}
                </aside>
            </div>
        {:else}
            <section class="empty-state">
                <span>IMG</span>
                <h2>没有发现在线图片</h2>
                <p>
                    已扫描章节 XHTML，未发现以 http:// 或 https://
                    开头的图片地址。
                </p>
                <button class="primary" type="button" on:click={reset}
                    >选择其他 EPUB</button
                >
            </section>
        {/if}
    </main>
{/if}

{#if showLogModal}
    <div class="log-overlay">
        <button
            class="log-backdrop"
            type="button"
            aria-label="关闭下载日志"
            on:click={() => {
                showLogModal = false;
            }}
        ></button>
        <dialog class="log-dialog" open aria-labelledby="download-log-title">
            <header>
                <div>
                    <span class="eyebrow">DOWNLOAD LOG</span>
                    <h2 id="download-log-title">在线图片下载日志</h2>
                    <p>
                        {result?.failedImages.length
                            ? `${result.failedImages.length} 张图片下载失败，原在线地址已保留。`
                            : "处理过程未完成。"}
                    </p>
                </div>
                <button
                    class="log-close"
                    type="button"
                    aria-label="关闭"
                    on:click={() => {
                        showLogModal = false;
                    }}>×</button
                >
            </header>
            <div class="log-content">
                {#if errorText}
                    <article class="log-entry general-error">
                        <strong>处理错误</strong>
                        <p>{errorText}</p>
                    </article>
                {/if}
                {#each result?.failedImages || [] as failure, index}
                    <article class="log-entry">
                        <span>{String(index + 1).padStart(2, "0")}</span>
                        <div>
                            <strong>{failure.url}</strong>
                            <p>{failure.message}</p>
                        </div>
                    </article>
                {/each}
            </div>
            <footer>
                <button
                    class="secondary"
                    type="button"
                    on:click={() => {
                        showLogModal = false;
                    }}>关闭</button
                >
                {#if result?.failedImages.length}
                    <button
                        class="retry-button"
                        type="button"
                        on:click={retryFromLog}
                        >重试失败项（{result.failedImages.length}）</button
                    >
                {/if}
            </footer>
        </dialog>
    </div>
{/if}

<style>
    .file-input {
        display: none;
    }
    .download-workspace {
        box-sizing: border-box;
        width: min(1180px, calc(100% - 36px));
        min-height: 100%;
        margin: 0 auto;
        padding: 24px 0 42px;
        color: var(--web-tool-text);
    }
    .workspace-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 18px;
        padding: 18px 20px;
        border: 1px solid var(--web-tool-border);
        border-radius: var(--web-tool-radius-md);
        background: var(--web-tool-surface);
    }
    .header-copy {
        min-width: 0;
        flex: 1 1 auto;
        overflow: hidden;
    }
    .eyebrow {
        color: var(--color-muted);
        font-size: 9px;
        font-weight: 900;
        letter-spacing: 0.15em;
    }
    h1,
    h2,
    p {
        margin: 0;
    }
    h1 {
        margin: 4px 0 5px;
        font-size: 22px;
    }
    .workspace-head p {
        max-width: 720px;
        overflow: hidden;
        color: var(--web-tool-muted);
        font-size: 12px;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .header-status {
        min-width: 0;
        min-height: 18px;
        display: flex;
        align-items: center;
        gap: 7px;
        margin-top: 5px;
        color: var(--color-muted);
        font-size: 10px;
        line-height: 1.4;
    }
    .header-status > span:last-child {
        min-width: 0;
        display: block;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .header-status .spinner {
        width: 11px;
        height: 11px;
        flex: 0 0 auto;
    }
    .header-actions {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        flex-wrap: nowrap;
        gap: 8px;
        flex: 0 0 auto;
    }
    .header-download {
        min-width: 154px;
    }
    .log-button {
        color: var(--color-warning);
        border-color: color-mix(in srgb, var(--color-warning) 42%, white);
        background: var(--color-warning-soft);
    }
    button {
        border: 0;
        cursor: pointer;
    }
    button:disabled {
        cursor: not-allowed;
        opacity: 0.55;
    }
    .primary,
    .secondary {
        min-height: 36px;
        padding: 0 16px;
        border-radius: var(--radius-sm);
        font-size: 12px;
        font-weight: 800;
    }
    .primary {
        background: var(--web-tool-accent);
        color: var(--color-accent-contrast);
    }
    .primary:hover:not(:disabled) {
        background: var(--color-accent-deep);
    }
    .secondary {
        border: 1px solid var(--web-tool-border);
        background: var(--color-surface-soft);
        color: var(--color-text-soft);
    }
    .metrics {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 10px;
        margin-top: 12px;
    }
    .metrics article {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        padding: 13px 16px;
        border: 1px solid var(--web-tool-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface-raised);
    }
    .metrics span {
        color: var(--web-tool-muted);
        font-size: 11px;
    }
    .metrics strong {
        color: var(--web-tool-accent);
        font-size: 20px;
    }
    .content-grid {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 350px;
        gap: 12px;
        margin-top: 12px;
    }
    .reference-panel,
    .style-panel {
        min-width: 0;
        border: 1px solid var(--web-tool-border);
        border-radius: var(--web-tool-radius-md);
        background: var(--web-tool-surface);
    }
    .reference-panel {
        overflow: hidden;
    }
    .style-panel {
        display: flex;
        flex-direction: column;
        overflow: visible;
    }
    .panel-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 15px 17px;
        border-bottom: 1px solid var(--color-border);
    }
    .panel-head h2 {
        font-size: 15px;
    }
    .panel-head p {
        margin-top: 4px;
        color: var(--web-tool-muted);
        font-size: 10px;
    }
    .text-button {
        padding: 5px 8px;
        background: transparent;
        color: var(--web-tool-accent);
        font-size: 11px;
        font-weight: 800;
        white-space: nowrap;
    }
    .reference-list {
        max-height: 440px;
        overflow: auto;
    }
    .reference-row {
        display: grid;
        grid-template-columns: 18px 30px minmax(0, 1fr);
        align-items: center;
        gap: 9px;
        padding: 12px 16px;
        border-bottom: 1px solid var(--color-canvas);
        cursor: pointer;
    }
    .reference-row:last-child {
        border-bottom: 0;
    }
    .reference-row:hover,
    .reference-row.selected {
        background: var(--color-accent-quiet);
    }
    .reference-row input {
        width: 15px;
        height: 15px;
        accent-color: var(--web-tool-accent);
    }
    .index {
        color: var(--color-muted);
        font: 10px var(--font-code);
    }
    .reference-copy {
        min-width: 0;
    }
    .reference-copy strong {
        display: block;
        overflow: hidden;
        color: var(--color-text);
        font: 11px var(--font-code);
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .reference-copy small {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
        margin-top: 5px;
        color: var(--color-muted);
        font-size: 10px;
    }
    .reference-copy b {
        padding: 1px 5px;
        border-radius: var(--radius-xs);
        background: var(--color-warning-soft);
        color: var(--color-warning);
        font-weight: 700;
    }
    .style-panel {
        padding-bottom: 14px;
    }
    :global(.style-select) {
        box-sizing: border-box;
        width: calc(100% - 30px);
        max-width: calc(100% - 30px);
        margin: 14px 15px 0;
    }
    .style-copy {
        padding: 13px 16px 9px;
    }
    .style-copy strong {
        font-size: 13px;
    }
    .style-copy p {
        margin-top: 4px;
        color: var(--web-tool-muted);
        font-size: 10px;
        line-height: 1.5;
    }
    iframe {
        box-sizing: border-box;
        width: calc(100% - 30px);
        min-height: 300px;
        height: auto;
        flex: 1 1 300px;
        margin: 0 15px 14px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas-glow);
        overflow: hidden;
    }
    .retry-button {
        min-height: 36px;
        padding: 0 16px;
        border-radius: var(--radius-sm);
        background: var(--color-warning);
        color: var(--color-surface);
        font-size: 12px;
        font-weight: 800;
    }
    .retry-button:hover:not(:disabled) {
        background: #8e550d;
    }
    .spinner {
        width: 15px;
        height: 15px;
        border: 2px solid var(--color-border);
        border-top-color: var(--web-tool-accent);
        border-radius: 50%;
        animation: spin 0.75s linear infinite;
    }
    .empty-state {
        display: grid;
        justify-items: center;
        gap: 9px;
        margin-top: 12px;
        padding: 80px 24px;
        border: 1px solid var(--web-tool-border);
        border-radius: var(--web-tool-radius-md);
        background: var(--web-tool-surface);
        text-align: center;
    }
    .empty-state > span {
        width: 58px;
        height: 58px;
        display: grid;
        place-items: center;
        border-radius: 50%;
        background: var(--web-tool-accent-soft);
        color: var(--web-tool-accent);
        font-size: 12px;
        font-weight: 900;
    }
    .empty-state h2 {
        font-size: 18px;
    }
    .empty-state p {
        margin-bottom: 10px;
        color: var(--web-tool-muted);
        font-size: 12px;
    }
    .log-overlay {
        position: fixed;
        inset: 0;
        z-index: 1200;
        display: grid;
        place-items: center;
        padding: 20px;
        box-sizing: border-box;
    }
    .log-backdrop {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        padding: 0;
        background: rgba(15, 23, 42, 0.58);
    }
    .log-dialog {
        position: relative;
        width: min(760px, calc(100vw - 32px));
        max-height: min(680px, calc(100vh - 40px));
        margin: 0;
        padding: 0;
        display: grid;
        grid-template-rows: auto minmax(0, 1fr) auto;
        overflow: hidden;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-md);
        background: var(--color-surface);
        color: var(--web-tool-text);
        box-shadow: var(--shadow-pop);
    }
    .log-dialog > header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 18px;
        padding: 18px 20px;
        border-bottom: 1px solid var(--color-border);
    }
    .log-dialog h2 {
        margin-top: 4px;
        font-size: 18px;
    }
    .log-dialog header p {
        margin-top: 5px;
        color: #718096;
        font-size: 11px;
    }
    .log-close {
        width: 32px;
        height: 32px;
        display: grid;
        place-items: center;
        flex: 0 0 auto;
        border-radius: var(--radius-xs);
        background: #f1f5f8;
        color: #526477;
        font-size: 20px;
        line-height: 1;
    }
    .log-content {
        min-height: 150px;
        overflow: auto;
        padding: 10px 20px;
        background: var(--color-surface-soft);
    }
    .log-entry {
        display: grid;
        grid-template-columns: 30px minmax(0, 1fr);
        gap: 10px;
        padding: 12px 0;
        border-bottom: 1px solid #e4eaf0;
    }
    .log-entry:last-child {
        border-bottom: 0;
    }
    .log-entry > span {
        color: var(--color-muted);
        font: 10px var(--font-code);
    }
    .log-entry strong {
        display: block;
        color: var(--color-text);
        font: 10px var(--font-code);
        line-height: 1.5;
        overflow-wrap: anywhere;
    }
    .log-entry p {
        margin-top: 6px;
        color: #a13f45;
        font-size: 11px;
        line-height: 1.55;
        overflow-wrap: anywhere;
    }
    .log-entry.general-error {
        grid-template-columns: 1fr;
        padding: 12px 14px;
        border: 1px solid var(--web-tool-error-border);
        border-radius: var(--radius-sm);
        background: var(--web-tool-error-bg);
    }
    .log-entry.general-error strong {
        color: var(--web-tool-error);
        font-family: inherit;
        font-size: 12px;
    }
    .log-dialog > footer {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 8px;
        padding: 13px 20px;
        border-top: 1px solid var(--color-border);
    }
    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }
    @media (max-width: 900px) {
        .content-grid {
            grid-template-columns: 1fr;
        }
        .style-panel {
            display: grid;
            grid-template-columns: minmax(180px, 0.8fr) 1.2fr;
            align-items: start;
        }
        .style-panel .panel-head {
            grid-column: 1 / -1;
        }
        iframe {
            grid-column: 2;
            grid-row: 2 / span 2;
            min-height: 220px;
            height: 220px;
            margin-top: 14px;
        }
    }
    @media (max-width: 650px) {
        .download-workspace {
            width: calc(100% - 24px);
            padding-top: 12px;
        }
        .metrics {
            grid-template-columns: repeat(2, 1fr);
        }
        .style-panel {
            display: block;
        }
        iframe {
            width: calc(100% - 30px);
            min-height: 240px;
            height: 240px;
        }
        .workspace-head {
            align-items: flex-start;
        }
        .header-actions {
            align-items: stretch;
            flex-direction: column;
        }
    }
</style>
