<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { message } from "@tauri-apps/plugin-dialog";
    import {
        cacheBrowserFile,
        exportEpubPath,
        readMobileSelection,
        safeFileName,
        selectionName,
    } from "$lib/mobileFlow";

    let fileInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let resultPath = "";
    let exportPath = "";
    let status = "选择 EPUB 后清理伪加密和异常结构。";
    let busy = false;

    function openPicker() {
        fileInputEl?.click();
    }

    async function processEpub(epubPath: string, name = "") {
        try {
            busy = true;
            selectedPath = epubPath;
            selectedName = name || selectionName(epubPath);
            exportPath = "";
            const result = await invoke<{
                source_path: string;
                processed_path: string;
                changed: boolean;
                action: string;
            }>("prepare_epub_for_open", { epubPath });
            resultPath = result.processed_path || epubPath;
            status = result.changed ? result.action : "这个 EPUB 暂时不需要解密或修复，可直接导出副本。";
        } catch (err) {
            status = "处理 EPUB 失败";
            await message(`处理 EPUB 失败：${err}`, { title: "解密 EPUB", kind: "error" });
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
            await processEpub(cachedPath, file.name);
        } catch (err) {
            status = "导入 EPUB 失败";
            await message(`导入 EPUB 失败：${err}`, { title: "解密 EPUB", kind: "error" });
        }
    }

    async function exportResult() {
        const path = resultPath || selectedPath;
        if (!path) return;
        try {
            busy = true;
            const base = selectionName(selectedName || "book.epub").replace(/\.epub$/i, "");
            const result = await exportEpubPath(path, safeFileName(`${base}-clean`, "epub"));
            exportPath = result.output_path;
            status = result.message;
        } catch (err) {
            status = "导出 EPUB 失败";
            await message(`导出 EPUB 失败：${err}`, { title: "解密 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    onMount(() => {
        const selection = readMobileSelection(window.location.search);
        if (selection.path) {
            void processEpub(selection.path, selection.name);
        }
    });
</script>

<svelte:head>
    <title>解密 EPUB</title>
</svelte:head>

<main class="page">
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".epub" on:change={onFileChange} />
    <header class="topbar">
        <a href="/mobile" aria-label="返回">
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M15 6L9 12L15 18"></path>
            </svg>
        </a>
        <h1>解密 EPUB</h1>
    </header>
    <section class="panel">
        <button type="button" on:click={openPicker} disabled={busy}>{busy ? "处理中" : "重新选择 EPUB"}</button>
        {#if selectedName}<strong>{selectedName}</strong>{/if}
        <p>{status}</p>
        {#if resultPath}<code>{resultPath}</code>{/if}
        {#if resultPath}
            <button class="secondary" type="button" on:click={exportResult} disabled={busy}>导出处理结果</button>
        {/if}
        {#if exportPath}<code>{exportPath}</code>{/if}
    </section>
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

    h1 {
        margin: 0;
        font-size: 22px;
        letter-spacing: 0;
    }

    .panel {
        display: grid;
        gap: 10px;
        margin-top: 10px;
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-radius: 8px;
        background: #fff;
        padding: 12px;
    }

    button {
        min-height: 36px;
        border: 0;
        border-radius: 8px;
        background: #8a5a16;
        color: #fff;
        font-weight: 900;
        font-size: 13px;
    }

    button.secondary {
        background: #f1eadf;
        color: #8a5a16;
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

    strong {
        font-size: 14px;
    }

    @media (min-width: 720px) {
        .page {
            max-width: 760px;
            margin: 0 auto;
        }
    }
</style>
