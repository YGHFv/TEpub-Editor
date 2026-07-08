<script lang="ts">
  import { goto } from "$app/navigation";
  import { base } from "$app/paths";
  import { onMount, tick } from "svelte";
  import {
    DEFAULT_TOC_REGEX_RULES,
    loadAppSettings,
    saveAppSettings,
    type TocRegexRule,
  } from "$lib/appSettings";
  import { platform } from "$lib/platform";
  import CustomSelect from "$lib/CustomSelect.svelte";
  import TxtCodeEditor from "$lib/TxtCodeEditor.svelte";
  import {
    applyBuiltinRegexPreview,
    applyChineseConvertPreview,
    buildBuiltinRegexPreview,
    buildChineseConvertPreview,
    type ProofBuiltinRuleId,
    type ProofConvertDirection,
    type ProofConvertPreviewRow,
    type ProofRegexPreviewRow,
    type ProofTocNode,
  } from "$lib/textProofing";

  type EncodingOption = "utf-8" | "gb18030" | "big5";
  type SearchMode = "plain" | "regex";
  type TocKind = "volume" | "chapter" | "meta";
  type ProofPreviewMode = "builtin" | "convert";
  type ReorderScope = "all" | "volumes" | "chapters" | "regex";
  type NumberStyle = "arabic" | "chinese";

  function appPath(path: string) {
    return `${base}${path.startsWith("/") ? path : `/${path}`}`;
  }

  type SearchMatch = {
    start: number;
    end: number;
  };

  type RawChapter = {
    title: string;
    line_number: number;
    level: number;
    is_meta: boolean;
    word_count: number;
  };

  type TocItem = RawChapter & {
    id: string;
    kind: TocKind;
    depth: number;
    parentId: string;
    volumeKey: string;
    hasChildren: boolean;
  };

  type TocCheckIssue = {
    id: string;
    title: string;
    line: number;
    msg: string;
  };

  type ReorderPreviewRow = {
    id: string;
    line: number;
    kind: TocKind;
    volumeKey: string;
    original: string;
    replacement: string;
    changed: boolean;
    included: boolean;
    sequenceBroken: boolean;
  };

  type ProofPreviewState = {
    open: boolean;
    mode: ProofPreviewMode;
    ruleId: ProofBuiltinRuleId | "";
    direction: ProofConvertDirection | "";
    title: string;
    rows: Array<ProofRegexPreviewRow | ProofConvertPreviewRow>;
    selectedIds: Set<string>;
    message: string;
  };

  const DRAFT_KEY = "tepub-web-text-editor-draft";
  const EPUB_HANDOFF_KEY = "tepub-web-text-editor-epub-handoff";
  const encodingLabels: Record<EncodingOption, string> = {
    "utf-8": "UTF-8",
    gb18030: "GB18030",
    big5: "Big5",
  };
  const ruleLevelOptions = [
    { value: "1", label: "卷" },
    { value: "3", label: "章" },
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
  const proofRules: Array<{ id: ProofBuiltinRuleId; label: string; detail: string }> = [
    { id: "title-brackets", label: "标题括号", detail: "只处理已识别目录标题中的括号内容" },
    { id: "ads", label: "清理广告", detail: "预览常见求票、PS、感言段落" },
    { id: "pinyin", label: "清理拼音", detail: "预览拼音注音和独立拼音行" },
    { id: "indent", label: "段落缩进", detail: "预览正文段落缩进调整" },
  ];

  let fileInput: HTMLInputElement | null = null;
  let editorEl: any = null;
  let tocListEl: HTMLDivElement | null = null;
  let sourceBytes: Uint8Array | null = null;
  let content = "";
  let fileName = "untitled.txt";
  let encoding: EncodingOption = "utf-8";
  let status = "就绪";
  let searchQuery = "";
  let replaceText = "";
  let searchMode: SearchMode = "plain";
  let matchCase = false;
  let searchError = "";
  let currentMatchIndex = -1;
  let matches: SearchMatch[] = [];
  let searchDirty = false;
  let busy = false;
  let tocRules: TocRegexRule[] = normalizeRules(loadAppSettings().customRegexRules);
  let chapters: RawChapter[] = [];
  let tocItems: TocItem[] = [];
  let selectedTocId = "";
  let expandedVolumes = new Set<string>();
  let tocLineOffsets = new Map<number, number>();
  let scanToken = 0;
  let regexPanelOpen = false;
  let searchPanelOpen = true;
  let proofPanelOpen = true;
  let checkPanelOpen = false;
  let reorderPanelOpen = false;
  let sequenceErrors: TocCheckIssue[] = [];
  let titleErrors: TocCheckIssue[] = [];
  let invalidSequenceIds = new Set<string>();
  let reorderPreviewRows: ReorderPreviewRow[] = [];
  let reorderScope: ReorderScope = "all";
  let reorderRegex = "";
  let reorderPerVolume = false;
  let volumeNumberStyle: NumberStyle = "chinese";
  let chapterNumberStyle: NumberStyle = "arabic";
  let reorderCollapsedVolumeKeys = new Set<string>();
  let proofPreview: ProofPreviewState = emptyProofPreview();
  let lineCount = 0;
  let charCount = 0;
  let wordCount = 0;
  let byteCount = 0;

  $: if (currentMatchIndex >= matches.length) currentMatchIndex = matches.length - 1;
  $: visibleTocItems = tocItems.filter((item) => item.kind !== "chapter" || !item.volumeKey || expandedVolumes.has(item.volumeKey));
  $: tocHighlightLineStarts = chapters
    .map((chapter) => tocLineOffsets.get(chapter.line_number))
    .filter((offset): offset is number => typeof offset === "number");
  $: visibleReorderRows = reorderPreviewRows.filter((row) => row.kind !== "chapter" || !reorderCollapsedVolumeKeys.has(row.volumeKey));
  $: selectedProofCount = proofPreview.selectedIds.size;
  $: hasLoadedText = Boolean(sourceBytes || content.trim());

  onMount(() => {
    const draft = localStorage.getItem(DRAFT_KEY);
    if (draft) {
      try {
        const parsed = JSON.parse(draft);
        if (typeof parsed?.content === "string") {
          resetEditorContent(parsed.content);
          fileName = typeof parsed.fileName === "string" && parsed.fileName.trim() ? parsed.fileName : fileName;
          tocLineOffsets = new Map();
          markSearchDirty();
          updateTextStats(true);
          status = "已恢复本地草稿";
        }
      } catch {
        localStorage.removeItem(DRAFT_KEY);
      }
    }
  });

  function normalizeRules(rules: Array<Partial<TocRegexRule>> | undefined): TocRegexRule[] {
    const source = Array.isArray(rules) && rules.length ? rules : DEFAULT_TOC_REGEX_RULES;
    return source.map((rule) => ({
      enabled: typeof rule.enabled === "boolean" ? rule.enabled : true,
      level: Number(rule.level) <= 1 ? 1 : 3,
      pattern: String(rule.pattern || "").trim(),
    }));
  }

  function emptyProofPreview(): ProofPreviewState {
    return {
      open: false,
      mode: "builtin",
      ruleId: "",
      direction: "",
      title: "",
      rows: [],
      selectedIds: new Set(),
      message: "",
    };
  }

  function syncContentFromEditor() {
    const nextContent = editorEl?.getContent?.();
    if (typeof nextContent === "string" && nextContent !== content) {
      content = nextContent;
    }
    return content;
  }

  function resetEditorContent(nextContent: string) {
    content = nextContent;
    editorEl?.resetDoc?.(nextContent);
  }

  async function scanLoadedContent(statusPrefix: string) {
    await tick();
    await scanToc(false);
    status = tocItems.length
      ? `${statusPrefix}，已识别 ${tocItems.length} 个目录项`
      : `${statusPrefix}，未识别到目录项`;
  }

  function handleEditorInput() {
    proofPreview = emptyProofPreview();
    tocLineOffsets = new Map();
    clearTocAnalysis();
    markSearchDirty();
    updateTextStats(false);
  }

  function handleEditorChange(nextContent: string) {
    content = nextContent;
    proofPreview = emptyProofPreview();
    tocLineOffsets = new Map();
    clearTocAnalysis();
    markSearchDirty();
    updateTextStats(true);
  }

  function clearTocAnalysis() {
    sequenceErrors = [];
    titleErrors = [];
    invalidSequenceIds = new Set();
    reorderPreviewRows = [];
  }

  function countWords(text: string) {
    return text.replace(/\s+/g, "").length;
  }

  function countLines(text: string) {
    if (!text) return 0;
    let count = 1;
    for (let index = 0; index < text.length; index += 1) {
      const code = text.charCodeAt(index);
      if (code === 10 || code === 13 || code === 0x2028 || code === 0x2029) {
        count += 1;
        if (code === 13 && text.charCodeAt(index + 1) === 10) index += 1;
      }
    }
    return count;
  }

  function updateTextStats(includeHeavy = true) {
    charCount = content.length;
    if (!content) {
      lineCount = 0;
      wordCount = 0;
      byteCount = 0;
      return;
    }

    if (includeHeavy) {
      lineCount = countLines(content);
      wordCount = countWords(content);
      byteCount = new TextEncoder().encode(content).length;
      return;
    }

    return;
  }

  function pickFile() {
    fileInput?.click();
  }

  async function onFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    try {
      sourceBytes = new Uint8Array(await file.arrayBuffer());
      fileName = file.name || "untitled.txt";
      const decoded = decodeSourceBytesAuto(sourceBytes);
      encoding = decoded.encoding;
      resetEditorContent(decoded.text);
      currentMatchIndex = -1;
      tocLineOffsets = new Map();
      markSearchDirty();
      proofPreview = emptyProofPreview();
      chapters = [];
      tocItems = [];
      updateTextStats(true);
      status = `已导入 ${fileName}，自动识别为 ${encodingLabels[encoding]}，正在扫描目录...`;
      await scanLoadedContent(`已导入 ${fileName}`);
      editorEl?.focus?.();
    } catch (error) {
      await platform.message(`导入失败：${String(error)}`, { title: "TXT 编辑器", kind: "error" });
    }
  }

  function decodeSourceBytesAuto(bytes: Uint8Array): { encoding: EncodingOption; text: string } {
    if (bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
      return { encoding: "utf-8", text: decodeBytes(bytes, "utf-8").replace(/^\uFEFF/, "") };
    }

    const candidates: Array<{ encoding: EncodingOption; text: string; score: number }> = [];
    for (const candidate of ["utf-8", "gb18030", "big5"] as EncodingOption[]) {
      try {
        const text = decodeBytes(bytes, candidate, candidate === "utf-8");
        candidates.push({ encoding: candidate, text, score: scoreDecodedText(text, candidate) });
      } catch {
        // Invalid UTF-8 is expected for many legacy TXT files.
      }
    }

    candidates.sort((a, b) => a.score - b.score);
    const best = candidates[0] ?? { encoding: "utf-8" as EncodingOption, text: decodeBytes(bytes, "utf-8") };
    return { encoding: best.encoding, text: best.text.replace(/^\uFEFF/, "") };
  }

  function decodeBytes(bytes: Uint8Array, label: EncodingOption, fatal = false) {
    return new TextDecoder(label, { fatal }).decode(bytes);
  }

  function scoreDecodedText(text: string, candidate: EncodingOption) {
    const replacement = (text.match(/\uFFFD/g) || []).length;
    const controls = (text.match(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g) || []).length;
    const mojibake = (text.match(/[\u00C0-\u00FF]/g) || []).length + (text.match(/\u952F\u65A4\u62F7/g) || []).length * 8;
    const cjk = (text.match(/[\u3400-\u9fff]/g) || []).length;
    const encodingBias = candidate === "utf-8" ? -3 : candidate === "gb18030" ? -1 : 0;
    return replacement * 120 + controls * 40 + mojibake * 12 - cjk * 0.02 + encodingBias;
  }

  function outputFileName() {
    const trimmed = fileName.trim() || "untitled.txt";
    return /\.[^.\\/]+$/.test(trimmed) ? trimmed : `${trimmed}.txt`;
  }

  async function downloadText() {
    syncContentFromEditor();
    if (!content) {
      await platform.message("当前没有可导出的文本。", { title: "TXT 编辑器", kind: "warning" });
      return;
    }
    const selected = await platform.saveDialog({ defaultPath: outputFileName() });
    if (!selected) return;
    await platform.writeFile(selected, Array.from(new TextEncoder().encode(content)));
    status = `已导出 ${selected.split(/[\\/]/).pop() || selected}`;
  }

  function escapeRegex(value: string) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function buildSearchRegex(query: string, mode: SearchMode, caseSensitive: boolean) {
    searchError = "";
    if (!query) return null;
    try {
      return new RegExp(mode === "regex" ? query : escapeRegex(query), `g${caseSensitive ? "" : "i"}`);
    } catch (error: any) {
      searchError = `正则错误：${error?.message || error}`;
      return null;
    }
  }

  function collectMatches(text: string, query: string, mode: SearchMode, caseSensitive: boolean) {
    const regex = buildSearchRegex(query, mode, caseSensitive);
    if (!regex) return [];
    const nextMatches: SearchMatch[] = [];
    let match: RegExpExecArray | null;
    while ((match = regex.exec(text))) {
      nextMatches.push({ start: match.index, end: match.index + match[0].length });
      if (match[0].length === 0) regex.lastIndex += 1;
      if (nextMatches.length > 20000) break;
    }
    return nextMatches;
  }

  function markSearchDirty() {
    searchDirty = true;
    searchError = "";
    currentMatchIndex = -1;
    matches = [];
  }

  function refreshMatches() {
    syncContentFromEditor();
    matches = collectMatches(content, searchQuery, searchMode, matchCase);
    searchDirty = false;
    if (currentMatchIndex >= matches.length) currentMatchIndex = matches.length - 1;
    if (!matches.length) currentMatchIndex = -1;
    return matches.length > 0;
  }

  function findLineStartOffset(lineNumber: number) {
    const targetLine = Math.max(1, lineNumber);
    if (targetLine <= 1) return 0;
    let line = 1;
    for (let index = 0; index < content.length; index += 1) {
      const code = content.charCodeAt(index);
      if (code === 10 || code === 13 || code === 0x2028 || code === 0x2029) {
        line += 1;
        if (code === 13 && content.charCodeAt(index + 1) === 10) index += 1;
        if (line === targetLine) return index + 1;
      }
    }
    return content.length;
  }

  function selectRange(start: number, end: number, lineNumber?: number) {
    editorEl?.selectRange?.(start, end);
  }

  async function scrollTocItemIntoView(id: string) {
    await tick();
    const row = Array.from(tocListEl?.querySelectorAll<HTMLElement>(".toc-row") || []).find((element) => element.dataset.tocId === id);
    row?.scrollIntoView({ block: "center", inline: "nearest" });
  }

  async function selectMatch(index: number) {
    if (!matches.length) return;
    currentMatchIndex = ((index % matches.length) + matches.length) % matches.length;
    const match = matches[currentMatchIndex];
    selectRange(match.start, match.end);
  }

  function findNextMatch() {
    if (searchDirty) refreshMatches();
    if (!matches.length) {
      status = searchQuery ? "未找到匹配项" : "请输入查找内容";
      return;
    }
    void selectMatch(currentMatchIndex + 1);
  }

  function findPreviousMatch() {
    if (searchDirty) refreshMatches();
    if (!matches.length) {
      status = searchQuery ? "未找到匹配项" : "请输入查找内容";
      return;
    }
    void selectMatch(currentMatchIndex - 1);
  }

  async function replaceCurrent() {
    if (searchDirty) refreshMatches();
    if (!matches.length) {
      status = "没有可替换的匹配项";
      return;
    }
    const index = currentMatchIndex >= 0 ? currentMatchIndex : 0;
    const match = matches[index];
    resetEditorContent(`${content.slice(0, match.start)}${replaceText}${content.slice(match.end)}`);
    tocLineOffsets = new Map();
    updateTextStats(true);
    markSearchDirty();
    proofPreview = emptyProofPreview();
    status = "已替换 1 处";
    await tick();
    selectRange(match.start, match.start + replaceText.length);
  }

  function replaceAllMatches() {
    if (searchDirty) refreshMatches();
    const regex = buildSearchRegex(searchQuery, searchMode, matchCase);
    if (!regex) {
      status = searchQuery ? searchError : "请输入查找内容";
      return;
    }
    const count = matches.length;
    if (count === 0) {
      status = "没有可替换的匹配项";
      return;
    }
    resetEditorContent(content.replace(regex, replaceText));
    tocLineOffsets = new Map();
    updateTextStats(true);
    currentMatchIndex = -1;
    markSearchDirty();
    proofPreview = emptyProofPreview();
    status = `已替换 ${count} 处`;
  }

  function compileTocRules() {
    const compiled: Array<{ regex: RegExp; level: number }> = [];
    for (const rule of tocRules) {
      if (!rule.enabled || !rule.pattern.trim()) continue;
      try {
        compiled.push({ regex: new RegExp(rule.pattern), level: Number(rule.level) <= 1 ? 1 : 3 });
      } catch {
        // Invalid rules are kept editable but ignored during scanning.
      }
    }
    return compiled;
  }

  function isMetaTitle(title: string) {
    return /^(?:内容简介|本书相关|简介|序(?:章|言)?|前言|楔子|后记|尾声|完本感言)\s*(?:[:：].*)?$/i.test(title.trim());
  }

  function itemKind(chapter: RawChapter): TocKind {
    if (chapter.is_meta) return "meta";
    return Number(chapter.level) <= 1 ? "volume" : "chapter";
  }

  function normalizeCatalogTitle(value: string) {
    return value.trim().replace(/[\s\u3000]*[:：][\s\u3000]*$/, "");
  }

  async function scanChaptersStreaming(token: number) {
    const compiled = compileTocRules();
    const found: RawChapter[] = [];
    const lineOffsets = new Map<number, number>();
    const source = content;
    const totalLength = source.length;
    let lineNumber = 1;
    let lineStart = 0;

    while (lineStart <= totalLength) {
      if (token !== scanToken) return [];

      let lineEnd = lineStart;
      while (lineEnd < totalLength) {
        const code = source.charCodeAt(lineEnd);
        if (code === 10 || code === 13 || code === 0x2028 || code === 0x2029) break;
        lineEnd += 1;
      }

      const titleText = source.slice(lineStart, lineEnd).trim();
      if (titleText) {
        const matched = compiled.find((rule) => {
          rule.regex.lastIndex = 0;
          return rule.regex.test(titleText);
        });
        if (matched) {
          lineOffsets.set(lineNumber, lineStart);
          found.push({
            title: normalizeCatalogTitle(titleText),
            line_number: lineNumber,
            level: matched.level,
            is_meta: isMetaTitle(titleText),
            word_count: 0,
          });
        }
      }

      if (lineEnd >= totalLength) break;
      const code = source.charCodeAt(lineEnd);
      lineStart = lineEnd + 1;
      if (code === 13 && source.charCodeAt(lineStart) === 10) lineStart += 1;
      lineNumber += 1;

      if (lineNumber > 0 && lineNumber % 1200 === 0) {
        status = `正在扫描目录：第 ${lineNumber} 行`;
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }

    lineCount = lineNumber;

    for (let index = 0; index < found.length; index += 1) {
      if (token !== scanToken) return [];
      const chapter = found[index];
      const start = lineOffsets.get(chapter.line_number) ?? 0;
      const end = index + 1 < found.length ? lineOffsets.get(found[index + 1].line_number) ?? totalLength : totalLength;
      chapter.word_count = countWords(source.slice(start, end));
      if (index > 0 && index % 80 === 0) {
        status = `正在统计目录：${index}/${found.length}`;
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }

    tocLineOffsets = lineOffsets;
    return found;
  }

  function buildToc(list: RawChapter[]) {
    let currentVolume = "";
    const childCounts = new Map<string, number>();
    const items = list.map((chapter, index): TocItem => {
      const kind = itemKind(chapter);
      const id = `toc-${index}-${chapter.line_number}`;
      if (kind === "volume") currentVolume = id;
      const parentId = kind === "chapter" ? currentVolume : "";
      const volumeKey = kind === "chapter" ? currentVolume : kind === "volume" ? id : "";
      if (parentId) childCounts.set(parentId, (childCounts.get(parentId) || 0) + 1);
      return {
        ...chapter,
        id,
        kind,
        parentId,
        volumeKey,
        depth: kind === "chapter" && currentVolume ? 2 : 1,
        hasChildren: false,
      };
    });
    return items.map((item) => ({ ...item, hasChildren: (childCounts.get(item.id) || 0) > 0 }));
  }

  async function scanToc(showStatus = true, token = ++scanToken) {
    syncContentFromEditor();
    if (!content.trim()) return;
    status = "正在扫描目录...";
    chapters = await scanChaptersStreaming(token);
    if (token !== scanToken) return;
    tocItems = buildToc(chapters);
    expandedVolumes = new Set(tocItems.filter((item) => item.kind === "volume" && item.hasChildren).map((item) => item.id));
    selectedTocId = "";
    runTocCheck(tocItems);
    if (showStatus) {
      status = tocItems.length ? `已识别 ${tocItems.length} 个目录项` : "没有识别到目录项";
    }
  }

  function tocNodes(): ProofTocNode[] {
    return tocItems.map((item) => ({
      id: item.id,
      title: item.title,
      line: item.line_number,
      type: item.kind === "volume" ? "Volume" : item.kind === "chapter" ? "Chapter" : "Meta",
      parentId: item.parentId,
      level: item.level,
    }));
  }

  function chineseToNum(text: string) {
    const map: Record<string, number> = {
      零: 0,
      〇: 0,
      一: 1,
      二: 2,
      两: 2,
      三: 3,
      四: 4,
      五: 5,
      六: 6,
      七: 7,
      八: 8,
      九: 9,
    };
    let total = 0;
    let section = 0;
    let current = 0;
    for (const char of text.replace(/\s+/g, "")) {
      if (char in map) {
        current = map[char];
      } else if (char === "十") {
        section += (current || 1) * 10;
        current = 0;
      } else if (char === "百") {
        section += (current || 1) * 100;
        current = 0;
      } else if (char === "千") {
        section += (current || 1) * 1000;
        current = 0;
      } else if (char === "万") {
        total += (section + current) * 10000;
        section = 0;
        current = 0;
      }
    }
    return total + section + current;
  }

  function extractTitleNum(text: string) {
    const chapter = text.match(/第\s*([0-9零〇一二两三四五六七八九十百千万]+)\s*[章回节]/);
    if (chapter) return /^\d+$/.test(chapter[1]) ? Number(chapter[1]) : chineseToNum(chapter[1]);
    const seq = text.match(/^序列\s*([0-9零〇一二两三四五六七八九十百千万]+)(?=\s|[:：。．.\-—–]|$)/);
    if (seq) return /^\d+$/.test(seq[1]) ? Number(seq[1]) : chineseToNum(seq[1]);
    const numeric = text.match(/^(\d+)/);
    return numeric ? Number(numeric[1]) : -1;
  }

  function runTocCheck(items = tocItems) {
    sequenceErrors = [];
    titleErrors = [];
    invalidSequenceIds = new Set();
    let lastNum = -1;

    for (const item of items) {
      if (item.kind === "meta") continue;
      if (item.kind === "volume") {
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
      if (/^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*[章卷回节]\s*$/.test(trimmed) || /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+\s*$/.test(trimmed) || /^\d+$/.test(trimmed)) {
        titleErrors.push({ id: item.id, title: item.title, line: item.line_number, msg: "无标题" });
      }
    }

    reorderPreviewRows = buildReorderPreviewRows(items);
    checkPanelOpen = sequenceErrors.length + titleErrors.length > 0;
  }

  function titleBody(text: string) {
    let body = text.replace(/\u3000/g, " ").replace(/[ \t]+/g, " ").trim();
    const number = "[0-9零〇一二两三四五六七八九十百千万]+";
    const patterns = [
      new RegExp(`^第\\s*${number}\\s*[卷部章回节](?:\\s*[:：。．.\\-—–]\\s*|\\s+)?`, "i"),
      new RegExp(`^卷\\s*${number}(?:\\s*[:：。．.\\-—–]\\s*|\\s+)?`, "i"),
      new RegExp(`^序列\\s*${number}(?:\\s*[:：。．.\\-—–]\\s*|\\s+)?`, "i"),
      /^\(?\s*[（【\[]?\s*\d+\s*[）】\]]?\s*[：:。．.\-—–\s]+/,
      /^\(?\s*[一二三四五六七八九十百千万零〇两]+\s*[：:。．.\-—–\s]+/,
      /^\d{1,5}\s*[：:。．.\-—–\s]+/,
      /^\d{1,5}(?=[\u4e00-\u9fff])\s*/,
    ];
    for (const re of patterns) {
      if (re.test(body)) {
        body = body.replace(re, "");
        break;
      }
    }
    return body.replace(/^(?:\s|[:：。．.\-—–])+/, "").trim();
  }

  function compileReorderRegex() {
    if (reorderScope !== "regex" || !reorderRegex.trim()) return null;
    try {
      return new RegExp(reorderRegex);
    } catch {
      return null;
    }
  }

  function shouldReorderItem(item: TocItem, customRegex: RegExp | null) {
    if (item.kind === "meta" || isMetaTitle(item.title)) return false;
    if (reorderScope === "all") return item.kind === "volume" || item.kind === "chapter";
    if (reorderScope === "volumes") return item.kind === "volume";
    if (reorderScope === "chapters") return item.kind === "chapter";
    return customRegex ? customRegex.test(item.title) : false;
  }

  function formatNumber(num: number, style: NumberStyle) {
    return style === "arabic" ? String(num) : toChineseNumber(num);
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

  function buildReorderPreviewRows(items = tocItems) {
    const rows: ReorderPreviewRow[] = [];
    let volumeIndex = 1;
    let globalChapterIndex = 1;
    let currentVolumeKey = "root";
    let currentVolumeChapterIndex = 1;
    const customRegex = compileReorderRegex();

    for (const item of items) {
      if (item.kind === "volume") {
        currentVolumeKey = item.id;
        currentVolumeChapterIndex = 1;
      }

      if (item.kind === "meta") {
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

      const included = shouldReorderItem(item, customRegex);
      const body = titleBody(item.title);
      let replacement = item.title;
      if (included && item.kind === "volume") {
        replacement = `第${formatNumber(volumeIndex++, volumeNumberStyle)}卷${body ? ` ${body}` : ""}`;
      } else if (included && item.kind === "chapter") {
        const index = reorderPerVolume ? currentVolumeChapterIndex++ : globalChapterIndex++;
        replacement = `第${formatNumber(index, chapterNumberStyle)}章${body ? ` ${body}` : ""}`;
      }

      rows.push({
        id: item.id,
        line: item.line_number,
        kind: item.kind,
        volumeKey: item.kind === "volume" ? item.id : currentVolumeKey,
        original: item.title,
        replacement,
        changed: included && item.title.trim() !== replacement,
        included,
        sequenceBroken: invalidSequenceIds.has(item.id),
      });
    }

    return rows;
  }

  function refreshReorderPreview() {
    reorderPreviewRows = buildReorderPreviewRows(tocItems);
  }

  function toggleReorderVolume(volumeKey: string) {
    const next = new Set(reorderCollapsedVolumeKeys);
    if (next.has(volumeKey)) next.delete(volumeKey);
    else next.add(volumeKey);
    reorderCollapsedVolumeKeys = next;
  }

  async function applyReorderToc() {
    syncContentFromEditor();
    if (!content.trim() || !chapters.length) return;
    const lines = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
    let changed = 0;

    for (const row of reorderPreviewRows) {
      if (!row.included || row.kind === "meta") continue;
      const lineIndex = row.line - 1;
      if (lineIndex < 0 || lineIndex >= lines.length) continue;
      const indent = lines[lineIndex].match(/^[\s\u3000]*/)?.[0] ?? "";
      if (lines[lineIndex].trim() !== row.replacement) {
        lines[lineIndex] = `${indent}${row.replacement}`;
        changed++;
      }
    }

    resetEditorContent(lines.join("\n"));
    updateTextStats(true);
    currentMatchIndex = -1;
    markSearchDirty();
    proofPreview = emptyProofPreview();
    await scanToc(false);
    status = changed ? `已重排 ${changed} 个目录标题` : "目录标题已经是当前顺序";
  }

  function lineStartOffset(lineNumber: number) {
    const scannedOffset = tocLineOffsets.get(lineNumber);
    if (typeof scannedOffset === "number") return scannedOffset;
    return findLineStartOffset(lineNumber);
  }

  function revealTocItem(item: TocItem) {
    selectedTocId = item.id;
    if (item.parentId) expandedVolumes = new Set([...expandedVolumes, item.parentId]);
    void scrollTocItemIntoView(item.id);
    setTimeout(() => {
      const start = lineStartOffset(item.line_number);
      const lineEnd = content.indexOf("\n", start);
      const end = lineEnd >= 0 ? lineEnd : content.length;
      selectRange(start, end, item.line_number);
    }, 0);
  }

  function locateTocItemFromEditorLine(lineStart: number) {
    const item = tocItems.find((toc) => lineStartOffset(toc.line_number) === lineStart);
    if (!item) return;
    selectedTocId = item.id;
    if (item.parentId) expandedVolumes = new Set([...expandedVolumes, item.parentId]);
    void scrollTocItemIntoView(item.id);
  }

  function revealTocItemById(id: string) {
    const item = tocItems.find((toc) => toc.id === id);
    if (item) revealTocItem(item);
  }

  function toggleVolume(item: TocItem) {
    if (item.kind !== "volume" || !item.hasChildren) {
      revealTocItem(item);
      return;
    }
    const next = new Set(expandedVolumes);
    if (next.has(item.id)) next.delete(item.id);
    else next.add(item.id);
    expandedVolumes = next;
    revealTocItem(item);
  }

  function updateTocRule(index: number, patch: Partial<TocRegexRule>) {
    tocRules = tocRules.map((rule, i) => (i === index ? { ...rule, ...patch } : rule));
  }

  function addTocRule(level = 3) {
    tocRules = [...tocRules, { enabled: true, level, pattern: "" }];
  }

  function removeTocRule(index: number) {
    tocRules = tocRules.filter((_, i) => i !== index);
  }

  function resetTocRules() {
    tocRules = DEFAULT_TOC_REGEX_RULES.map((rule) => ({ ...rule }));
    saveTocRules();
    void scanToc();
  }

  function saveTocRules() {
    const settings = loadAppSettings();
    settings.customRegexRules = normalizeRules(tocRules);
    saveAppSettings(settings);
    tocRules = settings.customRegexRules.map((rule) => ({ ...rule }));
    status = "目录正则已保存";
  }

  function proofRowTitle(row: ProofRegexPreviewRow | ProofConvertPreviewRow) {
    if ("ruleId" in row) {
      return `第 ${row.lineStart}${row.lineEnd !== row.lineStart ? `-${row.lineEnd}` : ""} 行`;
    }
    return `第 ${row.lineStart} 行`;
  }

  function proofRowOriginal(row: ProofRegexPreviewRow | ProofConvertPreviewRow) {
    return row.original;
  }

  function proofRowReplacement(row: ProofRegexPreviewRow | ProofConvertPreviewRow) {
    return row.replacement;
  }

  function locateProofRow(row: ProofRegexPreviewRow | ProofConvertPreviewRow) {
    const lineNumber = Math.max(1, row.lineStart || 1);
    const start = lineStartOffset(lineNumber);
    const lineEnd = content.indexOf("\n", start);
    const end = lineEnd >= 0 ? lineEnd : content.length;
    selectRange(start, end, lineNumber);
    status = `已定位到第 ${lineNumber} 行`;
  }

  function openBuiltinProofPreview(ruleId: ProofBuiltinRuleId) {
    if (!content.trim()) {
      status = "当前没有可校对的文本";
      return;
    }
    const rows = buildBuiltinRegexPreview(content, ruleId, tocNodes());
    const rule = proofRules.find((item) => item.id === ruleId);
    proofPreview = {
      open: true,
      mode: "builtin",
      ruleId,
      direction: "",
      title: rule?.label || "校对预览",
      rows,
      selectedIds: new Set(rows.map((row) => row.id)),
      message: rows.length ? `发现 ${rows.length} 处候选修改，请确认后应用。` : "没有发现候选修改。",
    };
    status = proofPreview.message;
  }

  async function openConvertPreview(direction: ProofConvertDirection) {
    if (!content.trim()) {
      status = "当前没有可转换的文本";
      return;
    }
    busy = true;
    try {
      const rows = await buildChineseConvertPreview(content, direction);
      proofPreview = {
        open: true,
        mode: "convert",
        ruleId: "",
        direction,
        title: direction === "simplified-to-traditional" ? "转繁体预览" : "转简体预览",
        rows,
        selectedIds: new Set(rows.map((row) => row.id)),
        message: rows.length ? `发现 ${rows.length} 处候选转换，请确认后应用。` : "没有发现候选转换。",
      };
      status = proofPreview.message;
    } catch (error) {
      await platform.message(`生成预览失败：${String(error)}`, { title: "TXT 编辑器", kind: "error" });
    } finally {
      busy = false;
    }
  }

  function toggleProofRow(id: string) {
    const next = new Set(proofPreview.selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    proofPreview = { ...proofPreview, selectedIds: next };
  }

  function setAllProofRows(selected: boolean) {
    proofPreview = {
      ...proofPreview,
      selectedIds: selected ? new Set(proofPreview.rows.map((row) => row.id)) : new Set(),
    };
  }

  function applyProofPreview() {
    if (!proofPreview.rows.length || proofPreview.selectedIds.size === 0) {
      status = "没有选中要应用的修改";
      return;
    }

    const result = proofPreview.mode === "convert"
      ? applyChineseConvertPreview(content, proofPreview.rows as ProofConvertPreviewRow[], proofPreview.selectedIds)
      : applyBuiltinRegexPreview(content, proofPreview.rows as ProofRegexPreviewRow[], proofPreview.selectedIds);

    resetEditorContent(result.text);
    tocLineOffsets = new Map();
    updateTextStats(true);
    currentMatchIndex = -1;
    markSearchDirty();
    proofPreview = emptyProofPreview();
    status = result.changedCount > 0 ? `已应用 ${result.changedCount} 处修改` : "没有可应用的修改";
  }

  async function prepareEpubHandoff() {
    syncContentFromEditor();
    if (!content.trim()) {
      status = "当前没有可制作 EPUB 的文本";
      return;
    }
    if (!tocItems.length) await scanToc(false);
    sessionStorage.setItem(
      EPUB_HANDOFF_KEY,
      JSON.stringify({
        fileName,
        content,
        rules: tocRules,
        chapters,
        createdAt: new Date().toISOString(),
      }),
    );
    status = "已准备 EPUB 制作数据";
    void goto(appPath("/toolbox/make-epub?fromTextEditor=1"));
  }
</script>

<svelte:head>
  <title>TXT 编辑器 - TEpub Editor</title>
</svelte:head>

<div class="text-editor-page">
  <input bind:this={fileInput} class="file-input" type="file" accept=".txt,.md,.html,.htm,text/*" on:change={onFileChange} />

  {#if !hasLoadedText}
    <div class="import-shell">
      <main class="empty-state">
        <section class="import-card">
          <h1>选择 TXT 文件开始编辑</h1>
          <p>支持 TXT、Markdown 和 HTML 文件，导入后可识别目录、查找替换并校对文本。</p>
          <div class="import-actions">
            <button type="button" class="primary" on:click={pickFile} disabled={busy}>选择 TXT 文件</button>
          </div>
        </section>
      </main>
    </div>
  {:else}
  <main class="workspace">
    <aside class="toc-sidebar">
      <div class="toc-head">
        <div class="toc-title-row">
          <h2>目录</h2>
          <small>{tocItems.length} 项</small>
        </div>
        <button type="button" on:click={() => scanToc()} disabled={!content}>扫描</button>
      </div>
      <div class="toc-list" bind:this={tocListEl}>
        {#if visibleTocItems.length === 0}
          <p>还没有目录项。</p>
        {:else}
          {#each visibleTocItems as item}
            <div
              class="toc-row"
              class:active={selectedTocId === item.id}
              class:volume={item.kind === "volume"}
              class:meta={item.kind === "meta"}
              data-toc-id={item.id}
              style={`--depth:${item.depth}`}
            >
              <button class="toc-main" type="button" on:click={() => item.kind === "volume" ? toggleVolume(item) : revealTocItem(item)}>
                <span class="fold" aria-label={item.kind === "volume" && item.hasChildren ? (expandedVolumes.has(item.id) ? "收起" : "展开") : undefined}>
                  {#if item.kind === "volume" && item.hasChildren}
                    <svg class:open={expandedVolumes.has(item.id)} viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M9 6L15 12L9 18"></path>
                    </svg>
                  {/if}
                </span>
                <strong>{item.title}</strong>
                <small>第 {item.line_number} 行 · {item.word_count} 字</small>
              </button>
            </div>
          {/each}
        {/if}
      </div>
    </aside>

    <section class="editor-shell">
      <div class="editor-toolbar">
        {#if sourceBytes}
          <input class="filename-input" bind:value={fileName} aria-label="文件名" />
        {:else}
          <button type="button" class="filename-import" on:click={pickFile}>导入文件</button>
        {/if}
        <div class="editor-stats" aria-label="文本统计">
          <span>{lineCount} 行</span>
          <span>{wordCount} 字</span>
        </div>
      </div>
      <div class="editor-code">
        <TxtCodeEditor
          bind:this={editorEl}
          doc={content}
          highlightLineStarts={tocHighlightLineStarts}
          onTocLineClick={locateTocItemFromEditorLine}
          onInput={handleEditorInput}
          onChange={handleEditorChange}
        />
      </div>
    </section>

    <aside class="side-panel">
      <div class="side-actions">
        <button type="button" on:click={prepareEpubHandoff}>制作 EPUB</button>
        <button type="button" class="primary" on:click={downloadText}>导出 TXT</button>
      </div>

      <section class="tool-section" class:collapsed={!regexPanelOpen}>
        <button class="section-toggle" type="button" on:click={() => (regexPanelOpen = !regexPanelOpen)}>
          <span>目录正则</span>
          <small>{tocRules.filter((rule) => rule.enabled).length}/{tocRules.length}</small>
          <b>{regexPanelOpen ? "▾" : "▸"}</b>
        </button>
        {#if regexPanelOpen}
          <div class="section-body">
            <div class="regex-rules">
              {#each tocRules as rule, index}
                <div class="regex-rule">
                  <input
                    type="checkbox"
                    class="rule-check"
                    aria-label="启用目录正则"
                    checked={rule.enabled}
                    on:change={(event) => updateTocRule(index, { enabled: (event.currentTarget as HTMLInputElement).checked })}
                  />
                  <CustomSelect
                    className="toc-rule-select"
                    value={String(rule.level)}
                    options={ruleLevelOptions}
                    placeholder="目录层级"
                    on:change={(event) => updateTocRule(index, { level: Number(event.detail) <= 1 ? 1 : 3 })}
                  />
                  <input value={rule.pattern} placeholder="目录正则" on:input={(event) => updateTocRule(index, { pattern: (event.currentTarget as HTMLInputElement).value })} />
                  <button type="button" class="danger icon-button" title="删除规则" on:click={() => removeTocRule(index)}>×</button>
                </div>
              {/each}
            </div>
            <div class="button-row">
              <button type="button" on:click={() => addTocRule(3)}>新增</button>
              <button type="button" on:click={saveTocRules}>保存</button>
              <button type="button" class="ghost" on:click={resetTocRules}>重置</button>
            </div>
          </div>
        {/if}
      </section>

      <section class="tool-section" class:collapsed={!searchPanelOpen}>
        <button class="section-toggle" type="button" on:click={() => (searchPanelOpen = !searchPanelOpen)}>
          <span>查找替换</span>
          <small>{searchMode === "regex" ? "正则" : "文本"}</small>
          <b>{searchPanelOpen ? "▾" : "▸"}</b>
        </button>
        {#if searchPanelOpen}
          <div class="section-body">
            <input bind:value={searchQuery} placeholder="查找" on:input={markSearchDirty} />
            <input bind:value={replaceText} placeholder="替换为" />
            <div class="segmented">
              <button type="button" class:active={searchMode === "plain"} on:click={() => { searchMode = "plain"; markSearchDirty(); }}>文本</button>
              <button type="button" class:active={searchMode === "regex"} on:click={() => { searchMode = "regex"; markSearchDirty(); }}>正则</button>
            </div>
            <label class="check-row">
              <input type="checkbox" bind:checked={matchCase} on:change={markSearchDirty} />
              区分大小写
            </label>
            <div class="button-row">
              <button type="button" on:click={findPreviousMatch}>上一个</button>
              <button type="button" on:click={findNextMatch}>下一个</button>
            </div>
            <div class="button-row">
              <button type="button" on:click={replaceCurrent}>替换</button>
              <button type="button" on:click={replaceAllMatches}>全部替换</button>
            </div>
            <p class:warning={!!searchError}>{searchError || (searchDirty && searchQuery ? "点击查找刷新匹配" : searchQuery ? `${matches.length} 个匹配项` : "未输入查找内容")}</p>
          </div>
        {/if}
      </section>

      <section class="tool-section" class:collapsed={!checkPanelOpen}>
        <button class="section-toggle" type="button" on:click={() => (checkPanelOpen = !checkPanelOpen)}>
          <span>目录检查</span>
          <small>{sequenceErrors.length + titleErrors.length} 个问题</small>
          <b>{checkPanelOpen ? "▾" : "▸"}</b>
        </button>
        {#if checkPanelOpen}
          <div class="section-body">
            {#if sequenceErrors.length || titleErrors.length}
              <div class="check-list">
                {#each sequenceErrors as item}
                  <button type="button" class="toc-check-row" on:click={() => revealTocItemById(item.id)}>
                    <strong>序号跳跃</strong>
                    <span>{item.title}</span>
                    <small>第 {item.line} 行 · {item.msg}</small>
                  </button>
                {/each}
                {#each titleErrors as item}
                  <button type="button" class="toc-check-row" on:click={() => revealTocItemById(item.id)}>
                    <strong>标题缺失</strong>
                    <span>{item.title}</span>
                    <small>第 {item.line} 行 · {item.msg}</small>
                  </button>
                {/each}
              </div>
            {:else}
              <p>当前目录没有发现明显问题。</p>
            {/if}
          </div>
        {/if}
      </section>

      <section class="tool-section" class:collapsed={!reorderPanelOpen}>
        <button class="section-toggle" type="button" on:click={() => (reorderPanelOpen = !reorderPanelOpen)}>
          <span>目录重排</span>
          <small>{visibleReorderRows.length} 项预览</small>
          <b>{reorderPanelOpen ? "▾" : "▸"}</b>
        </button>
        {#if reorderPanelOpen}
          <div class="section-body">
            <div class="reorder-options">
              <label>
                <span>重排范围</span>
                <CustomSelect
                  className="reorder-select"
                  bind:value={reorderScope}
                  options={reorderScopeOptions}
                  on:change={refreshReorderPreview}
                />
              </label>
              <label class="reorder-toggle">
                <input type="checkbox" bind:checked={reorderPerVolume} on:change={refreshReorderPreview} />
                每卷章节从第一章开始
              </label>
              <label>
                <span>卷序号</span>
                <CustomSelect
                  className="reorder-select"
                  bind:value={volumeNumberStyle}
                  options={numberStyleOptions}
                  on:change={refreshReorderPreview}
                />
              </label>
              <label>
                <span>章序号</span>
                <CustomSelect
                  className="reorder-select"
                  bind:value={chapterNumberStyle}
                  options={[numberStyleOptions[1], numberStyleOptions[0]]}
                  on:change={refreshReorderPreview}
                />
              </label>
              {#if reorderScope === "regex"}
                <label class="wide">
                  <span>重排正则</span>
                  <input bind:value={reorderRegex} on:input={refreshReorderPreview} placeholder="匹配需要重排的目录标题" />
                </label>
              {/if}
            </div>
            <div class="button-row">
              <button type="button" class="primary" on:click={applyReorderToc} disabled={busy || !chapters.length}>应用目录重排</button>
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
                  on:click={() => row.kind === "volume" ? toggleReorderVolume(row.id) : revealTocItemById(row.id)}
                >
                  <span>{row.original}</span>
                  <span class:changed={row.changed}>{row.replacement}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </section>

      <section class="tool-section" class:collapsed={!proofPanelOpen}>
        <button class="section-toggle" type="button" on:click={() => (proofPanelOpen = !proofPanelOpen)}>
          <span>校对预览</span>
          <small>{proofPreview.open ? `${selectedProofCount}/${proofPreview.rows.length}` : "确认后应用"}</small>
          <b>{proofPanelOpen ? "▾" : "▸"}</b>
        </button>
        {#if proofPanelOpen}
          <div class="section-body">
            <div class="proof-grid">
              {#each proofRules as rule}
                <button type="button" class="proof-button" disabled={busy || !content} on:click={() => openBuiltinProofPreview(rule.id)}>
                  <strong>{rule.label}</strong>
                  <span>{rule.detail}</span>
                </button>
              {/each}
            </div>
            <div class="button-row">
              <button type="button" disabled={busy || !content} on:click={() => openConvertPreview("simplified-to-traditional")}>预览转繁</button>
              <button type="button" disabled={busy || !content} on:click={() => openConvertPreview("traditional-to-simplified")}>预览转简</button>
            </div>
            {#if proofPreview.open}
              <div class="proof-preview">
                <div class="preview-head">
                  <strong>{proofPreview.title}</strong>
                  <span>{selectedProofCount}/{proofPreview.rows.length}</span>
                </div>
                <p>{proofPreview.message}</p>
                <div class="button-row">
                  <button type="button" class="ghost" on:click={() => setAllProofRows(true)}>全选</button>
                  <button type="button" class="ghost" on:click={() => setAllProofRows(false)}>全不选</button>
                  <button type="button" class="primary" on:click={applyProofPreview}>应用选中</button>
                </div>
                <div class="preview-list">
                  {#each proofPreview.rows.slice(0, 180) as row}
                    <div class="preview-row">
                      <input type="checkbox" aria-label="选择修改" checked={proofPreview.selectedIds.has(row.id)} on:change={() => toggleProofRow(row.id)} />
                      <button type="button" class="preview-locate" on:click={() => locateProofRow(row)}>
                        <b>{proofRowTitle(row)}</b>
                        <em>{proofRowOriginal(row) || "（删除）"}</em>
                        <i>{proofRowReplacement(row) || "（删除）"}</i>
                      </button>
                    </div>
                  {/each}
                  {#if proofPreview.rows.length > 180}
                    <p>仅显示前 180 项，其余选中状态仍会参与应用。</p>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </section>
    </aside>
  </main>
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    background: #eef2f7;
    color: #18212f;
    font-family: "Microsoft YaHei", "PingFang SC", system-ui, sans-serif;
    overflow: hidden;
  }

  .text-editor-page {
    height: 100vh;
    display: grid;
    grid-template-rows: 1fr;
    overflow: hidden;
  }

  .import-shell {
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    overflow: hidden;
  }

  .file-input {
    display: none;
  }

  h1,
  h2,
  p {
    margin: 0;
  }

  h1 {
    font-size: 18px;
    line-height: 1.3;
  }

  h2 {
    font-size: 15px;
    line-height: 1.3;
  }

  .button-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  button,
  input {
    font: inherit;
  }

  button {
    height: 34px;
    padding: 0 12px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    color: #18212f;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: #1677b8;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }

  button.primary {
    border-color: #1677b8;
    background: #1677b8;
    color: #ffffff;
  }

  button.ghost {
    background: #f8fafc;
  }

  button.danger {
    color: #b42318;
  }

  .icon-button {
    width: 34px;
    padding: 0;
  }

  .workspace {
    height: 100%;
    min-height: 0;
    display: grid;
    grid-template-columns: 300px minmax(420px, 1fr) 390px;
    gap: 14px;
    padding: 14px;
    box-sizing: border-box;
    overflow: hidden;
  }

  .empty-state {
    display: grid;
    align-items: start;
    justify-items: center;
    padding: 36px 128px;
    box-sizing: border-box;
  }

  .import-card {
    width: min(100%, 1792px);
    min-height: 342px;
    display: grid;
    align-content: center;
    justify-items: center;
    gap: 16px;
    border: 1px solid rgba(23, 27, 36, 0.08);
    border-radius: 8px;
    background: #ffffff;
    padding: 48px 24px;
    box-sizing: border-box;
    text-align: center;
  }

  .import-card h1 {
    font-size: 22px;
    line-height: 1.3;
  }

  .import-card p {
    max-width: 520px;
    color: #64748b;
    font-size: 14px;
    line-height: 1.6;
  }

  .import-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toc-sidebar,
  .editor-shell,
  .side-panel {
    min-height: 0;
    height: 100%;
  }

  .toc-sidebar {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    border: 1px solid #d8e0eb;
    border-radius: 8px;
    overflow: hidden;
    background: #ffffff;
  }

  .toc-head {
    min-height: 44px;
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
  }

  .toc-head small {
    color: #64748b;
    font-size: 12px;
  }

  .toc-title-row {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    white-space: nowrap;
  }

  .editor-shell {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    border: 1px solid #d8e0eb;
    border-radius: 8px;
    overflow: hidden;
    background: #ffffff;
  }

  .editor-toolbar {
    min-height: 44px;
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
  }

  .editor-stats {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    color: #475569;
    font-size: 13px;
    white-space: nowrap;
  }

  .filename-input {
    width: min(460px, 100%);
    flex: 1 1 auto;
  }

  .filename-import {
    width: min(220px, 100%);
    justify-self: start;
    background: #ffffff;
    font-weight: 700;
  }

  input {
    height: 34px;
    min-width: 0;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 0 10px;
    background: #ffffff;
    color: #18212f;
    box-sizing: border-box;
  }

  .editor-code {
    width: 100%;
    height: 100%;
    min-height: 0;
    background: #fffdf9;
    overflow: hidden;
  }

  .side-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: auto;
    padding: 0 2px 0 0;
    box-sizing: border-box;
  }

  .side-actions {
    flex: 0 0 auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding: 0 0 2px;
  }

  .side-actions button {
    width: 100%;
  }

  .tool-section {
    flex: 0 0 auto;
    border: 1px solid #d8e0eb;
    border-radius: 8px;
    background: #ffffff;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .section-toggle {
    width: 100%;
    height: 42px;
    border: 0;
    border-radius: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 18px;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    text-align: left;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
  }

  .tool-section.collapsed .section-toggle {
    border-bottom: 0;
  }

  .section-toggle span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 800;
  }

  .section-toggle small {
    color: #64748b;
    font-size: 12px;
    font-weight: 400;
  }

  .section-toggle b {
    color: #64748b;
    font-size: 15px;
    line-height: 1;
  }

  .section-body {
    padding: 12px;
    display: grid;
    gap: 10px;
  }

  .tool-section p {
    min-height: 20px;
    color: #64748b;
    font-size: 13px;
  }

  .tool-section p.warning {
    color: #b42318;
  }

  .tool-section input {
    width: 100%;
  }

  .regex-rules,
  .preview-list {
    display: grid;
    gap: 8px;
  }

  .regex-rule {
    display: grid;
    grid-template-columns: 18px 62px minmax(0, 1fr) 34px;
    gap: 6px;
    align-items: center;
  }

  .regex-rule :global(.toc-rule-select) {
    --control-height: 34px;
    min-width: 0;
  }

  .regex-rule :global(.toc-rule-select .custom-select-trigger) {
    padding: 0 8px;
    border-color: #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    box-shadow: none;
    font-size: 13px;
    font-weight: 700;
  }

  .rule-check {
    width: 16px;
    height: 16px;
    padding: 0;
  }

  .toc-list {
    display: grid;
    overflow: auto;
    align-content: start;
    padding: 6px 8px;
  }

  .toc-row {
    min-height: 48px;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    padding: 0 2px 0 calc(2px + var(--depth) * 18px);
    border-bottom: 1px solid #eceef2;
    text-align: left;
    background: transparent;
    color: inherit;
  }

  .toc-main {
    min-height: 48px;
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    grid-template-areas:
      "fold title"
      "fold meta";
    gap: 2px 8px;
    padding: 7px 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    color: inherit;
    text-align: left;
  }

  .toc-row strong {
    grid-area: title;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    line-height: 1.35;
  }

  .toc-row small {
    grid-area: meta;
    color: #858b96;
    font-size: 11px;
    line-height: 1.35;
  }

  .toc-row.active {
    border-radius: 8px;
    background: #e8f2f8;
    box-shadow: inset 3px 0 0 #1677b8;
  }

  .toc-row.volume {
    position: sticky;
    top: 0;
    z-index: 4;
    background: #fff;
    box-shadow: 0 1px 0 #eceef2;
  }

  .toc-row.volume strong {
    font-weight: 900;
  }

  .toc-row.meta {
    color: #6d5d00;
    background: #fffaf0;
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

  .fold svg {
    width: 16px;
    height: 16px;
    stroke: currentColor;
    stroke-width: 2.4;
    fill: none;
    transition: transform 0.14s ease;
  }

  .fold svg.open {
    transform: rotate(90deg);
  }

  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    padding: 4px;
    border-radius: 8px;
    background: #eef2f7;
  }

  .segmented button {
    border: 0;
    background: transparent;
  }

  .segmented button.active {
    background: #ffffff;
    color: #1677b8;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.1);
  }

  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #475569;
    font-size: 13px;
  }

  .check-row input {
    width: 16px;
    height: 16px;
  }

  .check-list {
    display: grid;
    gap: 6px;
    align-content: start;
  }

  .toc-check-row {
    height: auto;
    min-height: 46px;
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    grid-template-areas:
      "type meta"
      "type title";
    gap: 2px 8px;
    padding: 7px 9px;
    border: 1px solid #ece1d6;
    border-radius: 8px;
    background: #fffaf4;
    color: inherit;
    text-align: left;
  }

  .toc-check-row strong {
    grid-area: type;
    align-self: center;
    color: #9b5c1d;
    font-size: 12px;
  }

  .toc-check-row span {
    grid-area: title;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 800;
  }

  .toc-check-row small {
    grid-area: meta;
    color: #858b96;
    font-size: 11px;
  }

  .reorder-options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .reorder-options label {
    min-width: 0;
    display: grid;
    gap: 5px;
    color: #475569;
    font-size: 12px;
  }

  .reorder-options .wide {
    grid-column: 1 / -1;
  }

  .reorder-options :global(.reorder-select) {
    --control-height: 34px;
  }

  .reorder-options :global(.reorder-select .custom-select-trigger) {
    border-color: #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    box-shadow: none;
    font-size: 13px;
    font-weight: 700;
  }

  .reorder-toggle {
    grid-template-columns: 16px minmax(0, 1fr);
    grid-auto-flow: column;
    align-items: center;
    align-content: end;
    min-height: 54px;
    gap: 8px;
  }

  .reorder-toggle input {
    width: 16px;
    height: 16px;
    padding: 0;
  }

  .reorder-preview {
    max-height: 280px;
    display: grid;
    overflow: auto;
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
    position: sticky;
    top: 0;
    z-index: 2;
    background: #f5f6f9;
    color: #747986;
    font-size: 11px;
    font-weight: 900;
  }

  .reorder-row {
    border: 0;
    border-top: 1px solid #edf0f4;
    border-radius: 0;
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

  .reorder-row .changed {
    color: #1f7a5a;
    font-weight: 900;
  }

  .button-row {
    width: 100%;
  }

  .button-row button {
    flex: 1;
  }

  .proof-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .proof-button {
    height: auto;
    min-height: 58px;
    display: grid;
    gap: 3px;
    justify-items: start;
    text-align: left;
    padding: 9px 10px;
  }

  .proof-button strong {
    font-size: 14px;
  }

  .proof-button span {
    color: #64748b;
    font-size: 12px;
  }

  .proof-preview {
    display: grid;
    gap: 9px;
    padding: 10px;
    border-radius: 8px;
    background: #f8fafc;
  }

  .preview-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }

  .preview-list {
    max-height: 280px;
    overflow: auto;
  }

  .preview-row {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    padding: 8px;
    border-radius: 6px;
    background: #ffffff;
    border: 1px solid #e2e8f0;
    font-size: 12px;
  }

  .preview-row input {
    width: 16px;
    height: 16px;
  }

  .preview-locate {
    height: auto;
    min-width: 0;
    padding: 0;
    border: 0;
    display: grid;
    gap: 4px;
    text-align: left;
    background: transparent;
  }

  .preview-locate:hover {
    color: #1677b8;
  }

  .preview-row b,
  .preview-row em,
  .preview-row i {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-style: normal;
  }

  .preview-row em {
    color: #991b1b;
  }

  .preview-row i {
    color: #166534;
  }

  @media (max-width: 1040px) {
    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: 28vh minmax(0, 1fr) 30vh;
    }

    .side-panel {
      overflow: auto;
    }

    .editor-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .editor-stats {
      width: 100%;
      justify-content: flex-start;
    }

    .regex-rule {
      grid-template-columns: 18px 62px minmax(0, 1fr) 34px;
    }
  }

  @media (max-width: 620px) {
    .proof-grid {
      grid-template-columns: 1fr;
    }

    .regex-rule {
      grid-template-columns: 18px minmax(0, 1fr) 34px;
    }

    .regex-rule :global(.toc-rule-select) {
      grid-column: 2;
    }

    .regex-rule input:not(.rule-check) {
      grid-column: 1 / -1;
    }

    .icon-button {
      width: 34px;
    }
  }
</style>
