<script lang="ts">
    import { onMount } from "svelte";
    import { afterNavigate, goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { message } from "@tauri-apps/plugin-dialog";
    import {
        buildMobileRoute,
        cacheBrowserFile,
        exportEpubPath,
        safeFileName,
        selectionName,
    } from "$lib/mobileFlow";

    interface MobileEpubMetadata {
        title: string;
        author: string;
        publisher: string;
        description: string;
        epub_uuid: string;
        subtitle: string;
        series: string;
        maker: string;
        tags: string[];
    }

    interface MobileEpubCover {
        bytes: number[];
    }

    const emptyMeta: MobileEpubMetadata = {
        title: "",
        author: "",
        publisher: "",
        description: "",
        epub_uuid: "",
        subtitle: "",
        series: "",
        maker: "",
        tags: [],
    };

    let fileInputEl: HTMLInputElement | null = null;
    let coverInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let exportPath = "";
    let coverPreviewUrl = "";
    let metadata: MobileEpubMetadata = { ...emptyMeta };
    let tagsText = "";
    let loaded = false;
    let dirty = false;
    let busy = false;
    let status = "选择 EPUB 后编辑元数据。";
    let lastQueryLoadKey = "";

    function openPicker() {
        fileInputEl?.click();
    }

    function openCoverPicker() {
        coverInputEl?.click();
    }

    function markDirty() {
        dirty = true;
    }

    function splitTags(text: string) {
        return text.split(/[,，;；\n]/).map((item) => item.trim()).filter(Boolean);
    }

    function clearCoverPreview() {
        if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
        coverPreviewUrl = "";
    }

    function detectImageMime(bytes: Uint8Array) {
        if (bytes.length >= 12) {
            const riff = String.fromCharCode(...bytes.slice(0, 4));
            const webp = String.fromCharCode(...bytes.slice(8, 12));
            if (riff === "RIFF" && webp === "WEBP") return "image/webp";
        }
        if (bytes.length >= 4 && bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return "image/png";
        if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8) return "image/jpeg";
        if (bytes.length >= 3 && bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) return "image/gif";
        if (bytes.length >= 2 && bytes[0] === 0x42 && bytes[1] === 0x4d) return "image/bmp";
        if (bytes.length >= 4 && bytes[0] === 0x3c && bytes[1] === 0x73 && bytes[2] === 0x76 && bytes[3] === 0x67) return "image/svg+xml";
        return "image/jpeg";
    }

    async function loadCover(epubPath: string) {
        clearCoverPreview();
        try {
            const cover = await invoke<MobileEpubCover>("mobile_read_epub_cover", { epubPath });
            const bytes = new Uint8Array(cover.bytes ?? []);
            if (!bytes.length) return;
            const blob = new Blob([bytes], { type: detectImageMime(bytes) });
            coverPreviewUrl = URL.createObjectURL(blob);
        } catch {
            coverPreviewUrl = "";
        }
    }

    function exportName() {
        const title = metadata.title.trim();
        const author = metadata.author.trim();
        const base = title && author ? `${title}-${author}` : title || selectionName(selectedName || "book");
        return safeFileName(base, "epub");
    }

    function editRoute() {
        return buildMobileRoute("/mobile/edit", {
            path: selectedPath,
            name: selectedName,
        });
    }

    async function loadMetadata(epubPath: string, name = "") {
        if (!epubPath) return;
        busy = true;
        try {
            selectedPath = epubPath;
            selectedName = name || selectionName(epubPath);
            exportPath = "";
            status = "正在读取元数据...";
            metadata = await invoke<MobileEpubMetadata>("mobile_read_epub_metadata", { epubPath });
            await loadCover(epubPath);
            tagsText = metadata.tags.join(", ");
            loaded = true;
            dirty = false;
            status = "已读取元数据。";
        } catch (err) {
            loaded = false;
            status = "读取元数据失败";
            await message(`读取元数据失败：${err}`, { title: "编辑 EPUB", kind: "error" });
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
            const cachedPath = await cacheBrowserFile(file, "epub");
            await loadMetadata(cachedPath, file.name);
        } catch (err) {
            status = "导入 EPUB 失败";
            await message(`导入 EPUB 失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        }
    }

    async function onCoverChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file || !selectedPath || !loaded) return;

        try {
            busy = true;
            const bytes = new Uint8Array(await file.arrayBuffer());
            const cover = await invoke<MobileEpubCover>("mobile_update_epub_cover", {
                epubPath: selectedPath,
                coverData: Array.from(bytes),
            });
            const nextBytes = new Uint8Array(cover.bytes ?? []);
            clearCoverPreview();
            if (nextBytes.length) {
                const blob = new Blob([nextBytes], { type: detectImageMime(nextBytes) });
                coverPreviewUrl = URL.createObjectURL(blob);
            }
            dirty = false;
            exportPath = "";
            status = "封面已更新到应用缓存副本。";
        } catch (err) {
            status = "更换封面失败";
            await message(`更换封面失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function saveMetadata(showDialog = true) {
        if (!selectedPath || !loaded) return;
        const saved = await invoke<MobileEpubMetadata>("mobile_update_epub_metadata", {
            epubPath: selectedPath,
            metadata: { ...metadata, tags: splitTags(tagsText) },
        });
        metadata = saved;
        tagsText = saved.tags.join(", ");
        dirty = false;
        status = "元数据已保存到应用缓存副本。";
        if (showDialog) await message(status, { title: "编辑 EPUB", kind: "info" });
    }

    async function openStructureEditor() {
        if (!selectedPath || !loaded) return;
        try {
            busy = true;
            if (dirty) await saveMetadata(false);
            await goto(editRoute());
        } catch (err) {
            await message(`进入 EPUB 文件结构失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function onSaveAndExport() {
        if (!selectedPath) return;
        try {
            busy = true;
            try {
                await invoke("save_epub_to_disk", { epubPath: selectedPath });
            } catch (err) {
                const text = String(err ?? "");
                if (!text.includes("EPUB 未加载或缓存失效")) {
                    throw err;
                }
            }
            if (dirty) await saveMetadata(false);
            const result = await exportEpubPath(selectedPath, exportName());
            exportPath = result.output_path;
            status = result.message;
        } catch (err) {
            status = "保存并导出 EPUB 失败";
            await message(`保存并导出 EPUB 失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function queryLoadKey() {
        const params = new URLSearchParams(window.location.search);
        const path = params.get("path")?.trim() ?? "";
        const name = params.get("name")?.trim() ?? "";
        const refresh = params.get("refresh")?.trim() ?? "";
        return {
            key: `${path}|${name}|${refresh}`,
            path,
            name,
        };
    }

    async function syncFromQuery(force = false) {
        const next = queryLoadKey();
        if (!next.path) return;
        if (!force && next.key === lastQueryLoadKey) return;
        lastQueryLoadKey = next.key;
        await loadMetadata(next.path, next.name);
    }

    onMount(() => {
        void syncFromQuery(true);
    });

    afterNavigate(() => {
        void syncFromQuery(true);
    });
</script>

<svelte:head>
    <title>编辑 EPUB</title>
</svelte:head>

<main class="page">
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".epub" on:change={onFileChange} />
    <input bind:this={coverInputEl} class="file-input" type="file" accept="image/png,image/jpeg,image/webp,image/gif,image/bmp,image/svg+xml" on:change={onCoverChange} />
    <header class="topbar">
        <a href="/mobile" aria-label="返回">‹</a>
        <h1>编辑 EPUB</h1>
    </header>

    {#if !loaded}
        <section class="picker">
            <button type="button" on:click={openPicker} disabled={busy}>{busy ? "处理中" : "选择 EPUB"}</button>
            <p>{status}</p>
        </section>
    {/if}

    {#if loaded}
        <section class="form">
            <div class="hero-grid">
                <div class="hero-main">
                    <label class="hero-field">
                        <span>书名</span>
                        <input bind:value={metadata.title} on:input={markDirty} autocomplete="off" />
                    </label>
                    <label class="hero-field">
                        <span>作者</span>
                        <input bind:value={metadata.author} on:input={markDirty} autocomplete="off" />
                    </label>
                </div>
                <button class="cover-panel" type="button" aria-label="更换封面" title="更换封面" on:click={openCoverPicker} disabled={busy}>
                    {#if coverPreviewUrl}
                        <img src={coverPreviewUrl} alt="EPUB 封面" />
                    {:else}
                        <span>点击选择封面</span>
                    {/if}
                </button>
            </div>
            <label><span>副标题</span><input bind:value={metadata.subtitle} on:input={markDirty} autocomplete="off" /></label>
            <label><span>出版社</span><input bind:value={metadata.publisher} on:input={markDirty} autocomplete="off" /></label>
            <label><span>制作信息</span><input bind:value={metadata.maker} on:input={markDirty} autocomplete="off" /></label>
            <label><span>系列</span><input bind:value={metadata.series} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>标签</span><input bind:value={tagsText} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>UUID / 标识符</span><input bind:value={metadata.epub_uuid} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>简介</span><textarea bind:value={metadata.description} on:input={markDirty} rows="5"></textarea></label>
            <div class="action-row full">
                <button class="secondary" type="button" on:click={openStructureEditor} disabled={busy}>
                    {busy ? "处理中" : "编辑 EPUB 文件"}
                </button>
                <button type="button" on:click={onSaveAndExport} disabled={busy}>
                    {busy ? "处理中" : "保存并导出"}
                </button>
            </div>
            {#if exportPath}<code class="export-path full">{exportPath}</code>{/if}
        </section>
    {/if}
</main>

<style>
    :global(html),
    :global(body) {
        background: #f4f5f8;
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
        font-size: 28px;
        line-height: 1;
        text-decoration: none;
    }

    h1 {
        margin: 0;
        font-size: 22px;
        letter-spacing: 0;
    }

    .picker,
    .form {
        margin-top: 10px;
    }

    .picker,
    .form {
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-radius: 8px;
        background: #fff;
        padding: 12px;
    }

    .picker {
        display: grid;
        gap: 8px;
    }

    .form {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 10px;
    }

    label,
    .full {
        display: grid;
        gap: 6px;
        min-width: 0;
    }

    label.full,
    .full {
        grid-column: 1 / -1;
    }

    .hero-grid {
        grid-column: 1 / -1;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 112px;
        gap: 12px;
        align-items: stretch;
    }

    .hero-main {
        display: grid;
        gap: 10px;
    }

    .hero-field {
        min-height: 0;
    }

    .cover-panel {
        width: 100%;
        aspect-ratio: 3 / 4;
        min-height: 0;
        display: grid;
        place-items: center;
        overflow: hidden;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        background: #eef3f1;
        color: #7a828f;
        font-size: 12px;
        font-weight: 800;
        text-align: center;
        padding: 0;
    }

    .cover-panel img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }

    .cover-panel:disabled {
        opacity: 0.7;
    }

    label span {
        color: #626a78;
        font-size: 12px;
        font-weight: 800;
    }

    input,
    textarea {
        width: 100%;
        min-width: 0;
        box-sizing: border-box;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        padding: 8px 9px;
        background: #fff;
        color: inherit;
        font: inherit;
        font-size: 13px;
        line-height: 1.45;
    }

    textarea {
        resize: vertical;
    }

    button {
        min-height: 36px;
        border: 0;
        border-radius: 8px;
        background: #1f7a5a;
        color: #fff;
        font-weight: 900;
        font-size: 13px;
    }

    button.secondary {
        border: 1px solid rgba(31, 122, 90, 0.26);
        background: #edf6f1;
        color: #1f7a5a;
    }

    .action-row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 10px;
        align-items: stretch;
    }

    button:disabled {
        opacity: 0.6;
    }

    p,
    code {
        margin: 0;
        color: #747986;
        font-size: 13px;
        line-height: 1.5;
        word-break: break-all;
    }

    code {
        display: block;
    }

    .export-path {
        margin-top: 2px;
        width: 100%;
        box-sizing: border-box;
        overflow-wrap: anywhere;
    }

    @media (min-width: 720px) {
        .page {
            max-width: 760px;
            margin: 0 auto;
        }
    }
</style>
