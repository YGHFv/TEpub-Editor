/**
 * localStorage / sessionStorage key 集中定义（重构第 2 期第 3 条）。
 *
 * 目的：让"谁在用哪个 key、哪些 key 是跨页面共享的"一目了然，
 * 消除桌面/Web 页面共用 key 的隐性耦合。
 *
 * 硬性约束：这里的字符串值一个字符都不能改——存量用户数据保存在这些 key 下。
 * 动态 key（带文件路径 / 任务 id 后缀）导出前缀常量 + 生成函数，两者必须保持一致。
 */

// ===================== 全局设置 =====================

/**
 * 全局应用设置（JSON 对象）。**跨模块共享**：
 * - 桌面 editor 页（src/routes/editor/+page.svelte）直接 get/setItem；
 * - src/lib/webAccount.ts 在 Tauri 运行时作为 LEGACY_SETTINGS_KEY 读写同一份数据
 *   （Web 运行时则改用账号/游客作用域 key，见下方 "Web 账号" 一节）；
 * - src/lib/appSettings.ts 经 webAccount 的 read/writeScopedSettingsRaw 间接读写。
 */
export const APP_SETTINGS_KEY = "app-settings";

// ===================== editor（桌面 TXT 编辑器页） =====================

/** 崩溃恢复状态（文件路径 + 未保存内容），仅 editor 页使用。 */
export const CRASH_RECOVERY_KEY = "app-crash-recovery";

/** 窗口位置记忆前缀，后接 Tauri 窗口 label。仅 editor 页使用。 */
export const WINDOW_POS_PREFIX = "window_pos_";
export function windowPosKey(label: string) {
  return `${WINDOW_POS_PREFIX}${label}`;
}

/** AI 校对结果缓存前缀，后接打开的 TXT 文件绝对路径。仅 editor 页使用。 */
export const AI_PROOFING_CACHE_PREFIX = "ai-proofing-cache:";
export function aiProofingCacheKey(filePath: string) {
  return `${AI_PROOFING_CACHE_PREFIX}${filePath}`;
}

/** 手动标题覆盖前缀，后接打开的 TXT 文件绝对路径。仅 editor 页使用。 */
export const MANUAL_TITLE_OVERRIDES_PREFIX = "manual-title-overrides:";
export function manualTitleOverridesKey(filePath: string) {
  return `${MANUAL_TITLE_OVERRIDES_PREFIX}${filePath}`;
}

/**
 * TXT 编辑器字体大小。目前只有 src/lib/Editor.svelte 读取，
 * 代码库内没有写入方（历史遗留 key，用户机器上可能仍存有旧值）。
 */
export const EDITOR_FONT_SIZE_KEY = "editor-font-size";

// ===================== epub-editor（桌面 + Web 共用同组 key） =====================
// 注意：以下 key 被桌面版 src/routes/epub-editor/+page.svelte 与
// Web 版 src/routes/toolbox/epub-editor/+page.svelte **共同读写**，
// 两个页面的查找历史/选项互通。改动语义时两边都要考虑。

/** 查找选项（正则/范围/方向等）。目前仅桌面 epub-editor 读写，但与查找历史同族。 */
export const EPUB_SEARCH_OPTIONS_KEY = "epub-search-options";

/** 查找历史。**共享**：桌面 epub-editor + Web toolbox/epub-editor。 */
export const EPUB_FIND_HISTORY_KEY = "epub-find-history";

/** 替换历史。**共享**：桌面 epub-editor + Web toolbox/epub-editor。 */
export const EPUB_REPLACE_HISTORY_KEY = "epub-replace-history";

/**
 * EPUB 代码编辑器字体大小。定义于共享组件 src/lib/EpubCodeEditor.svelte，
 * 桌面与 Web 的 epub-editor 页面都通过该组件读写同一 key。
 */
export const EPUB_EDITOR_FONT_SIZE_KEY = "epub-editor-font-size";

// ===================== reader（阅读器） =====================

/** 阅读器外观/翻页设置（全局，不分书籍）。仅 reader 页使用。 */
export const READER_SETTINGS_KEY = "reader:settings";

/** 阅读进度前缀，后接 EPUB 文件路径。仅 reader 页使用。 */
export const READER_PROGRESS_PREFIX = "reader:progress:";
export function readerProgressKey(epubPath: string) {
  return `${READER_PROGRESS_PREFIX}${epubPath}`;
}

/** 书签前缀，后接 EPUB 文件路径（与 settings 独立）。仅 reader 页使用。 */
export const READER_BOOKMARKS_PREFIX = "reader:bookmarks:";
export function readerBookmarksKey(epubPath: string) {
  return `${READER_BOOKMARKS_PREFIX}${epubPath}`;
}

// ===================== library（书库） =====================

/** 书架显示设置（视图模式/预览模式等）。仅 library 页使用。 */
export const SHELF_SETTINGS_KEY = "shelf-settings";

// ===================== 启动参数 / 批处理（跨窗口共享） =====================

/**
 * 批处理任务配置前缀，后接任务 id。
 * **共享**：toolbox 页写入，batch-progress 窗口读取并回写。
 */
export const BATCH_TASK_PREFIX = "tepub-editor-batch-task:";
export function batchTaskKey(taskId: string) {
  return `${BATCH_TASK_PREFIX}${taskId}`;
}

// ===================== toolbox 工具页 =====================

/** 图片工具的 AI 生图配置（书名/作者等）。仅 toolbox/image-tools 页使用。 */
export const IMAGE_TOOLS_AI_CONFIG_KEY = "tepub-image-tools-ai-config";

/**
 * EPUB 样式库（{ version, styles } JSON）。**三处共享**：
 * - toolbox/epub-style-library 页（管理界面，读写）；
 * - toolbox/online-image-download 页（读取 illustration 样式）；
 * - mobile/make 页（读取已保存样式，含 toolbox/make-epub 复用同组件）。
 * 数据结构见 src/lib/epubStyleLibrary.ts 的 EpubStyleModule。
 */
export const EPUB_STYLE_LIBRARY_KEY = "tepub-epub-style-library-v1";

/** Web 文本编辑器本地草稿。仅 toolbox/text-editor 页使用。 */
export const TEXT_EDITOR_DRAFT_KEY = "tepub-web-text-editor-draft";

/**
 * 文本编辑器 → EPUB 制作的交接数据（sessionStorage）。
 * toolbox/text-editor 页写入后跳转 /toolbox/make-epub；
 * 注意：目前代码库内没有读取方（make-epub 复用的 mobile/make 组件尚未消费）。
 */
export const TEXT_EDITOR_EPUB_HANDOFF_KEY =
  "tepub-web-text-editor-epub-handoff";

// ===================== Web 客户端档案 =====================

/**
 * Web 端桌面/移动视图覆盖（"desktop" | "mobile"）。
 * **共享**：src/lib/clientProfile.ts 读写；src/app.html 的内联脚本在
 * SvelteKit 启动前读取同一字符串（内联脚本无法 import，那里是硬编码副本，
 * 改这里必须同步改 app.html）。
 */
export const WEB_VIEW_OVERRIDE_KEY = "tepub-web-view-mode";

// ===================== Web 账号（定义在 src/lib/webAccount.ts） =====================
// 以下 key 为 webAccount.ts 模块私有、未与任何页面直接共享，遵循"单一事实来源"
// 原则原地保留，此处仅登记以便全局检索：
// - "tepub-web-auth"                       登录会话（localStorage）
// - "tepub-web-local-users"                本地离线账号表（localStorage）
// - "tepub-web-guest-settings"             游客设置（sessionStorage）
// - "tepub-web-account-settings:<username>" 账号级设置（localStorage，动态后缀）
// 另：Tauri 运行时的 LEGACY_SETTINGS_KEY 即上方 APP_SETTINGS_KEY，webAccount.ts 从本模块导入。
