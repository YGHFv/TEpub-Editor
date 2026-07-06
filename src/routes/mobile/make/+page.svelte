<script lang="ts">
    import { onMount } from "svelte";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import { platform } from "$lib/platform";
    import {
        cacheBrowserFileStable,
        exportEpubPath,
        offerSystemExport,
        readMobileSelection,
        safeFileName,
        selectionName,
    } from "$lib/mobileFlow";

    interface RegexRule {
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

    const ruleLevelOptions = [
        { value: "1", label: "卷 / 元信息" },
        { value: "3", label: "章节" },
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

    let rules: RegexRule[] = [
        { level: 1, pattern: DEFAULT_META_VOLUME_REGEX },
        { level: 1, pattern: DEFAULT_VOLUME_REGEX },
        { level: 3, pattern: DEFAULT_META_BODY_REGEX },
        { level: 3, pattern: DEFAULT_CHAPTER_REGEX },
    ];

    let fileInputEl: HTMLInputElement | null = null;
    let coverInputEl: HTMLInputElement | null = null;
    let selectedPath = "";
    let selectedName = "";
    let content = "";
    let title = "";
    let author = "";
    let coverPath = "";
    let coverName = "";
    let coverPreviewUrl = "";
    let uuid = crypto.randomUUID?.() ?? "";
    let uuidAuto = true;
    let desktopMode = false;
    let backHref = "/mobile";
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
    let regexOpen = true;
    let tocOpen = true;
    let checkOpen = true;
    let reorderOpen = false;
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
    $: if (tocItems.length) {
        reorderPreviewRows = buildReorderPreviewRows(tocItems);
    }

    function resetResult() {
        makeResult = null;
        webEpubBytes = null;
        exportPath = "";
    }

    function openPicker() {
        fileInputEl?.click();
    }

    function openCoverPicker() {
        coverInputEl?.click();
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
            title = selectionName(selectedName).replace(/\.[^.]+$/, "");
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
                title = selectionName(file.name).replace(/\.[^.]+$/, "");
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
            coverPath = platform.isWeb ? `web-local:${file.name}` : await cacheBrowserFileStable(file, "cover");
            coverName = file.name;
            if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
            coverPreviewUrl = URL.createObjectURL(file);
            resetResult();
        } catch (err) {
            await platform.message(`封面导入失败：${err}`, { title: "制作 EPUB", kind: "error" });
        }
    }

    async function previewToc() {
        if (!content.trim()) return;
        resetResult();
        try {
            busy = true;
            chapters = platform.isWeb
                ? scanChaptersInBrowser(content, rules)
                : await platform.invoke<RawChapter[]>("mobile_scan_chapters", { content, rules });
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

    async function makeEpub() {
        if (!selectedPath || !content.trim()) return;
        try {
            busy = true;
            if (platform.isWeb) {
                const outputTitle = title.trim() || selectedName.replace(/\.[^.]+$/, "") || "book";
                webEpubBytes = buildSimpleEpubBytes(outputTitle, author.trim(), content, chapters);
                makeResult = {
                    output_path: `web-local:${safeFileName(outputTitle, "epub")}`,
                    title: outputTitle,
                    chapter_count: Math.max(1, chapters.length),
                    word_count: countWords(content),
                };
                status = `已生成《${makeResult.title}》，${makeResult.chapter_count} 个目录项，约 ${makeResult.word_count} 字。`;
                return;
            }
            makeResult = await platform.invoke<MobileMakeEpubResult>("mobile_make_epub", {
                sourcePath: selectedPath,
                title: title.trim() || selectedName.replace(/\.[^.]+$/, ""),
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
        rules = [...rules, { level, pattern: "" }];
    }

    function removeRule(index: number) {
        rules = rules.filter((_, i) => i !== index);
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

    function revealTocItem(id: string) {
        const next = new Set(expandedIds);
        let parentId = tocItems.find((item) => item.id === id)?.parentId;
        while (parentId) {
            next.add(parentId);
            parentId = tocItems.find((item) => item.id === parentId)?.parentId;
        }
        expandedIds = next;
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
                title: titleText,
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
            .replace(/"/g, "&quot;");
    }

    function chapterSections(text: string, list: RawChapter[]) {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const headings = list.filter((item) => itemKind(item.title, item.level, item.is_meta) !== "meta");
        if (headings.length === 0) {
            return [{ title: title.trim() || selectedName.replace(/\.[^.]+$/, "") || "正文", lines }];
        }
        return headings.map((heading, index) => {
            const start = Math.max(0, heading.line_number - 1);
            const end = index + 1 < headings.length ? Math.max(start + 1, headings[index + 1].line_number - 1) : lines.length;
            return {
                title: heading.title,
                lines: lines.slice(start + 1, end),
            };
        });
    }

    function buildChapterXhtml(chapterTitle: string, bodyLines: string[]) {
        const blocks = bodyLines
            .join("\n")
            .split(/\n\s*\n/)
            .map((part) => part.trim())
            .filter(Boolean);
        const body = blocks.length
            ? blocks.map((block) => `<p>${escapeXml(block).replace(/\n/g, "<br/>")}</p>`).join("\n")
            : "<p></p>";
        return `<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" lang="zh-CN">
<head><title>${escapeXml(chapterTitle)}</title><link rel="stylesheet" href="../Styles/style.css"/></head>
<body><h1>${escapeXml(chapterTitle)}</h1>${body}</body>
</html>`;
    }

    function buildSimpleEpubBytes(bookTitle: string, bookAuthor: string, text: string, list: RawChapter[]) {
        const sections = chapterSections(text, list);
        const files: { name: string; data: Uint8Array }[] = [];
        const encoder = new TextEncoder();
        const addText = (name: string, value: string) => files.push({ name, data: encoder.encode(value) });
        files.push({ name: "mimetype", data: encoder.encode("application/epub+zip") });
        addText(
            "META-INF/container.xml",
            `<?xml version="1.0" encoding="utf-8"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>`,
        );
        addText("OEBPS/Styles/style.css", "body{font-family:serif;line-height:1.75;}h1{text-align:center;}p{text-indent:2em;margin:0 0 .8em;}");
        sections.forEach((section, index) => {
            addText(`OEBPS/Text/chapter${String(index + 1).padStart(3, "0")}.xhtml`, buildChapterXhtml(section.title, section.lines));
        });
        const navItems = sections
            .map((section, index) => `<li><a href="Text/chapter${String(index + 1).padStart(3, "0")}.xhtml">${escapeXml(section.title)}</a></li>`)
            .join("");
        addText(
            "OEBPS/nav.xhtml",
            `<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="zh-CN"><head><title>目录</title></head><body><nav epub:type="toc" id="toc"><h1>目录</h1><ol>${navItems}</ol></nav></body></html>`,
        );
        const manifestItems = sections
            .map((_, index) => `<item id="chapter${index + 1}" href="Text/chapter${String(index + 1).padStart(3, "0")}.xhtml" media-type="application/xhtml+xml"/>`)
            .join("");
        const spineItems = sections.map((_, index) => `<itemref idref="chapter${index + 1}"/>`).join("");
        addText(
            "OEBPS/content.opf",
            `<?xml version="1.0" encoding="utf-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="bookid">urn:uuid:${escapeXml(uuid || crypto.randomUUID?.() || Date.now().toString())}</dc:identifier><dc:title>${escapeXml(bookTitle)}</dc:title><dc:creator>${escapeXml(bookAuthor || "未知作者")}</dc:creator><dc:language>zh-CN</dc:language></metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="style" href="Styles/style.css" media-type="text/css"/>${manifestItems}</manifest><spine>${spineItems}</spine></package>`,
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

    onMount(() => {
        const params = new URLSearchParams(window.location.search);
        desktopMode = params.get("view") === "desktop" || (platform.isWeb && !isCompactDevice());
        backHref = desktopMode ? "/" : "/mobile";
        const selection = readMobileSelection(window.location.search);
        if (selection.path) {
            void loadSource(selection.path, selection.name);
        }
    });
</script>

<svelte:head>
    <title>TEpub-Editor</title>
</svelte:head>

<main class="page" class:desktop-page={desktopMode}>
    <input bind:this={fileInputEl} class="file-input" type="file" accept=".txt,.md,.html,.htm" on:change={onFileChange} />
    <input bind:this={coverInputEl} class="file-input" type="file" accept="image/*" on:change={onCoverChange} />

    <header class="topbar">
        <a href={backHref} aria-label="返回">
            <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M15 6L9 12L15 18"></path>
            </svg>
        </a>
        <h1>制作 EPUB</h1>
        <button class="topbar-action" type="button" on:click={openPicker} disabled={busy}>选择文本</button>
    </header>

    {#if !selectedPath}
        <section class="empty-panel">
            <div>
                <h2>选择文本文件开始制作</h2>
                <p>支持 TXT、Markdown 和 HTML 文件，导入后可扫描目录、调整规则并生成 EPUB。</p>
            </div>
            <button type="button" on:click={openPicker} disabled={busy}>选择文本文件</button>
        </section>
    {/if}

    {#if selectedPath}
        <section class="meta">
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
        </section>

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
                <div class="regex-actions">
                    <button type="button" on:click={previewToc} disabled={busy}>重新扫描</button>
                    <button type="button" on:click={() => runTocCheck()} disabled={busy}>重新检查</button>
                </div>
                {#each rules as rule, index}
                    <div class="rule-row">
                        <CustomSelect
                            className="make-select compact-select"
                            value={String(rule.level)}
                            options={ruleLevelOptions}
                            on:change={(e) => (rules = rules.map((item, i) => (i === index ? { ...item, level: Number(e.detail) } : item)))}
                        />
                        <input bind:value={rule.pattern} autocomplete="off" />
                        <button type="button" on:click={() => removeRule(index)} aria-label="删除正则">×</button>
                    </div>
                {/each}
                <div class="rule-actions">
                    <button type="button" on:click={() => addRule(1)}>添加卷规则</button>
                    <button type="button" on:click={() => addRule(3)}>添加章节规则</button>
                </div>
            {/if}
        </section>

        <section class="toc-panel">
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
                <div class="status">{status}</div>
                {#if visibleToc.length}
                    <div class="toc-list">
                        {#each visibleToc as item}
                            <div
                                class="toc-row"
                                class:volume={item.kind === "volume"}
                                class:sequence-error={invalidSequenceIds.has(item.id)}
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

        <section class="check-panel">
            <button class="check-head" type="button" on:click={() => (checkOpen = !checkOpen)}>
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

        <section class="reorder-panel">
            <button class="check-head" type="button" on:click={() => (reorderOpen = !reorderOpen)}>
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

        <section class="bottom-actions">
            <button type="button" on:click={makeEpub} disabled={busy}>生成 EPUB</button>
            {#if makeResult}
                <button type="button" on:click={exportMadeEpub} disabled={busy}>导出 EPUB</button>
            {/if}
            {#if exportPath}<code>{exportPath}</code>{/if}
        </section>
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
        grid-template-columns: 38px minmax(0, 1fr) auto;
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

    .topbar-action {
        min-height: 34px;
        padding: 0 14px;
    }

    .meta,
    .regex-panel,
    .toc-panel,
    .check-panel,
    .reorder-panel,
    .empty-panel,
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
        padding: 6px 9px;
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

    .regex-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-bottom: 8px;
    }

    .regex-actions button {
        background: #e8f2f8;
        color: #1677b8;
    }

    .rule-row {
        display: grid;
        grid-template-columns: 86px minmax(0, 1fr) 34px;
        gap: 8px;
        margin-top: 8px;
    }

    .rule-row button {
        min-height: 34px;
        background: #f2e7e7;
        color: #9b3d4f;
        font-size: 18px;
    }

    .rule-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-top: 10px;
    }

    .toc-list {
        display: grid;
        margin-top: 10px;
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

    .toc-row.sequence-error strong {
        color: #b33636;
    }

    .toc-row.sequence-error {
        background: #fff4f2;
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
    }

    .check-row {
        min-height: 44px;
        display: grid;
        grid-template-columns: 70px minmax(0, 1fr);
        grid-template-areas:
            "type title"
            "type meta";
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
            display: grid;
            grid-template-columns: minmax(320px, 1fr) minmax(0, 2fr);
            grid-auto-rows: auto;
            align-items: start;
            gap: 12px;
            padding: 14px 0 28px;
        }

        .desktop-page .empty-panel,
        .desktop-page .bottom-actions {
            grid-column: 1 / -1;
        }

        .desktop-page .topbar {
            display: none;
        }

        .desktop-page .meta,
        .desktop-page .regex-panel,
        .desktop-page .toc-panel,
        .desktop-page .check-panel,
        .desktop-page .reorder-panel,
        .desktop-page .bottom-actions {
            margin-top: 0;
        }

        .desktop-page .meta {
            grid-column: 1;
            grid-row: 1;
            grid-template-columns: minmax(0, 1fr) 116px;
            grid-template-areas:
                "title cover"
                "author cover"
                "uuid cover";
            align-items: stretch;
            gap: 8px 10px;
            padding: 10px 12px;
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

        .desktop-page .toc-panel {
            grid-column: 1;
            grid-row: 3;
        }

        .desktop-page .regex-panel {
            grid-column: 1;
            grid-row: 2;
        }

        .desktop-page .check-panel {
            grid-column: 2;
            grid-row: 1 / span 2;
        }

        .desktop-page .reorder-panel {
            grid-column: 2;
            grid-row: 3;
        }

        .desktop-page .bottom-actions {
            grid-row: 4;
            grid-template-columns: auto auto minmax(0, 1fr);
            align-items: center;
        }

        .desktop-page .bottom-actions button {
            min-width: 128px;
            padding: 0 18px;
        }

        .desktop-page .toc-list {
            max-height: calc(100vh - 260px);
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
            max-height: 220px;
            overflow: auto;
        }

        .desktop-page .reorder-head,
        .desktop-page .reorder-row {
            min-height: 30px;
            padding: 0 8px;
        }

        .desktop-page .check-list {
            max-height: 180px;
            overflow: auto;
        }

        .desktop-page .check-row {
            grid-template-columns: 86px minmax(0, 1fr) auto;
            grid-template-areas: "type title meta";
            min-height: 40px;
            align-items: center;
        }

        .desktop-page .check-row small {
            justify-self: end;
            white-space: nowrap;
        }
    }
</style>
