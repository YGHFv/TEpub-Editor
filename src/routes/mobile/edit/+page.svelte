<script lang="ts">
    import { onMount, tick } from "svelte";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { message } from "@tauri-apps/plugin-dialog";
    import { buildMobileRoute, cacheBrowserFile, readMobileSelection, selectionName } from "$lib/mobileFlow";
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

    interface SearchMatch {
        file: FlatFile;
        from: number;
        to: number;
    }

    interface ConfirmSheetState {
        open: boolean;
        title: string;
        message: string;
        confirmLabel: string;
        secondaryLabel: string;
        cancelLabel: string;
        tone: "primary" | "danger";
    }

    type ConfirmSheetResult = "confirm" | "secondary" | "cancel";

    interface RenameSheetState {
        open: boolean;
        file: FlatFile | null;
        value: string;
    }

    let fileInputEl: HTMLInputElement | null = null;
    let addFileInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let bookTitle = "编辑 EPUB";
    let status = "从元数据页进入后浏览内部文件结构。";
    let busy = false;
    let fileTree: EpubFileNode[] = [];
    let flatFiles: FlatFile[] = [];
    let openGroups = new Set<string>();
    let editingFile: FlatFile | null = null;
    let editingContent = "";
    let epubEditorComponent: EpubCodeEditor | null = null;
    let editorDirty = false;
    let previewUrl = "";
    let fontPreviewFace: FontFace | null = null;
    let fontPreviewFamily = "";
    let addTargetGroup: FileGroup | null = null;
    let findPattern = "";
    let replacePattern = "";
    let replaceIsRegex = false;
    let replaceOnlyText = false;
    let replaceScope: "current" | "html" | "all" = "html";
    let replaceStatus = "";
    let replacePanelOpen = false;
    let currentMatch: { from: number; to: number; filePath: string } | null = null;
    let currentMatchIndex = 0;
    let currentMatchCount = 0;
    let thumbnailUrls = new Map<string, string>();
    let thumbnailLoading = new Set<string>();
    let fileActionTarget: FlatFile | null = null;
    let renameSheet: RenameSheetState = {
        open: false,
        file: null,
        value: "",
    };
    let confirmSheet: ConfirmSheetState = {
        open: false,
        title: "",
        message: "",
        confirmLabel: "确定",
        secondaryLabel: "",
        cancelLabel: "取消",
        tone: "primary",
    };

    const FILE_LAYER_STATE_KEY = "mobile-edit-file-open";
    let confirmResolver: ((result: ConfirmSheetResult) => void) | null = null;
    let restoringFileLayer = false;
    let thumbnailEpoch = 0;

    $: groups = buildGroups(flatFiles);

    function openPicker() {
        fileInputEl?.click();
    }

    function metadataRoute(refresh = "") {
        if (!selectedPath) return "/mobile";
        return buildMobileRoute("/mobile/metadata", {
            path: selectedPath,
            name: selectedName,
            refresh,
        });
    }

    async function loadEpub(epubPath: string, name = "") {
        try {
            busy = true;
            selectedName = name || selectionName(epubPath);
            selectedPath = epubPath;
            bookTitle = selectionName(selectedName).replace(/\.epub$/i, "");
            try {
                const meta = await invoke<MobileEpubMetadata>("mobile_read_epub_metadata", { epubPath });
                if (meta.title?.trim()) bookTitle = meta.title.trim();
            } catch (_) {
                // metadata is a nice-to-have for the mobile header
            }
            fileTree = await invoke<EpubFileNode[]>("extract_epub", { epubPath });
            refreshFlatFiles(fileTree);
            status = `已解包 ${flatFiles.length} 个文件。`;
        } catch (err) {
            status = "EPUB 解包失败";
            await message(`EPUB 解包失败：${err}`, { title: "编辑 EPUB", kind: "error" });
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
            await loadEpub(cachedPath, file.name);
        } catch (err) {
            status = "导入 EPUB 失败";
            await message(`导入 EPUB 失败：${err}`, { title: "编辑 EPUB", kind: "error" });
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

    function clearThumbnailUrls() {
        thumbnailEpoch += 1;
        for (const url of thumbnailUrls.values()) URL.revokeObjectURL(url);
        thumbnailUrls = new Map();
        thumbnailLoading = new Set();
    }

    async function loadThumbnail(file: FlatFile, epoch = thumbnailEpoch) {
        if (!selectedPath || !isImageFile(file) || thumbnailUrls.has(file.path) || thumbnailLoading.has(file.path)) return;
        const loading = new Set(thumbnailLoading);
        loading.add(file.path);
        thumbnailLoading = loading;
        try {
            const data = await invoke<unknown>("read_epub_file_binary", {
                epubPath: selectedPath,
                filePath: file.path,
            });
            const bytes = bytesFromInvoke(data);
            if (!bytes.length) return;
            const url = URL.createObjectURL(new Blob([bytes], { type: imageMimeFor(file, bytes) }));
            if (epoch !== thumbnailEpoch) {
                URL.revokeObjectURL(url);
                return;
            }
            const next = new Map(thumbnailUrls);
            next.set(file.path, url);
            thumbnailUrls = next;
        } catch {
            // thumbnail preview is best-effort
        } finally {
            const loadingNext = new Set(thumbnailLoading);
            loadingNext.delete(file.path);
            thumbnailLoading = loadingNext;
        }
    }

    function preloadThumbnails(files: FlatFile[], epoch = thumbnailEpoch) {
        for (const file of files) {
            if (isImageFile(file)) {
                void loadThumbnail(file, epoch);
            }
        }
    }

    function refreshFlatFiles(tree: EpubFileNode[], preservedOpenGroups?: Set<string>) {
        const nextFlatFiles = flattenFiles(tree);
        const nextGroups = buildGroups(nextFlatFiles);
        flatFiles = nextFlatFiles;
        openGroups = preservedOpenGroups
            ? new Set(nextGroups.map((group) => group.name).filter((name) => preservedOpenGroups.has(name)))
            : new Set(nextGroups.map((group) => group.name));
        clearThumbnailUrls();
        preloadThumbnails(nextFlatFiles, thumbnailEpoch);
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

    function isFontFile(file: FlatFile) {
        return file.file_type === "font" || /\.(?:ttf|otf|woff2?)$/i.test(file.name) || /\.(?:ttf|otf|woff2?)$/i.test(file.path);
    }

    function isSearchableTextFile(file: FlatFile) {
        return (
            !isImageFile(file) &&
            file.file_type !== "font" &&
            (["html", "css", "xml"].includes(file.file_type) || /\.(?:txt|js|json|opf|ncx)$/i.test(file.name))
        );
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

    function clearFontPreviewFace() {
        if (fontPreviewFace) {
            document.fonts.delete(fontPreviewFace);
            fontPreviewFace = null;
        }
        fontPreviewFamily = "";
    }

    async function reloadTree(preserve = new Set(openGroups), focusGroup?: string) {
        if (!selectedPath) return;
        if (focusGroup) preserve.add(focusGroup);
        fileTree = await invoke<EpubFileNode[]>("extract_epub", { epubPath: selectedPath });
        refreshFlatFiles(fileTree, preserve);
    }

    function hasFileLayerState(state?: unknown) {
        const historyState = state === undefined ? (typeof window !== "undefined" ? window.history.state : null) : state;
        return Boolean(historyState && typeof historyState === "object" && FILE_LAYER_STATE_KEY in (historyState as Record<string, unknown>));
    }

    function pushFileLayerState() {
        if (typeof window === "undefined" || hasFileLayerState()) return;
        const state = window.history.state && typeof window.history.state === "object" ? window.history.state : {};
        window.history.pushState({ ...state, [FILE_LAYER_STATE_KEY]: true }, "");
    }

    function requestCloseFileViaHistory() {
        if (typeof window === "undefined" || !editingFile) return false;
        if (!hasFileLayerState()) return false;
        window.history.back();
        return true;
    }

    function restoreFileLayerState() {
        if (typeof window === "undefined" || hasFileLayerState() || !editingFile) return;
        restoringFileLayer = true;
        pushFileLayerState();
        restoringFileLayer = false;
    }

    function openConfirmSheet(options: Omit<ConfirmSheetState, "open">) {
        if (confirmResolver) {
            confirmResolver("cancel");
            confirmResolver = null;
        }
        confirmSheet = { open: true, ...options };
        return new Promise<ConfirmSheetResult>((resolve) => {
            confirmResolver = resolve;
        });
    }

    function resolveConfirmSheet(result: ConfirmSheetResult) {
        const resolver = confirmResolver;
        confirmResolver = null;
        confirmSheet = {
            open: false,
            title: "",
            message: "",
            confirmLabel: "确定",
            secondaryLabel: "",
            cancelLabel: "取消",
            tone: "primary",
        };
        resolver?.(result);
    }

    function closeFileActions() {
        fileActionTarget = null;
    }

    function openFileActions(file: FlatFile, event: Event) {
        event.stopPropagation();
        fileActionTarget = file;
    }

    function openRenameSheet(file: FlatFile) {
        fileActionTarget = null;
        renameSheet = {
            open: true,
            file,
            value: file.name,
        };
    }

    function closeRenameSheet() {
        renameSheet = {
            open: false,
            file: null,
            value: "",
        };
    }

    function blurInteractiveFocus() {
        const active = document.activeElement;
        if (active instanceof HTMLElement) active.blur();
        epubEditorComponent?.getView()?.contentDOM.blur();
    }

    function toggleReplacePanel() {
        if (!replacePanelOpen) {
            blurInteractiveFocus();
        }
        replacePanelOpen = !replacePanelOpen;
    }

    async function openFile(file: FlatFile, keepReplacePanel = false) {
        if (!selectedPath) return;
        if (!editingFile) pushFileLayerState();
        clearPreviewUrl();
        clearFontPreviewFace();
        editingFile = file;
        editingContent = "";
        editorDirty = false;
        replacePanelOpen = keepReplacePanel ? replacePanelOpen : false;

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

        if (isFontFile(file)) {
            try {
                busy = true;
                const data = await invoke<unknown>("read_epub_file_binary", {
                    epubPath: selectedPath,
                    filePath: file.path,
                });
                const bytes = bytesFromInvoke(data);
                if (!bytes.length) throw new Error("字体数据为空");
                const ext = file.name.split(".").pop()?.toLowerCase();
                const mime = ext === "otf"
                    ? "font/otf"
                    : ext === "woff"
                      ? "font/woff"
                      : ext === "woff2"
                        ? "font/woff2"
                        : "font/ttf";
                previewUrl = URL.createObjectURL(new Blob([bytes], { type: mime }));
                fontPreviewFamily = `mobile-font-preview-${Date.now()}`;
                const face = new FontFace(fontPreviewFamily, `url(${previewUrl})`);
                fontPreviewFace = await face.load();
                document.fonts.add(fontPreviewFace);
                status = `正在预览：${file.path}`;
            } catch (err) {
                status = "读取字体失败";
                await message(`读取字体失败：${err}`, { title: "编辑 EPUB", kind: "error" });
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

    async function openTextFileAtMatch(match: SearchMatch) {
        if (!selectedPath) return;
        clearPreviewUrl();
        editingFile = match.file;
        editingContent = await invoke<string>("read_epub_file_content", {
            epubPath: selectedPath,
            filePath: match.file.path,
        });
        editorDirty = false;
        replacePanelOpen = true;
        status = `正在编辑：${match.file.path}`;
        await tick();
        setTimeout(() => applyMatchToEditor({ from: match.from, to: match.to, filePath: match.file.path }), 0);
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

    async function flushEditingFileIfDirty() {
        if (editingFile && editorDirty) {
            await saveEditingFile();
        }
    }

    async function promptStashEditingFile() {
        if (!editingFile || isImageFile(editingFile) || !editorDirty) return true;

        const result = await openConfirmSheet({
            title: "暂存当前文件修改？",
            message: `${editingFile.name} 还有未保存的内容。暂存后会保留到本次 EPUB 导出，不暂存会丢失当前文件这次修改。`,
            confirmLabel: "暂存并返回",
            secondaryLabel: "不暂存",
            cancelLabel: "继续编辑",
            tone: "primary",
        });

        if (result === "confirm") {
            await saveEditingFile();
            return true;
        }
        if (result === "secondary") {
            editorDirty = false;
            return true;
        }
        return false;
    }

    async function submitRenameFile() {
        const file = renameSheet.file;
        const nextName = renameSheet.value.trim();
        if (!selectedPath || !file) return;
        if (!nextName) {
            await message("文件名不能为空。", { title: "重命名文件", kind: "warning" });
            return;
        }
        if (/[\\/]/.test(nextName)) {
            await message("文件名不能包含斜杠。", { title: "重命名文件", kind: "warning" });
            return;
        }
        if (nextName === file.name) {
            closeRenameSheet();
            return;
        }

        const newPath = file.groupPath ? `${file.groupPath}/${nextName}` : nextName;
        try {
            busy = true;
            if (editingFile?.path === file.path && editorDirty && !isImageFile(editingFile)) {
                await saveEditingFile();
            }
            await invoke("rename_epub_file", {
                epubPath: selectedPath,
                oldPath: file.path,
                newPath,
            });

            if (editingFile?.path === file.path) {
                editingFile = { ...editingFile, name: nextName, path: newPath };
            }
            if (currentMatch?.filePath === file.path) {
                currentMatch = { ...currentMatch, filePath: newPath };
            }
            status = `已重命名：${file.name} → ${nextName}`;
            closeRenameSheet();
            await reloadTree(new Set(openGroups), file.group);
        } catch (err) {
            await message(`重命名失败：${err}`, { title: "重命名文件", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function requestDeleteFile(file: FlatFile) {
        fileActionTarget = null;
        const result = await openConfirmSheet({
            title: "删除这个文件？",
            message: `${file.name} 会从当前 EPUB 缓存副本中移除，保存并导出后才会体现在导出文件里。`,
            confirmLabel: "删除文件",
            secondaryLabel: "",
            cancelLabel: "取消",
            tone: "danger",
        });
        if (result !== "confirm" || !selectedPath) return;

        try {
            busy = true;
            await invoke("delete_epub_file", {
                epubPath: selectedPath,
                filePath: file.path,
            });
            if (editingFile?.path === file.path) {
                resetEditorState();
            }
            if (thumbnailUrls.has(file.path)) {
                const url = thumbnailUrls.get(file.path);
                if (url) URL.revokeObjectURL(url);
                const nextThumbs = new Map(thumbnailUrls);
                nextThumbs.delete(file.path);
                thumbnailUrls = nextThumbs;
            }
            status = `已删除：${file.path}`;
            await reloadTree(new Set(openGroups), file.group);
        } catch (err) {
            await message(`删除文件失败：${err}`, { title: "删除文件", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function activeSearchFiles() {
        if (replaceOnlyText) {
            return flatFiles.filter((file) => file.file_type === "html");
        }
        if (replaceScope === "current") {
            return editingFile && isSearchableTextFile(editingFile) ? [editingFile] : [];
        }
        const candidates = flatFiles.filter(isSearchableTextFile);
        if (replaceScope === "html") return candidates.filter((file) => file.file_type === "html");
        return candidates;
    }

    function normalizeSearchText(content: string) {
        return content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
    }

    function htmlTextSegments(content: string) {
        const segments: { from: number; to: number; text: string }[] = [];
        let inTag = false;
        let start = -1;
        for (let index = 0; index < content.length; index += 1) {
            const char = content[index];
            if (char === "<") {
                if (!inTag && start !== -1 && start < index) {
                    segments.push({ from: start, to: index, text: content.slice(start, index) });
                }
                inTag = true;
                start = -1;
            } else if (char === ">" && inTag) {
                inTag = false;
                start = index + 1;
            } else if (!inTag && start === -1) {
                start = index;
            }
        }
        if (!inTag && start !== -1 && start < content.length) {
            segments.push({ from: start, to: content.length, text: content.slice(start) });
        }
        return segments.filter((segment) => segment.text.trim().length > 0);
    }

    function buildRawMatchList(content: string, offset = 0) {
        const text = normalizeSearchText(content);
        const matches: { from: number; to: number }[] = [];
        if (!findPattern) return matches;

        try {
            if (replaceIsRegex) {
                const regex = new RegExp(findPattern, "g");
                let match: RegExpExecArray | null;
                while ((match = regex.exec(text)) !== null) {
                    if (!match[0].length) {
                        regex.lastIndex += 1;
                        continue;
                    }
                    matches.push({ from: offset + match.index, to: offset + match.index + match[0].length });
                }
            } else {
                let pos = 0;
                while ((pos = text.indexOf(findPattern, pos)) !== -1) {
                    matches.push({ from: offset + pos, to: offset + pos + findPattern.length });
                    pos += findPattern.length || 1;
                }
            }
        } catch {
            return [];
        }

        return matches;
    }

    function buildMatchList(content: string, file?: FlatFile) {
        if (!replaceOnlyText) return buildRawMatchList(content);
        if (file?.file_type !== "html") return [];
        return htmlTextSegments(content).flatMap((segment) => buildRawMatchList(segment.text, segment.from));
    }

    function applyMatchToEditor(match: { from: number; to: number; filePath: string }, focusEditor = false) {
        if (!epubEditorComponent) return;
        const view = epubEditorComponent.getView();
        if (!view) return;
        view.dispatch({
            selection: { anchor: match.from, head: match.to },
            scrollIntoView: true,
        });
        if (focusEditor) {
            view.focus();
        }
    }

    function refreshCurrentMatch(selectFirst = true) {
        if (!editingFile || !findPattern) {
            currentMatch = null;
            currentMatchCount = 0;
            currentMatchIndex = 0;
            return;
        }

        const currentPath = editingFile.path;
        const currentText = epubEditorComponent?.getView()?.state.doc.toString() || editingContent || "";
        const currentMatches = buildMatchList(currentText, editingFile);
        currentMatchCount = currentMatches.length;
        if (!currentMatches.length) {
            currentMatch = null;
            currentMatchIndex = 0;
            return;
        }

        if (selectFirst || !currentMatch) {
            currentMatchIndex = 1;
            currentMatch = { ...currentMatches[0], filePath: currentPath };
            applyMatchToEditor(currentMatch);
        }
    }

    async function collectSearchMatches() {
        const files = activeSearchFiles();
        const matches: SearchMatch[] = [];
        for (const file of files) {
            let content = "";
            if (editingFile?.path === file.path) {
                content = epubEditorComponent?.getView()?.state.doc.toString() || editingContent || "";
            } else {
                content = await invoke<string>("read_epub_file_content", {
                    epubPath: selectedPath,
                    filePath: file.path,
                });
            }
            for (const match of buildMatchList(content, file)) {
                matches.push({ file, ...match });
            }
        }
        return matches;
    }

    function replaceCurrentMatch() {
        if (!currentMatch || !editingFile || editingFile.path !== currentMatch.filePath) {
            replaceStatus = "请先定位到当前匹配项。";
            return;
        }
        const view = epubEditorComponent?.getView();
        if (!view) return;

        const selectedText = view.state.doc.sliceString(currentMatch.from, currentMatch.to);
        const replacement = replaceIsRegex ? selectedText.replace(new RegExp(findPattern), replacePattern) : replacePattern;
        view.dispatch({
            changes: {
                from: currentMatch.from,
                to: currentMatch.to,
                insert: replacement,
            },
        });
        editingContent = view.state.doc.toString();
        editorDirty = true;
        currentMatch = null;
        replaceStatus = "已替换当前匹配。";
        refreshCurrentMatch();
    }

    async function gotoCurrentMatch(direction: "prev" | "next") {
        if (!selectedPath || !editingFile || !findPattern) {
            replaceStatus = "请输入查找内容。";
            return;
        }

        try {
            busy = true;
            await flushEditingFileIfDirty();
            const matches = await collectSearchMatches();
            if (!matches.length) {
                currentMatch = null;
                currentMatchIndex = 0;
                currentMatchCount = 0;
                replaceStatus = "当前范围没有匹配项。";
                return;
            }

            const activeIndex = matches.findIndex((match) =>
                match.file.path === currentMatch?.filePath &&
                match.from === currentMatch?.from &&
                match.to === currentMatch?.to
            );
            const currentFileIndex = activeIndex === -1
                ? matches.findIndex((match) => match.file.path === editingFile?.path)
                : activeIndex;
            const baseIndex = currentFileIndex === -1 ? 0 : currentFileIndex;
            const nextIndex = direction === "next"
                ? (baseIndex + (activeIndex === -1 ? 0 : 1)) % matches.length
                : (baseIndex > 0 ? baseIndex - 1 : matches.length - 1);
            const nextMatch = matches[nextIndex];

            currentMatch = { from: nextMatch.from, to: nextMatch.to, filePath: nextMatch.file.path };
            currentMatchIndex = nextIndex + 1;
            currentMatchCount = matches.length;
            if (editingFile?.path !== nextMatch.file.path) {
                await openTextFileAtMatch(nextMatch);
            } else {
                applyMatchToEditor(currentMatch);
            }
            replaceStatus = `第 ${currentMatchIndex}/${currentMatchCount} 处：${nextMatch.file.name}`;
        } catch (err) {
            replaceStatus = "定位失败";
            await message(`定位失败：${err}`, { title: "查找替换", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function countPlainMatches(content: string, pattern: string) {
        if (!pattern) return 0;
        let count = 0;
        let pos = 0;
        while ((pos = content.indexOf(pattern, pos)) !== -1) {
            count += 1;
            pos += pattern.length || 1;
        }
        return count;
    }

    function replaceInTextSegment(content: string) {
        if (replaceIsRegex) {
            const regex = new RegExp(findPattern, "g");
            const matches = content.match(regex);
            return {
                text: content.replace(regex, replacePattern),
                count: matches?.length ?? 0,
            };
        }
        return {
            text: content.split(findPattern).join(replacePattern),
            count: countPlainMatches(content, findPattern),
        };
    }

    function replaceInText(content: string, file?: FlatFile) {
        if (!replaceOnlyText || file?.file_type !== "html") return replaceInTextSegment(content);

        let nextContent = content;
        let count = 0;
        const segments = htmlTextSegments(content);
        for (let index = segments.length - 1; index >= 0; index -= 1) {
            const segment = segments[index];
            const result = replaceInTextSegment(segment.text);
            if (result.count > 0) {
                count += result.count;
                nextContent = `${nextContent.slice(0, segment.from)}${result.text}${nextContent.slice(segment.to)}`;
            }
        }
        return { text: nextContent, count };
    }

    async function countBatchMatches() {
        if (!selectedPath || !findPattern) {
            replaceStatus = "请输入查找内容。";
            return;
        }

        try {
            busy = true;
            await flushEditingFileIfDirty();
            const files = activeSearchFiles();
            if (!files.length) {
                replaceStatus = "当前范围没有可查找的文本文件。";
                return;
            }

            const count = replaceOnlyText
                ? (await collectSearchMatches()).length
                : await invoke<number>("search_in_files", {
                    epubPath: selectedPath,
                    files: files.map((file) => file.path),
                    pattern: findPattern,
                    isRegex: replaceIsRegex,
                });
            replaceStatus = `共 ${count} 处匹配，范围 ${files.length} 个文件。`;
        } catch (err) {
            replaceStatus = "查找失败";
            await message(`查找失败：${err}`, { title: "批量查找替换", kind: "error" });
        } finally {
            busy = false;
        }
    }

    async function replaceBatchMatches() {
        if (!selectedPath || !findPattern) {
            replaceStatus = "请输入查找内容。";
            return;
        }

        const files = activeSearchFiles();
        if (!files.length) {
            replaceStatus = "当前范围没有可替换的文本文件。";
            return;
        }

        const confirmed = await openConfirmSheet({
            title: "执行批量替换？",
            message: `这会在 ${files.length} 个文件里写入替换结果，继续后会直接覆盖当前缓存中的对应文件。`,
            confirmLabel: "开始替换",
            secondaryLabel: "",
            cancelLabel: "取消",
            tone: "danger",
        });
        if (confirmed !== "confirm") return;

        try {
            busy = true;
            await flushEditingFileIfDirty();
            replaceStatus = "正在替换...";

            const encoder = new TextEncoder();
            const changedFiles: Record<string, number[]> = {};
            let total = 0;

            for (let index = 0; index < files.length; index += 1) {
                const file = files[index];
                replaceStatus = `正在处理 ${index + 1}/${files.length}`;
                let content = await invoke<string>("read_epub_file_content", {
                    epubPath: selectedPath,
                    filePath: file.path,
                });

                const result = replaceInText(content, file);
                if (result.count > 0 && result.text !== content) {
                    total += result.count;
                    changedFiles[file.path] = Array.from(encoder.encode(result.text));
                    if (editingFile?.path === file.path) {
                        editingContent = result.text;
                        epubEditorComponent?.resetDoc(result.text);
                        editorDirty = false;
                    }
                }
            }

            if (Object.keys(changedFiles).length) {
                await invoke("save_epub_files_batch", { epubPath: selectedPath, files: changedFiles });
            }

            replaceStatus = `已替换 ${total} 处，修改 ${Object.keys(changedFiles).length} 个文件。`;
            status = replaceStatus;
            refreshCurrentMatch();
        } catch (err) {
            replaceStatus = "批量替换失败";
            await message(`批量替换失败：${err}`, { title: "批量查找替换", kind: "error" });
        } finally {
            busy = false;
        }
    }

    function resetEditorState() {
        clearPreviewUrl();
        clearFontPreviewFace();
        editingFile = null;
        epubEditorComponent = null;
        editingContent = "";
        editorDirty = false;
        replacePanelOpen = false;
        currentMatch = null;
        currentMatchIndex = 0;
        currentMatchCount = 0;
    }

    async function closeEditor() {
        if (busy) return;
        if (requestCloseFileViaHistory()) return;
        try {
            busy = true;
            const ok = await promptStashEditingFile();
            if (!ok) return;
            resetEditorState();
        } finally {
            busy = false;
        }
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
            await reloadTree(new Set(openGroups), addTargetGroup.name);
            status = `已添加：${targetPath}`;
        } catch (err) {
            status = "添加文件失败";
            await message(`添加文件失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
            addTargetGroup = null;
        }
    }

    async function leaveEditorPage() {
        if (busy) return;
        try {
            if (editingFile) {
                await closeEditor();
                return;
            }
            busy = true;
            await flushEditingFileIfDirty();
            await invoke("save_epub_to_disk", { epubPath: selectedPath });
            await goto(metadataRoute(String(Date.now())));
        } catch (err) {
            await message(`返回编辑元数据页失败：${err}`, { title: "编辑 EPUB", kind: "error" });
        } finally {
            busy = false;
        }
    }

    onMount(() => {
        const selection = readMobileSelection(window.location.search);
        if (selection.path) {
            void loadEpub(selection.path, selection.name);
        }

        const handlePopState = () => {
            if (restoringFileLayer || !editingFile || hasFileLayerState()) return;
            void (async () => {
                if (busy) {
                    restoreFileLayerState();
                    return;
                }
                busy = true;
                try {
                    const ok = await promptStashEditingFile();
                    if (!ok) {
                        restoreFileLayerState();
                        return;
                    }
                    resetEditorState();
                } finally {
                    busy = false;
                }
            })();
        };

        window.addEventListener("popstate", handlePopState);
        return () => {
            window.removeEventListener("popstate", handlePopState);
            clearPreviewUrl();
            clearFontPreviewFace();
            clearThumbnailUrls();
        };
    });
</script>

<svelte:head>
    <title>编辑 EPUB</title>
</svelte:head>

<main class:editing={!!editingFile} class="editor-page">
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".epub" on:change={onFileChange} />
    <input bind:this={addFileInputEl} class="file-input" type="file" on:change={onAddFileChange} />

    <header class="topbar">
        <button class="back-button" type="button" aria-label="返回" on:click={leaveEditorPage}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M15 6L9 12L15 18"></path>
            </svg>
        </button>
        <h1>{bookTitle}</h1>
    </header>

    {#if !selectedPath}
        <section class="empty">
            <button type="button" on:click={openPicker} disabled={busy}>{busy ? "处理中" : "选择 EPUB"}</button>
            <p>{status}</p>
        </section>
    {:else}
        <p class="status">{status}</p>

        <section class="file-list">
            {#each groups as group}
                <div class="group">
                    <div class="group-head">
                        <button class="group-title" type="button" on:click={() => toggleGroup(group.name)}>
                            <span>{group.name}</span>
                        </button>
                        <div class="group-actions">
                            <button class="group-icon group-add" type="button" aria-label={`向 ${group.name} 添加文件`} on:click={() => openAddFilePicker(group)}>
                                <svg viewBox="0 0 24 24" aria-hidden="true">
                                    <path d="M12 5V19"></path>
                                    <path d="M5 12H19"></path>
                                </svg>
                            </button>
                            <button class="group-icon group-toggle" type="button" aria-label={openGroups.has(group.name) ? "折叠" : "展开"} on:click={() => toggleGroup(group.name)}>
                                <svg class:open={openGroups.has(group.name)} viewBox="0 0 24 24" aria-hidden="true">
                                    <path d="M9 6L15 12L9 18"></path>
                                </svg>
                            </button>
                        </div>
                    </div>
                    {#if openGroups.has(group.name)}
                        {#each group.files as file}
                            <div class="file-row">
                                <button class="file-open" type="button" on:click={() => openFile(file)}>
                                    {#if isImageFile(file)}
                                        <span class="thumb-shell">
                                            {#if thumbnailUrls.get(file.path)}
                                                <img class="thumb" src={thumbnailUrls.get(file.path)} alt={file.name} />
                                            {:else}
                                                <span class={iconClass(file)}>{iconFor(file)}</span>
                                            {/if}
                                        </span>
                                    {:else}
                                        <span class={iconClass(file)}>{iconFor(file)}</span>
                                    {/if}
                                    <span class="file-copy">
                                        <strong>{fileStem(file.name)}</strong>
                                        <small>{fileDetail(file)}</small>
                                    </span>
                                </button>
                                <button class="file-more" type="button" aria-label={`${file.name} 更多操作`} on:click={(event) => openFileActions(file, event)}>
                                    <svg viewBox="0 0 24 24" aria-hidden="true">
                                        <circle cx="12" cy="5" r="1.75"></circle>
                                        <circle cx="12" cy="12" r="1.75"></circle>
                                        <circle cx="12" cy="19" r="1.75"></circle>
                                    </svg>
                                </button>
                            </div>
                        {/each}
                    {/if}
                </div>
            {/each}
        </section>

        {#if editingFile}
            <section class={replacePanelOpen ? "mobile-editor replace-open" : "mobile-editor"}>
                <div class:image-head={isImageFile(editingFile) || isFontFile(editingFile)} class="editor-head">
                    {#if !isImageFile(editingFile) && !isFontFile(editingFile)}
                        <div>
                            <strong>{editingFile.name}</strong>
                            <small>{editingFile.path}</small>
                        </div>
                    {:else}
                        <div class="image-head-spacer"></div>
                    {/if}
                    <div class="editor-tools">
                        <button
                            class:active={replacePanelOpen}
                            class="icon-tool"
                            type="button"
                            aria-label={replacePanelOpen ? "收起查找替换" : "打开查找替换"}
                            title={replacePanelOpen ? "收起查找替换" : "打开查找替换"}
                            on:click={toggleReplacePanel}
                            disabled={isImageFile(editingFile)}
                        >
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <circle cx="11" cy="11" r="6.5"></circle>
                                <path d="M16 16L21 21"></path>
                            </svg>
                        </button>
                        <button
                            class="icon-tool save-tool"
                            type="button"
                            aria-label={editorDirty ? "保存文件" : "文件已保存"}
                            title={editorDirty ? "保存文件" : "文件已保存"}
                            on:click={saveEditingFile}
                            disabled={busy || !editorDirty || isImageFile(editingFile)}
                        >
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M5 4H16L19 7V20H5Z"></path>
                                <path d="M8 4V10H15V4"></path>
                                <path d="M9 20V14H15V20"></path>
                            </svg>
                        </button>
                        <button
                            class="icon-tool close-tool"
                            type="button"
                            aria-label="关闭文件预览"
                            title="关闭文件预览"
                            on:click={closeEditor}
                        >
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path d="M6 6L18 18"></path>
                                <path d="M18 6L6 18"></path>
                            </svg>
                        </button>
                    </div>
                </div>
                {#if isImageFile(editingFile)}
                    <div class="image-preview">
                        {#if previewUrl}
                            <img src={previewUrl} alt={editingFile.name} />
                        {:else}
                            <span>图片加载中</span>
                        {/if}
                    </div>
                {:else if isFontFile(editingFile)}
                    <div class="font-preview">
                        <div class="font-preview-card" style:font-family={fontPreviewFamily || "inherit"}>
                            <p class="font-sample-cn">字体预览 ABC abc 12345</p>
                            <p class="font-sample-cn">古龙世界里的吃瓜剑客</p>
                            <p class="font-sample-cn">风起云涌，江湖夜雨十年灯。</p>
                        </div>
                    </div>
                {:else}
                    <div class="code-editor-wrap">
                        <EpubCodeEditor
                            bind:this={epubEditorComponent}
                            doc={editingContent}
                            language={languageFor(editingFile)}
                            onChange={(value) => {
                                editingContent = value;
                                editorDirty = true;
                            }}
                            onSave={saveEditingFile}
                        />
                    </div>
                    {#if replacePanelOpen}
                        <section class="replace-panel">
                            <div class="replace-title">
                                <strong>查找替换</strong>
                                <small>{replaceOnlyText ? `${activeSearchFiles().length} 个 HTML` : replaceScope === "current" ? "当前文件" : `${activeSearchFiles().length} 个文件`}</small>
                            </div>
                            <div class="replace-grid">
                                <label>
                                    <span>查找</span>
                                    <input bind:value={findPattern} autocomplete="off" placeholder="输入查找内容" />
                                </label>
                                <label>
                                    <span>替换为</span>
                                    <input bind:value={replacePattern} autocomplete="off" placeholder="留空则删除" />
                                </label>
                            </div>
                            <div class="replace-actions">
                                <button type="button" on:click={() => gotoCurrentMatch("prev")} disabled={busy || !findPattern}>上一个</button>
                                <button type="button" on:click={countBatchMatches} disabled={busy || !findPattern}>计数</button>
                                <button type="button" on:click={() => gotoCurrentMatch("next")} disabled={busy || !findPattern}>下一个</button>
                                <button type="button" on:click={replaceCurrentMatch} disabled={busy || !findPattern}>替换当前</button>
                                <button type="button" on:click={replaceBatchMatches} disabled={busy || !findPattern}>替换全部</button>
                            </div>
                            <div class="replace-options">
                                <select bind:value={replaceScope} aria-label="查找范围" disabled={replaceOnlyText}>
                                    <option value="current">当前文件</option>
                                    <option value="html">HTML章节</option>
                                    <option value="all">所有文本</option>
                                </select>
                                <label class="replace-check">
                                    <input type="checkbox" bind:checked={replaceIsRegex} />
                                    <span>正则</span>
                                </label>
                                <label class="replace-check">
                                    <input type="checkbox" bind:checked={replaceOnlyText} />
                                    <span>仅文本</span>
                                </label>
                            </div>
                            <p>{replaceStatus || (currentMatchCount ? `第 ${currentMatchIndex}/${currentMatchCount} 处` : "在当前文件内定位，或按范围批量替换。")}</p>
                        </section>
                    {/if}
                {/if}
            </section>
        {/if}
    {/if}

    {#if confirmSheet.open}
        <div class="confirm-sheet-backdrop" role="presentation" on:click={() => resolveConfirmSheet("cancel")}></div>
        <div
            class="confirm-sheet"
            role="dialog"
            aria-modal="true"
            aria-labelledby="mobile-confirm-title"
            aria-describedby="mobile-confirm-message"
        >
            <div class="confirm-sheet-copy">
                <strong id="mobile-confirm-title">{confirmSheet.title}</strong>
                <p id="mobile-confirm-message">{confirmSheet.message}</p>
            </div>
            <div class="confirm-sheet-actions">
                <button class="sheet-cancel" type="button" on:click={() => resolveConfirmSheet("cancel")}>
                    {confirmSheet.cancelLabel}
                </button>
                {#if confirmSheet.secondaryLabel}
                    <button class="sheet-secondary" type="button" on:click={() => resolveConfirmSheet("secondary")}>
                        {confirmSheet.secondaryLabel}
                    </button>
                {/if}
                <button
                    class:danger={confirmSheet.tone === "danger"}
                    class="sheet-confirm"
                    type="button"
                    on:click={() => resolveConfirmSheet("confirm")}
                >
                    {confirmSheet.confirmLabel}
                </button>
            </div>
        </div>
    {/if}

    {#if fileActionTarget}
        <div class="confirm-sheet-backdrop" role="presentation" on:click={closeFileActions}></div>
        <div class="action-sheet" role="dialog" aria-modal="true" aria-labelledby="file-actions-title">
            <div class="confirm-sheet-copy">
                <strong id="file-actions-title">{fileActionTarget.name}</strong>
                <p>{fileActionTarget.path}</p>
            </div>
            <div class="action-sheet-actions">
                <button class="sheet-confirm" type="button" on:click={() => fileActionTarget && openRenameSheet(fileActionTarget)}>
                    重命名
                </button>
                <button class="sheet-secondary danger" type="button" on:click={() => fileActionTarget && requestDeleteFile(fileActionTarget)}>
                    删除文件
                </button>
                <button class="sheet-cancel" type="button" on:click={closeFileActions}>
                    取消
                </button>
            </div>
        </div>
    {/if}

    {#if renameSheet.open}
        <div class="confirm-sheet-backdrop" role="presentation" on:click={closeRenameSheet}></div>
        <div class="rename-sheet" role="dialog" aria-modal="true" aria-labelledby="rename-file-title">
            <div class="confirm-sheet-copy">
                <strong id="rename-file-title">重命名文件</strong>
                <p>{renameSheet.file?.path}</p>
            </div>
            <label class="rename-field">
                <span>文件名</span>
                <input bind:value={renameSheet.value} autocomplete="off" />
            </label>
            <div class="confirm-sheet-actions">
                <button class="sheet-cancel" type="button" on:click={closeRenameSheet}>取消</button>
                <button class="sheet-confirm" type="button" on:click={submitRenameFile} disabled={busy}>保存名称</button>
            </div>
        </div>
    {/if}
</main>

<style>
    :global(html),
    :global(body) {
        background: #f2f3f8;
    }

    .editor-page {
        height: 100dvh;
        min-height: 100dvh;
        box-sizing: border-box;
        padding: max(10px, env(safe-area-inset-top)) 0 max(44px, env(safe-area-inset-bottom));
        overflow-y: auto;
        background: #f2f3f8;
        color: #151923;
    }

    .editor-page.editing {
        overflow: hidden;
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

    .topbar .back-button {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: inherit;
        padding: 0;
    }

    .topbar .back-button svg {
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

    button {
        font: inherit;
    }

    .empty button {
        min-height: 36px;
        border: 0;
        border-radius: 8px;
        background: #1f7a5a;
        color: #fff;
        font-weight: 900;
    }

    button:disabled {
        opacity: 0.6;
    }

    .status {
        display: block;
        margin: 6px 14px;
        color: #747986;
        font-size: 12px;
        line-height: 1.5;
    }

    .replace-panel {
        display: grid;
        gap: 7px;
        margin: 8px 12px max(12px, calc(env(safe-area-inset-bottom) + 6px));
        border: 0;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.94);
        box-shadow: 0 10px 26px rgba(30, 38, 52, 0.08);
        padding: 12px 14px;
    }

    .replace-title {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
    }

    .replace-title strong {
        font-size: 14px;
        line-height: 1.25;
    }

    .replace-title small,
    .replace-panel p {
        margin: 0;
        color: #747986;
        font-size: 11px;
        line-height: 1.4;
    }

    .replace-panel label {
        display: grid;
        grid-template-columns: 48px minmax(0, 1fr);
        align-items: center;
        gap: 7px;
    }

    .replace-grid {
        display: grid;
        gap: 6px;
    }

    .replace-panel label span {
        color: #626a78;
        font-size: 11px;
        font-weight: 800;
    }

    .replace-panel input,
    .replace-panel select {
        width: 100%;
        min-width: 0;
        box-sizing: border-box;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        background: #fff;
        color: inherit;
        min-height: 34px;
        padding: 7px 8px;
        font: inherit;
        font-size: 12px;
    }

    .replace-options {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto auto;
        gap: 7px;
        align-items: center;
    }

    .replace-actions {
        display: grid;
        grid-template-columns: repeat(6, minmax(0, 1fr));
        gap: 7px;
        align-items: center;
    }

    .replace-panel .replace-check {
        min-height: 34px;
        grid-auto-flow: column;
        grid-template-columns: 16px auto;
        align-items: center;
        justify-content: start;
    }

    .replace-panel .replace-check input {
        width: 16px;
        height: 16px;
        padding: 0;
    }

    .replace-actions button {
        min-width: 0;
        min-height: 34px;
        border: 0;
        border-radius: 8px;
        background: #e6eee9;
        color: #1f7a5a;
        font-size: 12px;
        font-weight: 900;
        white-space: nowrap;
    }

    .replace-actions button:nth-child(4),
    .replace-actions button:last-child {
        background: #1f7a5a;
        color: #fff;
    }

    .replace-actions button:nth-child(1),
    .replace-actions button:nth-child(2),
    .replace-actions button:nth-child(3) {
        grid-column: span 2;
    }

    .replace-actions button:nth-child(4),
    .replace-actions button:nth-child(5) {
        grid-column: span 3;
    }

    .file-list {
        display: grid;
        gap: 12px;
        margin: 10px 12px 0;
        padding-bottom: 12px;
    }

    .group {
        overflow: hidden;
        border: 0;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.96);
        box-shadow: 0 10px 24px rgba(29, 36, 49, 0.06);
    }

    .group-head {
        width: 100%;
        min-height: 38px;
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 6px;
        box-sizing: border-box;
        padding: 8px 12px 4px;
        background: transparent;
        color: #8a8d96;
    }

    .group-title {
        min-width: 0;
        width: 100%;
        min-height: 26px;
        display: grid;
        align-items: center;
        border: 0;
        border-radius: 10px;
        background: transparent;
        color: inherit;
        text-align: left;
        padding: 0;
    }

    .group-title span {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-family: Georgia, "Times New Roman", serif;
        font-size: 11px;
        line-height: 1.1;
        letter-spacing: 0.04em;
    }

    .group-actions {
        display: grid;
        grid-auto-flow: column;
        gap: 0;
        align-items: center;
    }

    .group-icon {
        width: 24px;
        height: 24px;
        min-height: 24px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 8px;
        background: transparent;
        color: #a1a3ab;
        box-shadow: none;
    }

    .group-icon:hover {
        background: rgba(31, 122, 90, 0.08);
        color: #6d7685;
    }

    .group-icon svg,
    .file-more svg {
        width: 16px;
        height: 16px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1.9;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .group-toggle svg {
        transition: transform 0.16s ease;
    }

    .group-toggle svg.open {
        transform: rotate(90deg);
    }

    .file-row {
        width: 100%;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 34px;
        align-items: center;
        gap: 4px;
        box-sizing: border-box;
        border-top: 0;
        padding: 0 8px 0 12px;
    }

    .file-row + .file-row {
        position: relative;
    }

    .file-row + .file-row::before {
        content: "";
        position: absolute;
        left: 52px;
        right: 12px;
        top: 0;
        height: 1px;
        background: rgba(31, 39, 55, 0.06);
    }

    .file-open {
        width: 100%;
        min-height: 58px;
        display: grid;
        grid-template-columns: 38px minmax(0, 1fr);
        align-items: center;
        gap: 10px;
        border: 0;
        background: transparent;
        color: inherit;
        padding: 10px 0 10px 30px;
        text-align: left;
    }

    .icon,
    .thumb-shell {
        width: 34px;
        height: 34px;
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

    .thumb-shell {
        overflow: hidden;
        background: #e5edf0;
    }

    .thumb {
        width: 100%;
        height: 100%;
        display: block;
        object-fit: cover;
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

    .file-more {
        width: 30px;
        height: 30px;
        min-height: 30px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 10px;
        background: transparent;
        color: #848894;
        padding: 0;
        box-shadow: none;
    }

    .file-more:hover {
        background: rgba(31, 122, 90, 0.08);
        color: #5f6673;
    }

    .file-more svg {
        fill: currentColor;
        stroke: none;
    }

    .mobile-editor {
        position: fixed;
        inset: 0;
        z-index: 20;
        display: grid;
        grid-template-rows: auto minmax(0, 1fr) auto;
        gap: 0;
        box-sizing: border-box;
        height: 100dvh;
        min-height: 100dvh;
        overflow: hidden;
        padding: 0;
        background:
            radial-gradient(circle at top, rgba(255, 255, 255, 0.88), rgba(242, 243, 248, 0.96) 40%),
            #f2f3f8;
    }

    .confirm-sheet-backdrop {
        position: fixed;
        inset: 0;
        z-index: 30;
        background: rgba(20, 25, 35, 0.34);
        backdrop-filter: blur(10px);
    }

    .confirm-sheet {
        position: fixed;
        left: 14px;
        right: 14px;
        top: 50%;
        transform: translateY(-50%);
        max-width: 420px;
        margin: 0 auto;
        z-index: 31;
        display: grid;
        gap: 14px;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.97);
        box-shadow: 0 18px 40px rgba(25, 31, 43, 0.16);
        padding: 16px;
    }

    .action-sheet,
    .rename-sheet {
        position: fixed;
        left: 14px;
        right: 14px;
        top: 50%;
        transform: translateY(-50%);
        max-width: 420px;
        margin: 0 auto;
        z-index: 31;
        display: grid;
        gap: 14px;
        border-radius: 16px;
        background: rgba(255, 255, 255, 0.98);
        box-shadow: 0 18px 40px rgba(25, 31, 43, 0.16);
        padding: 16px;
    }

    .confirm-sheet-copy {
        display: grid;
        gap: 7px;
    }

    .confirm-sheet-copy strong {
        font-size: 16px;
        line-height: 1.25;
    }

    .confirm-sheet-copy p {
        margin: 0;
        color: #666f7d;
        font-size: 13px;
        line-height: 1.55;
    }

    .confirm-sheet-actions {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 8px;
    }

    .action-sheet-actions {
        display: grid;
        gap: 8px;
    }

    .confirm-sheet-actions button,
    .action-sheet-actions button {
        min-height: 38px;
        min-width: 0;
        border: 0;
        border-radius: 10px;
        font-size: 13px;
        font-weight: 900;
        box-shadow: none;
    }

    .sheet-cancel {
        background: #eef1f6;
        color: #4f5867;
    }

    .sheet-secondary {
        background: #f4ecee;
        color: #9b3d4f;
    }

    .sheet-secondary.danger {
        background: #f4ecee;
        color: #9b3d4f;
    }

    .sheet-confirm {
        background: #1f7a5a;
        color: #fff;
    }

    .sheet-confirm.danger {
        background: #cf5e50;
    }

    .rename-field {
        display: grid;
        gap: 6px;
    }

    .rename-field span {
        color: #626a78;
        font-size: 12px;
        font-weight: 800;
    }

    .rename-field input {
        width: 100%;
        box-sizing: border-box;
        border: 1px solid rgba(23, 27, 36, 0.12);
        border-radius: 8px;
        background: #fff;
        color: inherit;
        min-height: 38px;
        padding: 8px 10px;
        font: inherit;
    }

    .font-preview {
        display: grid;
        gap: 8px;
        align-content: start;
        padding: 8px 12px 16px;
    }

    .font-preview-card {
        display: grid;
        gap: 10px;
        border: 0;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.96);
        box-shadow: 0 12px 28px rgba(29, 36, 49, 0.08);
        padding: 18px;
    }

    .font-preview-card p {
        margin: 0;
        color: #1a1f29;
        line-height: 1.45;
    }

    .font-sample-cn:first-child {
        font-size: 14px;
        opacity: 0.82;
    }

    .font-sample-cn:nth-child(2) {
        font-size: 28px;
    }

    .font-sample-cn:last-child {
        font-size: 18px;
        opacity: 0.9;
    }

    .editor-head {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 10px;
        align-items: center;
        padding: max(10px, env(safe-area-inset-top)) 12px 8px;
        background: transparent;
        position: sticky;
        top: 0;
        z-index: 2;
    }

    .editor-head.image-head {
        padding-bottom: 0;
        background: transparent;
    }

    .editor-head div {
        min-width: 0;
        display: grid;
        gap: 2px;
    }

    .image-head-spacer {
        min-height: 1px;
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

    .editor-tools {
        display: grid;
        grid-auto-flow: column;
        gap: 8px;
        align-items: center;
    }

    .icon-tool {
        width: 34px;
        min-height: 34px;
        height: 34px;
        padding: 0;
        border: 0;
        border-radius: 8px;
        background: #e6eee9;
        color: #1f7a5a;
        display: grid;
        place-items: center;
    }

    .icon-tool svg {
        width: 18px;
        height: 18px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1.8;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .icon-tool.active,
    .icon-tool:hover:enabled {
        background: #d9ebf6;
        color: #1677b8;
    }

    .save-tool {
        background: #1f7a5a;
        color: #fff;
    }

    .save-tool:disabled {
        background: #d6e4dc;
        color: #6b8d7f;
    }

    .close-tool {
        background: #f1e9eb;
        color: #9b3d4f;
    }

    .image-preview {
        min-height: 260px;
        display: grid;
        place-items: center;
        overflow: auto;
        margin: 0 12px 12px;
        border: 0;
        border-radius: 18px;
        background: rgba(255, 255, 255, 0.72);
        box-shadow: inset 0 0 0 1px rgba(214, 219, 228, 0.6);
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
        height: 100%;
        box-sizing: border-box;
        margin: 0 12px;
        border: 0;
        border-radius: 18px;
        overflow: hidden;
        background: #fff;
        box-shadow:
            0 10px 28px rgba(29, 36, 49, 0.08),
            inset 0 0 0 1px rgba(214, 219, 228, 0.7);
    }

    .mobile-editor.replace-open .code-editor-wrap {
        height: 100%;
    }

    :global(.mobile-editor .cm-editor) {
        background: #fff;
    }

    :global(.mobile-editor .cm-scroller),
    :global(.mobile-editor .cm-content) {
        font-size: 12px;
        line-height: 1.55;
    }

    @media (min-width: 720px) {
        .editor-page {
            max-width: 760px;
            margin: 0 auto;
        }
    }
</style>
