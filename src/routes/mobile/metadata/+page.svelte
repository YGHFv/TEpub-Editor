<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { message } from "@tauri-apps/plugin-dialog";
    import { cacheBrowserFile, exportEpubPath, safeFileName, selectionName } from "$lib/mobileFlow";

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
    let selectedPath = "";
    let selectedName = "";
    let exportPath = "";
    let metadata: MobileEpubMetadata = { ...emptyMeta };
    let tagsText = "";
    let loaded = false;
    let dirty = false;
    let busy = false;
    let status = "选择 EPUB 后编辑元数据。";

    function openPicker() {
        fileInputEl?.click();
    }

    function markDirty() {
        dirty = true;
    }

    function splitTags(text: string) {
        return text.split(/[,，;；\n]/).map((item) => item.trim()).filter(Boolean);
    }

    function exportName() {
        const title = metadata.title.trim();
        const author = metadata.author.trim();
        const base = title && author ? `${title}-${author}` : title || selectionName(selectedName || "book");
        return safeFileName(base, "epub");
    }

    async function onFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        try {
            busy = true;
            selectedName = file.name;
            exportPath = "";
            selectedPath = await cacheBrowserFile(file, "epub");
            metadata = await invoke<MobileEpubMetadata>("mobile_read_epub_metadata", { epubPath: selectedPath });
            tagsText = metadata.tags.join(", ");
            loaded = true;
            dirty = false;
            status = "已读取元数据。";
        } catch (err) {
            status = "读取元数据失败";
            await message(`读取元数据失败：${err}`, { title: "编辑元数据", kind: "error" });
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
        if (showDialog) await message(status, { title: "编辑元数据", kind: "info" });
    }

    async function onSave() {
        try {
            busy = true;
            await saveMetadata(true);
        } catch (err) {
            status = "保存元数据失败";
            await message(`保存元数据失败：${err}`, { title: "编辑元数据", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function onExport() {
        if (!selectedPath) return;
        try {
            busy = true;
            if (dirty) await saveMetadata(false);
            const result = await exportEpubPath(selectedPath, exportName());
            exportPath = result.output_path;
            status = result.message;
        } catch (err) {
            status = "导出 EPUB 失败";
            await message(`导出 EPUB 失败：${err}`, { title: "编辑元数据", kind: "error" });
        } finally {
            busy = false;
        }
    }
</script>

<svelte:head>
    <title>编辑元数据</title>
</svelte:head>

<main class="page">
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".epub" on:change={onFileChange} />
    <header class="topbar">
        <a href="/mobile" aria-label="返回">‹</a>
        <h1>编辑元数据</h1>
    </header>

    <section class="picker">
        <button type="button" on:click={openPicker} disabled={busy}>{busy ? "处理中" : "选择 EPUB"}</button>
        <p>{status}</p>
    </section>

    {#if loaded}
        <form class="form" on:submit|preventDefault={onSave}>
            <label><span>书名</span><input bind:value={metadata.title} on:input={markDirty} autocomplete="off" /></label>
            <label><span>作者</span><input bind:value={metadata.author} on:input={markDirty} autocomplete="off" /></label>
            <label><span>副标题</span><input bind:value={metadata.subtitle} on:input={markDirty} autocomplete="off" /></label>
            <label><span>出版社</span><input bind:value={metadata.publisher} on:input={markDirty} autocomplete="off" /></label>
            <label><span>制作信息</span><input bind:value={metadata.maker} on:input={markDirty} autocomplete="off" /></label>
            <label><span>系列</span><input bind:value={metadata.series} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>标签</span><input bind:value={tagsText} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>UUID / 标识符</span><input bind:value={metadata.epub_uuid} on:input={markDirty} autocomplete="off" /></label>
            <label class="full"><span>简介</span><textarea bind:value={metadata.description} on:input={markDirty} rows="5"></textarea></label>
            <button type="submit" disabled={busy || !dirty}>{dirty ? "保存元数据" : "已保存"}</button>
            <button class="secondary" type="button" on:click={onExport} disabled={busy}>导出 EPUB</button>
            {#if exportPath}<code>{exportPath}</code>{/if}
        </form>
    {/if}
</main>

<style>
    :global(html), :global(body) { background: #f4f5f8; }
    .page { min-height: 100vh; box-sizing: border-box; padding: max(10px, env(safe-area-inset-top)) 14px max(22px, env(safe-area-inset-bottom)); background: #f4f5f8; color: #171b24; }
    .file-input { position: fixed; width: 1px; height: 1px; opacity: 0; pointer-events: none; }
    .topbar { display: grid; grid-template-columns: 38px minmax(0, 1fr); align-items: center; gap: 8px; min-height: 52px; }
    .topbar a { width: 34px; height: 34px; display: grid; place-items: center; color: inherit; font-size: 28px; line-height: 1; text-decoration: none; }
    h1 { margin: 0; font-size: 22px; letter-spacing: 0; }
    .picker, .form { margin-top: 10px; border: 1px solid rgba(23,27,36,.08); border-radius: 8px; background: #fff; padding: 12px; }
    .picker { display: grid; gap: 8px; }
    .form { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 10px; }
    label { display: grid; gap: 6px; min-width: 0; }
    label.full { grid-column: 1 / -1; }
    label span { color: #626a78; font-size: 12px; font-weight: 800; }
    input, textarea { width: 100%; min-width: 0; box-sizing: border-box; border: 1px solid rgba(23,27,36,.12); border-radius: 8px; padding: 8px 9px; background: #fff; color: inherit; font: inherit; font-size: 13px; line-height: 1.45; }
    textarea { resize: vertical; }
    button { min-height: 36px; border: 0; border-radius: 8px; background: #9b3d4f; color: #fff; font-weight: 900; font-size: 13px; }
    button.secondary { border: 1px solid rgba(155,61,79,.3); background: #f7e9ed; color: #9b3d4f; }
    button:disabled { opacity: .6; }
    p, code { margin: 0; color: #747986; font-size: 13px; line-height: 1.5; word-break: break-all; }
    code { grid-column: 1 / -1; }
    @media (min-width: 720px) { .page { max-width: 760px; margin: 0 auto; } }
</style>
