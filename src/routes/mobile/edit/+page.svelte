<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { message } from "@tauri-apps/plugin-dialog";
    import { cacheBrowserFile, exportEpubPath, safeFileName, selectionName } from "$lib/mobileFlow";
    import EpubCodeEditor from "$lib/EpubCodeEditor.svelte";

    interface EpubFileNode {
        name: string;
        path: string;
        file_type: "folder" | "html" | "css" | "xml" | "image" | "font" | "other";
        size?: number;
        title?: string;
        resolution?: string;
        children?: EpubFileNode[];
    }

    interface MobileEpubMetadata {
        title: string;
        author: string;
    }

    interface FlatFile extends EpubFileNode {
        group: string;
        groupPath: string;
    }

    interface FileGroup {
        name: string;
        path: string;
        files: FlatFile[];
    }

    let fileInputEl: HTMLInputElement | null = null;
    let addFileInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let bookTitle = "编辑 EPUB";
    let status = "选择 EPUB 后浏览内部文件结构。";
    let busy = false;
    let fileTree: EpubFileNode[] = [];
    let flatFiles: FlatFile[] = [];
    let openGroups = new Set<string>();
    let exportPath = "";
    let editingFile: FlatFile | null = null;
    let editingContent = "";
    let editorDirty = false;
    let previewUrl = "";
    let addTargetGroup: FileGroup | null = null;

    $: groups = buildGroups(flatFiles);

    function openPicker() {
        fileInputEl?.click();
    }

    async function onFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file) return;

        try {
            busy = true;
            exportPath = "";
            selectedName = file.name;
            selectedPath = await cacheBrowserFile(file, "epub");
            bookTitle = selectionName(file.name).replace(/\.epub$/i, "");
            try {
                const meta = await invoke<MobileEpubMetadata>("mobile_read_epub_metadata", { epubPath: selectedPath });
                if (meta.title?.trim()) bookTitle = meta.title.trim();
            } catch (_) {
                // metadata is a nice-to-have for the mobile header
            }
            fileTree = await invoke<EpubFileNode[]>("extract_epub", { epubPath: selectedPath });
            refreshFlatFiles(fileTree);
            status = `已解包 ${flatFiles.length} 个文件。`;
        } catch (err) {
            status = "EPUB 解包失败";
            await message(`EPUB 解包失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function flattenFiles(nodes: EpubFileNode[], out: FlatFile[] = []) {
        for (const node of nodes) {
            if (node.file_type === "folder" && node.children?.length) {
                flattenFiles(node.children, out);
            } else if (node.file_type !== "folder") {
                const parts = node.path.split("/");
                const groupPath = parts.length > 1 ? parts.slice(0, -1).join("/") : "";
                const group = groupPath ? groupPath.toUpperCase() : "ROOT";
                out.push({ ...node, group, groupPath });
            }
        }
        return out.sort((a, b) => a.group.localeCompare(b.group) || naturalName(a.name).localeCompare(naturalName(b.name)));
    }

    function refreshFlatFiles(tree: EpubFileNode[]) {
        flatFiles = flattenFiles(tree);
        openGroups = new Set(buildGroups(flatFiles).map((group) => group.name));
    }

    function naturalName(name: string) {
        return name.replace(/(\d+)/g, (m) => m.padStart(8, "0"));
    }

    function buildGroups(files: FlatFile[]): FileGroup[] {
        const map = new Map<string, FileGroup>();
        for (const file of files) {
            if (!map.has(file.group)) map.set(file.group, { name: file.group, path: file.groupPath, files: [] });
            map.get(file.group)?.files.push(file);
        }
        return [...map.values()];
    }

    function toggleGroup(name: string) {
        if (openGroups.has(name)) openGroups.delete(name);
        else openGroups.add(name);
        openGroups = new Set(openGroups);
    }

    function fileStem(name: string) {
        return name.replace(/\.[^.]+$/, "");
    }

    function fileDetail(file: FlatFile) {
        if (file.file_type === "html") return file.title?.trim() || "未命名章节";
        if (file.file_type === "css") return "层叠样式表";
        if (file.file_type === "xml") return file.name.toLowerCase().includes("toc") || file.name.endsWith(".ncx") ? "目录结构" : "元数据";
        if (file.file_type === "font") return `字体${file.size ? `  ${formatSize(file.size)}` : ""}`;
        if (isImageFile(file)) {
            return `图片${file.resolution ? `  ${file.resolution}` : ""}${file.size ? `  ${formatSize(file.size)}` : ""}`;
        }
        return file.size ? formatSize(file.size) : "文件";
    }

    function formatSize(size: number) {
        if (size >= 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)}MB`;
        if (size >= 1024) return `${Math.round(size / 1024)}KB`;
        return `${size}B`;
    }

    function iconFor(file: FlatFile) {
        if (isImageFile(file)) return "img";
        if (file.file_type === "font") return "T";
        if (file.file_type === "css") return "{}";
        if (file.file_type === "xml") return file.name.toLowerCase().includes("content") ? "db" : "≡";
        if (file.file_type === "html") return "</>";
        return "•";
    }

    function iconClass(file: FlatFile) {
        return `icon ${isImageFile(file) ? "image" : file.file_type}`;
    }

    function isEditable(file: FlatFile) {
        return !isImageFile(file) && ["html", "css", "xml", "other"].includes(file.file_type);
    }

    function isImageFile(file: FlatFile) {
        return file.file_type === "image" || /\.(?:jpe?g|png|gif|webp|bmp|svg)$/i.test(file.name) || /\.(?:jpe?g|png|gif|webp|bmp|svg)$/i.test(file.path);
    }

    function mimeFor(file: FlatFile) {
        const lower = file.name.toLowerCase();
        if (lower.endsWith(".png")) return "image/png";
        if (lower.endsWith(".gif")) return "image/gif";
        if (lower.endsWith(".webp")) return "image/webp";
        if (lower.endsWith(".svg")) return "image/svg+xml";
        if (lower.endsWith(".bmp")) return "image/bmp";
        return "image/jpeg";
    }

    function bytesFromInvoke(data: unknown) {
        if (data instanceof ArrayBuffer) return new Uint8Array(data);
        if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        if (Array.isArray(data)) return new Uint8Array(data);
        if (data && typeof data === "object" && "data" in data && Array.isArray((data as { data: unknown }).data)) {
            return new Uint8Array((data as { data: number[] }).data);
        }
        return new Uint8Array();
    }

    function imageMimeFor(file: FlatFile, bytes: Uint8Array) {
        if (bytes.length >= 12) {
            const riff = String.fromCharCode(...bytes.slice(0, 4));
            const webp = String.fromCharCode(...bytes.slice(8, 12));
            if (riff === "RIFF" && webp === "WEBP") return "image/webp";
        }
        if (bytes[0] === 0xff && bytes[1] === 0xd8) return "image/jpeg";
        if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return "image/png";
        if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) return "image/gif";
        if (bytes[0] === 0x42 && bytes[1] === 0x4d) return "image/bmp";
        return mimeFor(file);
    }

    function languageFor(file: FlatFile): "html" | "css" | "xml" | "other" {
        if (file.file_type === "html") return "html";
        if (file.file_type === "css") return "css";
        if (file.file_type === "xml") return "xml";
        return "other";
    }

    function clearPreviewUrl() {
        if (previewUrl) URL.revokeObjectURL(previewUrl);
        previewUrl = "";
    }

    async function openFile(file: FlatFile) {
        if (!selectedPath) return;
        clearPreviewUrl();
        editingFile = file;
        editingContent = "";
        editorDirty = false;

        if (isImageFile(file)) {
            try {
                busy = true;
                const data = await invoke<unknown>("read_epub_file_binary", {
                    epubPath: selectedPath,
                    filePath: file.path,
                });
                const bytes = bytesFromInvoke(data);
                if (!bytes.length) throw new Error("图片数据为空");
                const blob = new Blob([bytes], { type: imageMimeFor(file, bytes) });
                previewUrl = URL.createObjectURL(blob);
                status = `正在预览：${file.path}`;
            } catch (err) {
                status = "读取图片失败";
                await message(`读取图片失败：${err}`, { title: "编辑 EPUB", kind: "error" });
            } finally {
                busy = false;
            }
            return;
        }

        if (!isEditable(file)) {
            status = `${file.name} 暂不支持直接编辑。`;
            return;
        }
        try {
            busy = true;
            editingContent = await invoke<string>("read_epub_file_content", {
                epubPath: selectedPath,
                filePath: file.path,
            });
            editorDirty = false;
            status = `正在编辑：${file.path}`;
        } catch (err) {
            status = "读取文件失败";
            await message(`读取文件失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function saveEditingFile() {
        if (!selectedPath || !editingFile) return;
        try {
            busy = true;
            await invoke("save_epub_file_content", {
                epubPath: selectedPath,
                filePath: editingFile.path,
                content: editingContent,
            });
            editorDirty = false;
            status = `已保存：${editingFile.path}`;
        } catch (err) {
            status = "保存文件失败";
            await message(`保存文件失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function closeEditor() {
        clearPreviewUrl();
        editingFile = null;
        editingContent = "";
        editorDirty = false;
    }

    function openAddFilePicker(group: FileGroup) {
        addTargetGroup = group;
        addFileInputEl?.click();
    }

    async function onAddFileChange(event: Event) {
        const input = event.currentTarget as HTMLInputElement;
        const file = input.files?.[0];
        input.value = "";
        if (!file || !selectedPath || !addTargetGroup) return;

        try {
            busy = true;
            const targetPath = `${addTargetGroup.path ? `${addTargetGroup.path}/` : ""}${file.name}`.replace(/^\/+/, "");
            const bytes = new Uint8Array(await file.arrayBuffer());
            await invoke("add_epub_file_binary", {
                epubPath: selectedPath,
                filePath: targetPath,
                content: Array.from(bytes),
            });
            fileTree = await invoke<EpubFileNode[]>("extract_epub", { epubPath: selectedPath });
            refreshFlatFiles(fileTree);
            openGroups.add(addTargetGroup.name);
            openGroups = new Set(openGroups);
            status = `已添加：${targetPath}`;
        } catch (err) {
            status = "添加文件失败";
            await message(`添加文件失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
            addTargetGroup = null;
        }
    }

    async function exportEditedEpub() {
        if (!selectedPath) return;
        try {
            busy = true;
            status = "正在保存 EPUB 编辑缓存。";
            await invoke("save_epub_to_disk", { epubPath: selectedPath });
            const result = await exportEpubPath(selectedPath, safeFileName(selectedName || `${bookTitle}.epub`, "epub"));
            exportPath = result.output_path;
            status = result.message;
        } catch (err) {
            status = "保存或导出失败";
            await message(`保存或导出失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }
</script>

<svelte:head>
    <title>编辑 EPUB</title>
</svelte:head>

<main class="editor-page">
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".epub" on:change={onFileChange} />
    <input bind:this={addFileInputEl} class="file-input" type="file" on:change={onAddFileChange} />

    <header class="topbar">
        <a href="/mobile" aria-label="返回">‹</a>
        <h1>{bookTitle}</h1>
    </header>

    {#if !selectedPath}
        <section class="empty">
            <button type="button" on:click={openPicker} disabled={busy}>{busy ? "处理中" : "选择 EPUB"}</button>
            <p>{status}</p>
        </section>
    {:else}
        <section class="actions">
            <button type="button" on:click={openPicker} disabled={busy}>重新选择</button>
            <button type="button" on:click={exportEditedEpub} disabled={busy}>保存并导出</button>
        </section>
        <p class="status">{status}</p>
        {#if exportPath}<code>{exportPath}</code>{/if}

        <section class="file-list">
            {#each groups as group}
                <div class="group">
                    <div class="group-head">
                        <button class="group-title" type="button" on:click={() => toggleGroup(group.name)}>
                            <span>{group.name}</span>
                        </button>
                        <button class="group-add" type="button" aria-label={`向 ${group.name} 添加文件`} on:click={() => openAddFilePicker(group)}>＋</button>
                        <button class="group-toggle" type="button" aria-label={openGroups.has(group.name) ? "折叠" : "展开"} on:click={() => toggleGroup(group.name)}>
                            {openGroups.has(group.name) ? "⌄" : "›"}
                        </button>
                    </div>
                    {#if openGroups.has(group.name)}
                        {#each group.files as file}
                            <button class="file-row" type="button" on:click={() => openFile(file)}>
                                <span class={iconClass(file)}>{iconFor(file)}</span>
                                <span class="file-copy">
                                    <strong>{fileStem(file.name)}</strong>
                                    <small>{fileDetail(file)}</small>
                                </span>
                                <span class="more">⋮</span>
                            </button>
                        {/each}
                    {/if}
                </div>
            {/each}
        </section>

        {#if editingFile}
            <section class="mobile-editor">
                <div class="editor-head">
                    <div>
                        <strong>{editingFile.name}</strong>
                        <small>{editingFile.path}</small>
                    </div>
                    <button type="button" on:click={closeEditor}>关闭</button>
                </div>
                {#if isImageFile(editingFile)}
                    <div class="image-preview">
                        {#if previewUrl}
                            <img src={previewUrl} alt={editingFile.name} />
                        {:else}
                            <span>图片加载中</span>
                        {/if}
                    </div>
                {:else}
                    <div class="code-editor-wrap">
                        <EpubCodeEditor
                            doc={editingContent}
                            language={languageFor(editingFile)}
                            onChange={(value) => {
                                editingContent = value;
                                editorDirty = true;
                            }}
                            onSave={saveEditingFile}
                        />
                    </div>
                    <button class="save-file" type="button" on:click={saveEditingFile} disabled={busy || !editorDirty}>
                        {editorDirty ? "保存文件" : "已保存"}
                    </button>
                {/if}
            </section>
        {/if}
    {/if}
</main>

<style>
    :global(html),
    :global(body) {
        background: #f2f3f8;
    }

    .editor-page {
        min-height: 100vh;
        box-sizing: border-box;
        padding: max(10px, env(safe-area-inset-top)) 0 max(22px, env(safe-area-inset-bottom));
        background: #f2f3f8;
        color: #151923;
    }

    .file-input {
        position: fixed;
        width: 1px;
        height: 1px;
        opacity: 0;
        pointer-events: none;
    }

    .topbar {
        min-height: 52px;
        display: grid;
        grid-template-columns: 38px minmax(0, 1fr);
        align-items: center;
        gap: 8px;
        padding: 0 14px;
        background: transparent;
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
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 22px;
        line-height: 1.2;
        letter-spacing: 0;
    }

    .empty {
        display: grid;
        gap: 10px;
        margin: 10px 14px 0;
        border: 1px solid rgba(23, 27, 36, 0.08);
        border-radius: 8px;
        background: #fff;
        padding: 12px;
    }

    .empty p {
        margin: 0;
        color: #747986;
        font-size: 13px;
        line-height: 1.5;
    }

    .actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        padding: 10px 14px 5px;
    }

    button {
        font: inherit;
    }

    .empty button,
    .actions button {
        min-height: 36px;
        border: 0;
        border-radius: 8px;
        background: #1f7a5a;
        color: #fff;
        font-weight: 900;
    }

    .actions button:first-child {
        background: #e6eee9;
        color: #1f7a5a;
    }

    button:disabled {
        opacity: 0.6;
    }

    .status,
    code {
        display: block;
        margin: 6px 14px;
        color: #747986;
        font-size: 12px;
        line-height: 1.5;
        word-break: break-all;
    }

    .file-list {
        margin-top: 6px;
    }

    .group {
        border-top: 1px solid #dedfe6;
    }

    .group-head {
        width: 100%;
        height: 38px;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 28px 28px;
        align-items: center;
        gap: 4px;
        box-sizing: border-box;
        padding: 0 14px;
        background: transparent;
        color: #8a8d96;
    }

    .group-title,
    .group-add,
    .group-toggle {
        min-width: 0;
        width: 100%;
        height: 30px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 0;
        background: transparent;
        color: inherit;
    }

    .group-title {
        justify-items: start;
        text-align: left;
    }

    .group-title span {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-family: Georgia, "Times New Roman", serif;
        font-size: 13px;
        letter-spacing: 0;
    }

    .group-add,
    .group-toggle {
        color: #a1a3ab;
        font-size: 18px;
        font-weight: 400;
        text-align: center;
        line-height: 1;
    }

    .group-toggle {
        font-size: 18px;
        transform: translateY(-1px);
    }

    .file-row {
        width: 100%;
        min-height: 58px;
        display: grid;
        grid-template-columns: 38px minmax(0, 1fr) 20px;
        align-items: center;
        gap: 9px;
        box-sizing: border-box;
        padding: 7px 14px 7px 58px;
        border: 0;
        border-top: 1px solid #e2e4ea;
        background: transparent;
        color: inherit;
        text-align: left;
    }

    .icon {
        width: 34px;
        height: 36px;
        display: grid;
        place-items: center;
        border-radius: 7px;
        background: #8ea1bd;
        color: #fff;
        font-size: 13px;
        font-weight: 900;
    }

    .icon.html {
        background: #f0842f;
        font-size: 12px;
    }

    .icon.css {
        background: #55aee2;
    }

    .icon.xml {
        background: #67c8ad;
    }

    .icon.image {
        background: #79b8a8;
    }

    .icon.font {
        background: #c9ae70;
        font-size: 20px;
        font-family: Georgia, "Times New Roman", serif;
    }

    .file-copy {
        min-width: 0;
        display: grid;
        gap: 3px;
    }

    .file-copy strong {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 16px;
        line-height: 1.1;
    }

    .file-copy small {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #838894;
        font-family: Georgia, "Times New Roman", serif;
        font-size: 13px;
        line-height: 1.3;
    }

    .more {
        color: #848894;
        font-size: 22px;
        line-height: 1;
        text-align: center;
    }

    .mobile-editor {
        position: fixed;
        left: 0;
        right: 0;
        bottom: 0;
        z-index: 20;
        display: grid;
        gap: 8px;
        box-sizing: border-box;
        max-height: 78vh;
        padding: 12px 14px max(14px, env(safe-area-inset-bottom));
        border-top: 1px solid #d9dce4;
        background: #fbfbfd;
        box-shadow: 0 -12px 28px rgba(18, 24, 36, 0.16);
    }

    .editor-head {
        display: grid;
        grid-template-columns: minmax(0, 1fr) 58px;
        gap: 10px;
        align-items: center;
    }

    .editor-head div {
        min-width: 0;
        display: grid;
        gap: 2px;
    }

    .editor-head strong,
    .editor-head small {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .editor-head strong {
        font-size: 15px;
    }

    .editor-head small {
        color: #7d8490;
        font-size: 11px;
    }

    .editor-head button,
    .save-file {
        min-height: 34px;
        border: 0;
        border-radius: 8px;
        background: #e6eee9;
        color: #1f7a5a;
        font-weight: 900;
    }

    .image-preview {
        min-height: 260px;
        display: grid;
        place-items: center;
        overflow: auto;
        border: 1px solid #d6dbe4;
        border-radius: 8px;
        background: #eef0f5;
        color: #7d8490;
        font-size: 12px;
    }

    .image-preview img {
        display: block;
        max-width: 100%;
        max-height: 60vh;
        object-fit: contain;
    }

    .code-editor-wrap {
        width: 100%;
        height: min(54vh, 420px);
        box-sizing: border-box;
        border: 1px solid #d6dbe4;
        border-radius: 8px;
        overflow: hidden;
        background: #fff;
    }

    :global(.mobile-editor .cm-editor) {
        background: #fff;
    }

    :global(.mobile-editor .cm-scroller),
    :global(.mobile-editor .cm-content) {
        font-size: 12px;
        line-height: 1.55;
    }

    .save-file {
        background: #1f7a5a;
        color: #fff;
    }

    @media (min-width: 720px) {
        .editor-page {
            max-width: 760px;
            margin: 0 auto;
        }
    }
</style>
