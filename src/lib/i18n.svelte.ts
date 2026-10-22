import OpenCC from "opencc-js/cn2t";
import { STATIC_EN_MESSAGES } from "./i18nStaticEn";
import { DYNAMIC_EN_MESSAGES } from "./i18nDynamicEn";
import { RUNTIME_EN_MESSAGES } from "./i18nRuntimeEn";

export type AppLanguage = "zh-CN" | "zh-TW" | "en";
export type UiLanguagePreference = "auto" | AppLanguage;

export const UI_LANGUAGE_OPTIONS: Array<{
  value: UiLanguagePreference;
  label: string;
}> = [
  { value: "auto", label: "跟随系统" },
  { value: "zh-CN", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "en", label: "English" },
];

const EN_MESSAGES: Record<string, string> = {
  // Shared shell
  跟随系统: "Follow system",
  简体中文: "Simplified Chinese",
  繁體中文: "Traditional Chinese",
  工具箱: "Toolbox",
  书库: "Library",
  更多: "More",
  电脑版: "Desktop view",
  移动端工具: "Mobile tools",
  关于: "About",
  主导航: "Main navigation",
  页面导航: "Page navigation",
  返回: "Back",
  关闭: "Close",
  关闭提示: "Close notification",
  已打开的页面: "Open pages",
  关闭标签: "Close tab",
  "关闭 {title}": "Close {title}",
  最小化: "Minimize",
  还原: "Restore",
  最大化: "Maximize",
  "正在打开页面…": "Opening page…",
  上下文菜单: "Context menu",
  折叠: "Collapse",
  展开: "Expand",
  删除: "Delete",
  取消: "Cancel",
  保存: "Save",
  "保存中...": "Saving...",
  编辑: "Edit",
  新增: "Add",
  重置: "Reset",
  刷新: "Refresh",
  完成: "Done",
  处理中: "Processing",
  "处理中...": "Processing...",
  "加载中...": "Loading...",
  错误: "Error",
  警告: "Warning",
  提示: "Notice",
  全部: "All",
  无: "None",
  可选项: "Optional",
  可选: "Optional",
  请选择: "Select",
  阅读: "Read",
  当前文件: "Current file",
  所有文本: "All text",
  HTML章节: "HTML chapters",

  // Route titles and common pages
  "TXT 编辑器": "TXT Editor",
  "EPUB 编辑器": "EPUB Editor",
  "EPUB 阅读器": "EPUB Reader",
  阅读器: "Reader",
  "EPUB 元数据": "EPUB Metadata",
  查找替换: "Find & Replace",
  批量处理: "Batch processing",
  "TXT 转 EPUB": "TXT to EPUB",
  "EPUB 样式库": "EPUB Style Library",
  "EPUB 在线图片下载": "EPUB Online Images",
  "EPUB 进阶工具": "Advanced EPUB Tools",
  "EPUB 处理": "EPUB Processing",
  "EPUB 结构诊断": "EPUB Structure Check",
  "EPUB 字体处理": "EPUB Font Tools",
  "EPUB 文件编辑": "EPUB File Editor",
  "EPUB 解密": "EPUB Decrypt",

  // Toolbox
  工具列表: "Tool list",
  工具分类: "Tool categories",
  搜索工具: "Search tools",
  恢复自动页面: "Restore automatic view",
  切换电脑页面: "Switch to desktop view",
  设置: "Settings",
  没有匹配的工具: "No matching tools",
  版权信息: "Copyright",
  工具箱设置: "Toolbox settings",
  账号: "Account",
  通用: "General",
  文件关联: "File associations",
  "API 配置": "API configuration",
  界面主题: "Interface theme",
  界面语言: "Interface language",
  跟随系统主题: "Follow system theme",
  浅色: "Light",
  深色: "Dark",
  使用多窗口打开二级页面: "Open secondary pages in new windows",
  "关闭时在当前窗口内切换，顶部用标签栏在页面间跳转":
    "When off, pages open in this window and the tab bar switches between them",
  打开工具时隐藏工具箱主页: "Hide the toolbox while a tool is open",
  "显示 EPUB 预览滚动条": "Show EPUB preview scrollbar",
  "关闭时预览内容水平居中，不会有右侧留白":
    "When off, preview content is centered without extra space on the right",
  "注册到系统右键菜单和默认打开方式，修改时会写入注册表。":
    "Register in the system context menu and default-app list. Changes update the registry.",
  "EPUB 阅读": "Open EPUB for reading",
  "EPUB 编辑": "Open EPUB for editing",
  "制作 EPUB": "Create EPUB",
  "保存后的文字模型用于校对和书库智能匹配，生图模型用于图片处理。":
    "Text models are used for proofing and library matching; image models are used for image tools.",
  "暂无 API 配置，点击“新增”添加文字模型或生图模型。":
    "No APIs configured yet. Select Add to create a text or image model.",
  未填写模型: "Model not set",
  "未填写 API 地址": "API URL not set",
  "已保存 Key": "Key saved",
  "未填写 Key": "Key not set",
  "新增 API": "Add API",
  "编辑 API": "Edit API",
  模型类型: "Model type",
  文字模型: "Text model",
  生图模型: "Image model",
  名称: "Name",
  "API 地址": "API URL",
  模型: "Model",
  温度: "Temperature",
  "例如 DeepSeek / OpenAI 生图": "e.g. DeepSeek / OpenAI image",
  "请填写名称、API 地址和模型。": "Enter a name, API URL, and model.",
  "API 配置已保存。": "API configuration saved.",
  "API 配置已删除。": "API configuration deleted.",
  "当前账号：": "Current account: ",
  "（本机开发模式）": " (local development mode)",
  "设置会按账号独立保存。": "Settings are saved separately for each account.",
  退出登录: "Sign out",
  "未登录时，Web 端设置只保存在本次浏览器会话；关闭后重新进入会恢复默认。":
    "When signed out, web settings only last for this browser session. They reset when you return.",
  登录: "Sign in",
  注册: "Register",
  密码: "Password",
  "3-32 位字母、数字或下划线": "3-32 letters, numbers, or underscores",
  "至少 6 位": "At least 6 characters",
  注册并登录: "Register and sign in",
  "请填写账号和密码。": "Enter an account name and password.",
  "注册成功，已登录。": "Registration successful. You are signed in.",
  "登录成功。": "Signed in.",
  "已退出登录，当前修改只保存在本次浏览器会话。":
    "Signed out. Changes now only last for this browser session.",

  // Tool names and descriptions
  管理图书与元数据: "Manage books and metadata",
  "将 TXT 制作为标准 EPUB": "Create a standards-compliant EPUB from TXT",
  "浏览头图、标题、插图与转场样式":
    "Browse header, title, illustration, and transition styles",
  "下载远程插图并写入 EPUB": "Download remote images into an EPUB",
  "导入、校对、查找替换": "Import, proofread, find, and replace",
  "编辑结构、资源与元数据": "Edit structure, assets, and metadata",
  "打开 EPUB 进行阅读预览": "Open an EPUB for reading",
  "加密 EPUB 内嵌字体": "Encrypt fonts embedded in an EPUB",
  "使用对应 TXT 还原字体": "Restore fonts using the matching TXT",
  "加密 EPUB 正文文件": "Encrypt EPUB content files",
  "还原 EPUB 正文文件": "Restore EPUB content files",
  整理目录与引用: "Clean up navigation and references",
  "WebP 与 PNG/JPEG 互转": "Convert between WebP and PNG/JPEG",
  制作全屏封面与阅微横幅: "Create full-screen covers and Yuewei banners",
  "检查 OPF、manifest 和内部引用":
    "Check OPF, manifest, and internal references",
  按阅读顺序提取正文: "Extract content in reading order",
  "EPUB 2.0 与 3.0 互转": "Convert between EPUB 2.0 and 3.0",
  "转换正文、目录与元数据": "Convert content, navigation, and metadata",
  按规则清理广告段落: "Remove ad paragraphs by rule",
  "生成标准 ruby 注音": "Generate standard ruby annotations",
  "批注与 EPUB3 脚注互转": "Convert between notes and EPUB3 footnotes",
  "压缩 EPUB 内嵌图片": "Compress images embedded in an EPUB",
  写入或读取隐形文本水印: "Write or read invisible text watermarks",
  保留资源与阅读顺序: "Preserve assets and reading order",
  "按章节数拆分为多本 EPUB": "Split into multiple EPUBs by chapter count",
  移除正文未使用字形: "Remove unused glyphs from the content",
  "打开 Amazon 官方传书页面": "Open Amazon Send to Kindle",
  编辑阅读: "Editing & reading",
  格式转换: "Format conversion",
  加密解密: "Encryption & decryption",
  整理优化: "Optimization",

  // Library
  按模板重命名: "Rename from template",
  使用源文件名: "Use source filename",
  始终显示: "Always show",
  点击显示: "Show on click",
  始终隐藏: "Always hide",
  编辑元数据: "Edit metadata",
  新加入的在前: "Newest first",
  新加入的在后: "Newest last",
  "复制到 books/ 文件夹": "Copy to books/ folder",
  "仅索引文件位置（不复制）": "Index file location only (no copy)",
  复制到指定工作目录: "Copy to a custom work folder",
  未配置文字模型: "No text model configured",
  文件存储: "File storage",
  智能匹配: "Smart matching",
  书架显示: "Shelf display",
  "作者：": "Author: ",
  文件名: "Filename",
  格式: "Format",
  大小: "Size",
  添加日期: "Date added",
  创建日期: "Created",
  修改日期: "Modified",
  移除: "Remove",
  选择一本书查看详情: "Select a book to see details",
  图书列表列标题: "Book list columns",
  "按{column}排序": "Sort by {column}",
  "调整{column}列宽": "Resize {column} column",
  图书标签: "Book tags",
  选择显示列: "Choose columns",
  选择列: "Choose columns",
  "点击移除筛选: {tag}": "Remove filter: {tag}",
  "点击按“{tag}”筛选": "Filter by “{tag}”",
  简介: "Description",
  副标题: "Subtitle",
  出版社: "Publisher",
  制作信息: "Production details",
  制作系列: "Production series",
  制作时间: "Created",
  制作者: "Producer",
  系列: "Series",
  标签: "Tags",
  更换封面: "Change cover",
  保存并导出: "Save and export",
  "正在写入 EPUB，请稍候…": "Writing EPUB, please wait…",
  "匹配中...": "Matching...",
  "AI 自动匹配标签/简介": "AI-match tags/description",
  "选完前三级后,可在此添加自定义标签":
    "After selecting the first three levels, add custom tags here",
  建议: "Suggestions",
  类型: "Type",
  分类: "Category",
  主角: "Lead",
  没有匹配: "No matches",
  回车将其作为新标签添加: "Press Enter to add it as a new tag",
  "输入文字搜索已用过的标签,或按回车直接添加新标签":
    "Search used tags or press Enter to add a new tag",
  "回车 / 逗号 添加;退格 删除最后一个;Esc 关闭":
    "Enter/comma adds; Backspace removes the last; Esc closes",
  // Editor and reader controls
  打开文件: "Open file",
  "保存 (Ctrl+S)": "Save (Ctrl+S)",
  "撤销 (Ctrl+Z)": "Undo (Ctrl+Z)",
  "重做 (Ctrl+Y)": "Redo (Ctrl+Y)",
  撤销: "Undo",
  重做: "Redo",
  章节侧栏: "Chapter sidebar",
  查找与替换: "Find & Replace",
  "查找与替换 (Ctrl+F)": "Find & Replace (Ctrl+F)",
  校对: "Proofread",
  偏好设置: "Preferences",
  检查: "Check",
  "全部展开/折叠": "Expand/collapse all",
  卷: "Volume",
  章: "Chapter",
  "正在解压 EPUB…": "Extracting EPUB…",
  加载失败: "Failed to load",
  "本章剩余 {count} 页": "{count} pages left in this chapter",
  "约 {percent}%": "About {percent}%",
  目录: "Contents",
  书签: "Bookmarks",
  主题: "Theme",
  排版: "Typography",
  '暂无书签。点击上方"+ 添加当前位置"。':
    "No bookmarks yet. Select “+ Add current position” above.",
  "+ 添加当前位置": "+ Add current position",
  已添加: "Added",
  纸张: "Paper",
  护眼: "Eye care",
  羊皮: "Sepia",
  雪白: "Snow",
  暗夜: "Night",
  "更换背景图…": "Change background…",
  恢复默认: "Restore default",
  已应用自定义背景图: "Custom background applied",
  字号: "Font size",
  行距: "Line spacing",
  段距: "Paragraph spacing",
  字色: "Text color",
  加深: "Darker",
  黑字: "Black",
  字重: "Font weight",
  常规: "Regular",
  适中: "Medium",
  加粗: "Bold",
  左右: "Horizontal margins",
  上下: "Vertical margins",
  字体: "Font",
  "默认（衬线）": "Default (serif)",
  思源黑体: "Source Han Sans",
  微软雅黑: "Microsoft YaHei",
  苹方: "PingFang",
  宋体: "SimSun",
  楷体: "KaiTi",
  仿宋: "FangSong",
  思源宋体: "Source Han Serif",
  阅读字体: "Reading font",
  "EPUB 字体": "EPUB fonts",
  启用: "Enable",
  禁用: "Disable",
  翻页样式: "Page turning",
  平移翻页: "Slide pages",
  滚动翻页: "Scroll",
  列布局: "Column layout",
  单列: "Single column",
  双列: "Two columns",
  全屏下一页: "Full-screen next page",
  滚轮翻页: "Mouse-wheel page turning",
  方向键翻页: "Arrow-key page turning",
  // Editor settings
  显示设置: "Display",
  "把常用开关收成一列卡片，减少左右拉扯，和书库设置页保持同一套节奏。":
    "Keep common switches in one column, matching the library settings layout.",
  自动换行: "Word wrap",
  "编辑时按容器宽度自动换行，减少横向滚动。":
    "Wrap at the editor width to reduce horizontal scrolling.",
  显示空格: "Show spaces",
  "把空格可视化，便于清理多余缩进和格式噪点。":
    "Show spaces to spot extra indentation and formatting noise.",
  显示换行符: "Show line breaks",
  "把段落换行位置标出来，排查断行和章节格式更直接。":
    "Mark line breaks to inspect paragraph and chapter formatting.",
  保存清空撤销: "Clear undo history after save",
  "保存成功后重置当前撤销栈，避免历史状态和新内容混在一起。":
    "Reset the undo stack after a successful save.",
  字体子集化: "Font subsetting",
  "导出时只保留用到的字形，通常能进一步压缩 EPUB 体积。":
    "Keep only used glyphs when exporting to reduce EPUB size.",
  "单章字数检查:": "Chapter word-count check:",
  "校对 API:": "Proofing API:",
  "审批 API:": "Approval API:",
  "单章上限:": "Chapter limit:",
  "最长响应时间:": "Response timeout:",
  "自动审批:": "Auto-approve:",
  "AI 审核非自动建议，通过则自动应用，存疑保留人工审核。":
    "AI reviews non-automatic suggestions; approved changes apply automatically and doubtful ones remain for review.",
  "额外要求:": "Extra instructions:",
  "例如：保留作者口癖，不做风格润色。":
    "e.g. Preserve the author's voice; do not polish the style.",
  "可添加多个 OpenAI 兼容 API，分别供校对和自动审批选择。":
    "Add multiple OpenAI-compatible APIs for proofing and approval.",
  "暂无 API 配置，点击“新增”添加文字模型。":
    "No APIs configured. Select Add to create a text model.",
  外部字体: "External fonts",
  "导入后自动复制到当前书库目录下的“字体”文件夹，并可直接在块编辑里选择使用。":
    "Imported fonts are copied to the library Fonts folder and become available in block editing.",
  "导入中...": "Importing...",
  导入字体: "Import font",
  "重命名中...": "Renaming...",
  重命名: "Rename",
  "删除中...": "Deleting...",
  还没有导入外部字体: "No external fonts imported yet",
  暂无历史快照: "No history snapshots yet",
  暂无校对日志: "No proofing logs yet",
  选择左侧日志后查看完整内容: "Select a log on the left to view it",
  "共 {count} 个日志文件": "{count} log files",
  样式模板: "Style templates",
  "选择默认 CSS 模板，并直接编辑当前 EPUB 输出样式。":
    "Choose the default CSS template and edit the current EPUB output style.",
  "导入 CSS": "Import CSS",
  块编辑: "Block editor",
  源码: "Source",
  保存模板: "Save template",
  "内置模板（始终置顶）": "Built-in templates (always first)",
  恢复默认样式: "Restore default style",
  样式编辑: "Style editor",
  "当前模板：": "Current template:",
  样式块: "Style blocks",
  正则表达式: "Regular expression",
  "层级 {level}": "Level {level}",
  前缀层级: "Prefix level",
  "前缀识别，例如：番外、后日谈": "Prefix, e.g. Extra, Afterword",
  前缀: "Prefix",
  目录层级: "Contents level",
  输入正则表达式: "Enter a regular expression",
  预设: "Preset",
  选择预设正则: "Choose a preset regex",
  还原正则: "Restore regex",
  新增正则: "Add regex",

  // Import cards and common tool UI
  选择文件: "Choose file",
  "文件仅在当前设备中处理。": "Files are processed only on this device.",
  支持格式: "Supported formats",
  选择或拖入文件: "Choose or drop a file",
  正在读取文件: "Reading file",
  "处理中…": "Processing…",
  功能说明: "Features",
  "选择或拖入 EPUB 文件": "Choose or drop an EPUB",
  "选择 EPUB 文件": "Choose EPUB",
  选择文本文件: "Choose text file",
  重新选择: "Choose again",
  查看日志: "View log",
  下载: "Download",
  全部选择: "Select all",
  取消全选: "Clear selection",
  开始处理: "Start",
  取消处理: "Cancel",
  选择输出目录: "Choose output folder",
  打开输出目录: "Open output folder",
  重置输出路径: "Reset output path",
  输出目录: "Output folder",
  输出位置: "Output location",
  冲突处理: "Conflict handling",
  开始执行: "Start",
  取消任务: "Cancel task",
  重试失败: "Retry failed",
  导出报告: "Export report",
  导出日志: "Export log",
  清空日志: "Clear log",
  跳过: "Skipped",
  失败: "Failed",
  等待: "Waiting",
  异常: "Issues",
  处理日志: "Processing log",
  文件队列: "File queue",
  待处理列表: "Pending files",
  "尚未执行任务。": "No task has run yet.",
  "还没有加入 EPUB 文件或目录。": "No EPUB files or folders have been added.",
  "没有匹配的队列文件。": "No queue files match.",
  搜索文件名或路径: "Search filename or path",
  定位: "Locate",
  重新扫描: "Scan again",
  添加图书: "Add book",
  "搜索书名或作者...": "Search title or author...",
  解密入库中: "Decrypting and importing",
  入库中: "Importing",
  "{count} 本书": "{count} books",
  切换视图: "Change view",
  书库设置: "Library settings",
  "已筛选:": "Filtered:",
  点击移除该筛选: "Remove this filter",
  清除全部: "Clear all",
  本: "books",
  书库为空: "Your library is empty",
  "点击「+ 添加图书」或拖放文件到此处":
    "Select “+ Add book” or drop files here",
  无匹配结果: "No matching results",
  图书操作: "Book actions",
  编辑文件: "Edit file",
  打开文件位置: "Show in folder",
  重命名本地文件: "Rename local file",
  从书库移除: "Remove from library",
  图书: "Book",
  存储方式: "Storage mode",
  工作目录: "Work folder",
  未选择: "Not selected",
  更改: "Change",
  选择目录: "Choose folder",
  保存时更新修改日期: "Update modified date when saving",
  入库命名方式: "Import naming",
  命名模板: "Naming template",
  "例如: {template}": "e.g. {template}",
  自定义命名模板: "Custom naming template",
  "可用占位符：": "Available placeholders: ",
  自动展开为: "expands to",
  "用于自动匹配书库图书的标签和简介，不再和 TXT 编辑器智能校对共用用途设置。":
    "Automatically matches tags and descriptions for library books independently of TXT proofing settings.",
  "匹配 API": "Matching API",
  额外要求: "Extra instructions",
  九宫格显示书名: "Show titles in grid",
  九宫格显示作者: "Show authors in grid",
  预览区显示: "Preview visibility",
  双击图书时: "Double-click action",
  新书排序: "New book order",
  选择书库存放目录: "Choose library folder",
  常用位置: "Common locations",
  "正在读取目录…": "Reading folders…",
  当前目录: "Current folder",
  上一级: "Parent folder",
  这里没有子目录: "No subfolders",
  使用当前目录: "Use current folder",
  文件名冲突: "Filename conflict",
  同名文件已存在: "A file with this name already exists",
  "可省略扩展名，会自动按文件类型补回。":
    "The extension is optional and will be added automatically.",
  重命名图书: "Rename book",
  "当前:": "Current: ",
  编辑图书元数据: "Edit book metadata",
  基本: "Basic",
  书名: "Title",
  清除副标题: "Clear subtitle",
  "+ 添加副标题": "+ Add subtitle",
  作者: "Author",
  移除标签: "Remove tag",
  "点击此处选择/添加标签…": "Select or add tags…",
  "+ 标签": "+ Tag",
  库内没有匹配: "No matching tag in the library",
  "的标签 —": "tag —",
  "本书简介...": "Book description...",
  "可手动编辑或点击 ↻ 自动提取": "Edit manually or select ↻ to extract",
  "复制 UUID": "Copy UUID",
  "无 UUID 可复制": "No UUID to copy",
  "从 EPUB 文件重新读取元数据": "Reload metadata from the EPUB",
  已填写: "Filled",
  修改时间: "Modified",
  ...STATIC_EN_MESSAGES,
  ...DYNAMIC_EN_MESSAGES,
  ...RUNTIME_EN_MESSAGES,
};

function normalizeLocaleTag(value: string | null | undefined): string {
  return String(value || "")
    .trim()
    .replace(/_/g, "-")
    .toLowerCase();
}

export function isUiLanguagePreference(
  value: unknown,
): value is UiLanguagePreference {
  return (
    value === "auto" || value === "zh-CN" || value === "zh-TW" || value === "en"
  );
}

export function resolveAppLanguage(
  preference: UiLanguagePreference,
  languages?: readonly string[],
): AppLanguage {
  if (preference !== "auto") return preference;
  const candidates =
    languages ??
    (typeof navigator !== "undefined"
      ? navigator.languages?.length
        ? navigator.languages
        : [navigator.language]
      : []);
  for (const candidate of candidates) {
    const language = normalizeLocaleTag(candidate);
    if (!language) continue;
    if (language === "zh-tw" || language === "zh-hk" || language === "zh-mo") {
      return "zh-TW";
    }
    if (
      language === "zh-hant" ||
      language.startsWith("zh-hant-") ||
      language.includes("-hant")
    ) {
      return "zh-TW";
    }
    if (language === "zh" || language.startsWith("zh-")) return "zh-CN";
    if (language === "en" || language.startsWith("en-")) return "en";
  }
  return "en";
}

const toTraditional = OpenCC.Converter({ from: "cn", to: "twp" });

let languagePreference = $state<UiLanguagePreference>("auto");
let language = $state<AppLanguage>("zh-CN");

function interpolate(template: string, params?: Record<string, unknown>) {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, key) => {
    const value = params[key];
    return value === undefined || value === null ? match : String(value);
  });
}

function applyDocumentLanguage(next: AppLanguage) {
  if (typeof document === "undefined") return;
  document.documentElement.lang = next;
  document.documentElement.dataset.language = next;
}

export function initializeI18n(preference: UiLanguagePreference) {
  languagePreference = isUiLanguagePreference(preference) ? preference : "auto";
  language = resolveAppLanguage(languagePreference);
  applyDocumentLanguage(language);
}

export function setI18nPreference(preference: UiLanguagePreference) {
  initializeI18n(preference);
}

export function currentLanguage(): AppLanguage {
  return language;
}

export function currentLanguagePreference(): UiLanguagePreference {
  return languagePreference;
}

export function t(source: string, params?: Record<string, unknown>): string {
  const active = language;
  if (active === "en") {
    return interpolate(EN_MESSAGES[source] ?? source, params);
  }
  if (active === "zh-TW") {
    return interpolate(toTraditional(source), params);
  }
  return interpolate(source, params);
}

const LOCALIZABLE_DATA_FIELDS = new Set([
  "title",
  "label",
  "detail",
  "description",
  "hint",
  "message",
  "kicker",
  "note",
  "reason",
  "usage",
  "placeholder",
  "confirmLabel",
  "cancelLabel",
  "secondaryLabel",
  "outputLabel",
  "privacy",
]);

export function localizeUiData<T>(value: T): T {
  if (Array.isArray(value)) {
    return value.map((item) => localizeUiData(item)) as T;
  }
  if (!value || typeof value !== "object") return value;

  const result: Record<string, unknown> = {};
  for (const [key, child] of Object.entries(value)) {
    if (typeof child === "string" && LOCALIZABLE_DATA_FIELDS.has(key)) {
      result[key] = t(child);
    } else {
      result[key] = localizeUiData(child);
    }
  }
  return result as T;
}

const DEFAULT_AI_PROVIDER_NAMES = new Set([
  "默认 API",
  "新 API",
  "文字 API",
  "生图 API",
]);

export function localizeAiProviderName(name: string): string {
  return DEFAULT_AI_PROVIDER_NAMES.has(name) ? t(name) : name;
}

function parseLocalizedDate(
  value: number | string | Date | null | undefined,
): Date | null {
  if (value === null || value === undefined || value === "") return null;
  const date = value instanceof Date ? value : new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatDateLocalized(
  value: number | string | Date | null | undefined,
): string {
  const date = parseLocalizedDate(value);
  if (!date) return "";
  return date.toLocaleDateString(language, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

export function formatTimeLocalized(
  value: number | string | Date | null | undefined = new Date(),
): string {
  const date = parseLocalizedDate(value);
  if (!date) return "";
  return date.toLocaleTimeString(language, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function formatDateTimeLocalized(
  value: number | string | Date | null | undefined,
): string {
  const date = parseLocalizedDate(value);
  if (!date) return "";
  return date.toLocaleString(language, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
