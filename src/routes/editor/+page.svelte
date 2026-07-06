<script lang="ts">
    import { onMount, tick } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, emit } from "@tauri-apps/api/event";
    import { open, save, message, ask } from "@tauri-apps/plugin-dialog";
    // import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs"; // Removed to force use of custom backend
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
    import Editor from "$lib/Editor.svelte";
    import ContextMenu from "$lib/ContextMenu.svelte";
    import SettingsShell from "$lib/SettingsShell.svelte";
    import TagsEditor from "$lib/TagsEditor.svelte";
    import {
        loadAppSettings as loadGlobalAppSettings,
        saveAppSettings as saveGlobalAppSettings,
    } from "$lib/appSettings";
    import {
        applyTitleRewrite,
        applyBuiltinRegexPreview,
        applyChineseConvertPreview,
        buildProofPreviewSummary,
        buildBuiltinRegexPreview,
        buildChineseConvertPreview,
        buildTitleRewritePreview,
        convertChineseText,
        getChineseScriptProfile,
        PROOF_BUILTIN_REGEX_RULES,
        type ProofBuiltinRuleId,
        type ProofConvertDirection,
        type ProofConvertPreviewRow,
        type ProofNumberStyle,
        type ProofRegexPreviewRow,
        type ProofTitlePreviewRow,
        type ProofTitleScope,
        type ProofTitleRewriteOptions,
        type ProofTocNode,
        type ProofTransformResult,
    } from "$lib/textProofing";

    // --- [1. 完整的接口定义] ---
    interface RawChapter {
        title: string;
        line_number: number;
        level: number;
        is_meta: boolean;
        word_count: number;
    }
    interface TocNode {
        id: string;
        title: string;
        line_number: number;
        type: "Volume" | "Chapter" | "Meta";
        word_count: number;
        children: TocNode[];
        expanded: boolean;
        parentId?: string;
        level?: number;
    }
    interface MatchLocation {
        line: number;
        start_char: number;
        end_char: number;
    }
    interface SearchResult {
        found: boolean;
        count: number;
        matches: MatchLocation[];
    }
    interface CoverSearchResult {
        id: string;
        title: string;
        author: string;
        image_url: string;
        page_url: string;
        source: string;
        preferred: boolean;
    }
    interface FlatNode {
        id: string;
        line: number;
        parentId?: string;
        title: string;
        type: "Volume" | "Chapter" | "Meta";
        word_count: number;
        level?: number;
    }
    type EpubAsset = {
        name: string;
        path: string;
        category: string;
        role?: string;
    };
    type ImportedFontInfo = {
        family: string;
        css_value: string;
        file_name: string;
        path: string;
    };
    type StyleTemplateInfo = {
        id: string;
        name: string;
        file_name: string;
        path: string;
        is_builtin: boolean;
    };
    type StyleTemplateContent = {
        id: string;
        name: string;
        main_css: string;
        is_builtin: boolean;
    };
    interface CheckItem {
        id: string;
        title: string;
        line: number;
        msg: string;
        val: number | string;
        parentId?: string;
    }
    interface HistoryMeta {
        filename: string;
        path: string;
        timestamp: number;
        size: number;
    }
    interface ProofLogInfo {
        fileName: string;
        path: string;
        timestamp: number;
        size: number;
    }
    interface AiProofingConfig {
        enabled: boolean;
        baseUrl: string;
        apiKey: string;
        model: string;
        temperature: number;
        maxChapterChars: number;
        responseTimeoutSec: number;
        autoApprove: boolean;
        extraPrompt: string;
    }
    interface AiProviderConfig {
        id: string;
        name: string;
        kind?: "text" | "image";
        baseUrl: string;
        apiKey: string;
        model: string;
        temperature: number;
    }
    interface TxtAiProofingConfig {
        providerId: string;
        approvalProviderId: string;
    }
    interface LibraryAiMatchConfig {
        providerId: string;
        extraPrompt: string;
    }
    interface LibraryConfig {
        aiProofing?: AiProofingConfig;
        aiProviders?: AiProviderConfig[];
        txtAiProofing?: TxtAiProofingConfig;
        libraryAiMatch?: LibraryAiMatchConfig;
        txtEditorCloseAction?: "exit" | "library";
    }
    interface LibraryData {
        config: LibraryConfig;
        books?: unknown[];
    }
    interface AiProofingResponse {
        content: string;
    }
    interface AiProofingSuggestion {
        original: string;
        replacement: string;
        reason: string;
        type: string;
        confidence: number;
    }
    interface AiProofingApproval {
        original: string;
        approved: boolean;
        reason?: string;
    }
    interface AiProofingRow extends AiProofingSuggestion {
        id: string;
        chapterTitle: string;
        fullChapterRemoval?: boolean;
        lineStart: number;
        startChar: number;
        endChar: number;
        globalStart: number;
        globalEnd: number;
    }
    type AiProofingScope = "current" | "volume" | "all";
    interface AiProofingChapterRange {
        id: string;
        title: string;
        parentId?: string;
        startLine: number;
        endLine: number;
        startOffset: number;
        text: string;
    }
    interface AiProofingCacheState {
        filePath: string;
        contentMd5: string;
        savedAt: number;
        model: string;
        scope: AiProofingScope;
        view: "suggestions" | "approval" | "log";
        rows: AiProofingRow[];
        selectedIds: string[];
        logs: string[];
        approvalBatches?: AiApprovalAppliedBatch[];
        message: string;
        logPath?: string;
    }
    interface AiApprovalAppliedBatch {
        id: string;
        chapterTitle: string;
        rows: AiProofingRow[];
        beforeText: string;
        afterText: string;
        appliedAt: number;
        reverted: boolean;
        revertedRowIds?: string[];
    }
    type ManualTitleKind = "Volume" | "Chapter" | "Ignore";
    interface ManualTitleOverrideEntry {
        kind: ManualTitleKind;
        line: number;
        title: string;
        prevTitle?: string;
        nextTitle?: string;
    }

    // --- [2. 默认配置 (三大正则回归 & 新增动态生成规则)] ---
    interface CustomRegexRule {
        level: number;
        pattern: string;
    }

    const DEFAULT_META_VOLUME_REGEX =
        "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$";
    const DEFAULT_META_BODY_REGEX =
        "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$";
    const DEFAULT_META_REGEX = DEFAULT_META_VOLUME_REGEX;
    const LEGACY_VOLUME_REGEX =
        "^\\s*第[零〇一二两三四五六七八九十百千万0-9]+\\s*[卷部].*";
    const LEGACY_CHAPTER_REGEX =
        "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+).*";
    const DEFAULT_VOLUME_REGEX =
        "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*";
    const DEFAULT_CHAPTER_REGEX =
        "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*";

    const DEFAULT_SETTINGS = {
        customRegexRules: [
            { level: 1, pattern: DEFAULT_META_VOLUME_REGEX },
            { level: 1, pattern: DEFAULT_VOLUME_REGEX },
            { level: 3, pattern: DEFAULT_META_BODY_REGEX },
            { level: 3, pattern: DEFAULT_CHAPTER_REGEX }
        ] as CustomRegexRule[],
        wordCountMinThreshold: 2000,
        wordCountMaxThreshold: 6000,
        wordCountThreshold: 6000,
        clearHistoryOnSave: false,
        defaultEpubStyles: { "main.css": "", "font.css": "" },
        selectedStyleTemplateId: "builtin",
        subsetFonts: false,
        uiTheme: "modern" as "modern" | "classic" | "dark",
        wordWrap: true,
        showWhitespace: false,
        showLineBreaks: false,
        // Legacy fallbacks for compatibility
        volRegex: DEFAULT_VOLUME_REGEX,
        chapRegex: DEFAULT_CHAPTER_REGEX,
        metaRegex: DEFAULT_META_VOLUME_REGEX,
    };

    const DEFAULT_AI_PROOFING: AiProofingConfig = {
        enabled: false,
        baseUrl: "https://api.openai.com/v1",
        apiKey: "",
        model: "gpt-4o-mini",
        temperature: 0.1,
        maxChapterChars: 12000,
        responseTimeoutSec: 300,
        autoApprove: false,
        extraPrompt: "",
    };

    const DEFAULT_TXT_AI_PROOFING: TxtAiProofingConfig = {
        providerId: "",
        approvalProviderId: "",
    };

    const AI_PROOFING_SYSTEM_PROMPT = `你是网文小说文本校对助手。必须保守、精确，只处理确定的问题，不改变作者文风、人称、剧情、人物设定和叙述节奏。

校对标准：
一、标点符号类：
1. 网文中文正文中不应出现大部分半角标点，例如 , ! ? ; 以及不分左右的英文半角引号 " "。英文中括号 [] 应改为【】。破折号错误写作 —、一一、--、一— 等时，必须结合上下文确认确实应为破折号后才改为——。&、%、反斜杠等应转全角。
2. 允许的半角符号：英文词汇中的连接符 -；@#$^* 用于代替粗口；小数点；分数斜杠如 1/2。
3. 省略号只允许标准完整省略号……。不允许单个…、英文省略号...、两个及以上句号代替省略号。两个句号需要结合上下文判断。
4. 不允许嵌套双引号，内层引号应使用单引号。
5. 单引号不能独用，只能包含在双引号内。
6. 所有左右引号、括号都应匹配。
7. 禁止逗号句号等不规则标点连用。例外：？！、！？、……？、……！允许；？……、！……不允许。

二、需要删除：
1. 删除广告，包括章节开头宣传语、正文乱码、平台来源、QQ群号及其谐音/生僻字散布形式。
2. 删除章末或章首非剧情作者碎碎念，如求票、求收藏、PS 等；无 PS 标识也要根据上下文判断。
3. 删除防盗暗码：括号内三个无意义汉字组合，如（诺德好）；双引号内连续中文数字；正文中无意义阿拉伯数字或小数，如“夕阳下594赛跑”“夕阳下5.4赛跑”。必须区分真实剧情数字。
4. 如果整个章节明显不是剧情正文，而是请假条、上架感言、完结感言、作者的话、求票公告、更新说明等，应提出移除整章建议。此类建议 type 使用 remove_chapter，original 可填写章节标题或整章开头片段，replacement 为空。

三、需要修改：
1. 修正常见成语、短语错别字。
2. 修正错误的“的地得”。
3. 删除错误赘写叠字，如“我的的东西”。
4. 繁体字全部改为简体字，唯一例外是“薙”字不可改。
5. 中文之间不能出现普通空格。专有名词内部空格改为间隔号，例如【终极 龙炎炮】改为【终极·龙炎炮】。
6. 禁止硬回车。谨慎判断没有正确段尾标点的段落，是硬回车则合并，是漏标点则补标点。

只返回 JSON 对象，不要返回 Markdown。格式：
{"suggestions":[{"original":"原文中的精确片段；整章移除时可填章节标题或整章开头片段","replacement":"替换后的片段，删除则为空字符串","reason":"简短原因","type":"punctuation|delete|remove_chapter|typo|grammar|spacing|linebreak|traditional|other","confidence":0.0}]}
要求：
1. original 必须是本章原文中连续存在的精确片段。
2. 最多返回 20 条最高置信建议，reason 必须少于 40 个中文字符。
3. 保留段首缩进、空行、表情符号、颜文字、特殊符号、系统提示符号和括号内设定说明，不得把它们当乱码删除。
4. 人名、地名、专有名词、技能名、魔法名、等级名、角色口癖，除非上下文明确写错且可证明，否则不要修改。
5. 不要把“如同/好像/仿佛/似乎/大概/也许”等风格表达强行改成更直白的词。
6. 不要输出整章删除建议，除非整章几乎全是非正文公告/请假/感言，且没有连续剧情内容。
7. 不能确定的问题不要输出。`;

    const AI_APPROVAL_SYSTEM_PROMPT = `你是小说校对建议审批助手。你只判断给定建议是否应该自动应用。
审批原则：
1. 只批准确定无争议、不会改变剧情/设定/文风的建议。
2. 标点规范、明确错别字、明显重复字可批准。
3. 涉及用词风格、专有名词、战力设定、作者有意表达、剧情含义变化的建议必须拒绝。
4. 章删、广告删除、作者碎碎念删除，只有明显非正文时才批准。
5. 删除表情符号、颜文字、特殊符号、括号内设定说明、段首缩进的建议必须拒绝。
只返回 JSON，不要 Markdown。格式：
{"decisions":[{"original":"建议里的 original","approved":true,"reason":"少于20字"}]}`;

    const REGEX_PRESETS = [
        { label: "自定义", value: "" },
        { label: "^\\s*第[一二三..]+章.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+章.*$" },
        { label: "第X卷 标题 / 卷X 标题", value: DEFAULT_VOLUME_REGEX },
        { label: "终章 标题", value: "^\\s*终章(?:\\s+|[:：、.．\\-—])\\S+.*$" },
        { label: "番外 / 后日谈", value: "^\\s*(?:(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*$" },
        { label: "^\\s*第[一二三..]+回.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+回.*$" },
        { label: "^\\s*第[一二三..]+节.*$", value: "^\\s*第[一二三四五六七八九十零〇百千两]+节.*$" },
        { label: "^\\s*第\\d+章.*$", value: "^\\s*第\\d+章.*$" },
        { label: "内容简介 / 本书相关 / 完本感言", value: DEFAULT_META_VOLUME_REGEX },
        { label: "简介 / 前言 / 楔子 / 后记 / 尾声", value: DEFAULT_META_BODY_REGEX },
        { label: "^\\s*序列\\s*\\d+(?:\\s|[:：、.-]|$).*$", value: "^\\s*序列\\s*\\d+(?:\\s|[:：、.-]|$).*$" },
        { label: "^\\s*\\d+\\s*$", value: "^\\s*\\d+\\s*$" }
    ];

    function normalizeAiProofingConfig(config: Partial<AiProofingConfig> | undefined): AiProofingConfig {
        const merged = { ...DEFAULT_AI_PROOFING, ...(config || {}) };
        merged.enabled = Boolean(merged.enabled);
        merged.baseUrl = String(merged.baseUrl || DEFAULT_AI_PROOFING.baseUrl).trim();
        merged.apiKey = String(merged.apiKey || "");
        merged.model = String(merged.model || DEFAULT_AI_PROOFING.model).trim();
        merged.temperature = Math.max(0, Math.min(1, Number(merged.temperature) || DEFAULT_AI_PROOFING.temperature));
        merged.maxChapterChars = Math.max(1000, Math.floor(Number(merged.maxChapterChars) || DEFAULT_AI_PROOFING.maxChapterChars));
        merged.responseTimeoutSec = Math.max(30, Math.min(1800, Math.floor(Number(merged.responseTimeoutSec) || DEFAULT_AI_PROOFING.responseTimeoutSec)));
        merged.autoApprove = Boolean(merged.autoApprove);
        merged.extraPrompt = String(merged.extraPrompt || "");
        return merged;
    }

    function createAiProviderFromProofing(config: AiProofingConfig): AiProviderConfig {
        return {
            id: newAiProviderId(),
            name: config.model || "默认 API",
            kind: "text",
            baseUrl: config.baseUrl,
            apiKey: config.apiKey,
            model: config.model,
            temperature: config.temperature,
        };
    }

    function normalizeAiProvider(provider: Partial<AiProviderConfig> | undefined, index = 0): AiProviderConfig {
        const id = String(provider?.id || `provider-${index + 1}`).trim();
        const model = String(provider?.model || DEFAULT_AI_PROOFING.model).trim();
        return {
            id,
            name: String(provider?.name || model || `API ${index + 1}`).trim(),
            kind: provider?.kind === "image" ? "image" : "text",
            baseUrl: String(provider?.baseUrl || DEFAULT_AI_PROOFING.baseUrl).trim(),
            apiKey: String(provider?.apiKey || ""),
            model,
            temperature: Math.max(0, Math.min(1, Number(provider?.temperature) || DEFAULT_AI_PROOFING.temperature)),
        };
    }

    function normalizeAiProviders(config: LibraryConfig | undefined, fallback: AiProofingConfig) {
        const providers = (config?.aiProviders || [])
            .filter((item) => item?.kind !== "image")
            .map(normalizeAiProvider)
            .filter((item) => item.id);
        if (providers.length > 0) return providers;
        if (fallback.apiKey) {
            return [createAiProviderFromProofing(fallback)];
        }
        return [];
    }

    function providerToProofingConfig(provider: AiProviderConfig | undefined, base: AiProofingConfig) {
        const normalized = normalizeAiProvider(provider, 0);
        return normalizeAiProofingConfig({
            ...base,
            baseUrl: normalized.baseUrl,
            apiKey: normalized.apiKey,
            model: normalized.model,
            temperature: normalized.temperature,
        });
    }

    function findAiProvider(providerId: string) {
        return aiProviders.find((provider) => provider.id === providerId) || aiProviders[0];
    }

    function proofingConfigForProvider(providerId: string) {
        return providerToProofingConfig(findAiProvider(providerId), aiProofingConfig);
    }

    function newAiProviderId() {
        return `provider-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
    }

    function createBlankAiProvider(seed: Partial<AiProviderConfig> = {}): AiProviderConfig {
        return normalizeAiProvider(
            {
                id: seed.id || newAiProviderId(),
                name: seed.name || "新 API",
                kind: "text",
                baseUrl: seed.baseUrl || DEFAULT_AI_PROOFING.baseUrl,
                apiKey: seed.apiKey || "",
                model: seed.model || DEFAULT_AI_PROOFING.model,
                temperature: seed.temperature ?? DEFAULT_AI_PROOFING.temperature,
            },
            aiProviders.length,
        );
    }

    function getUrlFileParam() {
        try {
            const sp = new URLSearchParams(window.location.search);
            const f = sp.get("file");
            return f || null;
        } catch (e) {
            console.warn("解析 URL ?file= 失败:", e);
            return null;
        }
    }

    function isLegacyLooseMetaRegex(pattern: string | undefined) {
        if (!pattern) return false;
        const compact = pattern.replace(/\s+/g, "");
        return (
            compact ===
                "^\\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*" ||
            (compact.includes("序[章言]?") &&
                compact.includes("简介") &&
                compact.endsWith(".*"))
        );
    }

    function normalizeTocRegexRules(rule: any): CustomRegexRule[] {
        let level = Number(rule?.level ?? 3);
        let pattern = String(rule?.pattern ?? "");

        if (typeof rule?.type === "string") {
            level = rule.type === "Volume" || rule.type === "Meta" ? 1 : 3;
        }

        if (isLegacyLooseMetaRegex(pattern)) {
            return [
                { level: 1, pattern: DEFAULT_META_VOLUME_REGEX },
                { level: 3, pattern: DEFAULT_META_BODY_REGEX },
            ];
        }

        if (pattern === LEGACY_VOLUME_REGEX) {
            level = 1;
            pattern = DEFAULT_VOLUME_REGEX;
        }

        if (pattern === LEGACY_CHAPTER_REGEX) {
            level = 3;
            pattern = DEFAULT_CHAPTER_REGEX;
        }

        if (pattern.includes("[章回]")) {
            pattern = pattern.replace("[章回]", "(?:[章节]|回(?:[^合]|$))");
        }

        return [{ level, pattern }];
    }

    function buildTitlePrefixRegex(prefix: string) {
        const escaped = escapeRegExp(prefix.trim());
        return `^\\s*(?:(?:新增\\s*)?${escaped}(?:\\s+|[:：、.．\\-—])\\S+.*|【\\s*${escaped}\\s*】\\s*\\S+.*)$`;
    }

    function isLikelyTocTitle(title: string, level: number) {
        const trimmed = title.trim();
        if (!trimmed) return false;

        // Old loose "序" rules used to catch normal prose such as "序列8时...".
        if (level === 1 && /^序(?!\s*(?:章|言)?\s*(?:[:：]|$))/.test(trimmed)) {
            return false;
        }

        // A legitimate "序列8" heading should stop after the number or use a separator.
        // "序列8时..." / "序列7后..." are prose and should not become TOC entries.
        if (
            /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+[\u4e00-\u9fff]/.test(
                trimmed,
            ) &&
            !/^序列\s*[0-9零〇一二两三四五六七八九十百千万]+(?:\s|[:：、.．\-—]|$)/.test(
                trimmed,
            )
        ) {
            return false;
        }

        const chapterTail = trimmed.match(
            /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(章|节|回)(\S?)/,
        );
        if (chapterTail) {
            const keyword = chapterTail[1]; // 章 / 节 / 回
            const nextChar = chapterTail[2];

            // "第X节" immediately followed by Chinese text (no separator) is almost
            // always prose (e.g. 第二节课, 第一节内容), not a section heading.
            if (keyword === "节" && nextChar && /^[一-鿿]/.test(nextChar)) {
                return false;
            }

            if (
                nextChar &&
                !/^[：:、.．\-—]/.test(nextChar) &&
                /^[的了时侯候后前中里内外上下来去得地着过将把被与和及都也才只能已会在是有为对从用以课程数次目期]/.test(
                    nextChar,
                )
            ) {
                return false;
            }
        }

        const volumeTail = trimmed.match(
            /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(卷|部)(\S?)/,
        );
        if (level === 1 && volumeTail) {
            const nextChar = volumeTail[2];
            if (
                nextChar &&
                !/^[：:、.．\-—]/.test(nextChar) &&
                /^[的和与及是在有把被将就都也却但而或想写看说讲]/.test(nextChar)
            ) {
                return false;
            }
        }

        return true;
    }

    // --- [3. 核心状态] ---
    let filePath = "请打开一本小说...";
    let fileContent = "";
    let tocTree: TocNode[] = [];
    let flatToc: FlatNode[] = [];
    let manualTitleOverrides: Record<string, ManualTitleOverrideEntry> = {};
    let stats = { volumes: 0, chapters: 0 };
    let activeChapterId = "";
    let userCollapsedVolumeKeys = new Set<string>();
    let editorComponent: Editor;

    let showSidebar = true; // State
    // [Removed duplicate declarations]
    let isLoading = false;
    let isLoadingFile = false;
    let isModified = false;
    let isSaving = false;
    let isMobile = false;
    // 导航锁：点击目录跳转时暂时屏蔽滚动监听，防止目录乱跳
    let isNavigating = false;
    let scrollTimeout: any = null;
    let navTimer: any = null;
    let hasInitialized = false;

    // 面板显示状态
    let showSettingsPanel = false;
    let settingsActiveTab: "display" | "fonts" | "styles" | "toc" | "api" | "ai" | "proofLogs" | "history" = "display";
    const editorSettingsTabs = [
        { id: "display", label: "显示" },
        { id: "fonts", label: "字体" },
        { id: "styles", label: "样式" },
        { id: "toc", label: "目录" },
        { id: "api", label: "API 配置" },
        { id: "ai", label: "智能校对" },
        { id: "proofLogs", label: "校对日志" },
        { id: "history", label: "历史版本" },
    ];
    let showEpubModal = false;
    let showCheckPanel = false;
    let showProofPanel = false;
    let showHistoryPanel = false;
    let showRestoreConfirm = false;
    let showStyleSourceEditor = false;
    let restoreTargetSnapshot: any = null;
    let epubGenerationStatus: "idle" | "generating" | "success" = "idle";
    let tocPrefixLevel = 3;
    let tocPrefixText = "";

    // 功能数据
    let epubMeta = {
        title: "书名",
        creator: "作者",
        publisher: "",
        date: new Date().toISOString().split("T")[0],
        uuid: crypto.randomUUID(),
        md5: "",
        cover_path: "",
        description: "",
        tags: [] as string[],
        styles: { "main.css": "", "font.css": "" },
        assets: [] as EpubAsset[],
    };
    let coverPreviewUrl: string | null = null;
    let coverSearchResults: CoverSearchResult[] = [];
    let coverSearchMessage = "";
    let isCoverSearching = false;
    let isCoverApplying = false;
    let showAdvancedEpub = false;
    let customMetadata: { key: string; value: string }[] = [];
    let appSettings = { ...DEFAULT_SETTINGS };
    let historyList: HistoryMeta[] = [];
    let proofLogList: ProofLogInfo[] = [];
    let selectedProofLogPath = "";
    let selectedProofLogContent = "";
    let proofLogMessage = "";
    let importedFonts: ImportedFontInfo[] = [];
    let isImportingFont = false;
    let renamingFontFileName = "";
    let deletingFontFileName = "";
    let fontSettingsMessage = "";
    let styleTemplates: StyleTemplateInfo[] = [];
    let isImportingStyleTemplate = false;
    let isSavingStyleTemplate = false;
    let styleSettingsMessage = "";
    let currentStyleTemplateId = "builtin";
    let currentStyleTemplateName = "内置模板";
    let selectedStyleTemplateCss = "";
    let styleSourceDraft = "";
    let styleTemplateExtraCss = "";
    let stylePanelBaselineCss = "";

    type CssPropertyOption = { label: string; value: string };
    type CssPropertyItem = {
        label: string;
        name: string;
        value: string;
        options?: CssPropertyOption[];
        color?: boolean;
        hiddenInBlockEditor?: boolean;
    };
    type StyleBlock = {
        id: string;
        title: string;
        selector: string;
        note: string;
        accent: string;
        properties: CssPropertyItem[];
        hiddenInBlockEditor?: boolean;
    };

    const BUILTIN_FONT_FAMILY_OPTIONS: CssPropertyOption[] = [
        { label: "Maintext / 宋体", value: `"Maintext", "DK-SONGTI", "st", "宋体", "zw", sans-serif` },
        { label: "Title / 黑体", value: `"Title", "黑体", sans-serif` },
        { label: "系统宋体", value: `"宋体", SimSun, serif` },
        { label: "系统黑体", value: `"黑体", SimHei, sans-serif` },
        { label: "系统楷体", value: `"楷体", KaiTi, serif` },
    ];
    $: fontFamilyOptions = [
        ...BUILTIN_FONT_FAMILY_OPTIONS,
        ...importedFonts.map((font) => ({
            label: `${font.family} / 已导入`,
            value: font.css_value,
        })),
    ];
    const LINE_HEIGHT_OPTIONS: CssPropertyOption[] = [
        { label: "紧凑 130%", value: "130%" },
        { label: "标准 150%", value: "150%" },
        { label: "舒展 170%", value: "170%" },
        { label: "宽松 1.8", value: "1.8" },
    ];
    const TEXT_ALIGN_OPTIONS: CssPropertyOption[] = [
        { label: "两端对齐", value: "justify" },
        { label: "左对齐", value: "left" },
        { label: "居中", value: "center" },
        { label: "右对齐", value: "right" },
    ];
    const STYLE_BLOCK_DEFAULTS: StyleBlock[] = [
        {
            id: "book-body",
            title: "正文页面",
            selector: "body.te-book-body, body.te-chapter-page",
            note: "全书页面基础样式",
            accent: "#a6781d",
            properties: [
                { label: "字体", name: "font-family", value: `"Maintext", "DK-SONGTI", "st", "宋体", "zw", sans-serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "行间距", name: "line-height", value: "130%", options: LINE_HEIGHT_OPTIONS },
                { label: "左侧外边距", name: "margin-left", value: "1%" },
                { label: "右侧外边距", name: "margin-right", value: "1%" },
                { label: "水平对齐方式", name: "text-align", value: "justify", options: TEXT_ALIGN_OPTIONS },
                { label: "颜色", name: "background-color", value: "transparent", color: true },
            ],
        },
        {
            id: "paragraph",
            title: "正文段落",
            selector: "p.te-paragraph",
            note: "正文段落",
            accent: "#a6781d",
            properties: [
                { label: "字体", name: "font-family", value: `"DK-SONGTI", "st", "宋体", "zw", sans-serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "行间距", name: "line-height", value: "130%", options: LINE_HEIGHT_OPTIONS },
                { label: "左侧外边距", name: "margin-left", value: "1%" },
                { label: "右侧外边距", name: "margin-right", value: "1%" },
                { label: "水平对齐方式", name: "text-align", value: "justify", options: TEXT_ALIGN_OPTIONS },
                { label: "首行缩进", name: "text-indent", value: "2em" },
            ],
        },
        {
            id: "cover",
            title: "封面",
            selector: "body.te-cover-page, .te-cover-wrap",
            note: "封面页容器与封面图区域",
            accent: "#3d8c8a",
            properties: [
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "上边距", name: "margin-top", value: "3em" },
                { label: "下边距", name: "margin-bottom", value: "1em" },
            ],
        },
        {
            id: "cover-image",
            title: "封面图",
            selector: ".te-cover-image",
            note: "封面图片",
            accent: "#3d8c8a",
            hiddenInBlockEditor: true,
            properties: [
                { label: "宽度", name: "width", value: "40%" },
                { label: "阴影", name: "box-shadow", value: "3px 3px 3px #535353" },
                { label: "下边距", name: "margin-bottom", value: "0.5em" },
            ],
        },
        {
            id: "production-note",
            title: "制作说明",
            selector: ".te-production-card",
            note: "制作说明页主卡片",
            accent: "#8d6c42",
            properties: [
                { label: "外边距", name: "margin", value: "10% 7.25% 2.75% 7.25%" },
                { label: "内边距", name: "padding", value: "5.25%" },
                { label: "边框", name: "border", value: "1.5px solid #6C322D" },
                { label: "圆角", name: "border-radius", value: "5px" },
                { label: "颜色", name: "background-color", value: "rgba(255, 255, 255, 0.7)", color: true },
                { label: "背景图", name: "background", value: "url(../Images/production-card-bg.jpg) no-repeat top left", hiddenInBlockEditor: true },
                { label: "背景尺寸", name: "background-size", value: "35% auto", hiddenInBlockEditor: true },
            ],
        },
        {
            id: "intro",
            title: "内容简介",
            selector: "body.te-intro-page",
            note: "内容简介/简介页面",
            accent: "#3e7dbb",
            properties: [
                { label: "颜色", name: "background-color", value: "transparent", color: true },
                { label: "颜色", name: "border-color", value: "rgba(83, 83, 83, 0.5)", color: true },
                { label: "边框宽度", name: "border-width", value: "0.4em" },
            ],
        },
        {
            id: "intro-title",
            title: "简介标题",
            selector: ".te-intro-title",
            note: "内容简介标题",
            accent: "#3e7dbb",
            properties: [
                { label: "字体", name: "font-family", value: `"哥特宋"`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "字号", name: "font-size", value: "125%" },
                { label: "颜色", name: "color", value: "#00008B", color: true },
                { label: "水平对齐方式", name: "text-align", value: "left", options: TEXT_ALIGN_OPTIONS },
                { label: "外边距", name: "margin", value: "0.3em 0 0.5em 0" },
                { label: "缩进", name: "text-indent", value: "0" },
            ],
        },
        {
            id: "volume-title",
            title: "卷序",
            selector: ".te-volume-title",
            note: "卷序",
            accent: "#cc5f8c",
            properties: [
                { label: "字体", name: "font-family", value: `"哥特宋", serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "字号", name: "font-size", value: "1.2em" },
                { label: "颜色", name: "color", value: "#59bde6", color: true },
                { label: "字重", name: "font-weight", value: "600" },
                { label: "外边距", name: "margin", value: "2em 0 1em 0" },
                { label: "缩进", name: "text-indent", value: "0em" },
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "行间距", name: "line-height", value: "130%", options: LINE_HEIGHT_OPTIONS },
            ],
        },
        {
            id: "volume-subtitle",
            title: "卷名",
            selector: ".te-volume-subtitle",
            note: "卷名",
            accent: "#cc5f8c",
            properties: [
                { label: "字体", name: "font-family", value: `"哥特宋", serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "字号", name: "font-size", value: "1.2em" },
                { label: "颜色", name: "color", value: "#59bde6", color: true },
                { label: "外边距", name: "margin", value: "0em 0em 1em 0em" },
                { label: "缩进", name: "text-indent", value: "0em" },
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "行间距", name: "line-height", value: "110%", options: LINE_HEIGHT_OPTIONS },
            ],
        },
        {
            id: "volume-head-image",
            title: "卷头图",
            selector: ".te-volume-head-image",
            note: "定义后高级选项会出现卷头图图片槽",
            accent: "#cc5f8c",
            hiddenInBlockEditor: true,
            properties: [
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "外边距", name: "margin", value: "0.5em" },
                { label: "缩进", name: "text-indent", value: "0" },
            ],
        },
        {
            id: "volume-head-img",
            title: "卷头图片",
            selector: ".te-volume-head-img",
            note: "卷页顶部图片本体",
            accent: "#cc5f8c",
            hiddenInBlockEditor: true,
            properties: [
                { label: "宽度", name: "width", value: "70%" },
                { label: "最大宽度", name: "max-width", value: "100%" },
                { label: "显示", name: "display", value: "inline-block" },
            ],
        },
        {
            id: "chapter-title",
            title: "章节标题",
            selector: ".te-chapter-title",
            note: "章节序号与标题容器",
            accent: "#5c7fbf",
            properties: [
                { label: "字体", name: "font-family", value: `"黑体", sans-serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "字号", name: "font-size", value: "1.2em" },
                { label: "颜色", name: "color", value: "#c2181e", color: true },
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "字重", name: "font-weight", value: "900" },
                { label: "外边距", name: "margin", value: "2em 0 3em 0" },
            ],
        },
        {
            id: "chapter-number",
            title: "章节序号",
            selector: ".te-chapter-number",
            note: "章节序号徽标",
            accent: "#d48035",
            properties: [
                { label: "字体", name: "font-family", value: `"黑体", sans-serif`, options: BUILTIN_FONT_FAMILY_OPTIONS },
                { label: "字号", name: "font-size", value: "0.8em" },
                { label: "颜色", name: "color", value: "#413245", color: true },
                { label: "行间距", name: "line-height", value: "130%", options: LINE_HEIGHT_OPTIONS },
                { label: "字重", name: "font-weight", value: "900" },
                { label: "内边距", name: "padding", value: "0" },
            ],
        },
        {
            id: "chapter-head-image",
            title: "章节头图",
            selector: ".te-chapter-head-image",
            note: "章节页顶部图片容器",
            accent: "#5c7fbf",
            hiddenInBlockEditor: true,
            properties: [
                { label: "水平对齐方式", name: "text-align", value: "left", options: TEXT_ALIGN_OPTIONS },
                { label: "外边距", name: "margin", value: "0" },
                { label: "缩进", name: "text-indent", value: "0em" },
                { label: "出血", name: "duokan-bleed", value: "lefttopright" },
            ],
        },
        {
            id: "chapter-head-img",
            title: "章节头图片",
            selector: ".te-chapter-head-img",
            note: "章节页顶部图片本体",
            accent: "#5c7fbf",
            hiddenInBlockEditor: true,
            properties: [
                { label: "宽度", name: "width", value: "100%" },
            ],
        },
        {
            id: "divider",
            title: "分割线/分割图",
            selector: "p.te-divider-line, .te-divider-image",
            note: "章节内分割线与分割图容器",
            accent: "#6a7c5b",
            properties: [
                { label: "水平对齐方式", name: "text-align", value: "center", options: TEXT_ALIGN_OPTIONS },
                { label: "缩进", name: "text-indent", value: "0" },
                { label: "外边距", name: "margin", value: "1em 0" },
                { label: "内边距", name: "padding", value: "0" },
                { label: "行高", name: "line-height", value: "130%" },
            ],
        },
        {
            id: "divider-image",
            title: "分割图片",
            selector: ".te-divider-img",
            note: "孤立省略号替换成分割图时使用",
            accent: "#6a7c5b",
            hiddenInBlockEditor: true,
            properties: [
                { label: "宽度", name: "width", value: "200px" },
                { label: "最大宽度", name: "max-width", value: "100%" },
                { label: "边框", name: "border", value: "none" },
                { label: "垂直对齐", name: "vertical-align", value: "middle" },
            ],
        },
    ];
    let styleBlocks: StyleBlock[] = cloneStyleBlocks(STYLE_BLOCK_DEFAULTS);
    let activeStyleBlockId = STYLE_BLOCK_DEFAULTS.find((block) => !block.hiddenInBlockEditor)?.id || "";

    const STYLE_TEMPLATE_BUILTIN_ID = "builtin";
    const STYLE_TEMPLATE_HEADER = `@charset "utf-8";

@import url("font.css");

/* TEpub template schema: 1 */
/* @tepub-asset-slot productionCardBg type="image" label="制作说明背景图" placement="manual" selector=".te-production-card" */
/* @tepub-asset-slot volumeHead type="image" label="卷头图" placement="volume-before-title" selector=".te-volume-head-image .te-volume-head-img" */
/* @tepub-asset-slot chapterHead type="image" label="章节头图" placement="chapter-before-title" selector=".te-chapter-head-image .te-chapter-head-img" */
/* @tepub-asset-slot dividerImage type="image" label="分割图" placement="replace-ellipsis" selector=".te-divider-image .te-divider-img" */
/* Standard classes: te-cover-wrap te-cover-image te-production-card te-production-title te-production-text te-production-note te-production-logo te-production-logo-img te-intro-page te-intro-title te-intro-heading te-volume-page te-volume-title te-volume-subtitle te-volume-head-image te-volume-head-img te-chapter-page te-chapter-title te-chapter-number te-chapter-name te-chapter-head-image te-chapter-head-img te-paragraph te-divider-line te-divider-image te-divider-img */`;

    function cloneStyleBlocks(blocks: StyleBlock[]) {
        return blocks.map((block) => ({
            ...block,
            properties: block.properties.map((prop) => ({
                ...prop,
                options: prop.options ? [...prop.options] : undefined,
            })),
        }));
    }

    function escapeRegExp(value: string) {
        return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    }

    function channelToHex(value: number) {
        return Math.max(0, Math.min(255, Math.round(value))).toString(16).padStart(2, "0");
    }

    function rgbToHex(r: number, g: number, b: number) {
        return `#${channelToHex(r)}${channelToHex(g)}${channelToHex(b)}`;
    }

    function hexToRgb(hex: string) {
        const normalized = hex.trim().replace("#", "");
        if (/^[0-9a-f]{3}$/i.test(normalized)) {
            return {
                r: parseInt(normalized[0] + normalized[0], 16),
                g: parseInt(normalized[1] + normalized[1], 16),
                b: parseInt(normalized[2] + normalized[2], 16),
            };
        }
        if (/^[0-9a-f]{6}$/i.test(normalized)) {
            return {
                r: parseInt(normalized.slice(0, 2), 16),
                g: parseInt(normalized.slice(2, 4), 16),
                b: parseInt(normalized.slice(4, 6), 16),
            };
        }
        return { r: 0, g: 0, b: 0 };
    }

    function parseCssColorValue(value: string) {
        const raw = value.trim();
        if (!raw || raw.toLowerCase() === "transparent") {
            return { hex: "#00000000", swatch: "#000000" };
        }
        const hexMatch = raw.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
        if (hexMatch) {
            const normalized = raw.length === 4
                ? `#${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}`
                : raw.toUpperCase();
            const rgb = hexToRgb(normalized);
            return { hex: normalized, swatch: rgbToHex(rgb.r, rgb.g, rgb.b) };
        }
        const hexAlphaMatch = raw.match(/^#([0-9a-f]{8})$/i);
        if (hexAlphaMatch) {
            const normalized = raw.toUpperCase();
            return { hex: normalized, swatch: normalized.slice(0, 7) };
        }
        const rgbaMatch = raw.match(/^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([0-9.]+))?\s*\)$/i);
        if (rgbaMatch) {
            const alpha = rgbaMatch[4] === undefined ? 255 : Math.max(0, Math.min(255, Math.round(Number(rgbaMatch[4]) * 255)));
            const baseHex = rgbToHex(Number(rgbaMatch[1]), Number(rgbaMatch[2]), Number(rgbaMatch[3]));
            return {
                hex: alpha >= 255 ? baseHex : `${baseHex}${channelToHex(alpha).toUpperCase()}`,
                swatch: baseHex,
            };
        }
        return { hex: "#000000", swatch: "#000000" };
    }

    function normalizeHexColorInput(value: string) {
        const raw = value.trim().toUpperCase();
        if (/^#([0-9A-F]{6}|[0-9A-F]{8})$/.test(raw)) {
            return raw;
        }
        if (/^#([0-9A-F]{3}|[0-9A-F]{4})$/.test(raw)) {
            const chars = raw.slice(1).split("");
            return `#${chars.map((item) => item + item).join("")}`;
        }
        return null;
    }

    function buildCssColorValue(hex: string) {
        const normalized = normalizeHexColorInput(hex);
        if (!normalized) return hex;
        if (normalized.length === 9) {
            const alphaHex = normalized.slice(7, 9);
            if (alphaHex === "00") return "transparent";
            if (alphaHex === "FF") return normalized.slice(0, 7);
            const { r, g, b } = hexToRgb(normalized.slice(0, 7));
            const alpha = parseInt(alphaHex, 16) / 255;
            return `rgba(${r}, ${g}, ${b}, ${alpha.toFixed(2).replace(/0+$/, "").replace(/\.$/, "")})`;
        }
        return normalized;
    }

    function updateToolbarColorValue(blockId: string, propName: string, value: string) {
        const blockIndex = styleBlocks.findIndex((item) => item.id === blockId);
        const propIndex = styleBlocks.find((item) => item.id === blockId)?.properties.findIndex((item) => item.name === propName) ?? -1;
        if (blockIndex < 0 || propIndex < 0) return;
        const normalized = normalizeHexColorInput(value);
        if (!normalized) {
            updateToolbarStyleBlock(blockIndex, propIndex, value);
            return;
        }
        updateToolbarStyleBlock(blockIndex, propIndex, buildCssColorValue(normalized));
    }

    $: visibleStyleBlocks = styleBlocks
        .filter((block) => !block.hiddenInBlockEditor)
        .map((block) => ({
            ...block,
            properties: block.properties.filter((prop) => !prop.hiddenInBlockEditor),
        }));
    $: if (visibleStyleBlocks.length > 0 && !visibleStyleBlocks.some((block) => block.id === activeStyleBlockId)) {
        activeStyleBlockId = visibleStyleBlocks[0].id;
    }
    $: activeStyleBlock = visibleStyleBlocks.find((block) => block.id === activeStyleBlockId) || visibleStyleBlocks[0];

    function buildStyleBlocksCss(blocks = styleBlocks) {
        return blocks
            .map((block) => {
                const declarations = block.properties
                    .filter((prop) => prop.value.trim())
                    .map((prop) => `    ${prop.name}: ${prop.value.trim()};`)
                    .join("\n");
                return declarations ? `/* [tepub-block:${block.id}] ${block.title} */\n${block.selector} {\n${declarations}\n}` : "";
            })
            .filter(Boolean)
            .join("\n\n");
    }

    function buildStyleTemplateCss(blocks = styleBlocks, extraCss = styleTemplateExtraCss) {
        const blockCss = buildStyleBlocksCss(blocks);
        const tail = extraCss.trim();
        return `${STYLE_TEMPLATE_HEADER}\n\n${blockCss}${tail ? `\n\n${tail}\n` : "\n"}`;
    }

    function getBuiltinStyleTemplateCss() {
        return buildStyleTemplateCss(cloneStyleBlocks(STYLE_BLOCK_DEFAULTS), "");
    }

    function extractRuleDeclarations(css: string, selector: string) {
        const match = css.match(new RegExp(`${escapeRegExp(selector)}\\s*\\{([\\s\\S]*?)\\}`, "m"));
        if (!match) return null;
        const declarations = new Map<string, string>();
        for (const declaration of match[1].matchAll(/([-\w]+)\s*:\s*([^;]+);/g)) {
            declarations.set(declaration[1].trim(), declaration[2].trim());
        }
        return declarations;
    }

    function parseStyleTemplateCss(css: string) {
        const blocks = cloneStyleBlocks(STYLE_BLOCK_DEFAULTS).map((block) => {
            const declarations = extractRuleDeclarations(css, block.selector);
            if (!declarations) return block;
            return {
                ...block,
                properties: block.properties.map((prop) => ({
                    ...prop,
                    value: declarations.get(prop.name) ?? prop.value,
                })),
            };
        });

        let extraCss = css;
        extraCss = extraCss.replace(/@charset\s+"utf-8";\s*/i, "");
        extraCss = extraCss.replace(/@import\s+url\("font\.css"\);\s*/i, "");
        extraCss = extraCss.replace(/\/\*\s*TEpub template schema:\s*1\s*\*\/\s*/gi, "");
        extraCss = extraCss.replace(/\/\*\s*@tepub-asset-slot[\s\S]*?\*\/\s*/gi, "");
        extraCss = extraCss.replace(/\/\*\s*Standard classes:[\s\S]*?\*\/\s*/gi, "");
        extraCss = extraCss.replace(/\/\*\s*\[tepub-block:[\s\S]*?\*\/\s*/gi, "");
        for (const block of STYLE_BLOCK_DEFAULTS) {
            extraCss = extraCss.replace(
                new RegExp(`${escapeRegExp(block.selector)}\\s*\\{[\\s\\S]*?\\}\\s*`, "g"),
                "",
            );
        }

        return {
            blocks,
            extraCss: extraCss.trim(),
        };
    }

    function applyResolvedStyleTemplateCss(mainCss: string) {
        const effectiveCss = mainCss.trim() || getBuiltinStyleTemplateCss();
        const { blocks, extraCss } = parseStyleTemplateCss(effectiveCss);
        styleBlocks = blocks;
        styleTemplateExtraCss = extraCss;
        epubMeta.styles["main.css"] = effectiveCss;
        syncManagedFontAssets(blocks);
        epubMeta.styles = { ...epubMeta.styles };
    }

    function captureStylePanelBaseline() {
        stylePanelBaselineCss = epubMeta.styles["main.css"] || getBuiltinStyleTemplateCss();
    }

    function collectUsedImportedFonts(blocks = styleBlocks) {
        const used = new Map<string, ImportedFontInfo>();
        for (const block of blocks) {
            for (const prop of block.properties) {
                if (prop.name !== "font-family") continue;
                const matched = importedFonts.find((font) => font.css_value === prop.value.trim());
                if (matched) {
                    used.set(matched.family, matched);
                }
            }
        }
        return [...used.values()];
    }

    function buildManagedFontCss(usedFonts: ImportedFontInfo[]) {
        if (!usedFonts.length) return "";
        return usedFonts
            .map((font) => {
                const ext = font.file_name.split(".").pop()?.toLowerCase() || "ttf";
                const formatMap: Record<string, string> = {
                    ttf: "truetype",
                    otf: "opentype",
                    woff: "woff",
                    woff2: "woff2",
                };
                return `/* 外部导入字体：${font.family} */\n@font-face {\n    font-family: "${font.family}";\n    src: url("../Fonts/${font.file_name}") format("${formatMap[ext] || ext}");\n}`;
            })
            .join("\n\n");
    }

    function syncManagedFontAssets(blocks = styleBlocks) {
        const usedFonts = collectUsedImportedFonts(blocks);
        const managedAssets = usedFonts.map((font) => ({
            name: font.file_name,
            path: font.path,
            category: "fonts",
            role: `managed-font:${font.family}`,
        }));
        epubMeta.assets = [
            ...epubMeta.assets.filter((asset) => !(asset.category === "fonts" && asset.role?.startsWith("managed-font:"))),
            ...managedAssets,
        ];
        epubMeta.styles["font.css"] = buildManagedFontCss(usedFonts);
    }

    function syncToolbarStyleToEpubMeta() {
        epubMeta.styles["main.css"] = buildStyleTemplateCss();
        syncManagedFontAssets();
        epubMeta.styles = { ...epubMeta.styles };
    }

    function updateToolbarStyleBlock(blockIndex: number, propIndex: number, value: string) {
        if (blockIndex < 0 || propIndex < 0) return;
        styleBlocks = styleBlocks.map((block, i) => {
            if (i !== blockIndex) return block;
            return {
                ...block,
                properties: block.properties.map((prop, j) => j === propIndex ? { ...prop, value } : prop),
            };
        });
        syncToolbarStyleToEpubMeta();
    }

    function resetToolbarStyleBlocks() {
        applyResolvedStyleTemplateCss(stylePanelBaselineCss);
        styleSourceDraft = stylePanelBaselineCss;
    }

    function getVolumeCollapseKey(node: Pick<TocNode, "line_number" | "title">) {
        return `${node.line_number}:${node.title}`;
    }

    function toggleVolumeNode(node: TocNode) {
        node.expanded = !node.expanded;
        const key = getVolumeCollapseKey(node);

        if (node.expanded) {
            userCollapsedVolumeKeys.delete(key);
        } else {
            userCollapsedVolumeKeys.add(key);
        }

        userCollapsedVolumeKeys = new Set(userCollapsedVolumeKeys);
        tocTree = [...tocTree];
    }

    // 目录查找状态：匹配列表 + 当前索引，支持 find-next / find-prev
    let tocSearchMatches: TocNode[] = [];
    let tocSearchIndex = -1;
    let lastTocSearchKey = "";

    function buildTocTestFn(query: string, searchMode: string): ((title: string) => boolean) | null {
        if (searchMode === "regex") {
            try { const regex = new RegExp(query, "i"); return (t) => regex.test(t); }
            catch { return null; }
        } else if (searchMode === "extended") {
            const literal = query.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
            const lower = literal.toLowerCase();
            return (t) => t.toLowerCase().includes(lower);
        } else {
            const lower = query.toLowerCase();
            return (t) => t.toLowerCase().includes(lower);
        }
    }

    function findAllTocMatches(query: string, searchMode: string): TocNode[] {
        const testFn = buildTocTestFn(query, searchMode);
        if (!testFn) return [];
        const matches: TocNode[] = [];
        for (const vol of tocTree) {
            if (testFn(vol.title)) matches.push(vol);
            if (vol.children) {
                for (const ch of vol.children) {
                    if (testFn(ch.title)) matches.push(ch);
                }
            }
        }
        return matches;
    }

    function navigateToTocMatch(match: TocNode) {
        // 如果是子节点，确保父卷已展开
        const parentVol = tocTree.find((v) =>
            v.children?.some((c) => c.id === match.id),
        );
        if (parentVol && !parentVol.expanded) {
            parentVol.expanded = true;
            userCollapsedVolumeKeys.delete(getVolumeCollapseKey(parentVol));
            tocTree = [...tocTree];
        }
        if (editorComponent) {
            console.log("[PAGE] navigateToTocMatch scrolling to line " + match.line_number + " title=" + match.title);
            editorComponent.scrollToLine(match.line_number);
            activeChapterId = match.id;
        }
    }

    function handleTocSearch(query: string, actionType: string, searchMode: string) {
        console.log("[PAGE] handleTocSearch query=" + JSON.stringify(query) + " type=" + actionType + " mode=" + searchMode + " tocTreeLen=" + tocTree.length);
        if (!query) return;

        const searchKey = query + ":::" + searchMode;
        const isNewSearch = searchKey !== lastTocSearchKey;

        if (isNewSearch) {
            tocSearchMatches = findAllTocMatches(query, searchMode);
            tocSearchIndex = tocSearchMatches.length > 0 ? 0 : -1;
            lastTocSearchKey = searchKey;
            console.log("[PAGE] new search: " + tocSearchMatches.length + " matches");
        } else if (actionType === "find-next") {
            if (tocSearchMatches.length > 0) {
                tocSearchIndex = (tocSearchIndex + 1) % tocSearchMatches.length;
            }
        } else if (actionType === "find-prev") {
            if (tocSearchMatches.length > 0) {
                tocSearchIndex = (tocSearchIndex - 1 + tocSearchMatches.length) % tocSearchMatches.length;
            }
        }

        if (tocSearchIndex >= 0 && tocSearchIndex < tocSearchMatches.length) {
            navigateToTocMatch(tocSearchMatches[tocSearchIndex]);
            emit("search-status", { count: tocSearchMatches.length, current: tocSearchIndex + 1 });
        } else {
            emit("search-status", { count: 0 });
        }
    }

    function getProofTocNodes(): ProofTocNode[] {
        return flatToc.map((node) => ({
            id: node.id,
            title: node.title,
            line: node.line,
            type: node.type,
            parentId: node.parentId,
            level: node.level,
        }));
    }

    function getProofTitleOptions(): ProofTitleRewriteOptions {
        return {
            scope: proofTitleScope,
            regex: proofTitleRegex,
            volumeNumberStyle: proofVolumeNumberStyle,
            chapterNumberStyle: proofChapterNumberStyle,
            perVolume: proofPerVolume,
        };
    }

    let visibleProofPreviewRows: ProofTitlePreviewRow[] = [];
    let proofCheckMessage = "";

    $: {
        fileContent;
        flatToc;
        proofTitleScope;
        proofTitleRegex;
        proofVolumeNumberStyle;
        proofChapterNumberStyle;
        proofPerVolume;
        try {
            proofPreviewRows = buildTitleRewritePreview(
                fileContent,
                getProofTocNodes(),
                getProofTitleOptions(),
            );
            const summary = buildProofPreviewSummary(proofPreviewRows);
            proofPreviewMessage =
                proofActiveTab === "toc" && summary.total > 0
                    ? proofTitleScope === "numbers-only"
                        ? `数字转换预览 ${summary.total} 项，预计修改 ${summary.changed} 项`
                        : `目录预览 ${summary.total} 项，预计修改 ${summary.changed} 项`
                    : proofTitleScope === "regex" && proofTitleRegex.trim()
                      ? "没有匹配到可重排标题"
                      : proofActiveTab === "builtin"
                        ? builtinRegexMessage
                        : proofActiveTab === "ai"
                          ? `智能校对 ${aiProofingRows.length} 条建议`
                          : proofActiveTab === "check"
                          ? proofCheckMessage
                          : "没有可预览的标题";
        } catch (e: any) {
            proofPreviewRows = [];
            proofPreviewMessage = `预览失败: ${e?.message || e}`;
        }
    }

    $: visibleProofPreviewRows = proofPreviewRows.filter(
        (row) => row.kind === "volume" || !proofCollapsedVolumeKeys.has(row.volumeKey),
    );

    $: proofCheckMessage = `断序 ${sequenceErrors.length} 项，标题 ${titleErrors.length} 项，字数 ${wordCountErrors.length} 项`;

    $: {
        fileContent;
        flatToc;
        proofBuiltinRule;
        try {
            proofRegexPreviewRows = buildBuiltinRegexPreview(
                fileContent,
                proofBuiltinRule,
                getProofTocNodes(),
            );
            proofRegexSelectedIds = new Set();
            builtinRegexMessage =
                proofRegexPreviewRows.length > 0
                    ? `匹配 ${proofRegexPreviewRows.length} 项，默认全不选`
                    : "没有匹配项";
        } catch (e: any) {
            proofRegexPreviewRows = [];
            proofRegexSelectedIds = new Set();
            builtinRegexMessage = `预览失败: ${e?.message || e}`;
        }
    }

    async function applyProofResult(result: ProofTransformResult) {
        proofMessage = result.message;
        if (!fileContent) {
            proofMessage = "请先打开文本文件";
            return;
        }
        if (result.changedCount <= 0 || result.text === fileContent) {
            return;
        }

        editorComponent?.replaceAllContent(result.text);
        fileContent = result.text;
        isModified = true;
        allMatches = [];
        currentMatchIndex = -1;
        clearTimeout(autoRefreshTimer);
        await tick();
        await scanToc(result.text);
        await updateMd5(result.text);
        saveStateToCache(filePath ? (flatToc.find((node) => node.id === activeChapterId)?.line || 1) : 1);
    }

    async function applyProofTitleRewrite() {
        const result = applyTitleRewrite(
            fileContent,
            getProofTocNodes(),
            getProofTitleOptions(),
        );
        await applyProofResult(result);
    }

    function toggleProofVolumeCollapse(volumeKey: string) {
        const next = new Set(proofCollapsedVolumeKeys);
        if (next.has(volumeKey)) {
            next.delete(volumeKey);
        } else {
            next.add(volumeKey);
        }
        proofCollapsedVolumeKeys = next;
    }

    function jumpToProofTitleRow(row: ProofTitlePreviewRow) {
        editorComponent?.scrollToLine(row.line, true);
        proofMessage = "已定位到原标题位置";
    }

    function toggleProofRegexRow(rowId: string, checked: boolean) {
        const next = new Set(proofRegexSelectedIds);
        if (checked) {
            next.add(rowId);
        } else {
            next.delete(rowId);
        }
        proofRegexSelectedIds = next;
    }

    function setAllProofRegexRows(checked: boolean) {
        proofRegexSelectedIds = checked
            ? new Set(proofRegexPreviewRows.map((row) => row.id))
            : new Set();
    }

    function jumpToProofRegexRow(row: ProofRegexPreviewRow) {
        editorComponent?.scrollToLine(row.lineStart, true);
        proofMessage =
            row.lineEnd !== row.lineStart
                ? `已定位到匹配范围 ${row.lineStart}-${row.lineEnd} 行`
                : `已定位到匹配位置`;
    }

    async function applySelectedBuiltinRegex() {
        const result = applyBuiltinRegexPreview(
            fileContent,
            proofRegexPreviewRows,
            proofRegexSelectedIds,
        );
        await applyProofResult(result);
    }

    async function applyAllBuiltinRegex() {
        const result = applyBuiltinRegexPreview(
            fileContent,
            proofRegexPreviewRows,
            proofRegexPreviewRows.map((row) => row.id),
        );
        await applyProofResult(result);
    }

    async function runProofFullConvert() {
        proofMessage = "正在转换繁简，请稍候...";
        const result = await convertChineseText(fileContent, proofConvertDirection);
        await applyProofResult(result);
    }

    async function loadSharedLibrarySettings() {
        try {
            libraryData = await invoke<LibraryData>("load_library");
            const appSettings = saveGlobalAppSettings(loadGlobalAppSettings(libraryData?.config || {}));
            aiProofingConfig = normalizeAiProofingConfig(appSettings.aiProofing);
            aiProviders = appSettings.aiProviders
                .filter((provider) => provider.kind !== "image")
                .map((provider, index) => normalizeAiProvider(provider, index));
            txtAiProofingConfig = { ...DEFAULT_TXT_AI_PROOFING, ...appSettings.txtAiProofing };
            if (!txtAiProofingConfig.providerId || !findAiProvider(txtAiProofingConfig.providerId)) {
                txtAiProofingConfig.providerId = aiProviders[0]?.id || "";
            }
            if (!txtAiProofingConfig.approvalProviderId || !findAiProvider(txtAiProofingConfig.approvalProviderId)) {
                txtAiProofingConfig.approvalProviderId = txtAiProofingConfig.providerId;
            }
            aiProviderDraftId = txtAiProofingConfig.providerId || aiProviders[0]?.id || "";
            aiProofingConfig = providerToProofingConfig(findAiProvider(txtAiProofingConfig.providerId), aiProofingConfig);
            const action = appSettings.txtEditorCloseAction;
            if (action === "exit" || action === "library") {
                txtEditorCloseAction = action;
            }
        } catch (error) {
            console.warn("读取书库设置失败:", error);
            aiProofingConfig = normalizeAiProofingConfig(undefined);
            aiProviders = normalizeAiProviders(undefined, aiProofingConfig);
            txtAiProofingConfig = {
                providerId: aiProviders[0]?.id || "",
                approvalProviderId: aiProviders[0]?.id || "",
            };
            aiProviderDraftId = txtAiProofingConfig.providerId;
        }
    }

    async function saveSharedAiProofingSettings() {
        aiProofingConfig = normalizeAiProofingConfig(aiProofingConfig);
        aiProviders = aiProviders.map(normalizeAiProvider);
        if (!txtAiProofingConfig.providerId || !findAiProvider(txtAiProofingConfig.providerId)) {
            txtAiProofingConfig.providerId = aiProviders[0]?.id || "";
        }
        if (!txtAiProofingConfig.approvalProviderId || !findAiProvider(txtAiProofingConfig.approvalProviderId)) {
            txtAiProofingConfig.approvalProviderId = txtAiProofingConfig.providerId;
        }
        const provider = findAiProvider(txtAiProofingConfig.providerId);
        if (provider) {
            aiProofingConfig = providerToProofingConfig(provider, aiProofingConfig);
        }
        aiSettingsMessage = "正在保存...";
        try {
            const currentGlobalSettings = loadGlobalAppSettings(libraryData?.config || {});
            const imageProviders = currentGlobalSettings.aiProviders.filter((provider) => provider.kind === "image");
            saveGlobalAppSettings({
                ...currentGlobalSettings,
                aiProofing: aiProofingConfig,
                aiProviders: [...aiProviders.map((provider) => ({ ...provider, kind: "text" as const })), ...imageProviders],
                txtAiProofing: txtAiProofingConfig,
            });
            aiSettingsMessage = "已保存，工具箱设置与 TXT 编辑器设置已同步";
        } catch (error) {
            console.error("保存智能校对设置失败:", error);
            aiSettingsMessage = `保存失败：${error}`;
        }
    }

    function openSettingsApiTab() {
        settingsActiveTab = "api";
        aiSettingsMessage = "";
        apiEditorOpen = false;
        apiEditorDraft = null;
        loadSharedLibrarySettings();
    }

    function openSettingsAiTab() {
        settingsActiveTab = "ai";
        aiSettingsMessage = "";
        loadSharedLibrarySettings();
    }

    function selectedTxtAiProvider() {
        return findAiProvider(aiProviderDraftId);
    }

    function updateSelectedTxtAiProvider(field: keyof AiProviderConfig, value: string | number) {
        if (!apiEditorDraft) return;
        apiEditorDraft = normalizeAiProvider({ ...apiEditorDraft, [field]: value }, 0);
    }

    function addTxtAiProvider() {
        apiEditorMode = "new";
        aiProviderDraftId = "";
        apiEditorDraft = createBlankAiProvider({ name: "" });
        aiSettingsMessage = "";
        apiEditorOpen = true;
    }

    function editTxtAiProvider(provider: AiProviderConfig) {
        apiEditorMode = "edit";
        aiProviderDraftId = provider.id;
        apiEditorDraft = { ...provider };
        aiSettingsMessage = "";
        apiEditorOpen = true;
    }

    function cancelTxtAiProviderEditor() {
        apiEditorOpen = false;
        apiEditorDraft = null;
        aiSettingsMessage = "";
    }

    function saveTxtAiProviderEditor() {
        if (!apiEditorDraft) return;
        const provider = normalizeAiProvider({
            ...apiEditorDraft,
            id: apiEditorMode === "edit" ? aiProviderDraftId : apiEditorDraft.id,
            name: apiEditorDraft.name.trim() || "文字 API",
            baseUrl: apiEditorDraft.baseUrl.trim(),
            model: apiEditorDraft.model.trim(),
        });
        if (!provider.name.trim() || !provider.baseUrl.trim() || !provider.model.trim()) {
            aiSettingsMessage = "请填写名称、API 地址和模型。";
            return;
        }
        if (apiEditorMode === "edit") {
            aiProviders = aiProviders.map((item) => (item.id === aiProviderDraftId ? provider : item));
        } else {
            aiProviders = [...aiProviders, provider];
        }
        aiProviderDraftId = provider.id;
        if (!txtAiProofingConfig.providerId) txtAiProofingConfig.providerId = provider.id;
        if (!txtAiProofingConfig.approvalProviderId) txtAiProofingConfig.approvalProviderId = provider.id;
        if (txtAiProofingConfig.providerId === provider.id) {
            aiProofingConfig = providerToProofingConfig(provider, aiProofingConfig);
        }
        apiEditorOpen = false;
        apiEditorDraft = null;
        aiSettingsMessage = "API 配置已保存，点击完成后写入设置。";
    }

    async function removeTxtAiProvider(providerId: string) {
        const wasEditing = apiEditorOpen && aiProviderDraftId === providerId;
        aiProviders = aiProviders.filter((provider) => provider.id !== providerId);
        const fallbackId = aiProviders[0]?.id || "";
        if (txtAiProofingConfig.providerId === providerId) txtAiProofingConfig.providerId = fallbackId;
        if (txtAiProofingConfig.approvalProviderId === providerId) txtAiProofingConfig.approvalProviderId = txtAiProofingConfig.providerId || fallbackId;
        aiProviderDraftId = fallbackId;
        if (wasEditing) {
            cancelTxtAiProviderEditor();
        }
        aiSettingsMessage = "API 配置已删除，点击完成后写入设置。";
    }

    async function openSettingsHistoryTab() {
        settingsActiveTab = "history";
        if (!filePath || filePath === "请打开一本小说...") {
            historyList = [];
            return;
        }
        historyList = await invoke("get_history_list", {
            originalPath: filePath,
        });
    }

    async function loadProofLogs(selectFirst = false) {
        proofLogMessage = "正在读取校对日志...";
        try {
            proofLogList = await invoke<ProofLogInfo[]>("list_ai_proofing_logs");
            proofLogMessage = proofLogList.length ? "" : "暂无校对日志";
            if (selectFirst && proofLogList.length > 0) {
                await openProofLog(proofLogList[0]);
            }
        } catch (error) {
            console.error("读取校对日志失败:", error);
            proofLogList = [];
            selectedProofLogPath = "";
            selectedProofLogContent = "";
            proofLogMessage = `读取校对日志失败：${error}`;
        }
    }

    async function openSettingsProofLogsTab() {
        settingsActiveTab = "proofLogs";
        await loadProofLogs(!selectedProofLogPath);
    }

    async function openProofLog(log: ProofLogInfo) {
        selectedProofLogPath = log.path;
        proofLogMessage = "正在打开日志...";
        try {
            selectedProofLogContent = await invoke<string>("read_ai_proofing_log", { path: log.path });
            proofLogMessage = "";
        } catch (error) {
            console.error("打开校对日志失败:", error);
            selectedProofLogContent = "";
            proofLogMessage = `打开日志失败：${error}`;
        }
    }

    async function loadImportedFonts() {
        try {
            importedFonts = await invoke<ImportedFontInfo[]>("list_library_fonts");
            syncManagedFontAssets();
            epubMeta = { ...epubMeta };
        } catch (error) {
            console.error("加载外部字体失败:", error);
            importedFonts = [];
            fontSettingsMessage = "读取字体目录失败";
        }
    }

    async function openSettingsFontsTab() {
        settingsActiveTab = "fonts";
        fontSettingsMessage = "";
        await loadImportedFonts();
    }

    async function loadStyleTemplates() {
        try {
            styleTemplates = await invoke<StyleTemplateInfo[]>("list_style_templates");
        } catch (error) {
            console.error("加载样式模板失败:", error);
            styleTemplates = [{ id: STYLE_TEMPLATE_BUILTIN_ID, name: "内置模板", file_name: "builtin.css", path: "", is_builtin: true }];
            styleSettingsMessage = "读取样式模板失败";
        }
    }

    async function readStyleTemplateCss(templateId: string) {
        const template = await invoke<StyleTemplateContent>("read_style_template", { id: templateId });
        return template.main_css.trim() || (templateId === STYLE_TEMPLATE_BUILTIN_ID ? getBuiltinStyleTemplateCss() : "");
    }

    async function applySelectedStyleTemplate(templateId = appSettings.selectedStyleTemplateId || STYLE_TEMPLATE_BUILTIN_ID) {
        currentStyleTemplateId = templateId;
        const selectedTemplate = styleTemplates.find((item) => item.id === templateId);
        currentStyleTemplateName = selectedTemplate?.name || "当前模板";
        selectedStyleTemplateCss = await readStyleTemplateCss(templateId);
        applyResolvedStyleTemplateCss(selectedStyleTemplateCss);
        styleSourceDraft = epubMeta.styles["main.css"];
    }

    async function openSettingsStylesTab() {
        settingsActiveTab = "styles";
        styleSettingsMessage = "";
        await loadStyleTemplates();
        applyResolvedStyleTemplateCss(epubMeta.styles["main.css"] || selectedStyleTemplateCss || getBuiltinStyleTemplateCss());
        styleSourceDraft = epubMeta.styles["main.css"];
        captureStylePanelBaseline();
    }

    async function importExternalFont() {
        if (isImportingFont) return;
        const selection = await open({
            multiple: false,
            filters: [{ name: "Font", extensions: ["ttf", "otf", "woff", "woff2"] }],
        });
        if (!selection) return;

        isImportingFont = true;
        fontSettingsMessage = "正在导入字体...";
        try {
            const imported = await invoke<ImportedFontInfo>("import_library_font", {
                path: extractPickedPath(selection as string | string[]),
            });
            await loadImportedFonts();
            syncManagedFontAssets();
            epubMeta = { ...epubMeta };
            fontSettingsMessage = `已导入字体：${imported.family}`;
        } catch (error) {
            console.error("导入字体失败:", error);
            fontSettingsMessage = `导入失败：${error}`;
        } finally {
            isImportingFont = false;
        }
    }

    async function renameImportedFont(font: ImportedFontInfo) {
        const nextFamily = window.prompt("输入新的字体显示名", font.family)?.trim();
        if (!nextFamily || nextFamily === font.family) return;

        renamingFontFileName = font.file_name;
        fontSettingsMessage = "正在重命名字体...";
        try {
            await invoke<ImportedFontInfo>("rename_library_font", {
                fileName: font.file_name,
                family: nextFamily,
            });
            await loadImportedFonts();
            syncManagedFontAssets();
            epubMeta = { ...epubMeta };
            fontSettingsMessage = `已重命名为：${nextFamily}`;
        } catch (error) {
            console.error("重命名字体失败:", error);
            fontSettingsMessage = `重命名失败：${error}`;
        } finally {
            renamingFontFileName = "";
        }
    }

    async function deleteImportedFont(font: ImportedFontInfo) {
        const confirmed = await ask(`确定删除字体“${font.family}”吗？`, {
            title: "删除字体",
            kind: "warning",
        });
        if (!confirmed) return;

        deletingFontFileName = font.file_name;
        fontSettingsMessage = "正在删除字体...";
        try {
            await invoke("delete_library_font", {
                fileName: font.file_name,
            });
            await loadImportedFonts();
            syncManagedFontAssets();
            epubMeta = { ...epubMeta };
            fontSettingsMessage = `已删除字体：${font.family}`;
        } catch (error) {
            console.error("删除字体失败:", error);
            fontSettingsMessage = `删除失败：${error}`;
        } finally {
            deletingFontFileName = "";
        }
    }

    async function importStyleTemplateFile() {
        if (isImportingStyleTemplate) return;
        const selection = await open({
            multiple: false,
            filters: [{ name: "CSS", extensions: ["css"] }],
        });
        if (!selection) return;

        isImportingStyleTemplate = true;
        styleSettingsMessage = "正在导入样式模板...";
        try {
            const imported = await invoke<StyleTemplateInfo>("import_style_template", {
                path: extractPickedPath(selection as string | string[]),
            });
            await loadStyleTemplates();
            appSettings.selectedStyleTemplateId = imported.id;
            styleSettingsMessage = `已导入模板：${imported.name}`;
        } catch (error) {
            console.error("导入样式模板失败:", error);
            styleSettingsMessage = `导入失败：${error}`;
        } finally {
            isImportingStyleTemplate = false;
        }
    }

    async function saveCurrentStyleTemplate() {
        if (isSavingStyleTemplate) return;
        isSavingStyleTemplate = true;
        styleSettingsMessage = "正在保存当前模板...";
        try {
            if (showStyleSourceEditor) {
                applyResolvedStyleTemplateCss(styleSourceDraft);
            } else {
                syncToolbarStyleToEpubMeta();
            }
            const targetId = appSettings.selectedStyleTemplateId || STYLE_TEMPLATE_BUILTIN_ID;
            await invoke<StyleTemplateInfo>("save_style_template", {
                id: targetId,
                mainCss: epubMeta.styles["main.css"],
            });
            await loadStyleTemplates();
            await applySelectedStyleTemplate(targetId);
            captureStylePanelBaseline();
            styleSettingsMessage = `已保存到模板：${currentStyleTemplateName}`;
        } catch (error) {
            console.error("保存样式模板失败:", error);
            styleSettingsMessage = `保存失败：${error}`;
        } finally {
            isSavingStyleTemplate = false;
        }
    }

    async function restoreBuiltinStyleTemplateToDefault() {
        styleSettingsMessage = "正在恢复内置模板...";
        try {
            await invoke("restore_builtin_style_template");
            await loadStyleTemplates();
            if ((appSettings.selectedStyleTemplateId || STYLE_TEMPLATE_BUILTIN_ID) === STYLE_TEMPLATE_BUILTIN_ID) {
                await applySelectedStyleTemplate(STYLE_TEMPLATE_BUILTIN_ID);
                captureStylePanelBaseline();
            }
            styleSettingsMessage = "内置模板已恢复默认样式";
        } catch (error) {
            console.error("恢复内置模板失败:", error);
            styleSettingsMessage = `恢复失败：${error}`;
        }
    }

    async function saveEditorSettings() {
        try {
            const vols = appSettings.customRegexRules.filter(r => r.level === 1).map(r => `(${r.pattern})`);
            const chaps = appSettings.customRegexRules.filter(r => r.level >= 2).map(r => `(${r.pattern})`);
            appSettings.volRegex = vols.length > 0 ? vols.join("|") : "^$";
            appSettings.chapRegex = chaps.length > 0 ? chaps.join("|") : "^$";
        } catch (e) {}

        if (!appSettings.selectedStyleTemplateId) {
            appSettings.selectedStyleTemplateId = STYLE_TEMPLATE_BUILTIN_ID;
        }
        localStorage.setItem(
            "app-settings",
            JSON.stringify(appSettings),
        );
        if (settingsActiveTab === "ai" || settingsActiveTab === "api") {
            await saveSharedAiProofingSettings();
        }
        await applySelectedStyleTemplate(appSettings.selectedStyleTemplateId);
        showSettingsPanel = false;
        scanToc();
    }

    async function runProofConvertPreview() {
        proofMessage = "正在查找可转换内容...";
        const scriptProfile = getChineseScriptProfile(fileContent);
        proofConvertPreviewRows = await buildChineseConvertPreview(
            fileContent,
            proofConvertDirection,
        );
        proofConvertSelectedIds = new Set();
        if (
            proofConvertPreviewRows.length === 0 &&
            scriptProfile.dominant === "simplified" &&
            proofConvertDirection === "simplified-to-traditional"
        ) {
            proofMessage = "检测到主体为简体，已跳过简体转繁体查找，避免误扫整本";
        } else if (
            proofConvertPreviewRows.length === 0 &&
            scriptProfile.dominant === "traditional" &&
            proofConvertDirection === "traditional-to-simplified"
        ) {
            proofMessage = "检测到主体为繁体，已跳过繁体转简体查找，避免误扫整本";
        } else {
            proofMessage =
                proofConvertPreviewRows.length > 0
                    ? `找到 ${proofConvertPreviewRows.length} 处可转换内容，默认全不选`
                    : "没有找到可转换内容";
        }
    }

    function toggleProofConvertRow(rowId: string, checked: boolean) {
        const next = new Set(proofConvertSelectedIds);
        if (checked) {
            next.add(rowId);
        } else {
            next.delete(rowId);
        }
        proofConvertSelectedIds = next;
    }

    function setAllProofConvertRows(checked: boolean) {
        proofConvertSelectedIds = checked
            ? new Set(proofConvertPreviewRows.map((row) => row.id))
            : new Set();
    }

    async function applySelectedConvertRows() {
        const result = applyChineseConvertPreview(
            fileContent,
            proofConvertPreviewRows,
            proofConvertSelectedIds,
        );
        await applyProofResult(result);
        proofConvertPreviewRows = [];
        proofConvertSelectedIds = new Set();
    }

    async function applyAllConvertRows() {
        const result = applyChineseConvertPreview(
            fileContent,
            proofConvertPreviewRows,
            proofConvertPreviewRows.map((row) => row.id),
        );
        await applyProofResult(result);
        proofConvertPreviewRows = [];
        proofConvertSelectedIds = new Set();
    }

    function jumpToProofConvertRow(row: ProofConvertPreviewRow) {
        editorComponent?.selectMatch(row.lineStart, row.startChar, row.endChar);
        proofMessage = "已定位到可转换内容";
    }

    function buildLineOffsets(text: string) {
        const offsets: number[] = [];
        let cursor = 0;
        const lines = text.split("\n");
        for (let i = 0; i < lines.length; i++) {
            offsets.push(cursor);
            cursor += lines[i].length + (i < lines.length - 1 ? 1 : 0);
        }
        return offsets;
    }

    function offsetToLineChar(offsets: number[], offset: number) {
        let low = 0;
        let high = offsets.length - 1;
        while (low <= high) {
            const mid = Math.floor((low + high) / 2);
            if (offsets[mid] <= offset) low = mid + 1;
            else high = mid - 1;
        }
        const lineIndex = Math.max(0, high);
        return { line: lineIndex + 1, char: offset - offsets[lineIndex] };
    }

    function getChapterRanges(): AiProofingChapterRange[] {
        const lines = fileContent.split("\n");
        const offsets = buildLineOffsets(fileContent);
        const chapters = flatToc
            .filter((node) => node.type === "Chapter")
            .sort((a, b) => a.line - b.line);
        if (chapters.length === 0) {
            return [{
                id: "full-text",
                title: "全文",
                startLine: 1,
                endLine: lines.length,
                startOffset: 0,
                text: fileContent,
            }];
        }
        return chapters.map((node, index) => {
            const next = chapters[index + 1];
            const startLine = Math.max(1, node.line);
            const endLine = next ? Math.max(startLine, next.line - 1) : lines.length;
            const startOffset = offsets[startLine - 1] ?? 0;
            const endOffset =
                endLine >= lines.length
                    ? fileContent.length
                    : offsets[endLine] - 1;
            return {
                id: node.id,
                title: node.title,
                parentId: node.parentId,
                startLine,
                endLine,
                startOffset,
                text: fileContent.slice(startOffset, endOffset),
            };
        });
    }

    function getEditorCursorLineNumber() {
        return editorComponent?.getCursorLine?.()?.number || null;
    }

    function findChapterRangeAtLine(chapters: AiProofingChapterRange[], lineNumber: number | null) {
        if (!lineNumber) return chapters[0] || null;
        return chapters.find((chapter) => lineNumber >= chapter.startLine && lineNumber <= chapter.endLine) || chapters[0] || null;
    }

    function currentProofingChapters(scope: AiProofingScope, cursorLine: number | null) {
        const chapters = getChapterRanges();
        if (scope === "current") {
            const current = findChapterRangeAtLine(chapters, cursorLine);
            return current ? [current] : chapters.slice(0, 1);
        }
        if (scope === "volume") {
            const current = findChapterRangeAtLine(chapters, cursorLine);
            if (!current) return chapters.slice(0, 1);
            if (!current.parentId) return [current];
            const volumeChapters = chapters.filter((chapter) => chapter.parentId === current.parentId);
            return volumeChapters.length > 0 ? volumeChapters : [current];
        }
        return chapters;
    }

    $: hasProofVolumeScope = flatToc.some((node) => node.type === "Chapter" && Boolean(node.parentId));
    $: if (!hasProofVolumeScope && aiProofingScope === "volume") {
        aiProofingScope = "current";
    }

    function appendAiProofingLog(message: string) {
        const time = new Date().toLocaleTimeString();
        aiProofingLogs = [...aiProofingLogs, `[${time}] ${message}`];
    }

    function isAiFullChapterRemoval(suggestion: AiProofingSuggestion) {
        const type = suggestion.type.toLowerCase().replace(/[\s-]+/g, "_");
        return type === "remove_chapter" || type === "chapter_delete" || type === "delete_chapter";
    }

    function differsOnlyByDeDiDe(original: string, replacement: string) {
        if (!original || original.length !== replacement.length) return false;
        let changed = false;
        for (let i = 0; i < original.length; i += 1) {
            if (original[i] === replacement[i]) continue;
            if (!/[的地得]/.test(original[i]) || !/[的地得]/.test(replacement[i])) {
                return false;
            }
            changed = true;
        }
        return changed;
    }

    function isBracketedNarrativeText(text: string) {
        const trimmed = text.trim();
        return /^[（(【\[][\s\S]{1,160}[）)】\]]$/.test(trimmed);
    }

    function containsEmojiOrSymbolOnly(text: string) {
        const trimmed = text.trim();
        if (!trimmed) return false;
        if (/[\uD800-\uDBFF][\uDC00-\uDFFF]/.test(trimmed)) return true;
        if (/^[^\p{L}\p{N}\s]{1,12}$/u.test(trimmed)) return true;
        if (/^[TtＴｔ]{2,}$/.test(trimmed)) return true;
        if (/^[QAQqwQW；;:：'"\-_=+~^*#@!！?？<>/\\|()[\]{}【】（）…。，、·]+$/u.test(trimmed)) return true;
        return false;
    }

    function changesLeadingIndent(original: string, replacement: string) {
        const originalIndent = original.match(/^\s+/)?.[0] || "";
        const replacementIndent = replacement.match(/^\s+/)?.[0] || "";
        return originalIndent.length > replacementIndent.length;
    }

    function isShortNameLikeReplacement(original: string, replacement: string) {
        const o = original.trim();
        const r = replacement.trim();
        if (o === r || o.length > 4 || r.length > 4) return false;
        if (/^[A-Za-z]{1,4}[\u4e00-\u9fff]+$/.test(o) && /^[\u4e00-\u9fff]+$/.test(r)) return true;
        return /^[\u4e00-\u9fffA-Za-z·.]{1,4}$/.test(o) && /^[\u4e00-\u9fffA-Za-z·.]{1,4}$/.test(r);
    }

    function isStylePreferenceReplacement(original: string, replacement: string) {
        const pair = `${original.trim()}=>${replacement.trim()}`;
        return /如同=>如果|好像=>像|仿佛=>像|似乎=>好像|大概=>可能|也许=>可能|可以=>能够|一些=>一系列/.test(pair);
    }

    function getAiProofingRiskReason(row: AiProofingRow) {
        const type = row.type.toLowerCase().replace(/[\s-]+/g, "_");
        if (row.fullChapterRemoval || type === "remove_chapter") return "整章移除风险高";
        if ((type === "delete" || row.replacement === "") && containsEmojiOrSymbolOnly(row.original)) return "疑似表情/特殊符号";
        if ((type === "delete" || row.replacement === "") && isBracketedNarrativeText(row.original)) return "括号内描述不删除";
        if (changesLeadingIndent(row.original, row.replacement)) return "保留段首缩进";
        if (isShortNameLikeReplacement(row.original, row.replacement)) return "疑似人名/专名";
        if (isStylePreferenceReplacement(row.original, row.replacement)) return "疑似风格改写";
        return "";
    }

    function isAiProofingUnsafeSuggestion(row: AiProofingRow) {
        return Boolean(getAiProofingRiskReason(row));
    }

    function isAiProofingHardBlockedSuggestion(row: AiProofingRow) {
        const type = row.type.toLowerCase().replace(/[\s-]+/g, "_");
        if (row.fullChapterRemoval || type === "remove_chapter") return true;
        if ((type === "delete" || row.replacement === "") && containsEmojiOrSymbolOnly(row.original)) return true;
        if ((type === "delete" || row.replacement === "") && isBracketedNarrativeText(row.original)) return true;
        if (changesLeadingIndent(row.original, row.replacement)) return true;
        return false;
    }

    function isAiProofingAutoApplySuggestion(suggestion: AiProofingSuggestion) {
        const type = suggestion.type.toLowerCase().replace(/[\s-]+/g, "_");
        const reason = `${suggestion.reason} ${suggestion.original} ${suggestion.replacement}`;
        return (
            type === "punctuation" ||
            reason.includes("的地得") ||
            differsOnlyByDeDiDe(suggestion.original, suggestion.replacement) ||
            /[的地得][”"]?\s*应为\s*[“"]?[的地得]/.test(reason)
        );
    }

    function buildLocalEllipsisRows(chapter: { id: string; title: string; startOffset: number; text: string }, lineOffsets: number[]) {
        const rows: AiProofingRow[] = [];
        const pattern = /(\.{3,}|。{2,}|…+)/g;
        let match: RegExpExecArray | null;
        while ((match = pattern.exec(chapter.text))) {
            const original = match[0];
            if (original === "……") continue;
            const before = chapter.text[Math.max(0, match.index - 1)] || "";
            const after = chapter.text[match.index + original.length] || "";
            if (/^\.+$/.test(original) && /[A-Za-z0-9]/.test(before) && /[A-Za-z0-9]/.test(after)) {
                continue;
            }
            const globalStart = chapter.startOffset + match.index;
            const globalEnd = globalStart + original.length;
            const pos = offsetToLineChar(lineOffsets, globalStart);
            rows.push({
                id: `${chapter.id}-ellipsis-${globalStart}`,
                chapterTitle: chapter.title,
                original,
                replacement: "……",
                reason: "省略号应使用标准完整省略号",
                type: "punctuation",
                confidence: 1,
                lineStart: pos.line,
                startChar: pos.char,
                endChar: pos.char + original.length,
                globalStart,
                globalEnd,
            });
        }
        return rows;
    }

    function getAiProofingCacheKey(path = filePath) {
        if (!path || path === "请打开一本小说...") return "";
        return `ai-proofing-cache:${path}`;
    }

    function compactAiProofingLogText(text: string, limit = 180) {
        const compact = text.replace(/\s+/g, " ").trim();
        return compact.length > limit ? `${compact.slice(0, limit)}...` : compact;
    }

    function logAiProofingSuggestion(stage: string, row: AiProofingRow | AiProofingSuggestion, chapterTitle: string, extra = "") {
        appendAiProofingLog(
            [
                `建议/${stage}：${chapterTitle}`,
                `${row.type} ${(Number(row.confidence || 0) * 100).toFixed(0)}%`,
                `${compactAiProofingLogText(row.original)} -> ${compactAiProofingLogText(row.replacement || "删除")}`,
                row.reason ? `原因：${row.reason}` : "",
                extra,
            ].filter(Boolean).join("；"),
        );
    }

    function buildAiProofingLogContent(logPath = "") {
        const rows = aiProofingRows.map((row) => ({
            chapterTitle: row.chapterTitle,
            original: row.original,
            replacement: row.replacement,
            reason: row.reason,
            type: row.type,
            confidence: row.confidence,
            lineStart: row.lineStart,
            fullChapterRemoval: Boolean(row.fullChapterRemoval),
        }));
        const header = [
            `TXT: ${filePath}`,
            `模型: ${aiProofingConfig.model}`,
            `范围: ${aiProofingScope === "all" ? "全书逐章" : aiProofingScope === "volume" ? "当前卷" : "当前章"}`,
            `时间: ${new Date().toLocaleString()}`,
            logPath ? `日志文件: ${logPath}` : "",
        ].filter(Boolean);
        return [
            ...header,
            "",
            "===== 日志 =====",
            aiProofingLogs.join("\n") || "暂无日志",
            "",
            "===== 当前人工建议 =====",
            rows.length ? JSON.stringify(rows, null, 2) : "暂无待人工审核建议",
        ].join("\n");
    }

    function saveAiProofingCache(logPath = "") {
        const key = getAiProofingCacheKey();
        if (!key) return;
        const state: AiProofingCacheState = {
            filePath,
            contentMd5: epubMeta.md5 || "",
            savedAt: Date.now(),
            model: aiProofingConfig.model,
            scope: aiProofingScope,
            view: aiProofingView,
            rows: aiProofingRows,
            selectedIds: Array.from(aiProofingSelectedIds),
            logs: aiProofingLogs,
            approvalBatches: aiApprovalAppliedBatches,
            message: proofMessage,
            logPath,
        };
        try {
            localStorage.setItem(key, JSON.stringify(state));
        } catch (error) {
            console.warn("保存智能校对缓存失败:", error);
        }
    }

    function loadAiProofingCache() {
        const key = getAiProofingCacheKey();
        if (!key) return;
        try {
            const raw = localStorage.getItem(key);
            if (!raw) return;
            const state = JSON.parse(raw) as Partial<AiProofingCacheState>;
            if (!Array.isArray(state.rows) || !Array.isArray(state.logs)) return;
            if (state.contentMd5 && epubMeta.md5 && state.contentMd5 !== epubMeta.md5) {
                aiProofingRows = [];
                aiProofingSelectedIds = new Set();
                aiProofingLogs = [`[${new Date().toLocaleTimeString()}] 已跳过旧校对缓存：当前文本内容已变化`];
                proofMessage = "旧智能校对缓存已跳过：当前文本内容已变化";
                return;
            }
            aiProofingScope = state.scope === "all" || state.scope === "volume" || state.scope === "current" ? state.scope : "current";
            aiProofingView = state.view === "log" || state.view === "approval" ? state.view : "suggestions";
            aiProofingRows = state.rows as AiProofingRow[];
            const validIds = new Set(aiProofingRows.map((row) => row.id));
            aiProofingSelectedIds = new Set((state.selectedIds || []).filter((id) => validIds.has(id)));
            aiProofingLogs = state.logs;
            aiApprovalAppliedBatches = Array.isArray(state.approvalBatches) ? state.approvalBatches : [];
            proofMessage = state.message || (aiProofingRows.length ? `已恢复 ${aiProofingRows.length} 条智能校对建议` : "");
        } catch (error) {
            console.warn("读取智能校对缓存失败:", error);
        }
    }

    async function persistAiProofingLogFile() {
        if (!filePath || filePath === "请打开一本小说..." || aiProofingLogs.length === 0) {
            saveAiProofingCache();
            return "";
        }
        try {
            const path = await invoke<string>("save_ai_proofing_log", {
                txtPath: filePath,
                model: aiProofingConfig.model,
                content: buildAiProofingLogContent(),
            });
            appendAiProofingLog(`日志已保存：${path}`);
            saveAiProofingCache(path);
            return path;
        } catch (error) {
            appendAiProofingLog(`日志保存失败：${error}`);
            saveAiProofingCache();
            return "";
        }
    }

    function parseAiProofingSuggestions(content: string): AiProofingSuggestion[] {
        const trimmed = content.trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/i, "");
        let parsed: any;
        try {
            parsed = JSON.parse(trimmed);
        } catch (error) {
            const recovered = recoverAiProofingSuggestionsFromPartialJson(trimmed);
            if (recovered.length > 0) return recovered;
            throw error;
        }
        const source = Array.isArray(parsed) ? parsed : parsed?.suggestions;
        if (!Array.isArray(source)) return [];
        return normalizeAiProofingSuggestions(source);
    }

    function normalizeAiProofingSuggestions(source: any[]): AiProofingSuggestion[] {
        return source
            .map((item: any) => ({
                original: String(item?.original || ""),
                replacement: String(item?.replacement ?? ""),
                reason: readAiProofingReason(item),
                type: String(item?.type || "other"),
                confidence: Math.max(0, Math.min(1, Number(item?.confidence) || 0)),
            }))
            .filter((item) => item.original && item.original !== item.replacement);
    }

    function readAiProofingReason(item: any) {
        return String(
            item?.reason ??
            item?.["原因"] ??
            item?.explanation ??
            item?.["理由"] ??
            item?.note ??
            item?.comment ??
            "",
        ).trim();
    }

    function recoverAiProofingSuggestionsFromPartialJson(content: string): AiProofingSuggestion[] {
        const suggestions: any[] = [];
        const objectPattern =
            /\{\s*"original"\s*:\s*"((?:\\.|[^"\\])*)"\s*,\s*"replacement"\s*:\s*"((?:\\.|[^"\\])*)"\s*,\s*"reason"\s*:\s*"((?:\\.|[^"\\])*)"\s*,\s*"type"\s*:\s*"((?:\\.|[^"\\])*)"\s*,\s*"confidence"\s*:\s*([0-9.]+)[^}]*\}/g;
        let match: RegExpExecArray | null;
        while ((match = objectPattern.exec(content))) {
            try {
                suggestions.push({
                    original: JSON.parse(`"${match[1]}"`),
                    replacement: JSON.parse(`"${match[2]}"`),
                    reason: JSON.parse(`"${match[3]}"`),
                    type: JSON.parse(`"${match[4]}"`),
                    confidence: Number(match[5]),
                });
            } catch (_) {}
        }
        return normalizeAiProofingSuggestions(suggestions);
    }

    function buildAiUserPrompt(chapterTitle: string, chapterText: string) {
        const extra = aiProofingConfig.extraPrompt.trim();
        return [
            `章节标题：${chapterTitle || "未命名章节"}`,
            extra ? `用户额外要求：${extra}` : "",
            "请根据系统校对标准检查以下章节，只输出 JSON：",
            chapterText,
        ].filter(Boolean).join("\n\n");
    }

    function parseAiApprovalDecisions(content: string): AiProofingApproval[] {
        const trimmed = content.trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/i, "");
        const parsed = JSON.parse(trimmed);
        const source = Array.isArray(parsed) ? parsed : parsed?.decisions;
        if (!Array.isArray(source)) return [];
        return source
            .map((item: any) => ({
                original: String(item?.original || ""),
                approved: Boolean(item?.approved),
                reason: readAiProofingReason(item),
            }))
            .filter((item) => item.original);
    }

    function buildAiApprovalPrompt(chapterTitle: string, chapterText: string, rows: AiProofingRow[]) {
        const suggestions = rows.map((row) => ({
            original: row.original,
            replacement: row.replacement,
            reason: row.reason,
            type: row.type,
            confidence: row.confidence,
        }));
        return [
            `章节标题：${chapterTitle || "未命名章节"}`,
            "正文：",
            chapterText,
            "待审批建议 JSON：",
            JSON.stringify({ suggestions }, null, 2),
        ].join("\n\n");
    }

    async function approveAiProofingRows(chapterTitle: string, chapterText: string, rows: AiProofingRow[]) {
        if (!aiProofingConfig.autoApprove || rows.length === 0) {
            return { approvedRows: [] as AiProofingRow[], pendingRows: rows };
        }
        appendAiProofingLog(`自动审批：${chapterTitle}，待审 ${rows.length} 条`);
        try {
            const response = await invoke<AiProofingResponse>("run_ai_proofing", {
                request: {
                    config: proofingConfigForProvider(txtAiProofingConfig.approvalProviderId),
                    systemPrompt: AI_APPROVAL_SYSTEM_PROMPT,
                    userPrompt: buildAiApprovalPrompt(chapterTitle, chapterText, rows),
                },
            });
            const decisions = parseAiApprovalDecisions(response.content);
            const approvedKeys = new Set(
                decisions.filter((item) => item.approved).map((item) => item.original),
            );
            const approvedRows = rows.filter((row) => approvedKeys.has(row.original));
            const pendingRows = rows.filter((row) => !approvedKeys.has(row.original));
            appendAiProofingLog(`自动审批完成：通过 ${approvedRows.length} 条，存疑 ${pendingRows.length} 条`);
            return { approvedRows, pendingRows };
        } catch (error) {
            appendAiProofingLog(`自动审批失败，全部转人工审核：${error}`);
            return { approvedRows: [] as AiProofingRow[], pendingRows: rows };
        }
    }

    async function runAiProofing() {
        await loadSharedLibrarySettings();
        aiProofingConfig = proofingConfigForProvider(txtAiProofingConfig.providerId);
        if (!aiProofingConfig.apiKey.trim() || !aiProofingConfig.baseUrl.trim() || !aiProofingConfig.model.trim()) {
            proofMessage = "请先补全 API 地址、Key 和模型名";
            return;
        }
        if (!fileContent.trim()) {
            proofMessage = "请先打开文本文件";
            return;
        }
        if (flatToc.length === 0) {
            await scanToc(fileContent);
        }

        const cursorLine = getEditorCursorLineNumber();
        const chapters = currentProofingChapters(aiProofingScope, cursorLine);
        aiProofingRows = [];
        aiProofingSelectedIds = new Set();
        aiProofingLogs = [];
        aiProofingView = "suggestions";
        aiProofingRunning = true;
        aiProofingAbortRequested = false;
        const lineOffsets = buildLineOffsets(fileContent);
        let skipped = 0;
        let accepted = 0;
        let pendingRows: AiProofingRow[] = [];
        let autoApplyRows: AiProofingRow[] = [];
        let approvedApplyRows: AiProofingRow[] = [];

        try {
            const scopeLabel = aiProofingScope === "all" ? "全书逐章" : aiProofingScope === "volume" ? "当前卷" : "当前章";
            appendAiProofingLog(`开始智能校对：${scopeLabel}，指针行 ${cursorLine || "未知"}，共 ${chapters.length} 章`);
            for (let i = 0; i < chapters.length; i++) {
                if (aiProofingAbortRequested) {
                    appendAiProofingLog("用户请求停止，结束剩余章节");
                    break;
                }
                const chapter = chapters[i];
                if (chapter.text.length > aiProofingConfig.maxChapterChars) {
                    skipped++;
                    proofMessage = `跳过超长章节：${chapter.title}`;
                    appendAiProofingLog(`跳过超长章节 ${i + 1}/${chapters.length}：${chapter.title}（${chapter.text.length} 字）`);
                    continue;
                }
                proofMessage = `智能校对中 ${i + 1}/${chapters.length}：${chapter.title}`;
                appendAiProofingLog(`请求章节 ${i + 1}/${chapters.length}：${chapter.title}（发送 ${chapter.text.length}/${chapter.text.length} 字）`);
                let suggestions: AiProofingSuggestion[] = [];
                try {
                    const response = await invoke<AiProofingResponse>("run_ai_proofing", {
                        request: {
                            config: aiProofingConfig,
                            systemPrompt: AI_PROOFING_SYSTEM_PROMPT,
                            userPrompt: buildAiUserPrompt(chapter.title, chapter.text),
                        },
                    });
                    try {
                        suggestions = parseAiProofingSuggestions(response.content);
                    } catch (parseError) {
                        appendAiProofingLog(`响应 JSON 解析失败：${chapter.title}；${parseError}`);
                        appendAiProofingLog(`响应片段：${response.content.slice(0, 500).replace(/\s+/g, " ")}`);
                        throw parseError;
                    }
                    if (suggestions.length > 0 && !response.content.trim().endsWith("}")) {
                        appendAiProofingLog(`响应可能被截断，已采用 ${suggestions.length} 条完整建议`);
                    }
                    for (const suggestion of suggestions) {
                        logAiProofingSuggestion("返回", suggestion, chapter.title);
                    }
                } catch (error) {
                    appendAiProofingLog(`章节失败：${chapter.title}；${error}`);
                    throw error;
                }
                const chapterRows: AiProofingRow[] = [];
                const usedOffsets = new Set<number>();
                const localEllipsisRows = buildLocalEllipsisRows(chapter, lineOffsets);
                for (const row of localEllipsisRows) {
                    usedOffsets.add(row.globalStart - chapter.startOffset);
                }
                for (const suggestion of suggestions) {
                    const fullChapterRemoval = isAiFullChapterRemoval(suggestion);
                    const localStart = fullChapterRemoval ? 0 : chapter.text.indexOf(suggestion.original);
                    if (localStart < 0 || usedOffsets.has(localStart)) {
                        appendAiProofingLog(`忽略未匹配建议：${chapter.title} / ${suggestion.type} / ${suggestion.reason || suggestion.original.slice(0, 20)}`);
                        logAiProofingSuggestion("未匹配", suggestion, chapter.title);
                        continue;
                    }
                    usedOffsets.add(localStart);
                    const globalStart = chapter.startOffset + localStart;
                    const globalEnd = fullChapterRemoval
                        ? chapter.startOffset + chapter.text.length
                        : globalStart + suggestion.original.length;
                    const pos = offsetToLineChar(lineOffsets, globalStart);
                    chapterRows.push({
                        ...suggestion,
                        id: `${chapter.id}-${globalStart}-${chapterRows.length}`,
                        chapterTitle: chapter.title,
                        original: fullChapterRemoval ? chapter.title : suggestion.original,
                        replacement: fullChapterRemoval ? "" : suggestion.replacement,
                        fullChapterRemoval,
                        lineStart: pos.line,
                        startChar: pos.char,
                        endChar: fullChapterRemoval ? pos.char + Math.min(chapter.title.length || 1, chapter.text.length) : pos.char + suggestion.original.length,
                        globalStart,
                        globalEnd,
                    });
                }
                const allChapterRows = [...localEllipsisRows, ...chapterRows];
                const safeChapterRows: AiProofingRow[] = [];
                const blockedManualRows: AiProofingRow[] = [];
                let blockedRows = 0;
                for (const row of allChapterRows) {
                    const riskReason = getAiProofingRiskReason(row);
                    if (riskReason && isAiProofingHardBlockedSuggestion(row)) {
                        blockedRows++;
                        appendAiProofingLog(`拦截高风险建议：${row.original} -> ${row.replacement || "删除"}（${riskReason}）`);
                        logAiProofingSuggestion("拦截", row, row.chapterTitle, `风险：${riskReason}`);
                        blockedManualRows.push(row);
                    } else {
                        safeChapterRows.push(row);
                    }
                }
                const autoRows = safeChapterRows.filter((row) => isAiProofingAutoApplySuggestion(row));
                const manualCandidates = safeChapterRows.filter((row) => !isAiProofingAutoApplySuggestion(row));
                const approvalResult = await approveAiProofingRows(chapter.title, chapter.text, manualCandidates);
                const approvedRows = approvalResult.approvedRows;
                const manualRows = [...approvalResult.pendingRows, ...blockedManualRows];
                if (autoRows.length > 0) {
                    appendAiProofingLog(`自动应用：${chapter.title}，${autoRows.length} 条标点/的地得修正`);
                    for (const row of autoRows) {
                        appendAiProofingLog(`自动：${row.original} -> ${row.replacement || "删除"}（${row.reason}）`);
                        logAiProofingSuggestion("自动应用", row, row.chapterTitle);
                    }
                }
                if (approvedRows.length > 0) {
                    appendAiProofingLog(`审批通过自动应用：${chapter.title}，${approvedRows.length} 条`);
                    for (const row of approvedRows) {
                        appendAiProofingLog(`审批通过：${row.original} -> ${row.replacement || "删除"}（${row.reason}）`);
                        logAiProofingSuggestion("审批通过", row, row.chapterTitle);
                    }
                }
                for (const row of manualRows) {
                    logAiProofingSuggestion("人工待审", row, row.chapterTitle);
                }
                autoApplyRows = [...autoApplyRows, ...autoRows];
                approvedApplyRows = [...approvedApplyRows, ...approvedRows];
                pendingRows = [...pendingRows, ...manualRows];
                accepted += manualRows.length;
                aiProofingRows = pendingRows;
                appendAiProofingLog(`完成章节：${chapter.title}，返回 ${suggestions.length} 条，本地 ${localEllipsisRows.length} 条，自动 ${autoRows.length + approvedRows.length} 条，建议 ${manualRows.length} 条，拦截 ${blockedRows} 条`);
            }
            if (autoApplyRows.length > 0) {
                const result = applyAiProofingRowsToText(autoApplyRows, fileContent);
                if (result.changed > 0) {
                    await applyProofResult({
                        text: result.text,
                        changedCount: result.changed,
                        message: `已自动应用 ${result.changed} 条标点/的地得修正`,
                    });
                    pendingRows = shiftAiProofingRowsAfterEdits(pendingRows, result.edits);
                    approvedApplyRows = shiftAiProofingRowsAfterEdits(approvedApplyRows, result.edits);
                    aiProofingRows = pendingRows;
                    appendAiProofingLog(`已自动写入 ${result.changed} 条修正`);
                } else {
                    appendAiProofingLog("自动修正未写入：原文已变化或未匹配");
                }
            }
            if (approvedApplyRows.length > 0) {
                const beforeText = fileContent;
                const result = applyAiProofingRowsToText(approvedApplyRows, fileContent, { allowUnsafe: true });
                if (result.changed > 0) {
                    await applyProofResult({
                        text: result.text,
                        changedCount: result.changed,
                        message: `已应用 ${result.changed} 条 AI 审批建议`,
                    });
                    pendingRows = shiftAiProofingRowsAfterEdits(pendingRows, result.edits);
                    aiProofingRows = pendingRows;
                    aiApprovalAppliedBatches = [
                        ...aiApprovalAppliedBatches,
                        {
                            id: `approval-${Date.now()}`,
                            chapterTitle: approvedApplyRows[0]?.chapterTitle || "AI 审批",
                            rows: approvedApplyRows.filter((row) => result.appliedIds.has(row.id)),
                            beforeText,
                            afterText: result.text,
                            appliedAt: Date.now(),
                            reverted: false,
                        },
                    ];
                    appendAiProofingLog(`已写入 AI 审批修正 ${result.changed} 条，可在审批页撤销`);
                } else {
                    appendAiProofingLog("AI 审批修正未写入：原文已变化或未匹配");
                }
            }
            await updateMd5(fileContent);
            aiProofingSelectedIds = new Set(aiProofingRows.map((row) => row.id));
            proofMessage = aiProofingAbortRequested
                ? `已停止，找到 ${aiProofingRows.length} 条建议`
                : `智能校对完成，找到 ${accepted} 条建议${skipped ? `，跳过 ${skipped} 个超长章节` : ""}`;
            appendAiProofingLog(proofMessage);
        } catch (error) {
            console.error("智能校对失败:", error);
            proofMessage = `智能校对失败：${error}`;
            appendAiProofingLog(proofMessage);
        } finally {
            aiProofingRunning = false;
            aiProofingAbortRequested = false;
            await persistAiProofingLogFile();
        }
    }

    function applyAiProofingRowsToText(rows: AiProofingRow[], sourceText: string, options: { allowUnsafe?: boolean } = {}) {
        const sorted = [...rows].sort((a, b) => b.globalStart - a.globalStart);
        let text = sourceText;
        let changed = 0;
        const appliedIds = new Set<string>();
        const edits: Array<{ start: number; oldLength: number; newLength: number }> = [];

        for (const row of sorted) {
            if (!options.allowUnsafe && isAiProofingUnsafeSuggestion(row)) continue;
            const removeLength = row.globalEnd - row.globalStart;
            const current = text.slice(row.globalStart, row.globalStart + removeLength);
            if (row.fullChapterRemoval) {
                continue;
            }
            if (current !== row.original) continue;
            text = text.slice(0, row.globalStart) + row.replacement + text.slice(row.globalStart + row.original.length);
            changed++;
            appliedIds.add(row.id);
            edits.push({ start: row.globalStart, oldLength: row.original.length, newLength: row.replacement.length });
        }

        return { text, changed, appliedIds, edits };
    }

    function shiftAiProofingRowsAfterEdits(rows: AiProofingRow[], edits: Array<{ start: number; oldLength: number; newLength: number }>) {
        if (edits.length === 0) return rows;
        const orderedEdits = [...edits].sort((a, b) => a.start - b.start);
        return rows.map((row) => {
            let delta = 0;
            for (const edit of orderedEdits) {
                if (edit.start < row.globalStart) {
                    delta += edit.newLength - edit.oldLength;
                }
            }
            if (delta === 0) return row;
            return {
                ...row,
                globalStart: row.globalStart + delta,
                globalEnd: row.globalEnd + delta,
            };
        });
    }

    function stopAiProofing() {
        aiProofingAbortRequested = true;
        proofMessage = "正在停止智能校对...";
    }

    function toggleAiProofingRow(rowId: string, checked: boolean) {
        const next = new Set(aiProofingSelectedIds);
        if (checked) next.add(rowId);
        else next.delete(rowId);
        aiProofingSelectedIds = next;
        saveAiProofingCache();
    }

    function setAllAiProofingRows(checked: boolean) {
        aiProofingSelectedIds = checked
            ? new Set(aiProofingRows.map((row) => row.id))
            : new Set();
        saveAiProofingCache();
    }

    function jumpToAiProofingRow(row: AiProofingRow) {
        editorComponent?.selectMatch(row.lineStart, row.startChar, row.endChar);
        proofMessage = "已定位到智能校对建议";
    }

    function compactAiTooltipText(value: string, maxLength = 160) {
        const compact = value.replace(/\s+/g, " ").trim();
        return compact.length > maxLength
            ? `${compact.slice(0, maxLength - 1)}…`
            : compact;
    }

    function compactAiProofingCellText(value: string, maxLength = 34) {
        return compactAiTooltipText(value || "删除", maxLength);
    }

    function aiProofingReasonText(row: Pick<AiProofingSuggestion, "reason" | "type">) {
        const reason = String(row.reason || "").trim();
        if (reason) return reason;
        const type = String(row.type || "").trim();
        return type ? `未返回具体原因（类型：${type}）` : "未返回具体原因";
    }

    function buildAiProofingReplacementPreview(row: AiProofingRow) {
        if (row.fullChapterRemoval) {
            return "修改后：整章移除";
        }
        return `修改后：${compactAiTooltipText(row.replacement || "删除")}`;
    }

    function buildAiProofingReplacementTooltip(row: AiProofingRow) {
        return [
            `类型：${row.type}`,
            `置信度：${(row.confidence * 100).toFixed(0)}%`,
            `原因：${aiProofingReasonText(row)}`,
            buildAiProofingReplacementPreview(row),
        ].join("\n");
    }

    async function applySelectedAiProofingRows() {
        const selected = aiProofingRows
            .filter((row) => aiProofingSelectedIds.has(row.id))
            .sort((a, b) => b.globalStart - a.globalStart);
        if (selected.length === 0) return;
        const result = applyAiProofingRowsToText(selected, fileContent, { allowUnsafe: true });
        await applyProofResult({
            text: result.text,
            changedCount: result.changed,
            message: result.changed > 0 ? `已应用 ${result.changed} 条智能校对建议` : "原文已变化，未应用任何建议",
        });
        if (result.changed > 0) {
            const remainingRows = aiProofingRows.filter((row) => !result.appliedIds.has(row.id));
            aiProofingRows = shiftAiProofingRowsAfterEdits(remainingRows, result.edits);
            aiProofingSelectedIds = new Set(aiProofingRows.map((row) => row.id));
            appendAiProofingLog(`人工应用：${result.changed} 条，剩余 ${aiProofingRows.length} 条`);
        }
        saveAiProofingCache();
    }

    async function revertAiApprovalRow(batchId: string, rowId: string) {
        const batch = aiApprovalAppliedBatches.find((item) => item.id === batchId);
        if (!batch || batch.reverted || fileContent !== batch.afterText) {
            proofMessage = "当前文本已变化，无法安全撤销该审批";
            appendAiProofingLog(`审批撤销失败：${batch?.chapterTitle || batchId} / ${rowId}，文本已变化`);
            return;
        }
        const targetRow = batch.rows.find((row) => row.id === rowId);
        if (!targetRow) {
            proofMessage = "未找到对应审批结果";
            return;
        }
        if (batch.revertedRowIds?.includes(rowId)) {
            proofMessage = "该审批结果已撤销";
            return;
        }
        const revertedRows = new Set(batch.revertedRowIds || []);
        const remainingRows = batch.rows.filter((row) => !revertedRows.has(row.id) && row.id !== rowId);
        const reapplyResult = applyAiProofingRowsToText(remainingRows, batch.beforeText);
        if (reapplyResult.changed !== remainingRows.length) {
            proofMessage = "当前文本已变化，无法安全撤销该审批";
            appendAiProofingLog(`审批撤销失败：${batch.chapterTitle} / ${targetRow.original}，重建文本未完全匹配`);
            return;
        }
        await applyProofResult({
            text: reapplyResult.text,
            changedCount: 1,
            message: `已撤销 1 条 AI 审批改动`,
        });
        aiApprovalAppliedBatches = aiApprovalAppliedBatches.map((item) =>
            item.id === batchId
                ? {
                    ...item,
                    afterText: reapplyResult.text,
                    reverted: remainingRows.length === 0,
                    revertedRowIds: [...revertedRows, rowId],
                }
                : item,
        );
        appendAiProofingLog(`审批撤销：${batch.chapterTitle} / ${targetRow.original}`);
        saveAiProofingCache();
    }

    async function reapplyAiApprovalRow(batchId: string, rowId: string) {
        const batch = aiApprovalAppliedBatches.find((item) => item.id === batchId);
        if (!batch || fileContent !== batch.afterText) {
            proofMessage = "当前文本已变化，无法安全恢复该审批";
            appendAiProofingLog(`审批恢复失败：${batch?.chapterTitle || batchId} / ${rowId}，文本已变化`);
            return;
        }
        const targetRow = batch.rows.find((row) => row.id === rowId);
        if (!targetRow) {
            proofMessage = "未找到对应审批结果";
            return;
        }
        const revertedRows = new Set(batch.revertedRowIds || []);
        if (!revertedRows.has(rowId)) {
            proofMessage = "该审批结果当前已处于应用状态";
            return;
        }
        const activeRows = batch.rows.filter((row) => !revertedRows.has(row.id));
        const nextRows = [...activeRows, targetRow].sort((a, b) => b.globalStart - a.globalStart);
        const reapplyResult = applyAiProofingRowsToText(nextRows, batch.beforeText);
        if (reapplyResult.changed !== nextRows.length) {
            proofMessage = "当前文本已变化，无法安全恢复该审批";
            appendAiProofingLog(`审批恢复失败：${batch.chapterTitle} / ${targetRow.original}，重建文本未完全匹配`);
            return;
        }
        await applyProofResult({
            text: reapplyResult.text,
            changedCount: 1,
            message: `已恢复 1 条 AI 审批改动`,
        });
        revertedRows.delete(rowId);
        aiApprovalAppliedBatches = aiApprovalAppliedBatches.map((item) =>
            item.id === batchId
                ? {
                    ...item,
                    afterText: reapplyResult.text,
                    reverted: false,
                    revertedRowIds: Array.from(revertedRows),
                }
                : item,
        );
        appendAiProofingLog(`审批恢复：${batch.chapterTitle} / ${targetRow.original}`);
        saveAiProofingCache();
    }

    // 查找替换状态
    let findPattern = "";
    let replacePattern = "";
    let replaceMsg = "";
    let isRegex = false;
    let allMatches: MatchLocation[] = [];
    let currentMatchIndex = -1;

    // 校对面板状态
    let proofActiveTab: "toc" | "builtin" | "check" | "ai" = "check";
    let proofTextTool: "builtin" | "convert" = "builtin";
    let proofTitleScope: ProofTitleScope = "all";
    let proofTitleRegex = "";
    let proofVolumeNumberStyle: ProofNumberStyle = "chinese";
    let proofChapterNumberStyle: ProofNumberStyle = "arabic";
    let proofPerVolume = false;
    let proofPreviewRows: ProofTitlePreviewRow[] = [];
    let proofCollapsedVolumeKeys = new Set<string>();
    let proofPreviewMessage = "";
    let proofBuiltinRule: ProofBuiltinRuleId = "title-brackets";
    let proofRegexPreviewRows: ProofRegexPreviewRow[] = [];
    let proofRegexSelectedIds = new Set<string>();
    let builtinRegexMessage = "";
    let proofConvertDirection: ProofConvertDirection = "traditional-to-simplified";
    let proofConvertPreviewRows: ProofConvertPreviewRow[] = [];
    let proofConvertSelectedIds = new Set<string>();
    let aiProofingScope: AiProofingScope = "current";
    let aiProofingRows: AiProofingRow[] = [];
    let aiProofingSelectedIds = new Set<string>();
    let aiProofingRunning = false;
    let aiProofingAbortRequested = false;
    let aiProofingView: "suggestions" | "approval" | "log" = "suggestions";
    let aiProofingLogs: string[] = [];
    let aiApprovalAppliedBatches: AiApprovalAppliedBatch[] = [];
    let proofMessage = "";

    // 内容检查状态
    let isCheckModeOn = true;
    let invalidSequenceIds = new Set<string>();
    let sequenceErrors: CheckItem[] = [];
    let wordCountErrors: CheckItem[] = [];
    let titleErrors: CheckItem[] = []; // 新增：空标题检查
    let checkCollapseState = { seq: false, title: false, word: false };
    let longPressTimer: any;
    let autoRefreshTimer: any;

    // 拖拽与坐标
    let findPanelPos = { x: 0, y: 0 };
    let checkPanelPos = { x: 0, y: 0 };
    let isDragging = false;
    let dragStart = { x: 0, y: 0 };
    let activeDragTarget = "find"; // 'find' or 'check'

    // Close Dialog State
    let showCloseDialog = false;
    let isDialogSaving = false;
    let lastGeneratedEpubPath = ""; // New state variable
    let openedFromLibrary = false;
    let txtEditorCloseAction: "exit" | "library" = "library";
    let libraryData: LibraryData | null = null;
    let aiProofingConfig: AiProofingConfig = { ...DEFAULT_AI_PROOFING };
    let aiProviders: AiProviderConfig[] = [];
    let aiProviderDraftId = "";
    let apiEditorOpen = false;
    let apiEditorMode: "new" | "edit" = "new";
    let apiEditorDraft: AiProviderConfig | null = null;
    let txtAiProofingConfig: TxtAiProofingConfig = { ...DEFAULT_TXT_AI_PROOFING };
    let aiSettingsMessage = "";
    let isClosingEditorWindow = false;

    async function closeTxtEditorWindow() {
        if (isClosingEditorWindow) return;
        isClosingEditorWindow = true;
        localStorage.removeItem("app-crash-recovery");
        if (txtEditorCloseAction === "exit") {
            await invoke("exit_app");
            return;
        }
        if (openedFromLibrary) {
            await getCurrentWindow().destroy();
            return;
        }
        window.location.href = "/";
    }

    function handleDialogSave() {
        isDialogSaving = true;
        saveFile()
            .then(async () => {
                await closeTxtEditorWindow();
            })
            .catch(() => {
                isDialogSaving = false;
            });
    }

    async function handleDialogDiscard() {
        await closeTxtEditorWindow();
    }

    function handleDialogCancel() {
        showCloseDialog = false;
    }

    function startDrag(e: MouseEvent, target: "find" | "check") {
        if (
            (e.target as HTMLElement).tagName === "INPUT" ||
            (e.target as HTMLElement).tagName === "BUTTON" ||
            (e.target as HTMLElement).classList.contains("err-tag")
        )
            return;
        isDragging = true;
        activeDragTarget = target;
        const currentPos = target === "find" ? findPanelPos : checkPanelPos;
        dragStart = {
            x: e.clientX - currentPos.x,
            y: e.clientY - currentPos.y,
        };
        window.addEventListener("mousemove", handleDrag);
        window.addEventListener("mouseup", stopDrag);
    }
    function handleDrag(e: MouseEvent) {
        if (!isDragging) return;
        const newPos = {
            x: e.clientX - dragStart.x,
            y: e.clientY - dragStart.y,
        };
        if (activeDragTarget === "find") findPanelPos = newPos;
        else checkPanelPos = newPos;
    }
    function stopDrag() {
        isDragging = false;
        window.removeEventListener("mousemove", handleDrag);
        window.removeEventListener("mouseup", stopDrag);
    }

    onMount(() => {
        console.log("App mounting...");
        window.addEventListener("error", (e) => {
            console.error("Global Error Caught:", e.message, e.error);
        });
        window.addEventListener("unhandledrejection", (e) => {
            console.error("Unhandled Promise Rejection:", e.reason);
        });

        let unlistenClose: any;
        let unlistenDragDrop: any;

        const init = async () => {
            const { getCurrentWindow, LogicalPosition } = await import("@tauri-apps/api/window");
            const appWindow = getCurrentWindow();
            const label = appWindow.label;
            const initialFilePath = getUrlFileParam() || await invoke<string | null>("get_launch_args");
            if (initialFilePath) {
                localStorage.removeItem("app-crash-recovery");
            }

            // 1. 窗口位置恢复
            const savedPos = localStorage.getItem("window_pos_" + label);
            if (savedPos) {
                try {
                    const { x, y } = JSON.parse(savedPos);
                    await appWindow.setPosition(new LogicalPosition(x, y));
                } catch (e) {}
            }

            // 监听移动并保存
            appWindow.listen("tauri://move", async () => {
                try {
                    const pos = await appWindow.outerPosition();
                    localStorage.setItem("window_pos_" + label, JSON.stringify(pos));
                } catch (e) {}
            });

            // Listen for restore request from EPUB window
            appWindow.listen("restore-main-window", async () => {
                console.log("Received restore-main-window event");
                await appWindow.show();
                await appWindow.setFocus();
            });

            // 监听系统文件拖放：将 txt/md/epub 直接拖到窗口即可打开
            try {
                unlistenDragDrop = await appWindow.onDragDropEvent(async (event: any) => {
                    const payload = event?.payload;
                    if (!payload || payload.type !== "drop") return;
                    const paths: string[] = Array.isArray(payload.paths) ? payload.paths : [];
                    if (paths.length === 0) return;
                    await openDroppedFile(paths);
                });
            } catch (e) {
                console.warn("注册拖放监听失败:", e);
            }

            // 2. 移动端检测
            if (window.innerWidth < 768) {
                isMobile = true;
                showSidebar = false;
            }

            // 3. 读取设置
            const stored = localStorage.getItem("app-settings");
            if (stored) {
                try {
                    let parsed = JSON.parse(stored);
                    appSettings = { ...DEFAULT_SETTINGS, ...parsed };

                    if (isLegacyLooseMetaRegex(appSettings.metaRegex)) {
                        appSettings.metaRegex = DEFAULT_META_VOLUME_REGEX;
                    }
                    
                    // 核心初始化：确保 customRegexRules 存在并按照预期结构映射
                    if (!appSettings.customRegexRules || !Array.isArray(appSettings.customRegexRules)) {
                        appSettings.customRegexRules = [
                            { level: 1, pattern: appSettings.metaRegex || DEFAULT_META_VOLUME_REGEX },
                            { level: 1, pattern: appSettings.volRegex || DEFAULT_SETTINGS.volRegex },
                            { level: 3, pattern: DEFAULT_META_BODY_REGEX },
                            { level: 3, pattern: appSettings.chapRegex || DEFAULT_SETTINGS.chapRegex }
                        ].flatMap(normalizeTocRegexRules);
                    } else {
                        // 迁移：如果包含 type，无缝转换为 level
                        appSettings.customRegexRules =
                            appSettings.customRegexRules.flatMap(normalizeTocRegexRules);
                    }
                    if (!appSettings.selectedStyleTemplateId) {
                        appSettings.selectedStyleTemplateId = STYLE_TEMPLATE_BUILTIN_ID;
                    }
                } catch (e) {}
            }
            await loadSharedLibrarySettings();
            await loadImportedFonts();
            await loadStyleTemplates();
            if (appSettings.defaultEpubStyles?.["main.css"]?.trim()) {
                try {
                    const legacyMainCss = appSettings.defaultEpubStyles["main.css"].trim();
                    const builtinTemplate = await invoke<StyleTemplateContent>("read_style_template", {
                        id: STYLE_TEMPLATE_BUILTIN_ID,
                    });
                    if (builtinTemplate.main_css.trim() === legacyMainCss) {
                        await invoke("restore_builtin_style_template");
                        await loadStyleTemplates();
                    }
                    appSettings.defaultEpubStyles = { "main.css": "", "font.css": "" };
                    localStorage.setItem("app-settings", JSON.stringify(appSettings));
                } catch (e) {
                    console.warn("清理旧默认样式缓存失败:", e);
                }
            }
            await applySelectedStyleTemplate(appSettings.selectedStyleTemplateId);

            // 应用主题设置
            if (!Number.isFinite(Number(appSettings.wordCountMinThreshold))) {
                appSettings.wordCountMinThreshold = DEFAULT_SETTINGS.wordCountMinThreshold;
            }
            if (!Number.isFinite(Number(appSettings.wordCountMaxThreshold))) {
                appSettings.wordCountMaxThreshold = DEFAULT_SETTINGS.wordCountMaxThreshold;
            }
            if (!appSettings.uiTheme) appSettings.uiTheme = "modern";
            applyTheme(appSettings.uiTheme);

            // 目录查找通过 onTocSearch 回调处理（见 Editor 组件绑定）

            // 4. 崩溃恢复逻辑
            const savedState = localStorage.getItem("app-crash-recovery");
            if (savedState && !initialFilePath) {
                try {
                    const state = JSON.parse(savedState);
                    if (state.filePath && state.filePath !== "请打开一本小说...") {
                        filePath = state.filePath;
                        let diskContent = "";
                        try {
                            diskContent = await invoke("read_text_file", { path: filePath });
                        } catch (e) {
                            console.warn("File read fail:", e);
                        }

                        if (state.isModified && state.content && state.content !== diskContent) {
                            fileContent = state.content;
                            isModified = true;
                        } else {
                            fileContent = diskContent;
                            isModified = false;
                            if (state.isModified)
                                localStorage.removeItem("app-crash-recovery");
                        }

                        if (fileContent) {
                            await tick();
                            editorComponent?.resetDoc(fileContent);
                            await scanToc(fileContent);
                            epubMeta = extractMetadata(fileContent, filePath);
                            refreshEpubMetadata();
                            await updateMd5(fileContent);
                            loadAiProofingCache();
                            if (state.scrollLine) {
                                setTimeout(() => editorComponent?.scrollToLine(state.scrollLine), 200);
                            }
                        }
                    }
                } catch (e) {
                    console.error("Recovery failed:", e);
                    localStorage.removeItem("app-crash-recovery");
                }
            }

            // 5. 文件关联启动 / 由书库子窗口传入的 ?file= 参数
            setTimeout(async () => {
                if (initialFilePath) {
                    await openLocalFile(initialFilePath, true);
                }
                hasInitialized = true;
            }, initialFilePath ? 0 : 500);

            // 6. 关闭拦截
            await appWindow.setTitle("TEpub-Editor-TXT");
            try {
                const sp = new URLSearchParams(window.location.search);
                openedFromLibrary = sp.get("fromLibrary") === "1";
                if (!libraryData) await loadSharedLibrarySettings();
                const action = loadGlobalAppSettings(libraryData?.config || {}).txtEditorCloseAction;
                if (action === "exit" || action === "library") {
                    txtEditorCloseAction = action;
                }
            } catch (_) {}
            unlistenClose = await appWindow.onCloseRequested(async (event) => {
                if (isClosingEditorWindow) return;
                if (isModified) {
                    event.preventDefault();
                    showCloseDialog = true;
                } else {
                    event.preventDefault();
                    await closeTxtEditorWindow();
                }
            });
        };

        init();

        // 监听全选事件
        const handleSelectAll = () => {
            editorComponent?.selectAll();
        };
        const handleContextMenuAction = (event: Event) => {
            const detail = (event as CustomEvent).detail || {};
            if (["make-chapter-title", "make-volume-title", "remove-title"].includes(detail.action)) {
                applyEditorLineTitleAction(detail.action, detail.context);
            }
        };
        window.addEventListener("editor-select-all", handleSelectAll);
        window.addEventListener("context-menu-action", handleContextMenuAction);

        return () => {
            if (unlistenClose) unlistenClose();
            if (unlistenDragDrop) unlistenDragDrop();
            window.removeEventListener("editor-select-all", handleSelectAll);
            window.removeEventListener("context-menu-action", handleContextMenuAction);
        };
    });

    // --- [4. 核心逻辑实现] ---

    async function updateMd5(content: string) {
        try {
            epubMeta.md5 = await invoke("calculate_md5", { content });
        } catch (e) {}
    }

    function extractMetadata(content: string, path: string) {
        const meta = {
            title: "书名",
            creator: "作者",
            publisher: "",
            date: new Date().toISOString().split("T")[0],
            uuid: crypto.randomUUID(),
            md5: epubMeta.md5 || "",
            cover_path: epubMeta.cover_path || "",
            description: "",
            tags: [...epubMeta.tags],
            styles: { ...epubMeta.styles },
            assets: [...epubMeta.assets] as EpubAsset[],
        };

        // 默认书名
        const basename = path.split(/[\\/]/).pop()?.replace(/\.[^/.]+$/, "") || "未命名";
        meta.title = basename;

        try {
            const lines = content.split("\n").map(l => l.trim()).filter(l => l.length > 0);
            if (lines.length > 0) {
                const firstLine = lines[0];
                const secondLine = lines.length > 1 ? lines[1] : "";

                // 规则 1: 书名号提取 《...》
                const bracketMatch = firstLine.match(/《([^》]+)》/);
                if (bracketMatch) {
                    meta.title = bracketMatch[1].trim();
                }
                // 规则 2: "书名：" 前缀
                else if (firstLine.match(/^(?:书名|小说名|Title)[\s:：]+(.*)/i)) {
                    meta.title = firstLine.replace(/^(?:书名|小说名|Title)[\s:：]+/i, "").trim();
                }
                // 规则 3: 双行关联 (如果第二行是作者，第一行通常是书名)
                else if (secondLine.match(/^(?:作者|Author|By)[\s:：~]*(.*)/i)) {
                    meta.title = firstLine.trim();
                }
            }

            // 提取作者 (严格限制在前 2 行)
            const first2LinesForAuthor = lines.slice(0, 2).join("\n");
            const authorMatch = first2LinesForAuthor.match(/(?:^|\n)\s*(?:作者|Author|By)[\s:：~]*([^\n\r]+)/i);
            if (authorMatch && authorMatch[1]) {
                meta.creator = authorMatch[1].trim();
            }

            // 规则 4: 书名兜底（前两行未识别出《》或书名：时）
            if (!meta.title || meta.title === "书名" || meta.title === meta.creator) {
                 meta.title = basename;
            }

            // 3. 简介 (更精准的正则：保留首行缩进)
            const descMatch = content.match(/(?:^|\n)[^\S\n]*(?:内容)?(?:简介|Intro|Description)[\t ]*[:：]?[\t ]*(?:\r?\n)?([\s\S]+?)(?=\n\s*(?:第[零一二三四五六七八九十百千万0-9]+[卷部章回|卷部]|Chapter\s*\d+)|$)/i);
            if (descMatch && descMatch[1]) {
                const desc = descMatch[1].replace(/\s+$/, ""); // 仅修剪尾部空白
                if (desc.length > 0) {
                    meta.description = desc.length > 3000 ? desc.substring(0, 3000) + "..." : desc;
                }
            }
        } catch (e) {
            console.log("Metadata extract failed", e);
        }
        return meta;
    }

    function refreshEpubMetadata() {
        if (!fileContent) return;
        const fresh = extractMetadata(fileContent, filePath);
        
        // 如果文件内容已修改，或者当前仍是默认占位符，则更新主要字段
        const isTitleDefault = epubMeta.title === "书名" || !epubMeta.title;
        const isCreatorDefault = epubMeta.creator === "作者" || !epubMeta.creator;

        if (isModified || isTitleDefault) epubMeta.title = fresh.title;
        if (isModified || isCreatorDefault) epubMeta.creator = fresh.creator;
        if (isModified || !epubMeta.description) epubMeta.description = fresh.description;
        
        if (!epubMeta.styles["main.css"]) {
            epubMeta.styles["main.css"] = selectedStyleTemplateCss || getBuiltinStyleTemplateCss();
        }

        // UUID 保持不变除非为空
        if (!epubMeta.uuid) epubMeta.uuid = fresh.uuid;
        // 强制重新计算 MD5
        updateMd5(fileContent);
    }

    async function loadCoverPreview() {
        if (!epubMeta.cover_path) {
            if (coverPreviewUrl) {
                URL.revokeObjectURL(coverPreviewUrl);
                coverPreviewUrl = null;
            }
            return;
        }
        try {
            const normalizedPath = normalizeLocalPath(epubMeta.cover_path);
            if (normalizedPath !== epubMeta.cover_path) {
                epubMeta.cover_path = normalizedPath;
            }
            const data = await invoke<number[]>("read_binary_file", { path: normalizedPath });
            const ext = normalizedPath.split('.').pop()?.toLowerCase() || 'jpg';
            const mimeMap: Record<string, string> = { jpg: 'image/jpeg', jpeg: 'image/jpeg', png: 'image/png', webp: 'image/webp', gif: 'image/gif' };
            const mime = mimeMap[ext] || 'image/jpeg';
            const blob = new Blob([new Uint8Array(data)], { type: mime });
            if (coverPreviewUrl) URL.revokeObjectURL(coverPreviewUrl);
            coverPreviewUrl = URL.createObjectURL(blob);
        } catch (e) {
            console.error('封面预览加载失败:', e);
            coverPreviewUrl = null;
        }
    }

    function normalizeLocalPath(path: string): string {
        if (!path || !path.startsWith("file://")) return path;
        try {
            const url = new URL(path);
            if (url.protocol !== "file:") return path;
            let pathname = decodeURIComponent(url.pathname || "");
            if (/^\/[A-Za-z]:/.test(pathname)) {
                pathname = pathname.slice(1);
            }
            return pathname || path;
        } catch {
            return path;
        }
    }

    function extractPickedPath(selection: string | string[]): string {
        const raw = Array.isArray(selection) ? selection[0] : selection;
        return normalizeLocalPath(raw);
    }

    async function pickLocalCover() {
        const selection = await open({
            filters: [{ name: "Image", extensions: ["jpg", "png", "jpeg", "webp"] }],
        });
        if (!selection) return;
        epubMeta.cover_path = extractPickedPath(selection as string | string[]);
        coverSearchMessage = "已使用本地封面";
        await loadCoverPreview();
    }

    async function applyRemoteCover(result: CoverSearchResult, automatic = false) {
        if (isCoverApplying) return;
        isCoverApplying = true;
        try {
            const localPath = await invoke<string>("download_cover_to_temp", {
                imageUrl: result.image_url,
                title: epubMeta.title || result.title || "cover",
            });
            epubMeta.cover_path = localPath;
            await loadCoverPreview();
            coverSearchMessage = automatic
                ? `已自动使用 ${result.source} 封面`
                : `已使用《${result.title || epubMeta.title}》封面`;
            if (!automatic) {
                coverSearchResults = [];
            }
        } catch (e) {
            console.error("应用远程封面失败:", e);
            coverSearchMessage = automatic
                ? "自动应用失败，可手动选择其他封面"
                : "应用封面失败，请换一个结果或使用本地图片";
        } finally {
            isCoverApplying = false;
        }
    }

    async function searchCovers() {
        const title = epubMeta.title.trim();
        const author = epubMeta.creator.trim();
        if (!title || title === "书名") {
            coverSearchMessage = "请先填写书名";
            return;
        }

        isCoverSearching = true;
        coverSearchMessage = "正在搜索封面...";
        coverSearchResults = [];
        try {
            const results = await invoke<CoverSearchResult[]>("search_book_covers", { title, author });
            coverSearchResults = results;
            if (!results.length) {
                coverSearchMessage = "没有找到可用封面";
                return;
            }

            coverSearchMessage = `找到 ${results.length} 个封面`;
        } catch (e) {
            coverSearchMessage = `搜索封面失败：${e}`;
        } finally {
            isCoverSearching = false;
        }
    }

    async function openAdvancedEpubMetadata() {
        try {
            const existing = await WebviewWindow.getByLabel("epub-metadata");
            if (existing) {
                await existing.setFocus();
                return;
            }

            const win = new WebviewWindow("epub-metadata", {
                url: "/epub-metadata",
                title: "高级选项",
                width: 540,
                height: 620,
                minWidth: 540,
                minHeight: 620,
                resizable: true,
                decorations: true,
                center: true,
            });

            win.once("metadata-window-ready", async () => {
                await emit("init-metadata", {
                    meta: {
                        publisher: epubMeta.publisher,
                        uuid: epubMeta.uuid,
                        md5: epubMeta.md5,
                        styles: { ...epubMeta.styles },
                        assets: [...epubMeta.assets],
                    },
                    custom: customMetadata,
                });
            });

            const unlisten = await listen("update-metadata", (event: any) => {
                const { meta, custom } = event.payload;
                epubMeta.publisher = meta.publisher;
                epubMeta.uuid = meta.uuid;
                epubMeta.md5 = meta.md5;
                epubMeta.styles = { ...epubMeta.styles, ...(meta.styles || {}) };
                epubMeta.assets = [...(meta.assets || [])];
                customMetadata = [...(custom || [])];
            });

            win.once("tauri://destroyed", () => {
                unlisten();
            });
        } catch (e) {
            message("打开高级选项失败: " + e, { kind: "error" });
        }
    }

    function saveStateToCache(line: number) {
        if (isLoadingFile) return;
        // 限制缓存大小，防止 localStorage 溢出
        const state = {
            filePath,
            isModified,
            scrollLine: line,
            content:
                isModified && fileContent.length < 3000000 ? fileContent : null,
        };
        localStorage.setItem("app-crash-recovery", JSON.stringify(state));
    }

    async function openLocalFile(path: string, initialLaunch = false) {
        try {
            if (path) {
                // 检查是否是 EPUB 文件
                if (path.toLowerCase().endsWith(".epub")) {
                    const encodedPath = encodeURIComponent(path);
                    console.log("打开 EPUB 文件:", path);

                    if (initialLaunch) {
                        // Initial launch: Reuse the main window
                        const { getCurrentWindow, LogicalSize } = await import(
                            "@tauri-apps/api/window"
                        );
                        const appWindow = getCurrentWindow();
                        await appWindow.setTitle("TEpub-Editor-EPUB");
                        await appWindow.setSize(new LogicalSize(1200, 800));
                        window.location.href = `/epub-editor?file=${encodedPath}`;
                        return;
                    }

                    try {
                        // 打开新窗口显示 EPUB 编辑器
                        // 确保路径正确编码
                        console.log("编码后路径:", encodedPath);

                        const epubWindow = new WebviewWindow(
                            "epub-editor-" + Date.now(),
                            {
                                url: `/epub-editor?file=${encodedPath}`,
                                title: "TEpub-Editor-EPUB",
                                width: 1200,
                                height: 740,
                                minWidth: 1200,
                                minHeight: 740,
                                dragDropEnabled: true,
                                center: true, // Center the window
                            },
                        );

                        // 这里的事件监听可能不触发，改为直接执行隐藏逻辑
                        // Logic: Close main window if it's empty
                        console.log(
                            "Checking if main window should hide (Immediate). Content length:",
                            fileContent ? fileContent.length : 0,
                        );

                        const current = getCurrentWindow();
                        // 强制隐藏：只要不是在编辑已有的文件（通过内容是否为空判断），就隐藏
                        if (!fileContent || fileContent.trim().length === 0) {
                            console.log("Hiding main window...");
                            await current.hide();
                        } else {
                            console.log(
                                "Main window kept open. Content exists.",
                            );
                        }

                        // 无论隐藏与否，都监听错误
                        epubWindow.once("tauri://error", (e) => {
                            console.error("窗口创建失败:", e);
                            message("打开 EPUB 编辑器失败: " + e, {
                                title: "错误",
                                kind: "error",
                            });
                        });

                        // 监听已销毁事件：当 EPUB 窗口关闭时，恢复主窗口显示
                        epubWindow.once("tauri://destroyed", async () => {
                            console.log(
                                "EPUB window destroyed, restoring main window...",
                            );
                            const current = getCurrentWindow();
                            await current.show();
                            await current.setFocus();
                        });
                    } catch (e) {
                        console.error("EPUB 窗口打开错误:", e);
                        await message("打开 EPUB 编辑器失败: " + e, {
                            title: "错误",
                            kind: "error",
                        });
                    }
                    return; // EPUB 文件处理完毕，直接返回
                }

                isLoading = true;
                isLoadingFile = true;
                filePath = path;
                fileContent = "";
                tocTree = [];
                flatToc = [];
                activeChapterId = "";
                await tick();
                editorComponent?.resetDoc("");

                // 读取原生文本并施加终极降维打击：强力规范化换行符！
                let rawContent = await invoke<string>("read_text_file", {
                    path: filePath,
                });
                let content = rawContent.replace(
                    /\r\n|\r|\u2028|\u2029/g,
                    "\n",
                );

                // 【终极排错大招】防巨型单行核武器：如果此文件的恶劣排版中包含超乎想象的巨龙行（>800字没有一个物理回车），
                // CodeMirror 会在拖拽选区或滚动时因为几何测算彻底超载，并导致 posAtCoordsInline 读出 null 空指针崩溃。
                // 解决方案：为所有超长异端长句智能注入真正的换行！
                content = content
                    .split("\n")
                    .map((line) => {
                        if (line.length > 800) {
                            // 遇到八百字不换行的“伪文字段落”，在句号/叹号/问号后（包裹着引号时也行），并且后面跟着空格或什么都没有的地方，强制斩断加回车
                            return line.replace(
                                /([。\.\!\?][”’」』]*)(?=\s|\S)/g,
                                "$1\n",
                            );
                        }
                        return line;
                    })
                    .join("\n");

                fileContent = content;

                // 提取元数据
                epubMeta = extractMetadata(content, path);
                customMetadata = []; // 重置自定义元数据
                refreshEpubMetadata();

                editorComponent?.resetDoc(content);
                isModified = false;
                await updateMd5(content);
                await scanToc(content);
                loadAiProofingCache();
                if (isCheckModeOn) runFullCheck();

                isLoading = false;
                localStorage.removeItem("app-crash-recovery");
                setTimeout(() => {
                    isLoadingFile = false;
                }, 100);
            }
        } catch (e) {
            isLoading = false;
            console.error("Open file failed:", e);
            message(`打开文件失败: ${e}`, { kind: "error" });
        }
    }

    async function openDroppedFile(paths: string[]) {
        if (!paths || paths.length === 0) return;
        const firstSupported = paths.find((p) => /\.(txt|md|epub)$/i.test(p));
        if (!firstSupported) {
            await message("仅支持拖入 TXT/MD/EPUB 文件", { kind: "warning" });
            return;
        }
        await openLocalFile(firstSupported);
    }

    async function selectFile() {
        try {
            const selected = await open({
                multiple: false,
                filters: [
                    {
                        name: "所有支持的文件",
                        extensions: ["txt", "md", "epub"],
                    },
                    { name: "文本文件", extensions: ["txt", "md"] },
                    { name: "EPUB 文件", extensions: ["epub"] },
                ],
            });
            if (selected) {
                await openLocalFile(selected.toString());
            }
        } catch (e) {
            console.error("Select file failed:", e);
        }
    }

    async function saveFile() {
        if (!fileContent || isSaving) return;
        isSaving = true;
        try {
            if (filePath.startsWith("请打开")) {
                const path = await save({
                    filters: [{ name: "Text", extensions: ["txt"] }],
                });
                if (!path) {
                    isSaving = false;
                    return;
                }
                filePath = path;
            }
            // await writeTextFile(filePath, fileContent);
            await invoke("save_text_file", {
                path: filePath,
                content: fileContent,
            });
            // 调用后端保存历史
            await invoke("save_history", {
                originalPath: filePath,
                content: fileContent,
            }).catch(() => {});

            isModified = false;
            // Clear crash recovery on explicit save
            localStorage.removeItem("app-crash-recovery");
            // saveStateToCache(0); // Optional: re-save cleanslate or just remove. Removing is safer.
            await updateMd5(fileContent);
            await scanToc(fileContent);
            // await message("保存成功！"); // 移除弹窗，保持静默成功
        } catch (e) {
            await message(`保存失败: ${e}\n请确保已授予“所有文件访问权限”`, {
                kind: "error",
            });
        } finally {
            isSaving = false;
        }
    }

    // --- TOC 解析与同步 (含双向绑定) ---
    function manualTitleStorageKey() {
        if (!filePath || !/^[A-Za-z]:[\\/]|[\\/]/.test(filePath)) return "";
        return `manual-title-overrides:${filePath}`;
    }

    function loadManualTitleOverrides() {
        const key = manualTitleStorageKey();
        if (!key) {
            manualTitleOverrides = {};
            return;
        }
        try {
            const parsed = JSON.parse(localStorage.getItem(key) || "{}");
            manualTitleOverrides = Object.fromEntries(
                Object.entries(parsed).flatMap(([lineKey, value]) => {
                    const line = Number(lineKey);
                    if (!Number.isInteger(line) || line < 1) return [];
                    if (value === "Volume" || value === "Chapter" || value === "Ignore") {
                        return [[lineKey, { kind: value, line, title: "" }]];
                    }
                    if (value && typeof value === "object") {
                        const entry = value as Partial<ManualTitleOverrideEntry>;
                        if (entry.kind !== "Volume" && entry.kind !== "Chapter" && entry.kind !== "Ignore") {
                            return [];
                        }
                        const savedLine = Number(entry.line);
                        return [[lineKey, {
                            kind: entry.kind,
                            line: Number.isInteger(savedLine) && savedLine > 0 ? savedLine : line,
                            title: typeof entry.title === "string" ? entry.title : "",
                            prevTitle: typeof entry.prevTitle === "string" ? entry.prevTitle : undefined,
                            nextTitle: typeof entry.nextTitle === "string" ? entry.nextTitle : undefined,
                        }]];
                    }
                    return [];
                }),
            ) as Record<string, ManualTitleOverrideEntry>;
        } catch (_) {
            manualTitleOverrides = {};
        }
    }

    function saveManualTitleOverrides() {
        const key = manualTitleStorageKey();
        if (!key) return;
        const entries = Object.entries(manualTitleOverrides).filter(([, value]) =>
            value?.kind === "Volume" || value?.kind === "Chapter" || value?.kind === "Ignore",
        );
        if (entries.length === 0) {
            localStorage.removeItem(key);
        } else {
            localStorage.setItem(key, JSON.stringify(Object.fromEntries(entries)));
        }
    }

    function recomputeChapterWordCounts(text: string, chapters: RawChapter[]) {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const sorted = [...chapters].sort((a, b) => a.line_number - b.line_number);
        for (let i = 0; i < sorted.length; i += 1) {
            const start = Math.min(lines.length, sorted[i].line_number);
            const end = Math.max(
                start,
                Math.min(lines.length, (sorted[i + 1]?.line_number ?? lines.length + 1) - 1),
            );
            sorted[i].word_count = lines
                .slice(start, end)
                .reduce((sum, line) => sum + line.trim().length, 0);
        }
        return sorted;
    }

    function normalizeManualTitleText(text: string) {
        return text.trim().replace(/\s+/g, " ");
    }

    function findManualTitleLineInRange(
        lines: string[],
        title: string,
        startLine: number,
        endLine: number,
    ) {
        const wanted = normalizeManualTitleText(title);
        if (!wanted) return null;
        const start = Math.max(1, Math.min(lines.length, startLine));
        const end = Math.max(start, Math.min(lines.length, endLine));
        for (let lineNumber = start; lineNumber <= end; lineNumber += 1) {
            if (normalizeManualTitleText(lines[lineNumber - 1] || "") === wanted) {
                return lineNumber;
            }
        }
        return null;
    }

    function findManualTitleAnchorLine(
        chapters: RawChapter[],
        lines: string[],
        title: string | undefined,
        fallbackLine: number,
        side: "prev" | "next",
    ) {
        const wanted = normalizeManualTitleText(title || "");
        if (!wanted) return null;
        const chapterLines = chapters
            .filter((chapter) => normalizeManualTitleText(chapter.title) === wanted)
            .map((chapter) => chapter.line_number);
        const textLines = lines
            .map((line, index) => ({ lineNumber: index + 1, text: line }))
            .filter((line) => normalizeManualTitleText(line.text) === wanted)
            .map((line) => line.lineNumber);
        const candidates = [...new Set([...chapterLines, ...textLines])]
            .filter((lineNumber) => lineNumber >= 1 && lineNumber <= lines.length)
            .sort((a, b) => a - b);
        if (candidates.length === 0) return null;

        const directional = candidates.filter((lineNumber) =>
            side === "prev" ? lineNumber < fallbackLine : lineNumber > fallbackLine,
        );
        const pool = directional.length > 0 ? directional : candidates;
        return pool.reduce((best, lineNumber) =>
            Math.abs(lineNumber - fallbackLine) < Math.abs(best - fallbackLine)
                ? lineNumber
                : best,
        );
    }

    function resolveManualTitleOverrideLine(
        entry: ManualTitleOverrideEntry,
        lines: string[],
        chapters: RawChapter[],
    ) {
        const originalLine = Number.isInteger(entry.line) ? entry.line : 0;
        if (lines.length === 0) return null;
        const referenceLine = Math.max(1, Math.min(lines.length, originalLine || 1));
        const savedTitle = normalizeManualTitleText(entry.title);
        if (!savedTitle) return referenceLine;

        if (originalLine >= 1 && originalLine <= lines.length && normalizeManualTitleText(lines[originalLine - 1] || "") === savedTitle) {
            return originalLine;
        }

        const prevLine = findManualTitleAnchorLine(chapters, lines, entry.prevTitle, referenceLine, "prev");
        const nextLine = findManualTitleAnchorLine(chapters, lines, entry.nextTitle, referenceLine, "next");
        if (prevLine === null || nextLine === null || prevLine < nextLine) {
            const rangeLine = findManualTitleLineInRange(
                lines,
                entry.title,
                prevLine === null ? 1 : prevLine + 1,
                nextLine === null ? lines.length : nextLine - 1,
            );
            if (rangeLine !== null) return rangeLine;
        }

        const nearbyLine = findManualTitleLineInRange(
            lines,
            entry.title,
            referenceLine - 80,
            referenceLine + 80,
        );
        if (nearbyLine !== null) return nearbyLine;

        return findManualTitleLineInRange(lines, entry.title, 1, lines.length);
    }

    function getResolvedManualTitleOverrideMap(text: string, chapters: RawChapter[]) {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const resolved = new Map<number, ManualTitleOverrideEntry>();
        for (const entry of Object.values(manualTitleOverrides)) {
            const lineNumber = resolveManualTitleOverrideLine(entry, lines, chapters);
            if (lineNumber === null) continue;
            resolved.set(lineNumber, { ...entry, line: lineNumber });
        }
        return resolved;
    }

    function mergeManualTitleOverrides(text: string, chapters: RawChapter[]) {
        const lines = text.replace(/\r\n|\r|\u2028|\u2029/g, "\n").split("\n");
        const byLine = new Map<number, RawChapter>();
        const resolvedOverrides = getResolvedManualTitleOverrideMap(text, chapters);

        for (const chapter of chapters) {
            const override = resolvedOverrides.get(chapter.line_number);
            if (override?.kind === "Ignore") continue;
            if (override?.kind === "Volume" || override?.kind === "Chapter") continue;
            byLine.set(chapter.line_number, chapter);
        }

        for (const [lineNumber, override] of resolvedOverrides) {
            if (override.kind !== "Volume" && override.kind !== "Chapter") continue;
            const title = lines[lineNumber - 1]?.trim();
            if (!title) continue;
            byLine.set(lineNumber, {
                title,
                line_number: lineNumber,
                level: override.kind === "Volume" ? 1 : 3,
                is_meta: false,
                word_count: 0,
            });
        }

        return recomputeChapterWordCounts(text, [...byLine.values()]);
    }

    async function scanToc(textOverride?: string) {
        const text = textOverride ?? fileContent;
        if (!text) return;
        try {
            // 调用 Rust 正则扫描
            loadManualTitleOverrides();
            const rawList = await invoke<RawChapter[]>("scan_chapters", {
                content: text,
                rules: appSettings.customRegexRules,
            });
            const resolvedManualOverrides = getResolvedManualTitleOverrideMap(text, rawList);
            const tocItems = mergeManualTitleOverrides(text, rawList).filter((item) => {
                const override = resolvedManualOverrides.get(item.line_number);
                return override?.kind === "Volume" || override?.kind === "Chapter" || isLikelyTocTitle(item.title, item.level);
            });

            const tree: TocNode[] = [];
            flatToc = [];
            let uid = 0;
            let parentStack: TocNode[] = [];

            // 构建嵌套树
            for (const item of tocItems) {
                // Determine legacy type for UI styling backward compatibility
                const computedType = item.is_meta ? "Meta" : (item.level === 1 ? "Volume" : "Chapter");
                
                const node: TocNode = {
                    id: `n-${uid++}`,
                    title: item.title,
                    line_number: item.line_number,
                    type: computedType,
                    word_count: item.word_count,
                    children: [],
                    expanded:
                        computedType === "Volume"
                            ? !userCollapsedVolumeKeys.has(
                                  `${item.line_number}:${item.title}`,
                              )
                            : true,
                };

                const flatNode: FlatNode = {
                    id: node.id,
                    line: node.line_number,
                    title: node.title,
                    type: computedType,
                    word_count: node.word_count,
                };

                // Remove parents that are closed by this node
                while (parentStack.length > 0) {
                    let top = parentStack[parentStack.length - 1];
                    // If the parent is not Meta, and its level is STRICTLY smaller, it IS a valid parent.
                    // Wait, our level is 1..5. Smaller number = higher level (like h1). 
                    // So valid parent must have level < item.level.
                    // Example: Volume is level 1. Chapter is level 3. Parent (1) < Node (3).
                    // If we encounter another Level 1, we pop the previous Level 1 because 1 is not < 1.
                    if (top.level !== undefined && top.level < item.level && top.type !== "Meta") {
                        break;
                    }
                    if (top.type === "Volume" && item.level > 1) {
                        break; // Fallback for safely catching missing level property in old node trees
                    }
                    parentStack.pop();
                }

                if (parentStack.length > 0) {
                    let parent = parentStack[parentStack.length - 1];
                    // Only volume nodes act as parents in the tree representation 
                    node.parentId = parent.id;
                    parent.children.push(node);
                    flatNode.parentId = parent.id;
                } else {
                    tree.push(node);
                }

                // Add non-meta elements to stack using dynamic levels
                (node as any).level = item.level;
                parentStack.push(node);
                flatToc.push(flatNode);
            }
            flatToc = [...flatToc];
            tocTree = tree;

            // 更新统计
            let v = 0,
                c = 0;
            tocTree.forEach((n) => {
                if (n.type === "Volume") {
                    v++;
                    c += n.children.length;
                } else if (n.type === "Chapter") c++;
            });
            stats = { volumes: v, chapters: c };

            if (isCheckModeOn) runFullCheck();
        } catch (e) {}
    }
    let saveCacheTimer: ReturnType<typeof setTimeout> | null = null;

    let lastNavChapterId: string | null = null; // 导航锁定的章节ID
    let lastNavLine: number = 0; // 导航锁定章节的行号

    // 编辑器滚动时触发：高亮侧边栏
    async function handleScroll(state: {
        top: number;
        bottom: number;
        isAtBottom: boolean;
    }) {
        // 防抖: 每2秒最多保存一次状态到 localStorage（只保存 top 行号）
        if (!saveCacheTimer) {
            saveCacheTimer = setTimeout(() => {
                saveCacheTimer = null;
                saveStateToCache(state.top);
            }, 2000);
        }
        if (flatToc.length === 0) return;
        if (isNavigating) return; // 正在手动跳转，忽略滚动监听

        // 倒序查找上下边界分别对应的章节
        // 注意：CM6 使用 scrollIntoView(y:"start") 时，章节标题往往排在视口顶部往下 5-20 行的地方。
        // 所以我们加上 10 行的容差。如果某个章节标题出现在视口顶部这 10 行内，我们就认为当前处于该章节。
        let foundTop: FlatNode | null = null;
        let foundBottom: FlatNode | null = null;

        for (let i = flatToc.length - 1; i >= 0; i--) {
            if (!foundTop && flatToc[i].line <= state.top + 10) {
                foundTop = flatToc[i];
            }
            if (!foundBottom && flatToc[i].line <= state.bottom) {
                foundBottom = flatToc[i];
            }
            if (foundTop && foundBottom) break;
        }

        // 默认高亮视口最上方的章节
        // 但如果已经滚到了文档绝对底部，则高亮视口最下方的章节
        // 这样可以完美解决最后几章很短导致无法滚动到顶部时的高亮错位问题
        let found = state.isAtBottom ? foundBottom : foundTop;

        if (found && found.id !== activeChapterId) {
            activeChapterId = found.id;

            // 如果是卷内章节，确保父卷展开
            if (found.parentId) {
                const p = tocTree.find((n) => n.id === found!.parentId);
                if (
                    p &&
                    !p.expanded &&
                    !userCollapsedVolumeKeys.has(getVolumeCollapseKey(p))
                ) {
                    p.expanded = true;
                    tocTree = [...tocTree];
                    await tick();
                }
            }

            // 侧边栏自动滚动
            await tick();
            const el = document.getElementById(`toc-${activeChapterId}`);
            const tocList = document.querySelector(".toc-list");
            if (el && tocList) {
                const elRect = el.getBoundingClientRect();
                const listRect = tocList.getBoundingClientRect();
                // 仅当目标不在可视区域的中心时才微调滚动，避免频繁触发 reflow 抖动
                if (
                    elRect.top < listRect.top + 50 ||
                    elRect.bottom > listRect.bottom - 50
                ) {
                    const scrollAmount =
                        elRect.top -
                        listRect.top -
                        listRect.height / 2 +
                        elRect.height / 2;
                    tocList.scrollBy({ top: scrollAmount, behavior: "smooth" });
                }
            }
        }
    }

    // 处理选择时的目录同步
    async function handleSelectionChange(line: number) {
        if (isNavigating) return;
        handleScroll({ top: line, bottom: line, isAtBottom: false });
    }

    function buildManualTitleOverrideEntry(kind: ManualTitleKind, line: { number: number; text: string }) {
        const prev = [...flatToc].reverse().find((node) => node.line < line.number);
        const next = flatToc.find((node) => node.line > line.number);
        return {
            kind,
            line: line.number,
            title: line.text.trim(),
            prevTitle: prev?.title,
            nextTitle: next?.title,
        };
    }

    async function applyEditorLineTitleAction(action: string, context: any) {
        if (!editorComponent || !context) return;
        const line = editorComponent.getLineAtClientPos(Number(context.clientX), Number(context.clientY));
        if (!line) return;

        if (action === "make-volume-title") {
            manualTitleOverrides[String(line.number)] = buildManualTitleOverrideEntry("Volume", line);
        } else if (action === "make-chapter-title") {
            manualTitleOverrides[String(line.number)] = buildManualTitleOverrideEntry("Chapter", line);
        } else if (action === "remove-title") {
            manualTitleOverrides[String(line.number)] = buildManualTitleOverrideEntry("Ignore", line);
        } else {
            return;
        }

        saveManualTitleOverrides();
        await tick();
        await scanToc();
    }
    // 统一处理章节跳转点击
    function handleChapterClick(id: string, line: number) {
        console.log("handleChapterClick", id, line);

        // 1. 清理旧定时器
        if (scrollTimeout) {
            clearTimeout(scrollTimeout);
            scrollTimeout = null;
        }

        // 2. 开启导航锁
        isNavigating = true;

        // 3. 立即更新高亮 + 设置导航锁定目标
        activeChapterId = id;
        lastNavChapterId = id;
        lastNavLine = line;

        // 4. 执行滚动
        if (editorComponent) {
            editorComponent.scrollToLine(line, true);
        } else {
            console.error("Editor component not ready");
        }

        // 5. 手动滚动侧边栏（因为 handleScroll 被锁住了）
        requestAnimationFrame(() => {
            const el = document.getElementById(`toc-${id}`);
            if (el) {
                el.scrollIntoView({ behavior: "smooth", block: "center" });
            }
        });

        // 6. 解锁导航锁（handleScroll 会通过 lastNavChapterId 继续保护高亮）
        scrollTimeout = setTimeout(() => {
            isNavigating = false;
            scrollTimeout = null;
        }, 1200);
    }

    // --- 检查逻辑 ---
    function toggleCheckMode() {
        isCheckModeOn = !isCheckModeOn;
        if (isCheckModeOn) {
            scanToc();
            runFullCheck();
        } else {
            invalidSequenceIds.clear();
            tocTree = [...tocTree];
        }
    }

    function openProofCheckPanel() {
        closeAllPanels();
        showProofPanel = true;
        proofActiveTab = "check";
        isCheckModeOn = true;
        scanToc();
        runFullCheck();
    }

    function normalizeWordCheckSettings() {
        let min = Number(appSettings.wordCountMinThreshold);
        let max = Number(appSettings.wordCountMaxThreshold);
        if (!Number.isFinite(min) || min < 0) min = DEFAULT_SETTINGS.wordCountMinThreshold;
        if (!Number.isFinite(max) || max < 0) max = DEFAULT_SETTINGS.wordCountMaxThreshold;
        appSettings.wordCountMinThreshold = Math.floor(min);
        appSettings.wordCountMaxThreshold = Math.floor(max);
        localStorage.setItem("app-settings", JSON.stringify(appSettings));
        runFullCheck();
    }

    function startLongPress(e: Event) {
        if (isMobile) {
            e.preventDefault();
            (document.activeElement as HTMLElement)?.blur();
        }
        longPressTimer = setTimeout(() => {
            openProofCheckPanel();
            runFullCheck();
        }, 600);
    }

    // PC 端鼠标长按支持
    function handleMouseDown() {
        longPressTimer = setTimeout(() => {
            openProofCheckPanel();
            runFullCheck();
        }, 600);
    }

    // 中文数字转阿拉伯数字
    function chineseToNum(cn: string): number {
        const charMap: Record<string, number> = {
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
            十: 10,
            百: 100,
            千: 1000,
            万: 10000,
        };
        let result = 0,
            current = 0;
        for (const c of cn) {
            const v = charMap[c];
            if (v === undefined) return -1;
            if (v >= 10) {
                if (current === 0) current = 1;
                if (v === 10000) {
                    result = (result + current) * v;
                    current = 0;
                } else {
                    current *= v;
                    result += current;
                    current = 0;
                }
            } else {
                current = current * 10 + v;
            }
        }
        return result + current;
    }

    // 从标题中提取章节序号，优先匹配"第X章/回/节"格式
    function extractChapterNum(title: string): number {
        // 1. 优先匹配 "第X章/回/节" 格式（支持中文数字和阿拉伯数字）
        const m = title.match(
            /第\s*([0-9零一二三四五六七八九十百千万〇两]+)\s*[章回节]/,
        );
        if (m) {
            const raw = m[1];
            // 纯阿拉伯数字
            if (/^\d+$/.test(raw)) return parseInt(raw);
            // 中文数字
            return chineseToNum(raw);
        }

        // 2. 支持克系/玄幻常见等级标题："序列 8：小丑"、"序列八 小丑"。
        // 必须在序号后结束或出现分隔符，避免把正文 "序列8时..." 当标题序号。
        const seq = title.match(
            /^序列\s*([0-9零一二三四五六七八九十百千万〇两]+)(?=\s|[:：、.．\-—]|$)/,
        );
        if (seq) {
            const raw = seq[1];
            if (/^\d+$/.test(raw)) return parseInt(raw);
            return chineseToNum(raw);
        }

        // 3. 降级：标题以纯数字开头（如 "101 黑暗"）
        const m2 = title.match(/^(\d+)/);
        if (m2) return parseInt(m2[1]);
        return -1;
    }

    function runFullCheck() {
        sequenceErrors = [];
        wordCountErrors = [];
        titleErrors = [];
        invalidSequenceIds.clear();
        let lastNum = -1;
        for (const node of flatToc) {
            if (node.type === "Chapter") {
                const num = extractChapterNum(node.title);
                if (num !== -1) {
                    if (lastNum !== -1 && num !== lastNum + 1) {
                        invalidSequenceIds.add(node.id);
                        sequenceErrors.push({
                            id: node.id,
                            title: node.title,
                            line: node.line,
                            msg: `(${lastNum}-${num})`,
                            val: num,
                        });
                    }
                    lastNum = num;
                }

                // 空标题检查: 仅包含数字、序号，没有具体内容
                if (
                    /^第\s*[0-9零一二三四五六七八九十百千万]+\s*[章卷回节]\s*$/.test(
                        node.title.trim(),
                    ) ||
                    /^序列\s*[0-9零一二三四五六七八九十百千万〇两]+\s*$/.test(
                        node.title.trim(),
                    ) ||
                    /^\d+$/.test(node.title.trim())
                ) {
                    titleErrors.push({
                        id: node.id,
                        title: node.title,
                        line: node.line,
                        msg: "无标题",
                        val: 0,
                    });
                }

                if (node.word_count < appSettings.wordCountMinThreshold || node.word_count > appSettings.wordCountMaxThreshold) {
                    wordCountErrors.push({
                        id: node.id,
                        title: node.title,
                        line: node.line,
                        msg: node.word_count < appSettings.wordCountMinThreshold ? `低于 ${appSettings.wordCountMinThreshold}` : `高于 ${appSettings.wordCountMaxThreshold}`,
                        val: node.word_count,
                    });
                }
            } else if (node.type === "Volume") {
                // 新卷开始，重置序号计数
                lastNum = -1;
            }
        }
        tocTree = [...tocTree]; // 触发 Svelte 更新
    }

    // --- 查找替换逻辑 ---
    async function findNext() {
        if (!allMatches || allMatches.length === 0) await performFind();
        if (allMatches && allMatches.length > 0) {
            currentMatchIndex = (currentMatchIndex + 1) % allMatches.length;
            replaceMsg = `第 ${currentMatchIndex + 1}/${allMatches.length} 处`;
            editorComponent.selectMatch(
                allMatches[currentMatchIndex].line,
                allMatches[currentMatchIndex].start_char,
                allMatches[currentMatchIndex].end_char,
            );
        }
    }

    async function findPrev() {
        if (!allMatches || allMatches.length === 0) await performFind();
        if (allMatches && allMatches.length > 0) {
            currentMatchIndex =
                (currentMatchIndex - 1 + allMatches.length) % allMatches.length;
            replaceMsg = `第 ${currentMatchIndex + 1}/${allMatches.length} 处`;
            editorComponent.selectMatch(
                allMatches[currentMatchIndex].line,
                allMatches[currentMatchIndex].start_char,
                allMatches[currentMatchIndex].end_char,
            );
        }
    }

    async function performFind() {
        if (!fileContent || !findPattern) return;
        try {
            const res = await invoke<SearchResult>("advanced_search", {
                content: fileContent,
                pattern: findPattern,
                isRegex,
            });
            if (res.found) {
                allMatches = res.matches;
                currentMatchIndex = 0;
                replaceMsg = `第 1/${res.count} 处`;
                editorComponent.selectMatch(
                    allMatches[0].line,
                    allMatches[0].start_char,
                    allMatches[0].end_char,
                );
            } else {
                allMatches = [];
                replaceMsg = "未找到";
            }
        } catch (e) {
            replaceMsg = "正则错误";
        }
    }

    async function performReplaceAll() {
        if (!fileContent || !findPattern) return;
        const confirmed = await ask("确定执行全书替换吗？此操作无法撤销。", {
            kind: "warning",
        });
        if (!confirmed) return;

        try {
            const res = await invoke<string>("advanced_replace", {
                content: fileContent,
                pattern: findPattern,
                replacement: replacePattern,
                isRegex,
            });
            fileContent = res;
            editorComponent.resetDoc(res);
            replaceMsg = "替换完成";
            allMatches = [];
        } catch (e) {
            replaceMsg = "替换失败";
        }
    }

    // --- EPUB 导出 ---
    async function generateEpub() {
        if (!fileContent) return;

        // 必填项检查 (仅书名如果不填会无法生成有效OPF，其他可选)
        if (!epubMeta.title || epubMeta.title.trim() === "") {
            // 尝试使用文件名作为默认书名
            const basename =
                filePath
                    .split(/[\\/]/)
                    .pop()
                    ?.replace(/\.[^/.]+$/, "") || "未命名书籍";
            epubMeta.title = basename;
        }
        if (!epubMeta.uuid) epubMeta.uuid = crypto.randomUUID();
        // MD5 应该在文件加载时已计算，防卫性保留
        if (!epubMeta.md5) await updateMd5(fileContent);

        coverSearchResults = [];
        epubGenerationStatus = "generating";
        isLoading = true;
        try {
            const savePath = await save({
                filters: [{ name: "EPUB", extensions: ["epub"] }],
                defaultPath: epubMeta.title + ".epub",
            });
            if (!savePath) {
                isLoading = false;
                epubGenerationStatus = "idle";
                return;
            }

            let chapters = await invoke<RawChapter[]>("scan_chapters", {
                content: fileContent,
                rules: appSettings.customRegexRules,
            });
            loadManualTitleOverrides();
            const resolvedManualOverrides = getResolvedManualTitleOverrideMap(fileContent, chapters);
            chapters = mergeManualTitleOverrides(fileContent, chapters).filter((chapter) => {
                const override = resolvedManualOverrides.get(chapter.line_number);
                return override?.kind === "Volume" || override?.kind === "Chapter" || isLikelyTocTitle(chapter.title, chapter.level);
            });

            // 智能清洗
            const cleanRegex =
                /^(\s*(?:第[零一二三四五六七八九十百千万0-9]+[卷部章回]|Chapter\s*\d+|楔子|序[章言]?))\s*[:：]\s*/;
            chapters = chapters.map((c) => {
                c.title = c.title.replace(cleanRegex, "$1 ");
                return c;
            });

            await invoke("export_epub", {
                savePath,
                content: fileContent,
                chapters,
                metadata: {
                    title: epubMeta.title,
                    creator: epubMeta.creator,
                    publisher: epubMeta.publisher,
                    date: epubMeta.date,
                    uuid: epubMeta.uuid,
                    md5: epubMeta.md5,
                    cover_path: epubMeta.cover_path,
                    description: epubMeta.description,
                    tags: epubMeta.tags,
                    main_css: epubMeta.styles["main.css"],
                    font_css: epubMeta.styles["font.css"],
                    subset_fonts: !!appSettings.subsetFonts,
                    assets: epubMeta.assets,
                    ...Object.fromEntries(customMetadata.map(m => [m.key, m.value]))
                },
            });
            // 制作成功：设置状态为成功，在UI上显示操作按钮
            epubGenerationStatus = "success";

            // 保存此时的路径供按钮使用
            // (We can assume 'savePath' is available, but we need to store it in a state variable
            // if we want the button in HTML to access it easily?
            // actually 'savePath' is local. Let's create a module-level variable or just use the closure if we were inline.
            // Let's add a state variable `lastGeneratedEpubPath`.
            lastGeneratedEpubPath = savePath;
        } catch (e) {
            // 失败时显示错误并重置状态
            await message("制作失败: " + e, { kind: "error" });
            epubGenerationStatus = "idle";
        } finally {
            isLoading = false;
        }
    }

    async function openGeneratedEpubInEditor() {
        if (!lastGeneratedEpubPath) return;
        await openLocalFile(lastGeneratedEpubPath);
    }

    async function revealGeneratedEpub() {
        if (!lastGeneratedEpubPath) return;
        try {
            await invoke("reveal_in_explorer", { path: lastGeneratedEpubPath });
        } catch (e) {
            await message("打开文件位置失败: " + e, { kind: "error" });
        }
    }

    async function confirmRestore() {
        if (!restoreTargetSnapshot) return;

        try {
            // 1. 先保存当前版本为新历史
            if (filePath && fileContent) {
                await invoke("save_history", {
                    originalPath: filePath,
                    content: fileContent,
                });
            }

            // 2. 执行回退
            fileContent = await invoke("read_text_file", {
                path: restoreTargetSnapshot.path,
            });
            editorComponent.resetDoc(fileContent);

            // 3. 关闭所有弹窗并重新扫描目录
            showRestoreConfirm = false;
            closeAllPanels();
            await scanToc();
        } catch (e) {
            await message("回退失败: " + e, { kind: "error" });
        }
    }

    function applyTheme(theme: string) {
        document.documentElement.setAttribute("data-theme", theme);
        const meta = document.querySelector('meta[name="theme-color"]');
        if (meta) {
            const colors: Record<string, string> = {
                classic: "#f3f3f3",
                dark: "#14181d",
                modern: "#eef4f8",
            };
            meta.setAttribute("content", colors[theme] || "#eef4f8");
        }
    }

    function closeAllPanels() {
        showSettingsPanel = false;
        showEpubModal = false;
        showCheckPanel = false;
        showProofPanel = false;
        showHistoryPanel = false;
    }

    function toggleProofPanel() {
        if (showProofPanel) {
            showProofPanel = false;
            return;
        }
        closeAllPanels();
        showProofPanel = true;
    }

</script>

<svelte:head>
    <meta name="theme-color" content="#f3f3f3" />
    <meta
        name="viewport"
        content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover"
    />
</svelte:head>


<!-- <ContextMenu /> --> <!-- Removed duplicate at top -->


<main class="app-container" on:contextmenu|preventDefault>
    <header class="toolbar">
        <div class="btn-group">
            <button class="btn-primary" on:click={selectFile}>📂</button>
            <button
                class={isModified ? "btn-save-modified" : "btn-save-default"}
                on:click={saveFile}>💾</button
            >
            <button
                class="btn-secondary"
                on:click={() => editorComponent.triggerUndo()}>↩️</button
            >
            <button
                class="btn-secondary"
                on:click={() => editorComponent.triggerRedo()}>↪️</button
            >
            <button
                class="btn-secondary"
                on:click={() => (showSidebar = !showSidebar)}>📖</button
            >
            <button
                class="btn-secondary"
                on:click={() => {
                    closeAllPanels();
                    refreshEpubMetadata();
                    showEpubModal = true;
                    // 重置EPUB制作状态
                    epubGenerationStatus = "idle";
                    coverSearchResults = [];
                    coverSearchMessage = "";
                    loadCoverPreview();
                }}>📚</button
            >
        </div>
        <div class="toolbar-tail">
            <button
                class="btn-secondary"
                title="查找与替换 (Ctrl+F)"
                on:click={() => {
                    closeAllPanels();
                    if (editorComponent) {
                        editorComponent.openSearchWindow();
                    }
                }}>🔍</button
            >
            <button
                class="btn-secondary proof-tool-btn"
                title="校对"
                aria-label="校对"
                on:click={toggleProofPanel}
            >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M5 4.5h10.2a2.3 2.3 0 0 1 2.3 2.3v3.45" />
                    <path d="M5 8.5h8.5" />
                    <path d="M5 12.5h6.2" />
                    <path d="M5 16.5h4.1" />
                    <path d="m14.2 16.7 2.05 2.05L20.8 14.2" />
                    <path d="M4.5 3.5h11.2a3.8 3.8 0 0 1 3.8 3.8v2.2" />
                </svg>
            </button
            >
            <button
                class="btn-secondary"
                title="偏好设置"
                on:click={() => {
                    showSettingsPanel = true;
                    showEpubModal = false;
                    showHistoryPanel = false;
                }}>⚙️</button
            >
        </div>
    </header>

    <div class="main-body">
        {#if showSidebar && isMobile}
            <div
                role="presentation"
                class="sidebar-mask"
                on:click={() => (showSidebar = false)}
            ></div>
        {/if}

        {#if showSidebar}
            <aside class="sidebar">
                <!-- 头部固定，不再随列表滚动 -->
                <div class="sidebar-header-fixed">
                    <div class="sidebar-header-row">
                        <span>{stats.volumes}卷 {stats.chapters}章</span>
                        <div class="header-btns">
                            <button
                                class="icon-btn"
                                title="全部展开/折叠"
                                on:click={() => {
                                    const anyExpanded = tocTree.some(
                                        (n) => n.type === "Volume" && n.expanded,
                                    );
                                    const targetState = !anyExpanded;
                                    tocTree.forEach((n) => {
                                        if (n.type !== "Volume") return;
                                        n.expanded = targetState;
                                        const key = getVolumeCollapseKey(n);
                                        if (targetState) {
                                            userCollapsedVolumeKeys.delete(key);
                                        } else {
                                            userCollapsedVolumeKeys.add(key);
                                        }
                                    });
                                    userCollapsedVolumeKeys = new Set(
                                        userCollapsedVolumeKeys,
                                    );
                                    tocTree = [...tocTree];
                                }}>⇅</button
                            >
                            <button
                                class="mini-btn {(isCheckModeOn || (showProofPanel && proofActiveTab === 'check')) ? 'active' : ''}"
                                on:pointerdown={openProofCheckPanel}
                                on:click={openProofCheckPanel}>检查</button
                            >
                        </div>
                    </div>
                </div>

                <div class="toc-list">
                    {#each tocTree as node (node.id)}
                        <div
                            role="button"
                            tabindex="0"
                            id={`toc-${node.id}`}
                            class="toc-item {node.type === 'Volume'
                                ? 'vol-title'
                                : ''} {activeChapterId === node.id
                                ? 'active'
                                : ''}"
                            on:click={() =>
                                node.type === "Volume"
                                    ? toggleVolumeNode(node)
                                    : editorComponent.scrollToLine(
                                          node.line_number,
                                      )}
                            on:keydown={() => {}}
                        >
                            {#if node.type === "Volume"}
                                <span class="arrow"
                                    >{node.expanded ? "▼" : "▶"}</span
                                >
                            {/if}
                            <span
                                class="toc-title {invalidSequenceIds.has(
                                    node.id,
                                )
                                    ? 'text-error'
                                    : ''}">{node.title}</span
                            >
                            <span class="toc-count"
                                >{node.type === "Volume"
                                    ? node.children.length
                                    : node.word_count}</span
                            >
                        </div>

                        {#if node.expanded}
                            {#each node.children as child (child.id)}
                                <div
                                    role="button"
                                    tabindex="0"
                                    id={`toc-${child.id}`}
                                    class="toc-item indent {activeChapterId ===
                                    child.id
                                        ? 'active'
                                        : ''}"
                                    on:click={() =>
                                        handleChapterClick(
                                            child.id,
                                            child.line_number,
                                        )}
                                    on:keydown={() => {}}
                                >
                                    <span
                                        class="toc-title {invalidSequenceIds.has(
                                            child.id,
                                        )
                                            ? 'text-error'
                                            : ''}">{child.title}</span>
                                    <span class="toc-count"
                                        >{child.word_count}</span
                                    >
                                </div>
                            {/each}
                        {/if}
                    {/each}
                </div>
            </aside>
        {/if}

        <section class="editor-wrapper">
            {#if isLoading}<div class="loading">加载中...</div>{/if}
            <Editor
                bind:this={editorComponent}
                doc={fileContent}
                titleLines={flatToc.map((n) => n.line)}
                onChange={(v) => {
                    fileContent = v;
                    isModified = true;
                    // Debounced TOC Sync
                    clearTimeout(autoRefreshTimer);
                    autoRefreshTimer = setTimeout(() => scanToc(v), 200);
                }}
                onScroll={handleScroll}
                onSelectionChange={handleSelectionChange}
                wordWrap={appSettings.wordWrap}
                showWhitespace={appSettings.showWhitespace}
                showLineBreaks={appSettings.showLineBreaks}
                onTocSearch={handleTocSearch}
            />
        </section>

        {#if showProofPanel}
            <aside class="proof-panel">
                <div class="proof-header">
                    <div>
                        <div class="proof-title">校对</div>
                        <div class="proof-subtitle">{proofPreviewMessage}</div>
                    </div>
                    <button
                        class="icon-close proof-close"
                        title="关闭"
                        on:click={() => (showProofPanel = false)}>✕</button
                    >
                </div>

                <div class="proof-tabs">
                    <button
                        class:active={proofActiveTab === "check"}
                        on:click={() => {
                            proofActiveTab = "check";
                            runFullCheck();
                        }}>标题检查</button
                    >
                    <button
                        class:active={proofActiveTab === "toc"}
                        on:click={() => (proofActiveTab = "toc")}>目录重排</button
                    >
                    <button
                        class:active={proofActiveTab === "builtin"}
                        on:click={() => (proofActiveTab = "builtin")}>文本检查</button
                    >
                    <button
                        class:active={proofActiveTab === "ai"}
                        on:click={() => (proofActiveTab = "ai")}>智能校对</button
                    >
                </div>

                <div class="proof-body">
                    {#if proofActiveTab === "toc"}
                        <div class="proof-section proof-toc-controls">
                            <div class="proof-row">
                                <label for="proof-title-scope">范围</label>
                                <select id="proof-title-scope" bind:value={proofTitleScope}>
                                    <option value="all">卷和章</option>
                                    <option value="chapters">只排章节</option>
                                    <option value="volumes">只排卷部</option>
                                    <option value="numbers-only">只转数字</option>
                                    <option value="regex">正则选取</option>
                                </select>
                            </div>
                            {#if proofTitleScope === "regex"}
                                <div class="proof-row vertical">
                                    <label for="proof-title-regex">标题正则</label>
                                    <input
                                        id="proof-title-regex"
                                        bind:value={proofTitleRegex}
                                        placeholder="例如：^\\s*\\d+\\."
                                    />
                                </div>
                            {/if}
                            {#if proofTitleScope === "all" || proofTitleScope === "volumes" || proofTitleScope === "regex" || proofTitleScope === "numbers-only"}
                                <div class="proof-row">
                                    <label for="proof-volume-number-style">卷数字</label>
                                    <select id="proof-volume-number-style" bind:value={proofVolumeNumberStyle}>
                                        <option value="chinese">一二三四</option>
                                        <option value="arabic">1234</option>
                                    </select>
                                </div>
                            {/if}
                            {#if proofTitleScope === "all" || proofTitleScope === "chapters" || proofTitleScope === "regex" || proofTitleScope === "numbers-only"}
                                <div class="proof-row">
                                    <label for="proof-chapter-number-style">章数字</label>
                                    <select id="proof-chapter-number-style" bind:value={proofChapterNumberStyle}>
                                        <option value="chinese">一二三四</option>
                                        <option value="arabic">1234</option>
                                    </select>
                                </div>
                            {/if}
                            <label class="proof-check">
                                <input type="checkbox" bind:checked={proofPerVolume} />
                                每卷章节都从第一章开始
                            </label>
                            <button
                                class="proof-primary"
                                disabled={proofPreviewRows.length === 0}
                                on:click={applyProofTitleRewrite}
                            >
                                {proofTitleScope === "numbers-only" ? "应用数字转换" : "应用目录重排序"}
                            </button>
                        </div>

                        <div class="proof-preview">
                            <div class="proof-preview-head">
                                <span>原标题</span>
                                <span>修改后</span>
                            </div>
                            {#each visibleProofPreviewRows as row}
                                <div
                                    class:volume={row.kind === "volume"}
                                    class="proof-preview-row"
                                >
                                    <div
                                        class:sequence-broken={row.sequenceBroken}
                                        role="button"
                                        tabindex="0"
                                        title={row.sequenceBroken ? `断序：当前应为第 ${row.expectedIndex} 章，原标题为第 ${row.originalIndex} 章` : row.original}
                                        on:click={() => jumpToProofTitleRow(row)}
                                        on:keydown={(e) => {
                                            if (e.key === "Enter" || e.key === " ") {
                                                jumpToProofTitleRow(row);
                                            }
                                        }}
                                    >
                                        {#if row.kind === "volume"}
                                            <button
                                                class="proof-volume-toggle"
                                                title={proofCollapsedVolumeKeys.has(row.volumeKey) ? "展开本卷" : "折叠本卷"}
                                                on:click|stopPropagation={() => toggleProofVolumeCollapse(row.volumeKey)}
                                            >{proofCollapsedVolumeKeys.has(row.volumeKey) ? "▶" : "▼"}</button>
                                        {/if}
                                        <span>{row.original}</span>
                                    </div>
                                    <div class:cell-changed={row.changed} title={row.replacement}>{row.replacement}</div>
                                </div>
                            {:else}
                                <div class="proof-empty">暂无预览</div>
                            {/each}
                        </div>
                    {:else if proofActiveTab === "builtin"}
                        <div class="proof-section proof-regex-controls">
                            <div class="proof-row">
                                <label for="proof-text-tool">类型</label>
                                <select
                                    id="proof-text-tool"
                                    bind:value={proofTextTool}
                                    on:change={() => {
                                        proofRegexSelectedIds = new Set();
                                        proofConvertPreviewRows = [];
                                        proofConvertSelectedIds = new Set();
                                    }}
                                >
                                    <option value="builtin">内置规则</option>
                                    <option value="convert">繁简转换</option>
                                </select>
                            </div>
                            {#if proofTextTool === "builtin"}
                            <div class="proof-row">
                                <label for="proof-builtin-rule">规则</label>
                                <select id="proof-builtin-rule" bind:value={proofBuiltinRule}>
                                    {#each PROOF_BUILTIN_REGEX_RULES as rule}
                                        <option value={rule.id}>{rule.name}</option>
                                    {/each}
                                </select>
                            </div>
                            <div class="proof-rule-note">
                                {PROOF_BUILTIN_REGEX_RULES.find((rule) => rule.id === proofBuiltinRule)?.description}
                            </div>
                            <div class="proof-actions-row">
                                <button on:click={() => setAllProofRegexRows(true)}>全选</button>
                                <button on:click={() => setAllProofRegexRows(false)}>全不选</button>
                                <button
                                    class="proof-primary inline"
                                    disabled={proofRegexSelectedIds.size === 0}
                                    on:click={applySelectedBuiltinRegex}
                                >替换选中</button>
                                <button
                                    class="proof-primary inline"
                                    disabled={proofRegexPreviewRows.length === 0}
                                    on:click={applyAllBuiltinRegex}
                                >全部替换</button>
                            </div>
                            {:else}
                            <div class="proof-row">
                                <label for="proof-convert-direction">方向</label>
                                <select
                                    id="proof-convert-direction"
                                    bind:value={proofConvertDirection}
                                    on:change={() => {
                                        proofConvertPreviewRows = [];
                                        proofConvertSelectedIds = new Set();
                                    }}
                                >
                                    <option value="traditional-to-simplified">繁体转简体</option>
                                    <option value="simplified-to-traditional">简体转繁体</option>
                                </select>
                            </div>
                            <div class="proof-actions-row proof-convert-actions">
                                <button class="proof-primary inline" on:click={runProofConvertPreview}>
                                    查找
                                </button>
                                <button on:click={() => setAllProofConvertRows(true)}>全选</button>
                                <button on:click={() => setAllProofConvertRows(false)}>全不选</button>
                                <button
                                    class="proof-primary inline"
                                    disabled={proofConvertSelectedIds.size === 0}
                                    on:click={applySelectedConvertRows}
                                >替换选中</button>
                                <button
                                    class="proof-primary inline"
                                    disabled={proofConvertPreviewRows.length === 0}
                                    on:click={applyAllConvertRows}
                                >全部替换</button>
                                <button class="proof-primary inline" on:click={runProofFullConvert}>
                                    转换全文
                                </button>
                            </div>
                            {/if}
                        </div>

                        {#if proofTextTool === "builtin"}
                        <div class="proof-regex-preview">
                            <div class="proof-regex-head">
                                <span></span>
                                <span>匹配内容</span>
                                <span>替换后</span>
                            </div>
                            {#each proofRegexPreviewRows as row}
                                <label class="proof-regex-row">
                                    <input
                                        type="checkbox"
                                        checked={proofRegexSelectedIds.has(row.id)}
                                        on:change={(e) =>
                                            toggleProofRegexRow(
                                                row.id,
                                                (e.currentTarget as HTMLInputElement).checked,
                                            )}
                                    />
                                    <span
                                        role="button"
                                        tabindex="0"
                                        title={row.original}
                                        on:click|preventDefault={() => jumpToProofRegexRow(row)}
                                        on:keydown|preventDefault={(e) => {
                                            if (e.key === "Enter" || e.key === " ") {
                                                jumpToProofRegexRow(row);
                                            }
                                        }}
                                    >
                                        {row.original}
                                    </span>
                                    <span title={row.replacement || "删除"}>
                                        {row.replacement || "删除"}
                                    </span>
                                </label>
                            {:else}
                                <div class="proof-empty">暂无匹配</div>
                            {/each}
                        </div>
                        {:else}
                        <div class="proof-regex-preview">
                            <div class="proof-regex-head">
                                <span></span>
                                <span>原文</span>
                                <span>转换后</span>
                            </div>
                            {#each proofConvertPreviewRows as row}
                                <label class="proof-regex-row">
                                    <input
                                        type="checkbox"
                                        checked={proofConvertSelectedIds.has(row.id)}
                                        on:change={(e) =>
                                            toggleProofConvertRow(
                                                row.id,
                                                (e.currentTarget as HTMLInputElement).checked,
                                            )}
                                    />
                                    <span
                                        role="button"
                                        tabindex="0"
                                        title={row.original}
                                        on:click|preventDefault={() => jumpToProofConvertRow(row)}
                                        on:keydown|preventDefault={(e) => {
                                            if (e.key === "Enter" || e.key === " ") {
                                                jumpToProofConvertRow(row);
                                            }
                                        }}
                                    >
                                        {row.original}
                                    </span>
                                    <span title={row.replacement}>{row.replacement}</span>
                                </label>
                            {:else}
                                <div class="proof-empty">点击查找后显示可转换内容</div>
                            {/each}
                        </div>
                        {/if}
                    {:else if proofActiveTab === "check"}
                        <div class="proof-check-panel">
                            <div class="proof-section proof-word-check-controls">
                                <div class="proof-row">
                                    <label for="proof-word-min">低于字数</label>
                                    <input
                                        id="proof-word-min"
                                        type="number"
                                        min="0"
                                        bind:value={appSettings.wordCountMinThreshold}
                                        on:change={normalizeWordCheckSettings}
                                    />
                                </div>
                                <div class="proof-row">
                                    <label for="proof-word-max">高于字数</label>
                                    <input
                                        id="proof-word-max"
                                        type="number"
                                        min="0"
                                        bind:value={appSettings.wordCountMaxThreshold}
                                        on:change={normalizeWordCheckSettings}
                                    />
                                </div>
                            </div>
                            <div class="proof-actions-row check-actions">
                                <button class="proof-primary inline" on:click={runFullCheck}>
                                    重新检查
                                </button>
                            </div>

                            <div class="proof-check-list">
                                <div class="check-sec">
                                    <div
                                        class="sec-title"
                                        role="button"
                                        tabindex="0"
                                        on:click={() => (checkCollapseState.seq = !checkCollapseState.seq)}
                                        on:keydown={(e) => e.key === "Enter" && (checkCollapseState.seq = !checkCollapseState.seq)}
                                    >
                                        <span>{checkCollapseState.seq ? "▶" : "▼"} 断序检查 ({sequenceErrors.length})</span>
                                    </div>
                                    {#if !checkCollapseState.seq}
                                        <div class="tag-list">
                                            {#each sequenceErrors as e}
                                                <button
                                                    class="err-tag"
                                                    on:click={() => handleChapterClick(e.id, e.line)}
                                                >
                                                    <span class="err-tag-msg">{e.msg}</span>
                                                    <span class="err-tag-title">{e.title}</span>
                                                </button>
                                            {:else}<span class="toc-count">无</span>{/each}
                                        </div>
                                    {/if}
                                </div>

                                <div class="check-sec">
                                    <div
                                        class="sec-title"
                                        role="button"
                                        tabindex="0"
                                        on:click={() => (checkCollapseState.title = !checkCollapseState.title)}
                                        on:keydown={(e) => e.key === "Enter" && (checkCollapseState.title = !checkCollapseState.title)}
                                    >
                                        <span>{checkCollapseState.title ? "▶" : "▼"} 标题内容 ({titleErrors.length})</span>
                                    </div>
                                    {#if !checkCollapseState.title}
                                        <div class="tag-list">
                                            {#each titleErrors as e}
                                                <button
                                                    class="err-tag"
                                                    on:click={() => handleChapterClick(e.id, e.line)}
                                                >{e.title}</button>
                                            {:else}<span class="toc-count">无</span>{/each}
                                        </div>
                                    {/if}
                                </div>

                                <div class="check-sec">
                                    <div
                                        class="sec-title"
                                        role="button"
                                        tabindex="0"
                                        on:click={() => (checkCollapseState.word = !checkCollapseState.word)}
                                        on:keydown={(e) => e.key === "Enter" && (checkCollapseState.word = !checkCollapseState.word)}
                                    >
                                        <span>{checkCollapseState.word ? "▶" : "▼"} 字数检查 ({wordCountErrors.length})</span>
                                    </div>
                                    {#if !checkCollapseState.word}
                                        <div class="tag-list">
                                            {#each wordCountErrors as e}
                                            <button
                                                class="err-tag"
                                                on:click={() => handleChapterClick(e.id, e.line)}
                                            >
                                                <span class="err-tag-title">{e.title}</span>
                                                <span class="err-tag-count">{e.val}</span>
                                            </button>
                                            {:else}<span class="toc-count">无</span>{/each}
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                    {:else}
                        <div class="proof-section">
                            <div class="proof-row">
                                <label for="ai-proof-scope">范围</label>
                <select id="ai-proof-scope" bind:value={aiProofingScope} disabled={aiProofingRunning}>
                    <option value="current">当前章</option>
                    {#if hasProofVolumeScope}
                    <option value="volume">当前卷</option>
                    {/if}
                    <option value="all">全书逐章</option>
                </select>
                            </div>
                            <div class="proof-actions-row proof-convert-actions proof-ai-actions">
                                <button
                                    class="proof-primary proof-ai-action-btn"
                                    disabled={aiProofingRunning}
                                    on:click={runAiProofing}
                                >{aiProofingRunning ? "校对中..." : "开始"}</button>
                                <button class="proof-ai-action-btn" disabled={!aiProofingRunning} on:click={stopAiProofing}>停止</button>
                                <button
                                    class="proof-ai-action-btn"
                                    class:active={aiProofingView === "suggestions"}
                                    on:click={() => (aiProofingView = "suggestions")}
                                >建议</button>
                                {#if aiProofingConfig.autoApprove}
                                <button
                                    class="proof-ai-action-btn"
                                    class:active={aiProofingView === "approval"}
                                    on:click={() => (aiProofingView = "approval")}
                                >审批</button>
                                {/if}
                                <button
                                    class="proof-ai-action-btn"
                                    class:active={aiProofingView === "log"}
                                    on:click={() => (aiProofingView = "log")}
                                >日志</button>
                            </div>
                        </div>

                        {#if aiProofingView === "suggestions"}
                        <div class="proof-regex-preview">
                            <div class="proof-regex-head">
                                <span></span>
                                <span>原文</span>
                                <span>建议</span>
                            </div>
                            <div class="ai-proof-selection-bar">
                                <span>已选 {aiProofingSelectedIds.size} / {aiProofingRows.length}</span>
                                <button disabled={aiProofingRunning} on:click={() => setAllAiProofingRows(true)}>全选</button>
                                <button disabled={aiProofingRunning} on:click={() => setAllAiProofingRows(false)}>全不选</button>
                                <button
                                    class="proof-primary inline"
                                    disabled={aiProofingRunning || aiProofingSelectedIds.size === 0}
                                    on:click={applySelectedAiProofingRows}
                                >应用选中</button>
                            </div>
                            {#each aiProofingRows as row}
                                <label class="proof-regex-row">
                                    <input
                                        type="checkbox"
                                        checked={aiProofingSelectedIds.has(row.id)}
                                        on:change={(e) =>
                                            toggleAiProofingRow(
                                                row.id,
                                                (e.currentTarget as HTMLInputElement).checked,
                                            )}
                                    />
                                    <span
                                        role="button"
                                        tabindex="0"
                                        title={`${row.chapterTitle}\n原因：${aiProofingReasonText(row)}\n原文：${row.original}`}
                                        on:click|preventDefault={() => jumpToAiProofingRow(row)}
                                        on:keydown|preventDefault={(e) => {
                                            if (e.key === "Enter" || e.key === " ") {
                                                jumpToAiProofingRow(row);
                                            }
                                        }}
                                    >
                                        {compactAiProofingCellText(row.original)}
                                    </span>
                                    <span
                                        class="ai-proof-replacement-cell"
                                        title={buildAiProofingReplacementTooltip(row)}
                                        aria-label={buildAiProofingReplacementTooltip(row)}
                                    >
                                        <span class="ai-proof-replacement-text">
                                            {row.fullChapterRemoval ? "移除整章" : compactAiProofingCellText(row.replacement || "删除")}
                                        </span>
                                    </span>
                                </label>
                            {:else}
                                <div class="proof-empty">点击开始校对后显示建议</div>
                            {/each}
                        </div>
                        {:else if aiProofingView === "approval"}
                        <div class="proof-regex-preview">
                            <div class="proof-regex-head approval-head">
                                <span>状态</span>
                                <span>审批通过</span>
                                <span>操作</span>
                            </div>
                            {#each aiApprovalAppliedBatches as batch}
                                {#each batch.rows as row}
                                    <div class="proof-regex-row approval-row">
                                        <span>{(batch.revertedRowIds || []).includes(row.id) ? "已撤销" : "已应用"}</span>
                                        <span
                                            role="button"
                                            tabindex="0"
                                            class="approval-result-cell approval-result-link"
                                            on:click|preventDefault={() => jumpToAiProofingRow(row)}
                                            on:keydown|preventDefault={(e) => {
                                                if (e.key === "Enter" || e.key === " ") {
                                                    jumpToAiProofingRow(row);
                                                }
                                            }}
                                        >
                                            <small>{row.original} -> {row.replacement || "删除"}（{row.reason}）</small>
                                        </span>
                                        <span class="approval-action-cell">
                                            {#if (batch.revertedRowIds || []).includes(row.id)}
                                                <button
                                                    type="button"
                                                    class="mini-action"
                                                    disabled={aiProofingRunning}
                                                    on:click={() => reapplyAiApprovalRow(batch.id, row.id)}
                                                >应用</button>
                                            {:else}
                                                <button
                                                    type="button"
                                                    class="mini-action"
                                                    disabled={aiProofingRunning}
                                                    on:click={() => revertAiApprovalRow(batch.id, row.id)}
                                                >撤销</button>
                                            {/if}
                                        </span>
                                    </div>
                                {/each}
                            {:else}
                                <div class="proof-empty">暂无 AI 审批应用记录</div>
                            {/each}
                        </div>
                        {:else}
                        <div
                            role="log"
                            class="ai-proof-log ai-proof-log-view"
                            data-native-context-menu="true"
                            on:contextmenu|stopPropagation
                        >
                            {#each aiProofingLogs as item}
                                <div>{item}</div>
                            {:else}
                                <div>暂无日志</div>
                            {/each}
                        </div>
                        {/if}
                    {/if}
                </div>

                <div class="proof-footer">
                    {proofMessage || "处理结果会显示在这里，文本修改后可用撤销返回。"}
                </div>
            </aside>
        {/if}

    </div>

    {#if showSettingsPanel}
        <div
            role="presentation"
            class="modal-overlay"
            on:click={() => {
                showSettingsPanel = false;
            }}
        >
            <div role="presentation" on:click|stopPropagation>
                <SettingsShell
                    title="偏好设置"
                    tabs={editorSettingsTabs}
                    activeTab={settingsActiveTab}
                    onTabChange={(tabId) => {
                        if (tabId === "fonts") openSettingsFontsTab();
                        else if (tabId === "styles") openSettingsStylesTab();
                        else if (tabId === "api") openSettingsApiTab();
                        else if (tabId === "ai") openSettingsAiTab();
                        else if (tabId === "proofLogs") openSettingsProofLogsTab();
                        else if (tabId === "history") openSettingsHistoryTab();
                        else settingsActiveTab = tabId as typeof settingsActiveTab;
                    }}
                    onClose={() => (showSettingsPanel = false)}
                    actionLabel="保存并应用"
                    onAction={saveEditorSettings}
                    shellClass="editor-settings-modal"
                    contentClass="editor-settings-content"
                >
                    {#if settingsActiveTab === 'display'}
                        <div class="settings-section display-settings-panel">
                            <div class="section-title">显示设置</div>
                            <div class="section-hint">把常用开关收成一列卡片，减少左右拉扯，和书库设置页保持同一套节奏。</div>
                            <div class="settings-toggle-list">
                                <label class="settings-toggle-card" for="wordWrap">
                                    <div class="settings-toggle-copy">
                                        <span class="settings-toggle-title">自动换行</span>
                                        <span class="settings-toggle-note">编辑时按容器宽度自动换行，减少横向滚动。</span>
                                    </div>
                                    <input id="wordWrap" type="checkbox" bind:checked={appSettings.wordWrap} />
                                </label>
                                <label class="settings-toggle-card" for="showWhitespace">
                                    <div class="settings-toggle-copy">
                                        <span class="settings-toggle-title">显示空格</span>
                                        <span class="settings-toggle-note">把空格可视化，便于清理多余缩进和格式噪点。</span>
                                    </div>
                                    <input id="showWhitespace" type="checkbox" bind:checked={appSettings.showWhitespace} />
                                </label>
                                <label class="settings-toggle-card" for="showLineBreaks">
                                    <div class="settings-toggle-copy">
                                        <span class="settings-toggle-title">显示换行符</span>
                                        <span class="settings-toggle-note">把段落换行位置标出来，排查断行和章节格式更直接。</span>
                                    </div>
                                    <input id="showLineBreaks" type="checkbox" bind:checked={appSettings.showLineBreaks} />
                                </label>
                                <label class="settings-toggle-card" for="clh">
                                    <div class="settings-toggle-copy">
                                        <span class="settings-toggle-title">保存清空撤销</span>
                                        <span class="settings-toggle-note">保存成功后重置当前撤销栈，避免历史状态和新内容混在一起。</span>
                                    </div>
                                    <input id="clh" type="checkbox" bind:checked={appSettings.clearHistoryOnSave} />
                                </label>
                                <label class="settings-toggle-card" for="subsetFonts">
                                    <div class="settings-toggle-copy">
                                        <span class="settings-toggle-title">字体子集化</span>
                                        <span class="settings-toggle-note">导出时只保留用到的字形，通常能进一步压缩 EPUB 体积。</span>
                                    </div>
                                    <input id="subsetFonts" type="checkbox" bind:checked={appSettings.subsetFonts} />
                                </label>
                            </div>
                        </div>
                        <!-- 撤销开关 -->
                        <div class="set-row" style="display: none;">
                            <label for="wth" style="display: none;">单章字数检查:</label>
                            <input
                                id="wth"
                                type="hidden"
                                bind:value={appSettings.wordCountMaxThreshold}
                                style="width: 80px;"
                            />
                        </div>
                    {:else if settingsActiveTab === 'fonts'}
                        <div class="font-settings-panel">
                            <div class="font-settings-head">
                                <div>
                                    <div class="font-settings-title">外部字体</div>
                                    <div class="font-settings-note">导入后自动复制到当前书库目录下的“字体”文件夹，并可直接在块编辑里选择使用。</div>
                                </div>
                                <button class="grid-btn blue" disabled={isImportingFont} on:click={importExternalFont}>
                                    {isImportingFont ? "导入中..." : "导入字体"}
                                </button>
                            </div>
                            {#if fontSettingsMessage}
                                <div class="font-settings-status">{fontSettingsMessage}</div>
                            {/if}
                            <div class="font-settings-list">
                                {#each importedFonts as font}
                                    <div class="font-settings-item">
                                        <div class="font-settings-item-top">
                                            <div class="font-settings-meta">
                                                <strong>{font.family}</strong>
                                                <span>{font.file_name}</span>
                                            </div>
                                            <div class="font-settings-actions">
                                                <button
                                                    class="mini-action"
                                                    type="button"
                                                    disabled={renamingFontFileName === font.file_name || deletingFontFileName === font.file_name}
                                                    on:click={() => renameImportedFont(font)}
                                                >{renamingFontFileName === font.file_name ? "重命名中..." : "重命名"}</button>
                                                <button
                                                    class="mini-action"
                                                    type="button"
                                                    disabled={deletingFontFileName === font.file_name || renamingFontFileName === font.file_name}
                                                    on:click={() => deleteImportedFont(font)}
                                                >{deletingFontFileName === font.file_name ? "删除中..." : "删除"}</button>
                                            </div>
                                        </div>
                                        <code>{font.css_value}</code>
                                    </div>
                                {:else}
                                    <div class="empty-msg">还没有导入外部字体</div>
                                {/each}
                            </div>
                        </div>
                    {:else if settingsActiveTab === 'styles'}
                        <div class="font-settings-panel style-settings-panel">
                            <div class="font-settings-head">
                                <div>
                                    <div class="font-settings-title">样式模板</div>
                                    <div class="font-settings-note">选择默认 CSS 模板，并直接编辑当前 EPUB 输出样式。</div>
                                </div>
                                <div class="style-settings-actions">
                                    <button class="mini-action style-toolbar-btn style-toolbar-btn-primary" disabled={isImportingStyleTemplate} on:click={importStyleTemplateFile}>
                                        {isImportingStyleTemplate ? "导入中..." : "导入 CSS"}
                                    </button>
                                    <button class="mini-action style-toolbar-btn" on:click={() => {
                                        if (showStyleSourceEditor) {
                                            applyResolvedStyleTemplateCss(styleSourceDraft);
                                            showStyleSourceEditor = false;
                                        } else {
                                            syncToolbarStyleToEpubMeta();
                                            styleSourceDraft = epubMeta.styles["main.css"];
                                            showStyleSourceEditor = true;
                                        }
                                    }}>{showStyleSourceEditor ? "块编辑" : "源码"}</button>
                                    <button class="mini-action style-toolbar-btn" on:click={resetToolbarStyleBlocks}>重置</button>
                                    <button class="mini-action primary style-toolbar-btn" disabled={isSavingStyleTemplate} on:click={saveCurrentStyleTemplate}>{isSavingStyleTemplate ? "保存中..." : "保存模板"}</button>
                                </div>
                            </div>
                            {#if styleSettingsMessage}
                                <div class="font-settings-status">{styleSettingsMessage}</div>
                            {/if}
                            <div class="style-template-list">
                                {#each styleTemplates as template}
                                    <label class="style-template-item {appSettings.selectedStyleTemplateId === template.id ? 'active' : ''}">
                                        <div class="style-template-main">
                                            <input
                                                type="radio"
                                                name="editor-style-template"
                                                checked={appSettings.selectedStyleTemplateId === template.id}
                                                on:change={() => {
                                                    appSettings.selectedStyleTemplateId = template.id;
                                                    currentStyleTemplateName = template.name;
                                                }}
                                            />
                                            <div class="style-template-meta">
                                                <strong>{template.name}</strong>
                                                <span>{template.is_builtin ? "内置模板（始终置顶）" : template.file_name}</span>
                                            </div>
                                        </div>
                                        {#if template.is_builtin}
                                            <button class="mini-action" type="button" on:click|stopPropagation={restoreBuiltinStyleTemplateToDefault}>恢复默认样式</button>
                                        {/if}
                                    </label>
                                {/each}
                            </div>
                            <div class="style-settings-editor">
                                <div class="style-panel-header in-settings">
                                    <div>
                                        <div class="style-panel-title">样式编辑</div>
                                        <div class="style-panel-subtitle">当前模板：{currentStyleTemplateName}</div>
                                    </div>
                                </div>
                                {#if showStyleSourceEditor}
                                    <div class="style-source-editor">
                                        <textarea
                                            spellcheck="false"
                                            bind:value={styleSourceDraft}
                                            on:input={() => {
                                                epubMeta.styles["main.css"] = styleSourceDraft;
                                                epubMeta.styles = { ...epubMeta.styles };
                                            }}
                                        ></textarea>
                                    </div>
                                {:else}
                                    <div class="style-block-editor-layout">
                                        <nav class="style-block-nav" aria-label="样式块">
                                        {#each visibleStyleBlocks as block}
                                            <button
                                                type="button"
                                                class:active={activeStyleBlockId === block.id}
                                                style={`--block-accent:${block.accent}`}
                                                on:click={() => (activeStyleBlockId = block.id)}
                                            >
                                                <strong>{block.title}</strong>
                                                <span>{block.note}</span>
                                            </button>
                                        {/each}
                                        </nav>
                                        <div class="style-block-detail">
                                        {#if activeStyleBlock}
                                            <section class="style-block-card" style={`--block-accent:${activeStyleBlock.accent}`}>
                                                <header class="style-block-head">
                                                    <div>
                                                        <strong>{activeStyleBlock.title}</strong>
                                                        <span>{activeStyleBlock.note}</span>
                                                    </div>
                                                    <code>
                                                        {#each activeStyleBlock.selector.split(",").map((item) => item.trim()).filter(Boolean) as selector}
                                                            <span>{selector}</span>
                                                        {/each}
                                                    </code>
                                                </header>
                                                <div class="style-prop-list">
                                                    {#each activeStyleBlock.properties as prop}
                                                        <label class="style-prop-row">
                                                            <span class="prop-title">
                                                                <span class="prop-label">{prop.label}</span>
                                                                <span class="prop-name">{prop.name}</span>
                                                            </span>
                                                            {#if prop.options}
                                                                <select
                                                                    value={prop.value}
                                                                    on:change={(event) => updateToolbarStyleBlock(styleBlocks.findIndex((item) => item.id === activeStyleBlock.id), styleBlocks.find((item) => item.id === activeStyleBlock.id)?.properties.findIndex((item) => item.name === prop.name) ?? -1, event.currentTarget.value)}>
                                                                    {#each (prop.name === "font-family" ? fontFamilyOptions : prop.options) as option}
                                                                        <option value={option.value}>{option.label}</option>
                                                                    {/each}
                                                                </select>
                                                            {:else if prop.color}
                                                                {@const parsedColor = parseCssColorValue(prop.value)}
                                                                <span class="color-value-control">
                                                                    <input
                                                                        class="color-swatch"
                                                                        type="color"
                                                                        value={parsedColor.swatch}
                                                                        title={prop.value}
                                                                        on:input={(event) => updateToolbarColorValue(activeStyleBlock.id, prop.name, event.currentTarget.value)}
                                                                    />
                                                                    <input
                                                                        class="color-hex-input"
                                                                        value={parsedColor.hex}
                                                                        placeholder="#RRGGBB / #RRGGBBAA"
                                                                        on:input={(event) => updateToolbarColorValue(activeStyleBlock.id, prop.name, event.currentTarget.value)}
                                                                    />
                                                                </span>
                                                            {:else}
                                                                <input
                                                                    value={prop.value}
                                                                    on:input={(event) => updateToolbarStyleBlock(styleBlocks.findIndex((item) => item.id === activeStyleBlock.id), styleBlocks.find((item) => item.id === activeStyleBlock.id)?.properties.findIndex((item) => item.name === prop.name) ?? -1, event.currentTarget.value)}
                                                                />
                                                            {/if}
                                                        </label>
                                                    {/each}
                                                </div>
                                            </section>
                                        {/if}
                                        </div>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {:else if settingsActiveTab === 'toc'}
                        <div class="rules-header">正则表达式</div>
                        <div class="rule-helper-row">
                            <select class="rule-type rule-type-helper" bind:value={tocPrefixLevel}>
                                <option value={1}>层级 1</option>
                                <option value={2}>层级 2</option>
                                <option value={3}>层级 3</option>
                                <option value={4}>层级 4</option>
                                <option value={5}>层级 5</option>
                            </select>
                            <input
                                class="rule-prefix-input"
                                bind:value={tocPrefixText}
                                placeholder="前缀识别，例如：番外、后日谈"
                            />
                            <button
                                type="button"
                                class="rule-btn add"
                                on:click={() => {
                                    const prefix = tocPrefixText.trim();
                                    if (!prefix) return;
                                    appSettings.customRegexRules = [
                                        ...appSettings.customRegexRules,
                                        { level: tocPrefixLevel, pattern: buildTitlePrefixRegex(prefix) },
                                    ];
                                    tocPrefixText = "";
                                    tocPrefixLevel = 3;
                                }}
                            >＋ 前缀</button>
                        </div>
                        <div class="rules-list">
                            {#each appSettings.customRegexRules as rule, idx}
                                <div class="rule-item" style="gap: 8px; align-items: center;">
                                    <select class="rule-type" bind:value={rule.level} style="width: 112px; flex-shrink: 0;">
                                        <option value={1}>层级 1</option>
                                        <option value={2}>层级 2</option>
                                        <option value={3}>层级 3</option>
                                        <option value={4}>层级 4</option>
                                        <option value={5}>层级 5</option>
                                    </select>

                                    <div class="rule-input-group">
                                        <input
                                            class="rule-input"
                                            type="text"
                                            bind:value={rule.pattern}
                                            placeholder="输入正则表达式"
                                        />
                                        <div class="rule-arrow-visual">▼</div>
                                        <select
                                            class="rule-hidden-select"
                                            on:change={(e) => {
                                                const val = e.currentTarget.value;
                                                if(val) rule.pattern = val;
                                                e.currentTarget.value = "";
                                            }}
                                        >
                                            <option value="">选择预设正则</option>
                                            {#each REGEX_PRESETS as preset}
                                                {#if preset.value}
                                                    <option value={preset.value}>{preset.value}</option>
                                                {/if}
                                            {/each}
                                        </select>
                                    </div>

                                    <button class="rule-btn remove" on:click={() => {
                                        appSettings.customRegexRules.splice(idx, 1);
                                        appSettings.customRegexRules = [...appSettings.customRegexRules];
                                    }}>－</button>
                                </div>
                            {/each}
                        </div>
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-top: 10px;">
                            <button class="grid-btn" style="padding: 4px 10px; font-size: 13px;" on:click={() => {
                                appSettings.customRegexRules = [
                                    { level: 1, pattern: DEFAULT_META_VOLUME_REGEX },
                                    { level: 1, pattern: DEFAULT_SETTINGS.volRegex },
                                    { level: 3, pattern: DEFAULT_META_BODY_REGEX },
                                    { level: 3, pattern: DEFAULT_SETTINGS.chapRegex }
                                ];
                            }}>↺ 还原正则</button>
                            <button class="grid-btn" style="padding: 4px 10px; font-size: 13px; color: #0066b8; border-color: #0066b8;" on:click={() => {
                                appSettings.customRegexRules.push({ level: 3, pattern: "" });
                                appSettings.customRegexRules = [...appSettings.customRegexRules];
                            }}>＋ 新增正则</button>
                        </div>

                    {:else if settingsActiveTab === 'api'}
                        <div class="ai-settings-form">
                            {#if aiSettingsMessage}
                                <div class="font-settings-status">{aiSettingsMessage}</div>
                            {/if}
                            <div class="font-settings-head ai-provider-head">
                                <div>
                                    <div class="font-settings-title">API 配置</div>
                                    <div class="font-settings-note">可添加多个 OpenAI 兼容 API，分别供校对和自动审批选择。</div>
                                </div>
                                <div class="style-settings-actions">
                                    <button class="mini-action" type="button" on:click={addTxtAiProvider}>新增</button>
                                </div>
                            </div>

                            {#if aiProviders.length === 0 && !apiEditorOpen}
                                <div class="api-empty">暂无 API 配置，点击“新增”添加文字模型。</div>
                            {:else}
                                <div class="api-list">
                                    {#each aiProviders as provider}
                                        <div class="api-item">
                                            <div class="api-item-main">
                                                <div class="api-item-title">
                                                    <strong>{provider.name || provider.model}</strong>
                                                    <span>文字模型</span>
                                                </div>
                                                <div class="api-item-meta">
                                                    <span>{provider.model || "未填写模型"}</span>
                                                    <span>{provider.baseUrl || "未填写 API 地址"}</span>
                                                    <span>{provider.apiKey ? "已保存 Key" : "未填写 Key"}</span>
                                                </div>
                                            </div>
                                            <div class="api-item-actions">
                                                <button class="mini-action" type="button" on:click={() => editTxtAiProvider(provider)}>编辑</button>
                                                <button class="mini-action danger" type="button" on:click={() => removeTxtAiProvider(provider.id)}>删除</button>
                                            </div>
                                        </div>
                                    {/each}
                                </div>
                            {/if}

                            {#if apiEditorOpen && apiEditorDraft}
                            <div class="api-editor">
                                <div class="api-editor-head">
                                    <strong>{apiEditorMode === "new" ? "新增 API" : "编辑 API"}</strong>
                                    <div class="api-editor-actions">
                                        <button class="mini-action primary" type="button" on:click={saveTxtAiProviderEditor}>保存</button>
                                        <button class="mini-action" type="button" on:click={cancelTxtAiProviderEditor}>取消</button>
                                    </div>
                                </div>
                                <div class="set-row">
                                    <label for="aiProviderName">名称:</label>
                                    <input id="aiProviderName" type="text" value={apiEditorDraft.name || ""} on:input={(e) => updateSelectedTxtAiProvider("name", e.currentTarget.value)} />
                                </div>
                                <div class="set-row">
                                    <label for="aiProviderBaseUrl">API 地址:</label>
                                    <input id="aiProviderBaseUrl" type="text" value={apiEditorDraft.baseUrl || ""} on:input={(e) => updateSelectedTxtAiProvider("baseUrl", e.currentTarget.value)} placeholder="https://api.openai.com/v1" />
                                </div>
                                <div class="set-row">
                                    <label for="aiProviderKey">API Key:</label>
                                    <input id="aiProviderKey" type="password" value={apiEditorDraft.apiKey || ""} on:input={(e) => updateSelectedTxtAiProvider("apiKey", e.currentTarget.value)} placeholder="sk-..." />
                                </div>
                                <div class="set-row">
                                    <label for="aiProviderModel">模型:</label>
                                    <input id="aiProviderModel" type="text" value={apiEditorDraft.model || ""} on:input={(e) => updateSelectedTxtAiProvider("model", e.currentTarget.value)} placeholder="deepseek-chat / gpt-4o-mini" />
                                </div>
                                <div class="set-row">
                                    <label for="aiProviderTemperature">温度:</label>
                                    <input id="aiProviderTemperature" type="number" min="0" max="1" step="0.1" value={apiEditorDraft.temperature ?? 0.1} on:input={(e) => updateSelectedTxtAiProvider("temperature", Number(e.currentTarget.value))} />
                                </div>
                            </div>
                            {/if}
                        </div>
                    {:else if settingsActiveTab === 'ai'}
                        <div class="ai-settings-form">
                            {#if aiSettingsMessage}
                                <div class="font-settings-status">{aiSettingsMessage}</div>
                            {/if}
                            <div class="set-row">
                                <label for="aiProofProvider">校对 API:</label>
                                <select id="aiProofProvider" bind:value={txtAiProofingConfig.providerId} on:change={() => (aiProofingConfig = proofingConfigForProvider(txtAiProofingConfig.providerId))}>
                                    {#each aiProviders as provider}
                                        <option value={provider.id}>{provider.name || provider.model}</option>
                                    {/each}
                                </select>
                            </div>
                            <div class="set-row">
                                <label for="aiApprovalProvider">审批 API:</label>
                                <select id="aiApprovalProvider" bind:value={txtAiProofingConfig.approvalProviderId}>
                                    {#each aiProviders as provider}
                                        <option value={provider.id}>{provider.name || provider.model}</option>
                                    {/each}
                                </select>
                            </div>
                            <div class="set-row">
                                <label for="aiMaxChars">单章上限:</label>
                                <input id="aiMaxChars" type="number" min="1000" step="1000" bind:value={aiProofingConfig.maxChapterChars} />
                            </div>
                            <div class="set-row">
                                <label for="aiTimeoutSec">最长响应时间:</label>
                                <input id="aiTimeoutSec" type="number" min="30" max="1800" step="30" bind:value={aiProofingConfig.responseTimeoutSec} />
                            </div>
                            <label class="set-row ai-toggle-row" for="aiAutoApprove">
                                <span>自动审批:</span>
                                <input id="aiAutoApprove" type="checkbox" bind:checked={aiProofingConfig.autoApprove} />
                                <small>AI 审核非自动建议，通过则自动应用，存疑保留人工审核。</small>
                            </label>
                            <div class="set-row ai-textarea-row">
                                <label for="aiExtraPrompt">额外要求:</label>
                                <textarea id="aiExtraPrompt" rows="4" bind:value={aiProofingConfig.extraPrompt} placeholder="例如：保留作者口癖，不做风格润色。"></textarea>
                            </div>
                        </div>
                    {:else if settingsActiveTab === 'proofLogs'}
                        <div class="proof-log-settings">
                            <div class="proof-log-toolbar">
                                <span>{proofLogMessage || `共 ${proofLogList.length} 个日志文件`}</span>
                                <button class="mini-action" type="button" on:click={() => loadProofLogs(false)}>刷新</button>
                            </div>
                            <div class="proof-log-browser">
                                <div class="proof-log-list">
                                    {#each proofLogList as log}
                                        <button
                                            type="button"
                                            class:active={selectedProofLogPath === log.path}
                                            on:click={() => openProofLog(log)}
                                        >
                                            <span class="proof-log-name">{log.fileName}</span>
                                            <span class="proof-log-meta">
                                                {new Date(log.timestamp * 1000).toLocaleString()} · {(log.size / 1024).toFixed(1)}KB
                                            </span>
                                        </button>
                                    {:else}
                                        <div class="empty-msg">暂无校对日志</div>
                                    {/each}
                                </div>
                                <textarea
                                    class="proof-log-content"
                                    readonly
                                    value={selectedProofLogContent || "选择左侧日志后查看完整内容"}
                                    data-native-context-menu="true"
                                    on:contextmenu|stopPropagation
                                ></textarea>
                            </div>
                        </div>
                    {:else}
                        <div class="history-settings-card">
                            {#each historyList as h}
                                <button
                                    class="hist-item"
                                    on:click={() => {
                                        restoreTargetSnapshot = h;
                                        showRestoreConfirm = true;
                                    }}
                                >
                                    <span class="hist-time">{new Date(h.timestamp * 1000).toLocaleString()}</span>
                                    <span class="hist-size">{(h.size / 1024).toFixed(1)}KB</span>
                                </button>
                            {:else}
                                <div class="empty-msg">暂无历史快照</div>
                            {/each}
                        </div>
                    {/if}
                </SettingsShell>
            </div>
        </div>
    {:else if showEpubModal || showHistoryPanel}
        <div
            role="presentation"
            class="modal-overlay"
            on:click={() => {
                showEpubModal = false;
                showHistoryPanel = false;
            }}
        >
            <div
                role="presentation"
                class="modal-content"
                class:epub-modal-shell={showEpubModal}
                on:click|stopPropagation
            >
                {#if showEpubModal}
                    <div class="p-header">
                        <span>制作 EPUB</span>
                        <button class="icon-close" on:click={closeAllPanels}>✕</button>
                    </div>
                    <div class="p-body epub-modal-body">
                        <div class="epub-main-layout">
                            <!-- 左侧：主要信息 -->
                            <div class="epub-fields-column">
                                <div class="set-row compact">
                                    <label for="et">书名:</label>
                                    <input id="et" type="text" bind:value={epubMeta.title} class="epub-input-small" />
                                </div>
                                <div class="set-row compact">
                                    <label for="ec">作者:</label>
                                    <input id="ec" type="text" bind:value={epubMeta.creator} class="epub-input-small" />
                                </div>
                                <div class="set-row compact align-start">
                                    <label for="ed">简介:</label>
                                    <textarea
                                        id="ed"
                                        rows="6"
                                        bind:value={epubMeta.description}
                                        class="epub-textarea"
                                        placeholder="请输入书籍简介..."
                                    ></textarea>
                                </div>
                                <div class="set-row compact align-start">
                                    <label>标签:</label>
                                    <TagsEditor bind:tags={epubMeta.tags} suggestions={[]} />
                                </div>
                            </div>

                            <!-- 右侧：封面预览 -->
                            <div class="epub-cover-column">
                                <div
                                    class="epub-cover-preview"
                                    on:click={pickLocalCover}
                                    role="button"
                                    tabindex="0"
                                    on:keydown={(e) => e.key === 'Enter' && (e.target as HTMLElement).click()}
                                >
                                    {#if coverPreviewUrl}
                                        <img src={coverPreviewUrl} alt="封面预检" />
                                        <div class="cover-hint">点击更换封面</div>
                                    {:else if epubMeta.cover_path}
                                        <div class="no-cover">
                                            <span>⏳</span>
                                            <span>加载中...</span>
                                        </div>
                                    {:else}
                                        <div class="no-cover">
                                            <span>➕</span>
                                            <span>添加封面</span>
                                        </div>
                                    {/if}
                                </div>
                                <div class="cover-source-actions">
                                    <button
                                        class="cover-source-btn primary"
                                        disabled={isCoverSearching || isCoverApplying}
                                        on:click={searchCovers}
                                    >
                                        {isCoverSearching ? "搜索中..." : "搜索封面"}
                                    </button>
                                    <button
                                        class="cover-source-btn"
                                        disabled={isCoverApplying}
                                        on:click={pickLocalCover}
                                    >
                                        本地图片
                                    </button>
                                </div>
                                {#if coverSearchMessage}
                                    <div class="cover-search-status">{coverSearchMessage}</div>
                                {/if}
                            </div>
                        </div>

                        {#if coverSearchResults.length}
                            <div class="cover-results-panel">
                                <div class="cover-results-head">
                                    <span>封面结果</span>
                                    <small>点击图片使用，结果按书名和封面来源排序</small>
                                </div>
                                <div class="cover-result-grid" aria-label="封面搜索结果">
                                    {#each coverSearchResults as result (result.image_url)}
                                        <button
                                            class="cover-result-card"
                                            disabled={isCoverApplying}
                                            title={`${result.title}${result.source ? ` / ${result.source}` : ""}`}
                                            on:click={() => applyRemoteCover(result)}
                                        >
                                            <span class="cover-result-image-wrap">
                                                <img src={result.image_url} alt="" loading="lazy" aria-hidden="true" />
                                            </span>
                                            <span class="cover-result-info">
                                                <span class="cover-result-title">{result.title || "未命名"}</span>
                                                <span class="cover-result-source">{result.source || "未知来源"}</span>
                                            </span>
                                        </button>
                                    {/each}
                                </div>
                            </div>
                        {/if}

                        {#if epubGenerationStatus !== "idle"}
                            <div class="epub-status-line" class:success={epubGenerationStatus === "success"}>
                                {epubGenerationStatus === "generating" ? "正在制作 EPUB..." : "EPUB 制作完成"}
                            </div>
                        {/if}

                        <div class="epub-modal-footer">
                            {#if epubGenerationStatus !== "success"}
                                <button class="epub-cancel" on:click={openAdvancedEpubMetadata}>
                                    高级选项
                                </button>
                            {/if}
                            {#if epubGenerationStatus === "success"}
                                <button class="epub-cancel" on:click={openGeneratedEpubInEditor}>
                                    用 EPUB 编辑器打开
                                </button>
                                <button class="epub-cancel" on:click={revealGeneratedEpub}>
                                    打开文件位置
                                </button>
                            {/if}
                            <button
                                class="epub-confirm"
                                disabled={epubGenerationStatus === "generating"}
                                on:click={generateEpub}
                            >
                                {epubGenerationStatus === "generating"
                                    ? "制作中..."
                                    : "开始制作"}
                            </button>
                        </div>
                    </div>
                {:else if showHistoryPanel}
                    <div class="p-header">
                        <div style="display:flex; align-items:center;">
                            <button
                                class="icon-close"
                                style="font-size:18px; margin-right:8px; transform:rotate(180deg);"
                                on:click={() => {
                                    showHistoryPanel = false;
                                    showSettingsPanel = true;
                                }}>➜</button
                            >
                            <span>历史版本</span>
                        </div>
                        <button class="icon-close" on:click={closeAllPanels}
                            >✕</button
                        >
                    </div>
                    <div class="p-body scroll-p">
                        {#each historyList as h}
                            <button
                                class="hist-item"
                                on:click={() => {
                                    restoreTargetSnapshot = h;
                                    showRestoreConfirm = true;
                                }}
                            >
                                <span
                                    class="hist-time"
                                    >{new Date(
                                        h.timestamp * 1000,
                                    ).toLocaleString()}</span
                                >
                                <span class="hist-size">{(h.size / 1024).toFixed(1)}KB</span>
                            </button>
                        {:else}
                            <div class="empty-msg">暂无历史快照</div>
                        {/each}
                    </div>
                {/if}
            </div>
        </div>
    {/if}

    <!-- 历史回退确认弹窗 -->
    {#if showRestoreConfirm}
        <div
            role="presentation"
            class="modal-overlay"
            on:click={() => {
                showRestoreConfirm = false;
                restoreTargetSnapshot = null;
            }}
        >
            <div
                role="presentation"
                class="modal-content"
                style="max-width: 400px; padding: 30px; text-align: center;"
                on:click|stopPropagation
            >
                <div
                    style="font-size: 18px; margin-bottom: 20px; font-weight: bold;"
                >
                    确认回退到历史版本？
                </div>
                <div style="color: #666; margin-bottom: 30px; line-height:1.6;">
                    当前版本将自动保存为新的历史记录。<br />
                    此操作可以再次回退。
                </div>
                <div style="display: flex; gap: 12px; justify-content: center;">
                    <button
                        class="btn-small"
                        style="flex: 1; max-width: 120px;"
                        on:click={() => {
                            showRestoreConfirm = false;
                            restoreTargetSnapshot = null;
                        }}
                    >
                        取消
                    </button>
                    <button
                        class="btn-small"
                        style="flex: 1; max-width: 120px; background: linear-gradient(135deg, #0066b8, #0088dd); color: white; border: none;"
                        on:click={confirmRestore}
                    >
                        确认回退
                    </button>
                </div>
            </div>
        </div>
    {/if}

    {#if false && showCheckPanel}
        <div
            class="check-panel"
            style="left: {checkPanelPos.x}px; top: {checkPanelPos.y}px;"
        >
            <!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
            <div
                class="find-header"
                on:mousedown={(e) => startDrag(e, "check")}
                role="application"
                aria-label="拖拽以移动内容检查面板"
            >
                <span class="find-title">全书检查</span>
                <button
                    class="icon-close"
                    on:click={() => (showCheckPanel = false)}>✕</button
                >
            </div>
            <div
                class="find-body scroll-p"
                style="max-height: 400px; overflow-y: auto;"
            >
                <!-- 章节跳转连贯性 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.seq = !checkCollapseState.seq)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.seq = !checkCollapseState.seq)}
                    >
                        <span>{checkCollapseState.seq ? "▶" : "▼"} 断序检查 ({sequenceErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.seq}
                        <div class="tag-list">
                            {#each sequenceErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >
                                    <span class="err-tag-msg">{e.msg}</span>
                                    <span class="err-tag-title">{e.title}</span>
                                </button>
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 标题空 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.title = !checkCollapseState.title)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.title = !checkCollapseState.title)}
                    >
                        <span>{checkCollapseState.title ? "▶" : "▼"} 标题内容 ({titleErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.title}
                        <div class="tag-list">
                            {#each titleErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >{e.title}</button>
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>

                <!-- 字数 -->
                <div class="check-sec">
                    <div
                        class="sec-title"
                        role="button"
                        tabindex="0"
                        on:click={() => (checkCollapseState.word = !checkCollapseState.word)}
                        on:keydown={(e) => e.key === 'Enter' && (checkCollapseState.word = !checkCollapseState.word)}
                    >
                        <span>{checkCollapseState.word ? "▶" : "▼"} 字数检查 ({wordCountErrors.length})</span>
                    </div>
                    {#if !checkCollapseState.word}
                        <div class="tag-list">
                            {#each wordCountErrors as e}
                                <button
                                    class="err-tag"
                                    on:click={() => handleChapterClick(e.id, e.line)}
                                >{e.title} ({e.val})</button
                                >
                            {:else}<span class="toc-count">无</span>{/each}
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    {/if}
</main>

<!-- Context Menu -->
<ContextMenu enableTitleActions={true} />

{#if showCloseDialog}
    <div class="dialog-overlay">
        <div class="dialog">
            <div class="dialog-header">未保存的更改</div>
            <div class="dialog-content">
                当前文件包含未保存的更改，是否保存并退出？
            </div>
            <div class="dialog-actions">
                <!-- 假设 saveFile 已存在 -->
                <button
                    class="btn primary"
                    on:click={handleDialogSave}
                    disabled={isDialogSaving}
                >
                    {isDialogSaving ? "保存中..." : "保存"}
                </button>
                <button
                    class="btn danger"
                    on:click={handleDialogDiscard}
                    disabled={isDialogSaving}>不保存</button
                >
                <button
                    class="btn secondary"
                    on:click={handleDialogCancel}
                    disabled={isDialogSaving}>取消</button
                >
            </div>
        </div>
    </div>
{/if}

<style>
    /* Dialog Styles (Matched with Epub Editor) */
    .dialog-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 2000; /* High z-index */
    }

    .dialog {
        background: white;
        padding: 20px;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        min-width: 300px;
        color: #333;
    }

    .dialog-header {
        font-size: 18px;
        font-weight: bold;
        margin-bottom: 15px;
    }

    .dialog-content {
        margin-bottom: 20px;
        color: #333;
    }

    .dialog-actions {
        display: flex;
        justify-content: flex-end;
        gap: 10px;
    }

    .btn {
        padding: 8px 16px;
        border-radius: 4px;
        border: none;
        cursor: pointer;
        font-weight: 500;
        font-size: 14px;
    }
    .btn.primary {
        background: #2196f3;
        color: white;
    }

    .btn.danger {
        background: #f44336;
        color: white;
    }

    .btn.secondary {
        background: #e0e0e0;
        color: #333;
    }
    :global(body) {
        margin: 0;
        background: #fff;
        overflow: hidden;
        -webkit-touch-callout: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        user-select: none;

        font-family: system-ui;
    }
    .app-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 100vw;
    }
    .toolbar {
        padding-top: env(safe-area-inset-top);
        background: #f3f3f3;
        height: 44px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding-left: 10px;
        padding-right: 10px;
        border-bottom: 1px solid #ddd;
        z-index: 100;
    }
    .btn-group {
        display: flex;
        gap: 6px;
    }
    .toolbar-tail {
        display: flex;
        gap: 6px;
        margin-left: auto;
    }
    button {
        height: 34px;
        min-width: 40px;
        border-radius: 6px;
        border: 1px solid #ccc;
        background: #fff;
        font-size: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        outline: none;
        transition: 0.1s;
    }
    button:active {
        background: #eee;
        transform: scale(0.96);
    }
    .btn-primary {
        background: #0066b8;
        color: #fff;
        border: none;
    }
    .btn-save-modified {
        background: #d32f2f;
        color: #fff;
        border: none;
        animation: pulse 2s infinite;
    }
    @keyframes pulse {
        0% {
            opacity: 1;
        }
        50% {
            opacity: 0.7;
        }
        100% {
            opacity: 1;
        }
    }

    .main-body {
        flex: 1;
        display: flex;
        overflow: hidden;
        position: relative;
    }
    .sidebar {
        width: 280px;
        background: #f8f8f8;
        border-right: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
    }

    .proof-panel {
        width: 400px;
        min-width: 360px;
        max-width: min(440px, 44vw);
        background: #fff;
        border-left: 1px solid #ddd;
        display: flex;
        flex-direction: column;
        flex-shrink: 0;
        overflow: hidden;
    }

    .proof-tool-btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 0;
    }

    .proof-tool-btn svg {
        width: 20px;
        height: 20px;
        fill: none;
        stroke: currentColor;
        stroke-width: 1.9;
        stroke-linecap: round;
        stroke-linejoin: round;
    }

    .style-panel-header {
        min-height: 58px;
        padding: 8px 10px;
        border-bottom: 1px solid #dfe5ec;
        background: rgba(255, 255, 255, 0.86);
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        box-sizing: border-box;
    }

    .style-panel-title {
        color: #172434;
        font-size: 16px;
        font-weight: 900;
        line-height: 1.3;
    }

    .style-panel-subtitle {
        color: #758294;
        font-size: 11px;
        line-height: 1.4;
        margin-top: 2px;
    }

    .mini-action {
        height: 34px;
        min-width: 0;
        padding: 0 12px;
        border: 1px solid #d7e0ea;
        border-radius: 10px;
        background: #fff;
        color: #526071;
        font-size: 12px;
        font-weight: 700;
        box-shadow: 0 4px 12px rgba(15, 23, 42, 0.06);
    }

    .mini-action.primary {
        border-color: #1677b8;
        background: #e6f2fb;
        color: #11679f;
    }

    .mini-action.danger {
        border-color: #f1b8b8;
        color: #c03535;
    }

    .style-block-editor-layout {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-columns: 178px minmax(0, 1fr);
        gap: 0;
        overflow: hidden;
    }

    .style-block-nav {
        min-height: 0;
        overflow-y: auto;
        padding: 12px;
        border-right: 1px solid #e1e6ee;
        background: rgba(248, 250, 252, 0.9);
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .style-block-nav button {
        width: 100%;
        height: auto;
        min-width: 0;
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        gap: 3px;
        padding: 10px 11px;
        border: 1px solid transparent;
        border-radius: 8px;
        background: transparent;
        color: #536171;
        text-align: left;
        box-shadow: none;
        transform: none;
    }

    .style-block-nav button.active {
        border-color: rgba(22, 119, 184, 0.24);
        background: #fff;
        color: var(--block-accent);
        box-shadow: 0 6px 14px rgba(23, 36, 52, 0.07);
    }

    .style-block-nav strong {
        width: 100%;
        font-size: 13px;
        line-height: 1.3;
        text-align: left;
    }

    .style-block-nav span {
        width: 100%;
        max-width: 100%;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #8793a2;
        font-size: 10px;
        line-height: 1.25;
        text-align: left;
    }

    .style-block-detail {
        min-width: 0;
        min-height: 0;
        overflow-y: auto;
        padding: 14px;
    }

    .style-block-card {
        overflow: hidden;
        border: 1px solid #e1e6ee;
        border-radius: 8px;
        background: rgba(250, 251, 255, 0.9);
        box-shadow: 0 6px 16px rgba(23, 36, 52, 0.05);
        min-height: max-content;
    }

    .style-block-head {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        padding: 14px 16px;
        border-bottom: 1px solid #e1e6ee;
        background: rgba(255, 255, 255, 0.76);
    }

    .style-block-head strong {
        display: block;
        color: var(--block-accent);
        font-family: Georgia, "Times New Roman", serif;
        font-size: 0.98rem;
        letter-spacing: 0;
        line-height: 1.25;
    }

    .style-block-head span {
        display: block;
        margin-top: 2px;
        color: #7b8794;
        font-size: 10px;
        line-height: 1.35;
    }

    .style-block-head code {
        padding: 5px 8px;
        border-radius: 10px;
        background: rgba(166, 120, 29, 0.1);
        color: var(--block-accent);
        font-family: Consolas, monospace;
        font-size: 10px;
        display: grid;
        gap: 4px;
        line-height: 1.45;
        text-align: right;
        max-width: 180px;
    }

    .style-block-head code span {
        display: block;
        margin-top: 0;
        white-space: normal;
        word-break: break-all;
    }

    .style-prop-list {
        display: grid;
        padding: 10px 16px 16px;
    }

    .style-prop-row {
        display: grid;
        grid-template-columns: minmax(112px, 150px) minmax(0, 1fr);
        align-items: center;
        gap: 14px;
        min-height: 50px;
        border-bottom: 1px solid rgba(226, 232, 240, 0.8);
    }

    .style-prop-row:last-child {
        border-bottom: 0;
    }

    .prop-title {
        display: grid;
        gap: 2px;
        min-width: 0;
    }

    .prop-label {
        color: #8a94a3;
        font-size: 10px;
        white-space: nowrap;
        line-height: 1.2;
    }

    .prop-name {
        color: #151923;
        font-family: Georgia, "Times New Roman", serif;
        font-size: 0.92rem;
        font-weight: 800;
        letter-spacing: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        line-height: 1.2;
    }

    .style-prop-row input,
    .style-prop-row select {
        width: 100%;
        min-width: 0;
        max-width: 100%;
        box-sizing: border-box;
        border: 1px solid transparent;
        border-radius: 6px;
        background: rgba(255, 255, 255, 0.78);
        color: #172434;
        font: inherit;
        font-size: 0.8rem;
        padding: 7px 10px;
    }

    .style-prop-row input:focus,
    .style-prop-row select:focus {
        outline: none;
        border-color: #1677b8;
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.16);
        background: #fff;
    }

    .color-value-control {
        display: grid;
        grid-template-columns: 28px minmax(0, 1fr);
        grid-template-areas: "swatch text";
        align-items: center;
        gap: 6px;
        min-width: 0;
        width: 100%;
        max-width: 100%;
        box-sizing: border-box;
    }

    .style-prop-row input.color-swatch {
        grid-area: swatch;
        width: 28px;
        height: 28px;
        padding: 2px;
        border-color: #d8e1ea;
        background: #fff;
        cursor: pointer;
    }

    .style-prop-row input.color-hex-input {
        grid-area: text;
        font-family: Consolas, "SFMono-Regular", monospace;
        text-transform: uppercase;
    }

    .style-source-editor {
        flex: 1;
        min-height: 0;
        padding: 14px;
        display: flex;
    }

    .font-settings-panel {
        display: grid;
        gap: 12px;
        min-height: 0;
    }

    .style-template-list {
        display: grid;
        gap: 10px;
    }

    .style-template-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 10px 12px;
        border: 1px solid #e0e6ed;
        border-radius: 8px;
        background: #fff;
    }

    .style-template-item.active {
        border-color: #1677b8;
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.1);
    }

    .style-template-main {
        display: flex;
        align-items: center;
        gap: 10px;
        min-width: 0;
        flex: 1;
    }

    .style-template-main input[type="radio"] {
        width: auto;
        flex-shrink: 0;
    }

    .style-template-meta {
        display: grid;
        gap: 4px;
        min-width: 0;
    }

    .style-template-meta strong {
        font-size: 13px;
        color: #1b2d3f;
    }

    .style-template-meta span {
        font-size: 11px;
        color: #6d7887;
        word-break: break-all;
    }

    .font-settings-head {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 12px;
    }

    .font-settings-title {
        font-size: 14px;
        font-weight: 800;
        color: #253547;
    }

    .font-settings-note {
        margin-top: 4px;
        font-size: 12px;
        line-height: 1.5;
        color: #708093;
    }

    .font-settings-status {
        padding: 8px 10px;
        border-radius: 8px;
        background: #f5f9fc;
        color: #486277;
        font-size: 12px;
        line-height: 1.5;
    }

    .style-settings-panel {
        gap: 16px;
    }

    .style-settings-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 8px;
    }

    .style-toolbar-btn {
        min-width: 88px;
        justify-content: center;
    }

    .style-toolbar-btn-primary {
        border-color: #1e8fd2;
        background: linear-gradient(135deg, #2a95d8 0%, #1ab0c8 100%);
        color: #fff;
        box-shadow: 0 10px 20px rgba(30, 143, 210, 0.2);
    }

    .style-toolbar-btn-primary:disabled {
        opacity: 0.72;
        box-shadow: none;
    }

    .style-settings-editor {
        min-height: 360px;
        overflow: hidden;
        border: 1px solid #e1e6ee;
        border-radius: 10px;
        background: #f6f7fb;
        display: flex;
        flex-direction: column;
    }

    .style-panel-header.in-settings {
        min-height: 50px;
    }

    .ai-settings-form {
        display: flex;
        flex-direction: column;
        gap: 12px;
        max-width: 100%;
        padding-top: 4px;
    }

    .ai-settings-form .set-row {
        margin-bottom: 0;
    }

    .ai-provider-head {
        margin-bottom: 4px;
    }

    .api-empty,
    .api-item,
    .api-editor {
        border: 1px solid #e1e6ee;
        border-radius: 8px;
        background: #fff;
    }

    .api-empty {
        padding: 10px 12px;
        color: #667085;
        font-size: 13px;
        line-height: 1.5;
    }

    .api-list {
        display: grid;
        gap: 10px;
    }

    .api-item {
        min-width: 0;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 12px;
    }

    .api-item-main {
        min-width: 0;
        display: grid;
        gap: 6px;
    }

    .api-item-title,
    .api-item-actions,
    .api-editor-head,
    .api-editor-actions {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .api-item-title strong {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 14px;
        color: #253547;
    }

    .api-item-title span {
        flex: 0 0 auto;
        padding: 2px 7px;
        border-radius: 999px;
        background: #eaf5fc;
        color: #176b94;
        font-size: 11px;
        font-weight: 800;
    }

    .api-item-meta {
        min-width: 0;
        display: flex;
        flex-wrap: wrap;
        gap: 6px 12px;
        color: #708093;
        font-size: 12px;
        line-height: 1.4;
    }

    .api-item-meta span {
        max-width: 100%;
        overflow-wrap: anywhere;
    }

    .api-editor {
        margin-top: 2px;
        padding: 12px;
        display: grid;
        gap: 12px;
        background: #f9fbfd;
    }

    .api-editor-head {
        justify-content: space-between;
    }

    .api-editor-head strong {
        color: #253547;
        font-size: 14px;
    }

    .ai-settings-form .set-row label {
        width: 110px;
        flex-shrink: 0;
        font-weight: bold;
        color: #444;
        font-size: 13px;
        line-height: 1.4;
    }

    .ai-settings-form .set-row {
        align-items: center;
        justify-content: flex-start;
        gap: 10px;
    }

    .ai-settings-form .set-row input:not([type="checkbox"]),
    .ai-settings-form .set-row select,
    .ai-settings-form .set-row textarea {
        flex: 1 1 auto;
        min-width: 0;
        min-height: 34px;
        box-sizing: border-box;
        font-size: 13px;
    }

    .ai-settings-form .ai-textarea-row {
        align-items: flex-start;
    }

    .ai-settings-form .ai-textarea-row label {
        padding-top: 8px;
    }

    .ai-settings-form .set-row textarea {
        flex: 1;
        width: 100%;
        min-height: 78px;
        resize: vertical;
        line-height: 1.55;
    }

    .ai-settings-form .ai-toggle-row {
        display: grid;
        grid-template-columns: 110px 24px minmax(0, 1fr);
        align-items: center;
        gap: 10px;
    }

    .ai-settings-form .ai-toggle-row input {
        width: 16px;
        height: 16px;
        min-height: 0;
        accent-color: #0b92b3;
    }

    .ai-settings-form .ai-toggle-row small {
        color: #667085;
        font-size: 11px;
        line-height: 1.45;
    }

    .proof-log-settings {
        display: flex;
        flex-direction: column;
        gap: 10px;
        min-height: 0;
        height: 100%;
    }

    .proof-log-toolbar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        color: #52606d;
        font-size: 12px;
    }

    .proof-log-browser {
        flex: 1;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(180px, 0.9fr) minmax(0, 1.35fr);
        gap: 10px;
    }

    .proof-log-list {
        min-height: 0;
        overflow: auto;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding-right: 2px;
    }

    .proof-log-list button {
        width: 100%;
        min-height: 54px;
        display: grid;
        gap: 4px;
        padding: 9px 10px;
        border: 1px solid #e0e6ed;
        border-radius: 8px;
        background: #fff;
        text-align: left;
    }

    .proof-log-list button.active {
        border-color: #1677b8;
        background: #eef8ff;
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.1);
    }

    .proof-log-name {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: #1b2d3f;
        font-size: 12px;
        font-weight: 800;
    }

    .proof-log-meta {
        color: #748295;
        font-size: 11px;
    }

    .proof-log-content {
        width: 100%;
        min-width: 0;
        min-height: 0;
        resize: none;
        padding: 10px;
        border: 1px solid #d6e0eb;
        border-radius: 8px;
        background: #fbfcfe;
        color: #334155;
        font-family: Consolas, "Microsoft YaHei", monospace;
        font-size: 11px;
        line-height: 1.55;
        box-sizing: border-box;
    }

    .font-settings-list {
        display: grid;
        gap: 8px;
        max-height: 320px;
        overflow: auto;
        padding-right: 2px;
    }

    .font-settings-item {
        display: grid;
        gap: 6px;
        padding: 10px 12px;
        border: 1px solid #e0e6ed;
        border-radius: 8px;
        background: #fff;
    }

    .font-settings-item-top {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        gap: 12px;
    }

    .font-settings-meta {
        display: grid;
        gap: 2px;
    }

    .font-settings-meta strong {
        font-size: 13px;
        color: #1b2d3f;
    }

    .font-settings-meta span {
        font-size: 11px;
        color: #7b8794;
    }

    .font-settings-item code {
        font-size: 11px;
        color: #466176;
        white-space: normal;
        word-break: break-word;
    }

    .font-settings-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: 6px;
        flex-shrink: 0;
    }

    .style-source-editor textarea {
        flex: 1;
        width: 100%;
        min-height: 0;
        resize: none;
        box-sizing: border-box;
        border: 1px solid #d6e0eb;
        border-radius: 8px;
        background: #fff;
        color: #172434;
        font-family: Consolas, "SFMono-Regular", monospace;
        font-size: 12px;
        line-height: 1.55;
        padding: 12px;
        outline: none;
        box-shadow: inset 0 1px 3px rgba(23, 36, 52, 0.05);
    }

    .style-source-editor textarea:focus {
        border-color: #1677b8;
        box-shadow: 0 0 0 3px rgba(22, 119, 184, 0.16);
    }

    .proof-header {
        height: 54px;
        min-height: 54px;
        padding: 6px 12px;
        border-bottom: 1px solid #e5e5e5;
        background: #f8fafc;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
        box-sizing: border-box;
    }

    .proof-header > div:first-child {
        min-width: 0;
    }

    .proof-title {
        color: #333;
        font-size: 16px;
        font-weight: 800;
        line-height: 1.4;
    }

    .proof-subtitle {
        color: #777;
        font-size: 12px;
        line-height: 1.5;
        margin-top: 2px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .proof-close {
        flex-shrink: 0;
    }

    .proof-tabs {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 6px;
        padding: 8px 10px;
        border-bottom: 1px solid #e7e7e7;
        background: #fbfbfb;
    }

    .proof-tabs button {
        height: 30px;
        min-width: 0;
        font-size: 12px;
        border-radius: 6px;
    }

    .proof-tabs button.active {
        background: #0066b8;
        border-color: #0066b8;
        color: #fff;
    }

    .proof-body {
        flex: 1;
        min-height: 0;
        overflow: hidden;
        padding: 12px;
        display: flex;
        flex-direction: column;
        gap: 12px;
        background: #fff;
    }

    .proof-section {
        display: flex;
        flex-direction: column;
        gap: 10px;
        flex-shrink: 0;
    }

    .proof-toc-controls {
        max-height: 232px;
    }

    .proof-word-check-controls {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 10px;
    }

    .proof-word-check-controls .proof-row {
        min-width: 0;
    }

    .proof-word-check-controls .proof-row label {
        width: auto;
        min-width: 74px;
        white-space: nowrap;
    }

    .proof-word-check-controls .proof-row input {
        min-width: 0;
        width: 100%;
        box-sizing: border-box;
    }

    .proof-row {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .proof-row.vertical {
        align-items: stretch;
        flex-direction: column;
        gap: 6px;
    }

    .proof-row label,
    .proof-check {
        color: #555;
        font-size: 13px;
        font-weight: 700;
    }

    .proof-row label {
        width: 72px;
        flex-shrink: 0;
    }

    .proof-row.vertical label {
        width: auto;
    }

    .proof-row input,
    .proof-row select {
        min-width: 0;
        flex: 1;
        box-sizing: border-box;
        border: 1px solid #ccc;
        border-radius: 6px;
        color: #333;
        font-size: 13px;
        outline: none;
    }

    .proof-row input {
        padding: 7px 9px;
        background: #fff;
    }

    .proof-row select,
    .set-row select,
    .rule-type {
        min-height: var(--control-height);
        padding: 7px 38px 7px 12px;
        background-color: color-mix(in srgb, var(--color-surface) 94%, var(--color-accent-quiet));
        background-image:
            linear-gradient(45deg, transparent 50%, var(--color-text-soft) 50%),
            linear-gradient(135deg, var(--color-text-soft) 50%, transparent 50%),
            linear-gradient(180deg, rgba(255, 255, 255, 0.82), rgba(241, 248, 253, 0.82));
        background-position:
            calc(100% - 20px) 50%,
            calc(100% - 14px) 50%,
            0 0;
        background-size:
            6px 6px,
            6px 6px,
            100% 100%;
        background-repeat: no-repeat;
        appearance: none;
        -webkit-appearance: none;
        box-shadow: var(--shadow-xs);
    }

    .proof-row input:focus,
    .proof-row select:focus {
        border-color: #0066b8;
        box-shadow: 0 0 0 2px rgba(0, 102, 184, 0.18);
    }

    .proof-check {
        display: flex;
        align-items: center;
        gap: 8px;
        font-weight: 500;
        cursor: pointer;
        line-height: 1.5;
    }

    .proof-check input {
        width: 16px;
        height: 16px;
        margin: 0;
        accent-color: #0066b8;
    }

    .proof-primary {
        width: 100%;
        height: 36px;
        background: #0066b8;
        border-color: #0066b8;
        color: #fff;
        font-size: 13px;
        font-weight: 700;
    }

    .proof-primary:disabled {
        opacity: 0.46;
        cursor: not-allowed;
    }

    .proof-primary.inline {
        width: auto;
        min-width: 96px;
        padding: 0 12px;
    }

    .proof-preview {
        flex: 1;
        min-height: 0;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        overflow: auto;
        background: #fff;
    }

    .proof-preview-head,
    .proof-preview-row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 0;
    }

    .proof-preview-head {
        position: sticky;
        top: 0;
        z-index: 3;
        background: #f4f6f8;
        color: #555;
        font-size: 12px;
        font-weight: 800;
    }

    .proof-preview-head span,
    .proof-preview-row div {
        min-width: 0;
        padding: 8px 10px;
        border-right: 1px solid #eee;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .proof-preview-head span:last-child,
    .proof-preview-row div:last-child {
        border-right: 0;
    }

    .proof-preview-row {
        border-top: 1px solid #f0f0f0;
        color: #555;
        font-size: 12px;
    }

    .proof-preview-row.volume {
        position: sticky;
        top: 31px;
        z-index: 2;
        background: #f8fafc;
        color: #2d4d64;
        font-weight: 800;
        box-shadow: 0 1px 0 #e6edf3;
    }

    .proof-preview-row.volume .cell-changed {
        background: #edf8ff;
    }

    .proof-preview-row .sequence-broken {
        background: #fff3f3;
        color: #b42318;
    }

    .proof-preview-row .cell-changed {
        background: #f3fbff;
        color: #1f4f6b;
    }

    .proof-preview-row > div:first-child {
        display: flex;
        align-items: center;
        cursor: pointer;
    }

    .proof-preview-row > div:first-child > span {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .proof-volume-toggle {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 20px;
        height: 20px;
        min-width: 20px;
        margin: 0 6px 0 0;
        padding: 0;
        border: 0;
        background: transparent;
        color: inherit;
        box-shadow: none;
        font-size: 11px;
        vertical-align: middle;
    }

    .proof-check-panel {
        flex: 1;
        min-height: 0;
        display: flex;
        flex-direction: column;
        gap: 10px;
        overflow: hidden;
    }

    .proof-check-list {
        flex: 1;
        min-height: 0;
        overflow: auto;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        background: #fff;
    }

    .check-actions {
        grid-template-columns: 1fr;
    }

    .proof-empty {
        padding: 18px;
        text-align: center;
        color: #999;
        font-size: 13px;
    }

    .proof-rule-note {
        color: #666;
        font-size: 12px;
        line-height: 1.5;
        padding: 8px 10px;
        border-radius: 6px;
        background: #f7f9fb;
        border: 1px solid #e5e8ed;
    }

    .ai-proof-log {
        flex: 1;
        min-height: 0;
        max-height: none;
        overflow: auto;
        padding: 8px 10px;
        border: 1px solid #dfe5ec;
        border-radius: 6px;
        background: #fbfcfe;
        color: #52606d;
        font-family: Consolas, "Microsoft YaHei", monospace;
        font-size: 11px;
        line-height: 1.5;
        white-space: pre-wrap;
        cursor: text;
        -webkit-user-select: text;
        -moz-user-select: text;
        user-select: text;
    }

    .proof-actions-row {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        gap: 8px;
        align-items: center;
    }

    .proof-actions-row button {
        min-width: 0;
        height: 34px;
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .proof-convert-actions {
        grid-template-columns: repeat(auto-fit, minmax(64px, 1fr));
    }

    .proof-convert-actions button {
        width: 100%;
    }

    .proof-convert-actions button.active {
        border-color: #1697b8;
        background: #e5f7fb;
        color: #087a98;
        font-weight: 800;
    }

    .ai-proof-selection-bar {
        position: sticky;
        top: 29px;
        z-index: 1;
        display: grid;
        grid-template-columns: minmax(0, 1fr) 58px 68px 86px;
        gap: 8px;
        align-items: center;
        padding: 8px 10px;
        border-top: 1px solid #e7edf3;
        border-bottom: 1px solid #e7edf3;
        background: #fbfdff;
        color: #607086;
        font-size: 12px;
    }

    .ai-proof-selection-bar button {
        min-width: 0;
        height: 30px;
        padding: 0 8px;
        border: 1px solid #dce4ec;
        border-radius: 6px;
        background: #fff;
        color: #52606d;
        font-size: 12px;
        white-space: nowrap;
    }

    .proof-regex-preview {
        flex: 1;
        min-height: 0;
        overflow: auto;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        background: #fff;
    }

    .proof-regex-head,
    .proof-regex-row {
        display: grid;
        grid-template-columns: 34px minmax(0, 1.15fr) minmax(0, 0.85fr);
        align-items: stretch;
    }

    .proof-regex-head {
        position: sticky;
        top: 0;
        z-index: 1;
        background: #f4f6f8;
        color: #555;
        font-size: 12px;
        font-weight: 800;
    }

    .proof-regex-head span,
    .proof-regex-row > span {
        min-width: 0;
        padding: 8px 10px;
        border-right: 1px solid #eee;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .proof-regex-row {
        border-top: 1px solid #f0f0f0;
        color: #555;
        font-size: 12px;
        cursor: pointer;
    }

    .proof-regex-row input {
        width: 16px;
        height: 16px;
        margin: auto;
        accent-color: #0066b8;
    }

    .ai-proof-replacement-cell {
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .ai-proof-replacement-text {
        display: block;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .approval-head,
    .approval-row {
        grid-template-columns: 74px minmax(0, 1fr) 82px;
    }

    .proof-ai-actions {
        grid-template-columns: repeat(5, minmax(0, 1fr));
    }

    .proof-ai-actions button {
        width: 100%;
        min-width: 0;
    }

    .proof-ai-action-btn {
        width: 100%;
        min-width: 0 !important;
        height: 36px !important;
        padding: 0 10px !important;
        justify-content: center;
    }

    .approval-result-cell {
        display: flex;
        flex-direction: column;
        gap: 4px;
        align-items: flex-start;
        white-space: normal !important;
        overflow: visible !important;
        text-overflow: clip !important;
        line-height: 1.45;
    }

    .approval-result-link {
        cursor: pointer;
        transition: background 0.12s ease;
    }

    .approval-result-link:hover {
        background: #f7fbff;
    }

    .approval-result-link:focus {
        outline: none;
        background: #f2f9ff;
        box-shadow: inset 0 0 0 1px rgba(0, 102, 184, 0.24);
    }

    .approval-result-cell small {
        display: block;
        width: 100%;
        color: #637588;
        font-size: 12px;
        line-height: 1.45;
        white-space: normal;
        word-break: break-word;
    }

    .approval-action-cell {
        display: flex;
        align-items: center;
    }

    .approval-row .mini-action {
        width: 100%;
        height: 26px;
        font-size: 12px;
    }

    .proof-footer {
        min-height: 38px;
        padding: 9px 12px;
        border-top: 1px solid #e5e5e5;
        background: #fafafa;
        color: #0066b8;
        font-size: 12px;
        line-height: 1.5;
        box-sizing: border-box;
    }

    .sidebar-header-fixed {
        background: #eee;
        border-bottom: 1px solid #ddd;
        flex-shrink: 0;
        z-index: 20;
    }
    .sidebar-header-row {
        padding: 10px;
        display: flex;
        justify-content: space-between;
        font-size: 12px;
        font-weight: bold;
        align-items: center;
    }
    .header-btns {
        display: flex;
        gap: 5px;
    }
    .icon-btn {
        width: 26px;
        height: 26px;
        padding: 0;
        font-size: 14px;
        border: 1px solid #ccc;
        background: #fff;
        cursor: pointer;
        border-radius: 4px;
    }

    .toc-list {
        flex: 1;
        overflow-y: auto;
    }
    .toc-item {
        padding: 12px;
        font-size: 14px;
        border-bottom: 1px solid #eee;
        display: flex;
        /* justify-content: space-between; Removed to fix centering issue */
        align-items: center;
        cursor: pointer;
        cursor: pointer;
        position: relative; /* Fix z-index stacking */
        z-index: 1;
    }
    .indent {
        padding-left: 28px;
        background: #fafafa;
    }
    .toc-item.active {
        background: #d4e8fa;
        color: #0066b8;
        border-left: 4px solid #0066b8;
        font-weight: bold;
    }
    /* 卷标吸顶 */
    .vol-title {
        background: #eaeaea;
        font-weight: bold;
        position: sticky;
        top: 0;
        z-index: 10;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    }
    .text-error {
        color: #d32f2f;
        font-weight: bold;
    }
    .toc-count {
        color: #999;
        font-size: 11px;
        margin-left: auto; /* Push to right */
    }
    .arrow {
        font-size: 10px;
        margin-right: 8px;
        color: #888;
        width: 12px;
        display: inline-block;
    }
    .mini-btn {
        font-size: 11px;
        height: 26px;
        padding: 0 10px;
        border-radius: 4px;
        border: 1px solid #ccc;
        background: #fff;
    }
    .mini-btn.active {
        background: #0066b8;
        color: #fff;
    }

    .editor-wrapper {
        flex: 1;
        overflow: hidden;
        position: relative;
    }
    .loading {
        position: absolute;
        inset: 0;
        background: rgba(255, 255, 255, 0.8);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 50;
    }

    .epub-textarea {
        flex: 1;
        min-width: 0;
        width: 100%;
        box-sizing: border-box;
        padding: 8px;
        background: #fdfdfd;
        border: 1px solid #ddd;
        border-radius: 4px;
        resize: vertical;
        font-family: inherit;
        font-size: 13px;
        line-height: 1.6;
        /* text-indent: 2em; 移除强制缩进，使用原文缩进 */
    }

    .p-header {
        width: 100%;
        box-sizing: border-box;
        padding: 10px 15px;
        background: #f8fafc;
        border-bottom: 1px solid #eceff1;
        display: flex;
        justify-content: space-between;
        align-items: center;
        user-select: none;
    }
    .p-body {
        padding: 15px;
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .modal-overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 2000;
        padding: 20px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }
    .modal-content {
        background: #fff;
        width: 100%;
        max-width: 520px;
        border-radius: 20px;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }

    .epub-modal-shell {
        width: min(900px, calc(100vw - 48px));
        max-width: min(900px, calc(100vw - 48px));
        max-height: calc(100vh - 48px);
    }

    .epub-modal-shell .p-body {
        min-width: 0;
        min-height: 0;
        overflow: auto;
    }

    .history-settings-card {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .hist-item {
        width: 100%;
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        column-gap: 24px;
        text-align: left;
    }

    .hist-time {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .hist-size {
        min-width: 72px;
        justify-self: end;
        text-align: right;
        color: #64748b;
        font-family: monospace;
        font-size: 12px;
    }

    @media (max-width: 760px) {
        .proof-log-browser {
            grid-template-columns: 1fr;
        }
    }
    .rules-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }
    .rule-item {
        display: flex;
        align-items: center;
        gap: 10px;
    }
    .rule-type {
        width: 112px;
        height: 32px;
        border: 1px solid #ccc;
        border-radius: 6px;
        font-size: 13px;
    }
    .rule-input-group {
        display: flex;
        flex: 1;
        align-items: center;
        position: relative;
    }
    .rule-input {
        flex: 1;
        height: 32px;
        padding: 0 10px;
        border: 1px solid #ccc;
        border-right: none;
        border-radius: 6px 0 0 6px;
        font-size: 13px;
        background: #fff;
        min-width: 0;
        z-index: 1;
    }
    .rule-arrow-visual {
        width: 32px;
        height: 32px;
        border: 1px solid #ccc;
        border-radius: 0 6px 6px 0;
        background: #f9f9f9;
        display: flex;
        align-items: center;
        justify-content: center;
        color: #666;
        font-size: 10px;
        flex-shrink: 0;
    }
    .rule-hidden-select {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        opacity: 0;
        appearance: none;
        -webkit-appearance: none;
        cursor: pointer;
        z-index: 2;
        clip-path: inset(0 0 0 calc(100% - 32px));
    }
    .rule-btn {
        width: 32px;
        height: 32px;
        min-width: 32px;
        border: 1px solid #ccc;
        border-radius: 6px;
        background: #fff;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: 0.1s;
    }
    .rule-btn:hover {
        background: #fee;
        color: #d32f2f;
        border-color: #ffcdd2;
    }
    .rule-btn.add:hover {
        background: #f3f8ff;
        color: #0066b8;
        border-color: #b9d8ff;
    }
    .rules-header {
        font-size: 13px;
        font-weight: bold;
        color: #666;
        margin-bottom: 5px;
    }
    .rule-helper-row {
        display: flex;
        gap: 8px;
        align-items: center;
        margin-bottom: 8px;
    }
    .rule-type-helper {
        width: 112px;
        flex-shrink: 0;
    }
    .rule-prefix-input {
        flex: 1;
        min-width: 0;
        height: 32px;
        padding: 0 10px;
        border: 1px solid #ccc;
        border-radius: 6px;
        font-size: 13px;
        background: #fff;
    }

    /* EPUB 制作面板重构样式 */
    .epub-modal-body {
        width: 100%;
        max-width: none !important;
        box-sizing: border-box;
        font-size: 13px;
        color: #444;
        display: flex;
        flex-direction: column;
        min-height: 0;
        max-height: calc(100vh - 150px);
    }

    .epub-main-layout {
        display: flex;
        gap: 24px;
        margin-bottom: 12px;
        min-width: 0;
        flex: 0 0 auto;
    }

    .epub-fields-column {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .epub-cover-column {
        width: 162px;
        flex-shrink: 0;
        display: flex;
        flex-direction: column;
        min-height: 0;
    }

    .set-row.compact {
        margin-bottom: 0;
        gap: 12px;
        align-items: center;
        min-width: 0;
    }
    .set-row.align-start {
        align-items: flex-start !important;
    }
    .set-row.align-start label {
        margin-top: 10px !important;
    }

    .set-row.compact label {
        width: 50px;
        font-weight: 500;
        color: #666;
        font-size: 13px;
        margin: 0;
    }

    .epub-input-small {
        height: 32px !important;
        font-size: 13px !important;
        padding: 0 10px !important;
        border: 1px solid #ddd;
        border-radius: 4px;
        flex: 1;
        min-width: 0;
        width: auto;
    }

    .epub-cover-preview {
        width: 100%;
        aspect-ratio: 3 / 4;
        height: auto;
        border: 2px dashed #eee;
        border-radius: 8px;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        overflow: hidden;
        position: relative;
        background: #fafafa;
        transition: all 0.2s;
    }

    .epub-cover-preview:hover {
        border-color: #0088dd;
        background: #f0f8ff;
    }

    .epub-cover-preview img {
        width: 100%;
        height: 100%;
        object-fit: contain;
        object-position: center;
    }

    .epub-cover-preview .no-cover {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        color: #aaa;
    }

    .epub-cover-preview .no-cover span:first-child {
        font-size: 24px;
    }

    .cover-hint {
        position: absolute;
        left: 8px;
        right: 8px;
        bottom: 8px;
        width: auto;
        border-radius: 999px;
        background: rgba(0,0,0,0.58);
        color: white;
        font-size: 11px;
        padding: 4px 8px;
        text-align: center;
        opacity: 0;
        transition: opacity 0.2s;
    }

    .epub-cover-preview:hover .cover-hint {
        opacity: 1;
    }

    .cover-source-actions {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 8px;
        margin-top: 10px;
    }

    .cover-source-btn {
        height: 32px;
        border: 1px solid var(--color-border);
        border-radius: 8px;
        background: #fff;
        color: var(--color-text-soft);
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        box-shadow: var(--shadow-xs);
        transition: all 0.18s ease;
    }

    .cover-source-btn.primary {
        border-color: transparent;
        background: linear-gradient(135deg, #2098d1, #14a89d);
        color: #fff;
    }

    .cover-source-btn:hover:not(:disabled) {
        transform: translateY(-1px);
        box-shadow: 0 8px 18px rgba(31, 142, 186, 0.18);
    }

    .cover-source-btn:disabled {
        cursor: not-allowed;
        opacity: 0.6;
    }

    .cover-search-status {
        margin-top: 8px;
        color: var(--color-muted);
        font-size: 12px;
        line-height: 1.4;
        max-height: 34px;
        overflow: hidden;
        word-break: break-word;
    }

    .cover-results-panel {
        margin: 0 0 12px;
        padding: 12px;
        border: 1px solid var(--color-border);
        border-radius: 10px;
        background: rgba(255, 255, 255, 0.76);
        box-shadow: var(--shadow-xs);
        display: flex;
        flex: 0 1 auto;
        flex-direction: column;
        min-height: 0;
        max-height: clamp(300px, 40vh, 420px);
        overflow: hidden;
    }

    .cover-results-head {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 12px;
        margin-bottom: 10px;
    }

    .cover-results-head span {
        color: var(--color-text);
        font-size: 14px;
        font-weight: 700;
    }

    .cover-results-head small {
        min-width: 0;
        color: var(--color-muted);
        font-size: 12px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .cover-result-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(96px, 116px));
        justify-content: start;
        gap: 12px;
        align-content: start;
        grid-auto-rows: max-content;
        max-height: none;
        padding: 2px 2px 4px;
        flex: 1 1 auto;
        min-height: 0;
        overflow-y: auto;
    }

    .cover-result-card {
        position: relative;
        display: flex;
        flex-direction: column;
        align-items: stretch;
        justify-content: flex-start;
        min-width: 0;
        width: 100%;
        height: auto;
        min-height: 0 !important;
        padding: 0;
        border: 1px solid var(--color-border);
        border-radius: 10px;
        background: #fff;
        overflow: hidden;
        cursor: pointer;
        box-shadow: var(--shadow-xs);
        text-align: left;
        transition: all 0.18s ease;
    }

    .cover-result-card:hover:not(:disabled) {
        border-color: var(--color-accent);
        transform: translateY(-1px);
        box-shadow: 0 10px 24px rgba(31, 142, 186, 0.16);
    }

    .cover-result-image-wrap {
        position: relative;
        display: block;
        width: 100%;
        aspect-ratio: 3 / 4;
        height: auto;
        min-height: 0;
        flex: 0 0 auto;
        background: #f3f7fa;
    }

    .cover-result-card img {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    .cover-result-info {
        display: grid;
        gap: 2px;
        padding: 7px 8px 8px;
        min-width: 0;
        width: 100%;
        box-sizing: border-box;
    }

    .cover-result-title {
        display: block;
        min-width: 0;
        color: var(--color-text-soft);
        font-size: 12px;
        font-weight: 650;
        line-height: 1.25;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .cover-result-source {
        min-width: 0;
        color: var(--color-muted);
        font-size: 11px;
        line-height: 1.2;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .epub-status-line {
        margin: -2px 0 12px;
        padding: 10px 12px;
        border: 1px solid rgba(31, 142, 186, 0.24);
        border-radius: 8px;
        background: rgba(31, 142, 186, 0.08);
        color: var(--color-accent-deep);
        font-size: 13px;
        font-weight: 700;
    }

    .epub-status-line.success {
        border-color: rgba(20, 168, 157, 0.3);
        background: rgba(20, 168, 157, 0.1);
        color: #0b776f;
    }

    .epub-modal-footer {
        display: flex;
        gap: 12px;
        margin-top: auto;
        padding-top: 10px;
        min-width: 0;
        flex: 0 0 auto;
        background: transparent;
    }

    .epub-modal-footer button {
        flex: 1;
        height: 40px;
        font-size: 14px;
    }

    /* 检查面板样式 */
    .check-sec {
        padding: 8px 12px;
        border-bottom: 1px solid #eee;
    }
    .check-sec:last-child {
        border-bottom: none;
    }
    .sec-title {
        font-size: 13px;
        font-weight: bold;
        color: #444;
        cursor: pointer;
        padding: 4px 0;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .tag-list {
        display: flex;
        flex-direction: column;
        gap: 6px;
        padding: 8px 0;
    }
    .err-tag {
        width: 100%;
        min-width: 0;
        box-sizing: border-box;
        background: #fff5f5;
        border: 1px solid #ffcdd2;
        border-radius: 6px;
        padding: 6px 10px;
        font-size: 12px;
        color: #d32f2f;
        cursor: pointer;
        text-align: left;
        line-height: 1.4;
        transition: 0.2s;
        display: flex;
        flex-direction: row;
        align-items: center;
        gap: 8px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        justify-content: flex-start;
    }
    .err-tag:hover {
        background: #ffebee;
        border-color: #ef5350;
    }
    .err-tag-title {
        font-weight: bold;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .err-tag-msg {
        flex-shrink: 0;
        font-size: 11px;
        color: #f44336;
        font-weight: bold;
        font-family: monospace;
    }

    .err-tag-count {
        flex-shrink: 0;
        margin-left: auto;
        color: #6b7280;
        font-size: 11px;
        font-weight: 700;
        font-family: monospace;
    }
    
    .find-header {
        padding: 10px 14px;
        background: #f8fafc;
        border-bottom: 1px solid #eee;
        display: flex;
        justify-content: space-between;
        align-items: center;
        cursor: move;
    }
    .find-title {
        font-size: 14px;
        font-weight: bold;
        color: #333;
    }
    .icon-close {
        background: none;
        border: none;
        font-size: 16px;
        color: #999;
        cursor: pointer;
        padding: 4px;
        min-width: 0;
    }
    .icon-close:hover {
        color: #f44336;
    }
    .scroll-p::-webkit-scrollbar {
        width: 6px;
    }
    .scroll-p::-webkit-scrollbar-thumb {
        background: #ddd;
        border-radius: 3px;
    }

    /* Modern UI overrides */
    :global(body) {
        background: var(--gradient-app);
        color: var(--color-text);
        font-family: var(--font-ui);
    }

    .app-container {
        background: rgba(246, 250, 253, 0.68);
    }

    .toolbar {
        height: 52px;
        padding: env(safe-area-inset-top) 14px 0;
        background: rgba(255, 255, 255, 0.78);
        border-bottom: 1px solid var(--color-border);
        box-shadow: var(--shadow-xs);
        backdrop-filter: blur(18px) saturate(1.15);
    }

    .btn-group {
        gap: 8px;
    }

    button {
        height: var(--control-height);
        min-width: 38px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: linear-gradient(180deg, #ffffff, var(--color-surface-soft));
        color: var(--color-text-soft);
        box-shadow: var(--shadow-xs);
        cursor: pointer;
        transition:
            transform var(--transition-fast),
            border-color var(--transition-fast),
            background var(--transition-fast),
            box-shadow var(--transition-fast),
            color var(--transition-fast);
    }

    button:hover:not(:disabled) {
        border-color: var(--color-border-strong);
        background: var(--color-hover);
        color: var(--color-text);
        box-shadow: var(--shadow-sm);
    }

    button:active:not(:disabled) {
        background: var(--color-active);
        transform: translateY(1px) scale(0.98);
        box-shadow: var(--shadow-xs);
    }

    button:disabled {
        opacity: 0.52;
        box-shadow: none;
    }

    .btn-primary,
    .btn.primary,
    .epub-confirm {
        background: var(--gradient-accent);
        border-color: transparent;
        color: #fff;
        box-shadow: 0 10px 22px rgba(22, 119, 184, 0.2);
    }

    .btn-secondary,
    .btn-save-default,
    .btn.secondary,
    .epub-cancel,
    .btn-small {
        background: rgba(255, 255, 255, 0.84);
        color: var(--color-text-soft);
    }

    .btn-save-modified,
    .btn.danger {
        background: var(--gradient-danger);
        border-color: transparent;
        color: #fff;
        animation: pulse 2s infinite;
    }

    .main-body {
        background: rgba(239, 245, 250, 0.78);
    }

    .sidebar {
        width: 292px;
        background: rgba(255, 255, 255, 0.82);
        border-right: 1px solid var(--color-border);
        box-shadow: 10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .proof-panel {
        background: rgba(255, 255, 255, 0.86);
        border-right: 1px solid var(--color-border);
        box-shadow: 10px 0 30px rgba(23, 36, 52, 0.05);
        backdrop-filter: blur(16px);
    }

    .proof-header,
    .proof-tabs,
    .proof-footer {
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(246, 249, 252, 0.9));
        border-color: var(--color-border);
    }

    .proof-title {
        color: var(--color-text);
    }

    .proof-subtitle,
    .proof-row label,
    .proof-check,
    .proof-rule-note {
        color: var(--color-text-soft);
    }

    .proof-body {
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.44), rgba(246, 249, 252, 0.64));
    }

    .proof-tabs button {
        border-radius: var(--radius-xs);
    }

    .proof-tabs button.active,
    .proof-primary {
        background: var(--gradient-accent);
        border-color: transparent;
        color: #fff;
        box-shadow: 0 10px 22px rgba(22, 119, 184, 0.18);
    }

    .proof-row input,
    .proof-row select,
    .proof-preview,
    .proof-regex-preview,
    .proof-rule-note {
        border-color: var(--color-border);
        background: rgba(255, 255, 255, 0.9);
        color: var(--color-text);
        border-radius: var(--radius-sm);
    }

    .proof-row input:focus,
    .proof-row select:focus {
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
    }

    .proof-preview-head,
    .proof-regex-head {
        background: var(--color-surface-soft);
        color: var(--color-text-soft);
    }

    .proof-preview-head span,
    .proof-preview-row div,
    .proof-preview-row,
    .proof-regex-head span,
    .proof-regex-row > span,
    .proof-regex-row {
        border-color: var(--color-border);
    }

    .proof-preview-row,
    .proof-regex-row {
        color: var(--color-text-soft);
    }

    .proof-preview-row.volume {
        background: var(--color-surface-soft);
        color: var(--color-text);
        box-shadow: 0 1px 0 var(--color-border);
    }

    .proof-preview-row.volume .cell-changed {
        background: var(--color-accent-quiet);
    }

    .proof-preview-row .sequence-broken {
        background: var(--color-danger-soft);
        color: var(--color-danger);
    }

    .proof-preview-row .cell-changed {
        background: var(--color-accent-quiet);
        color: var(--color-accent-deep);
    }

    .proof-check-list {
        border-color: var(--color-border);
        background: rgba(255, 255, 255, 0.9);
        border-radius: var(--radius-sm);
    }

    .proof-footer {
        color: var(--color-accent-deep);
    }

    .sidebar-mask {
        background: rgba(23, 36, 52, 0.28);
        backdrop-filter: blur(4px);
    }

    .sidebar-header-fixed {
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(246, 249, 252, 0.92));
        border-bottom: 1px solid var(--color-border);
    }

    .sidebar-header-row {
        padding: 12px;
        color: var(--color-text-soft);
        letter-spacing: 0.02em;
    }

    .icon-btn,
    .mini-btn,
    .rule-btn {
        border: 1px solid var(--color-border);
        background: rgba(255, 255, 255, 0.86);
        color: var(--color-text-soft);
        border-radius: var(--radius-xs);
        box-shadow: none;
    }

    .icon-btn {
        width: 30px;
        height: 30px;
        min-width: 30px;
    }

    .mini-btn {
        height: 28px;
        border-radius: 999px;
        padding: 0 12px;
    }

    .mini-btn.active {
        background: var(--gradient-accent);
        border-color: transparent;
        color: #fff;
    }

    .toc-list {
        padding: 8px;
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.42), rgba(246, 249, 252, 0.64));
    }

    .toc-item {
        margin: 2px 0;
        padding: 10px 12px;
        border-bottom: 0;
        border-left: 3px solid transparent;
        border-radius: var(--radius-sm);
        color: var(--color-text-soft);
        transition:
            background var(--transition-fast),
            color var(--transition-fast),
            border-color var(--transition-fast),
            transform var(--transition-fast);
    }

    .toc-item:hover {
        background: var(--color-hover);
        color: var(--color-text);
    }

    .indent {
        background: transparent;
    }

    .toc-item.active {
        background: var(--color-accent-soft);
        color: var(--color-accent-deep);
        border-left-color: var(--color-accent);
        box-shadow: inset 0 0 0 1px rgba(22, 119, 184, 0.12);
        font-weight: 700;
    }

    .vol-title {
        background: rgba(255, 255, 255, 0.88);
        color: var(--color-text);
        box-shadow: 0 8px 18px rgba(23, 36, 52, 0.06);
        backdrop-filter: blur(14px);
    }

    .toc-count,
    .arrow {
        color: var(--color-muted);
    }

    .editor-wrapper {
        background: var(--color-surface);
        box-shadow: inset 1px 0 0 rgba(255, 255, 255, 0.7);
    }

    .loading {
        background: rgba(255, 255, 255, 0.72);
        color: var(--color-muted);
        backdrop-filter: blur(8px);
    }

    .dialog-overlay,
    .modal-overlay {
        background: rgba(14, 24, 36, 0.36);
        backdrop-filter: blur(10px);
    }

    .dialog,
    .modal-content {
        background: var(--color-surface-raised);
        border: 1px solid rgba(255, 255, 255, 0.78);
        border-radius: var(--radius-lg);
        box-shadow: var(--shadow-pop);
        color: var(--color-text);
        backdrop-filter: blur(18px) saturate(1.08);
    }

    .dialog {
        min-width: 340px;
        padding: 24px;
    }

    .dialog-header,
    .find-title {
        color: var(--color-text);
        letter-spacing: 0.01em;
    }

    .dialog-content,
    .set-row label,
    .rules-header,
    .sec-title {
        color: var(--color-text-soft);
    }

    .p-header,
    .find-header {
        background: linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(246, 249, 252, 0.9));
        border-bottom: 1px solid var(--color-border);
    }

    .set-row input,
    .set-row select,
    .set-row textarea,
    .epub-input-small,
    .epub-textarea,
    .rule-type,
    .rule-input {
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: rgba(255, 255, 255, 0.9);
        color: var(--color-text);
        transition:
            border-color var(--transition-fast),
            box-shadow var(--transition-fast),
            background var(--transition-fast);
    }

    .set-row input:focus,
    .set-row select:focus,
    .set-row textarea:focus,
    .epub-input-small:focus,
    .epub-textarea:focus,
    .rule-type:focus,
    .rule-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: #fff;
    }
    .rule-prefix-input:focus {
        outline: none;
        border-color: var(--color-accent);
        box-shadow: var(--focus-ring);
        background: #fff;
    }

    .rule-arrow-visual {
        border-color: var(--color-border);
        background: var(--color-surface-soft);
        color: var(--color-muted);
    }

    .rule-btn:hover {
        background: var(--color-danger-soft);
        border-color: rgba(215, 68, 82, 0.3);
        color: var(--color-danger);
    }

    .epub-cover-preview {
        border-color: var(--color-border);
        border-radius: var(--radius-md);
        background: linear-gradient(135deg, rgba(255, 255, 255, 0.78), rgba(226, 243, 255, 0.54));
    }

    .epub-cover-preview:hover {
        border-color: var(--color-accent);
        background: var(--color-accent-quiet);
        box-shadow: var(--shadow-sm);
    }

    .check-sec {
        border-bottom: 1px solid var(--color-border);
    }

    .err-tag {
        background: var(--color-danger-soft);
        border-color: rgba(215, 68, 82, 0.22);
        border-radius: var(--radius-sm);
        color: var(--color-danger);
    }

    .err-tag:hover {
        background: #ffe7eb;
        border-color: rgba(215, 68, 82, 0.38);
    }

    .err-tag-msg {
        color: var(--color-danger);
        font-family: var(--font-code);
    }

    .icon-close {
        width: 30px;
        height: 30px;
        min-width: 30px;
        border-radius: 999px;
        color: var(--color-muted);
        box-shadow: none;
    }

    .icon-close:hover {
        background: var(--color-danger-soft);
        color: var(--color-danger);
    }

    .epub-modal-footer button,
    .dialog-actions .btn,
    .btn-small {
        height: 38px;
        border-radius: var(--radius-sm);
    }

    .scroll-p::-webkit-scrollbar-thumb {
        background: linear-gradient(180deg, #bacbda, #93a8bb);
        border-radius: 999px;
    }

    /* Keep TXT volume folding and sticky headers precise. */
    .toc-list {
        position: relative;
        padding: 0;
        scroll-padding-top: 42px;
    }

    .toc-item {
        margin: 2px 8px;
    }

    .vol-title {
        position: sticky;
        top: 0;
        z-index: 30;
        margin: 0;
        border-radius: 0;
        border-bottom: 1px solid var(--color-border);
        background: rgba(255, 255, 255, 0.96);
        box-shadow: 0 6px 14px rgba(23, 36, 52, 0.08);
    }

    .vol-title:hover {
        background: rgba(246, 249, 252, 0.98);
    }

    .vol-title.active {
        border-left-color: var(--color-accent);
    }

    .indent {
        margin-left: 14px;
        padding-left: 24px;
    }

    @media (max-width: 768px) {
        .proof-panel {
            position: absolute;
            inset: 0 0 0 auto;
            width: min(92vw, 400px);
            max-width: 92vw;
            z-index: 80;
        }
    }
</style>
