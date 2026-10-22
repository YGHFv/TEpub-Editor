import {
  notifySettingsChanged,
  readScopedSettingsRaw,
  writeScopedSettingsRaw,
} from "$lib/webAccount";
import {
  isUiLanguagePreference,
  type UiLanguagePreference,
} from "$lib/i18n.svelte";

export type UiTheme = "light" | "dark" | "system";
export type EffectiveTheme = "light" | "dark";
export type AiProviderKind = "text" | "image";

export interface TocRegexRule {
  enabled: boolean;
  level: number;
  pattern: string;
}

export interface AiProofingConfig {
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

export interface AiProviderConfig {
  id: string;
  name: string;
  kind: AiProviderKind;
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
}

export interface TxtAiProofingConfig {
  providerId: string;
  approvalProviderId: string;
}

export interface GlobalAppSettings {
  uiTheme: UiTheme;
  uiLanguage: UiLanguagePreference;
  assocEpubRead: boolean;
  assocEpubEdit: boolean;
  assocTxtMakeEpub: boolean;
  closeLibraryOnTxtOpen: boolean;
  closeLibraryOnEpubOpen: boolean;
  closeLibraryOnToolboxOpen: boolean;
  closeToolboxOnToolOpen: boolean;
  /** EPUB 编辑/阅读预览是否显示滚动条。默认隐藏：滚动条会占掉右侧 8px，
      书页就不在预览列正中，观感很别扭 */
  previewScrollbar: boolean;
  /** 二级页面（书库/编辑器/工具）是否各自开新窗口。默认关闭：
      关闭时在主窗口内跳转，顶部用标签栏切换；打开才回到多窗口行为 */
  secondaryWindows: boolean;
  txtEditorCloseAction: "exit" | "library";
  aiProofing: AiProofingConfig;
  aiProviders: AiProviderConfig[];
  txtAiProofing: TxtAiProofingConfig;
  customRegexRules: TocRegexRule[];
}

export const DEFAULT_AI_PROOFING: AiProofingConfig = {
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

export const DEFAULT_TOC_REGEX_RULES: TocRegexRule[] = [
  {
    enabled: true,
    level: 1,
    pattern: "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$",
  },
  {
    enabled: true,
    level: 1,
    pattern:
      "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*",
  },
  {
    enabled: true,
    level: 3,
    pattern: "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$",
  },
  {
    enabled: true,
    level: 3,
    pattern:
      "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*",
  },
];

export const DEFAULT_APP_SETTINGS: GlobalAppSettings = {
  uiTheme: "system",
  uiLanguage: "auto",
  assocEpubRead: false,
  assocEpubEdit: false,
  assocTxtMakeEpub: false,
  closeLibraryOnTxtOpen: true,
  closeLibraryOnEpubOpen: true,
  closeLibraryOnToolboxOpen: true,
  closeToolboxOnToolOpen: true,
  previewScrollbar: false,
  secondaryWindows: false,
  txtEditorCloseAction: "library",
  aiProofing: { ...DEFAULT_AI_PROOFING },
  aiProviders: [],
  txtAiProofing: { providerId: "", approvalProviderId: "" },
  customRegexRules: DEFAULT_TOC_REGEX_RULES.map((rule) => ({ ...rule })),
};

export function normalizeAiProofingConfig(
  config: Partial<AiProofingConfig> | undefined,
): AiProofingConfig {
  const merged = { ...DEFAULT_AI_PROOFING, ...(config || {}) };
  merged.enabled = Boolean(merged.enabled);
  merged.baseUrl = String(merged.baseUrl || DEFAULT_AI_PROOFING.baseUrl).trim();
  merged.apiKey = String(merged.apiKey || "");
  merged.model = String(merged.model || DEFAULT_AI_PROOFING.model).trim();
  merged.temperature = Math.max(
    0,
    Math.min(1, Number(merged.temperature) || DEFAULT_AI_PROOFING.temperature),
  );
  merged.maxChapterChars = Math.max(
    1000,
    Math.floor(
      Number(merged.maxChapterChars) || DEFAULT_AI_PROOFING.maxChapterChars,
    ),
  );
  merged.responseTimeoutSec = Math.max(
    30,
    Math.min(
      1800,
      Math.floor(
        Number(merged.responseTimeoutSec) ||
          DEFAULT_AI_PROOFING.responseTimeoutSec,
      ),
    ),
  );
  merged.autoApprove = Boolean(merged.autoApprove);
  merged.extraPrompt = String(merged.extraPrompt || "");
  return merged;
}

export function newAiProvider(
  seed: Partial<AiProviderConfig> = {},
): AiProviderConfig {
  const kind: AiProviderKind = seed.kind === "image" ? "image" : "text";
  const defaultModel =
    kind === "image" ? "gpt-image-1" : DEFAULT_AI_PROOFING.model;
  const model = String(seed.model || defaultModel).trim();
  return {
    id: String(
      seed.id ||
        `provider-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    ),
    name: String(
      seed.name || model || (kind === "image" ? "生图 API" : "文字 API"),
    ).trim(),
    kind,
    baseUrl: String(seed.baseUrl || DEFAULT_AI_PROOFING.baseUrl).trim(),
    apiKey: String(seed.apiKey || ""),
    model,
    temperature: Math.max(
      0,
      Math.min(1, Number(seed.temperature) || DEFAULT_AI_PROOFING.temperature),
    ),
  };
}

export function normalizeAiProvider(
  provider: Partial<AiProviderConfig> | undefined,
  index = 0,
): AiProviderConfig {
  return newAiProvider({
    id: String(provider?.id || `provider-${index + 1}`),
    name: provider?.name,
    kind: provider?.kind,
    baseUrl: provider?.baseUrl,
    apiKey: provider?.apiKey,
    model: provider?.model,
    temperature: provider?.temperature,
  });
}

export function normalizeAiProviders(
  providers: Array<Partial<AiProviderConfig>> | undefined,
  fallback?: Partial<AiProofingConfig>,
): AiProviderConfig[] {
  const list = (providers || [])
    .map(normalizeAiProvider)
    .filter((provider) => provider.id);
  if (list.length > 0) return list;
  if (fallback?.apiKey)
    return [newAiProvider(normalizeAiProofingConfig(fallback))];
  return [];
}

export function providerToProofingConfig(
  provider: AiProviderConfig | undefined,
  base: Partial<AiProofingConfig> | undefined,
): AiProofingConfig {
  const fallback = normalizeAiProofingConfig(base);
  if (!provider || provider.kind === "image") return fallback;
  return normalizeAiProofingConfig({
    ...fallback,
    baseUrl: provider.baseUrl,
    apiKey: provider.apiKey,
    model: provider.model,
    temperature: provider.temperature,
  });
}

function isUiTheme(value: unknown): value is UiTheme {
  return value === "light" || value === "dark" || value === "system";
}

function migrateUiTheme(value: unknown): UiTheme | null {
  if (value === "modern" || value === "classic") return "light";
  if (isUiTheme(value)) return value;
  return null;
}

function isCloseAction(value: unknown): value is "exit" | "library" {
  return value === "exit" || value === "library";
}

function readStoredSettings(): Record<string, any> {
  return readScopedSettingsRaw();
}

export function normalizeTocRegexRules(
  rules: Array<Partial<TocRegexRule>> | undefined,
): TocRegexRule[] {
  const source =
    Array.isArray(rules) && rules.length ? rules : DEFAULT_TOC_REGEX_RULES;
  return source
    .map((rule) => ({
      enabled: typeof rule.enabled === "boolean" ? rule.enabled : true,
      level: Number(rule.level) <= 1 ? 1 : 3,
      pattern: String(rule.pattern || "").trim(),
    }))
    .filter((rule) => rule.pattern);
}

export function normalizeAppSettings(
  raw: Record<string, any> = {},
): GlobalAppSettings {
  const aiProofing = normalizeAiProofingConfig(raw.aiProofing);
  const settings: GlobalAppSettings = {
    ...DEFAULT_APP_SETTINGS,
    ...raw,
    uiTheme: migrateUiTheme(raw.uiTheme) ?? DEFAULT_APP_SETTINGS.uiTheme,
    uiLanguage: isUiLanguagePreference(raw.uiLanguage)
      ? raw.uiLanguage
      : DEFAULT_APP_SETTINGS.uiLanguage,
    assocEpubRead: Boolean(raw.assocEpubRead),
    assocEpubEdit: Boolean(raw.assocEpubEdit),
    assocTxtMakeEpub: Boolean(raw.assocTxtMakeEpub),
    closeLibraryOnTxtOpen:
      typeof raw.closeLibraryOnTxtOpen === "boolean"
        ? raw.closeLibraryOnTxtOpen
        : true,
    closeLibraryOnEpubOpen:
      typeof raw.closeLibraryOnEpubOpen === "boolean"
        ? raw.closeLibraryOnEpubOpen
        : true,
    closeLibraryOnToolboxOpen:
      typeof raw.closeLibraryOnToolboxOpen === "boolean"
        ? raw.closeLibraryOnToolboxOpen
        : true,
    closeToolboxOnToolOpen:
      typeof raw.closeToolboxOnToolOpen === "boolean"
        ? raw.closeToolboxOnToolOpen
        : true,
    previewScrollbar:
      typeof raw.previewScrollbar === "boolean" ? raw.previewScrollbar : false,
    secondaryWindows:
      typeof raw.secondaryWindows === "boolean" ? raw.secondaryWindows : false,
    txtEditorCloseAction: isCloseAction(raw.txtEditorCloseAction)
      ? raw.txtEditorCloseAction
      : "library",
    aiProofing,
    aiProviders: normalizeAiProviders(raw.aiProviders, aiProofing),
    txtAiProofing: {
      providerId: String(raw.txtAiProofing?.providerId || ""),
      approvalProviderId: String(
        raw.txtAiProofing?.approvalProviderId ||
          raw.txtAiProofing?.providerId ||
          "",
      ),
    },
    customRegexRules: normalizeTocRegexRules(raw.customRegexRules),
  };
  return ensureAiProviderSelections(settings);
}

export function loadAppSettings(
  legacySource: Record<string, any> = {},
): GlobalAppSettings {
  const stored = readStoredSettings();
  const merged = { ...legacySource, ...stored };
  if (
    !Array.isArray(stored.aiProviders) &&
    Array.isArray(legacySource.aiProviders)
  ) {
    merged.aiProviders = legacySource.aiProviders;
  }
  if (!stored.aiProofing && legacySource.aiProofing) {
    merged.aiProofing = legacySource.aiProofing;
  }
  if (!stored.txtAiProofing && legacySource.txtAiProofing) {
    merged.txtAiProofing = legacySource.txtAiProofing;
  }
  return normalizeAppSettings(merged);
}

export function ensureAiProviderSelections(
  settings: GlobalAppSettings,
): GlobalAppSettings {
  const aiProviders = normalizeAiProviders(
    settings.aiProviders,
    settings.aiProofing,
  );
  const firstTextId =
    aiProviders.find((provider) => provider.kind !== "image")?.id || "";
  const txtAiProofing = {
    providerId: settings.txtAiProofing?.providerId || firstTextId,
    approvalProviderId:
      settings.txtAiProofing?.approvalProviderId ||
      settings.txtAiProofing?.providerId ||
      firstTextId,
  };
  if (
    !aiProviders.some(
      (provider) =>
        provider.id === txtAiProofing.providerId && provider.kind !== "image",
    )
  ) {
    txtAiProofing.providerId = firstTextId;
  }
  if (
    !aiProviders.some(
      (provider) =>
        provider.id === txtAiProofing.approvalProviderId &&
        provider.kind !== "image",
    )
  ) {
    txtAiProofing.approvalProviderId = txtAiProofing.providerId || firstTextId;
  }
  return {
    ...settings,
    aiProofing: normalizeAiProofingConfig(settings.aiProofing),
    aiProviders,
    txtAiProofing,
  };
}

export function saveAppSettings(
  settings: GlobalAppSettings,
): GlobalAppSettings {
  const normalized = ensureAiProviderSelections(normalizeAppSettings(settings));
  if (typeof localStorage !== "undefined") {
    writeScopedSettingsRaw(normalized);
  }
  applyTheme(normalized.uiTheme);
  notifySettingsChanged();
  return normalized;
}

/** 「预览是否显示滚动条」，默认关闭 */
export function isPreviewScrollbarEnabled(): boolean {
  return loadAppSettings().previewScrollbar === true;
}

/** 「使用多窗口打开二级页面」，默认关闭（同窗口 + 顶部标签栏） */
export function usesMultiWindow() {
  return loadAppSettings().secondaryWindows === true;
}

/** 各实际主题下的窗口/画布底色，供 <meta theme-color> 与原生标题栏同步使用 */
export const THEME_CANVAS_COLORS: Record<EffectiveTheme, string> = {
  light: "#eef4f8",
  dark: "#191e26",
};

const SYSTEM_DARK_QUERY = "(prefers-color-scheme: dark)";
export const THEME_RESOLVED_EVENT = "tepub-theme-resolved";
let activeThemePreference: UiTheme = "system";
let systemThemeMedia: MediaQueryList | null = null;
let systemThemeListenerInstalled = false;

/** 系统是否处于深色。非浏览器环境按浅色处理。 */
export function systemPrefersDark(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia(SYSTEM_DARK_QUERY).matches
  );
}

/** 把「跟随系统」解析成真正用于渲染的 light/dark。 */
export function resolveUiTheme(
  theme: UiTheme | string | null | undefined,
): EffectiveTheme {
  const normalized = migrateUiTheme(theme) ?? "system";
  if (normalized === "dark") return "dark";
  if (normalized === "system") return systemPrefersDark() ? "dark" : "light";
  return "light";
}

function applyResolvedTheme() {
  if (typeof document === "undefined") return;
  const resolved = resolveUiTheme(activeThemePreference);
  const changed = document.documentElement.dataset.theme !== resolved;
  document.documentElement.setAttribute("data-theme", resolved);
  document.documentElement.dataset.themePreference = activeThemePreference;
  // color-scheme 让原生控件（滚动条、下拉、表单）一起跟着切，
  // 否则深色皮肤下 WebView 仍按浅色画默认控件
  document.documentElement.style.colorScheme = resolved;
  const meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute("content", THEME_CANVAS_COLORS[resolved]);
  // app.html 的内联脚本在首帧前就设过内联底色；这里接管后要撤掉，
  // 否则以后切回浅色时那层内联深色还压在上面（内联样式优先级高于 app.css）
  document.documentElement.style.removeProperty("background");
  syncAndroidSystemBars(resolved === "dark");
  void syncNativeWindowTheme(resolved);
  if (changed) {
    window.dispatchEvent(
      new CustomEvent(THEME_RESOLVED_EVENT, { detail: resolved }),
    );
  }
}

function ensureSystemThemeListener() {
  if (
    systemThemeListenerInstalled ||
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  )
    return;
  systemThemeMedia = window.matchMedia(SYSTEM_DARK_QUERY);
  const onChange = () => {
    if (activeThemePreference === "system") applyResolvedTheme();
  };
  systemThemeMedia.addEventListener?.("change", onChange);
  // 旧版 WebView 只有 addListener
  if (!systemThemeMedia.addEventListener) {
    systemThemeMedia.addListener?.(onChange);
  }
  systemThemeListenerInstalled = true;
}

export function applyTheme(theme: UiTheme) {
  activeThemePreference = migrateUiTheme(theme) ?? "system";
  ensureSystemThemeListener();
  applyResolvedTheme();
}

/**
 * Android WebView 的系统栏是原生绘制的，CSS 的 data-theme 管不到图标颜色。
 * MainActivity 通过 addJavascriptInterface 暴露这个很小的桥；非 Android
 * 环境没有对象时直接跳过。首次调用可能早于 WebView 注入，稍后再补一次。
 */
function syncAndroidSystemBars(dark: boolean) {
  if (typeof window === "undefined") return;
  const bridge = (window as Window & {
    TepubAndroid?: { setDarkMode?: (value: boolean) => void };
  }).TepubAndroid;
  if (typeof bridge?.setDarkMode === "function") {
    try {
      bridge.setDarkMode(dark);
      return;
    } catch {
      // 原生侧异常不应影响主题切换
    }
  }
  window.setTimeout(() => {
    const retry = (window as Window & {
      TepubAndroid?: { setDarkMode?: (value: boolean) => void };
    }).TepubAndroid;
    try {
      // 定时器触发时重新解析当前偏好。连续快速切换主题时，较早排下的
      // 重试不能拿着旧值把刚刚生效的系统栏图标颜色覆盖回去。
      retry?.setDarkMode?.(resolveUiTheme(activeThemePreference) === "dark");
    } catch {
      // Ignore: web builds never expose the bridge.
    }
  }, 250);
}

/**
 * Tauri 窗口的标题栏由系统绘制，跟 CSS 主题无关。
 * 切到深色时要显式告诉 WebView2/AppKit，否则顶部标题行永远是浅色
 * （截图里深色界面顶着一整条白条就是这个原因）。
 */
async function syncNativeWindowTheme(theme: EffectiveTheme) {
  try {
    // 动态 import：web 构建里不该把 tauri 模块带进来
    const { isTauriRuntime } = await import("$lib/webAccount");
    if (!isTauriRuntime()) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    await win.setTheme(theme);
    // 原生标题栏重绘有一帧延迟，同帧再刷一次背景色兜底
    await win.setBackgroundColor?.(THEME_CANVAS_COLORS[theme]);
  } catch (error) {
    // 权限未授予或不是 Tauri 运行时：只影响标题栏配色，不阻断主题切换
    console.warn("同步原生窗口主题失败:", error);
  }
}
