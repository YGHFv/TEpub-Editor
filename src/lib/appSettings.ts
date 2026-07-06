import { notifySettingsChanged, readScopedSettingsRaw, writeScopedSettingsRaw } from "$lib/webAccount";

export type UiTheme = "modern" | "classic" | "dark";
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
  assocEpubRead: boolean;
  assocEpubEdit: boolean;
  assocTxtMakeEpub: boolean;
  closeLibraryOnTxtOpen: boolean;
  closeLibraryOnEpubOpen: boolean;
  closeLibraryOnToolboxOpen: boolean;
  closeToolboxOnToolOpen: boolean;
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
  { enabled: true, level: 1, pattern: "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$" },
  {
    enabled: true,
    level: 1,
    pattern: "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*",
  },
  { enabled: true, level: 3, pattern: "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$" },
  {
    enabled: true,
    level: 3,
    pattern: "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*",
  },
];

export const DEFAULT_APP_SETTINGS: GlobalAppSettings = {
  uiTheme: "modern",
  assocEpubRead: false,
  assocEpubEdit: false,
  assocTxtMakeEpub: false,
  closeLibraryOnTxtOpen: true,
  closeLibraryOnEpubOpen: true,
  closeLibraryOnToolboxOpen: true,
  closeToolboxOnToolOpen: true,
  txtEditorCloseAction: "library",
  aiProofing: { ...DEFAULT_AI_PROOFING },
  aiProviders: [],
  txtAiProofing: { providerId: "", approvalProviderId: "" },
  customRegexRules: DEFAULT_TOC_REGEX_RULES.map((rule) => ({ ...rule })),
};

export function normalizeAiProofingConfig(config: Partial<AiProofingConfig> | undefined): AiProofingConfig {
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

export function newAiProvider(seed: Partial<AiProviderConfig> = {}): AiProviderConfig {
  const kind: AiProviderKind = seed.kind === "image" ? "image" : "text";
  const defaultModel = kind === "image" ? "gpt-image-1" : DEFAULT_AI_PROOFING.model;
  const model = String(seed.model || defaultModel).trim();
  return {
    id: String(seed.id || `provider-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`),
    name: String(seed.name || model || (kind === "image" ? "生图 API" : "文字 API")).trim(),
    kind,
    baseUrl: String(seed.baseUrl || DEFAULT_AI_PROOFING.baseUrl).trim(),
    apiKey: String(seed.apiKey || ""),
    model,
    temperature: Math.max(0, Math.min(1, Number(seed.temperature) || DEFAULT_AI_PROOFING.temperature)),
  };
}

export function normalizeAiProvider(provider: Partial<AiProviderConfig> | undefined, index = 0): AiProviderConfig {
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
  const list = (providers || []).map(normalizeAiProvider).filter((provider) => provider.id);
  if (list.length > 0) return list;
  if (fallback?.apiKey) return [newAiProvider(normalizeAiProofingConfig(fallback))];
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
  return value === "modern" || value === "classic" || value === "dark";
}

function isCloseAction(value: unknown): value is "exit" | "library" {
  return value === "exit" || value === "library";
}

function readStoredSettings(): Record<string, any> {
  return readScopedSettingsRaw();
}

export function normalizeTocRegexRules(rules: Array<Partial<TocRegexRule>> | undefined): TocRegexRule[] {
  const source = Array.isArray(rules) && rules.length ? rules : DEFAULT_TOC_REGEX_RULES;
  return source
    .map((rule) => ({
      enabled: typeof rule.enabled === "boolean" ? rule.enabled : true,
      level: Number(rule.level) <= 1 ? 1 : 3,
      pattern: String(rule.pattern || "").trim(),
    }))
    .filter((rule) => rule.pattern);
}

export function normalizeAppSettings(raw: Record<string, any> = {}): GlobalAppSettings {
  const aiProofing = normalizeAiProofingConfig(raw.aiProofing);
  const settings: GlobalAppSettings = {
    ...DEFAULT_APP_SETTINGS,
    ...raw,
    uiTheme: isUiTheme(raw.uiTheme) ? raw.uiTheme : DEFAULT_APP_SETTINGS.uiTheme,
    assocEpubRead: Boolean(raw.assocEpubRead),
    assocEpubEdit: Boolean(raw.assocEpubEdit),
    assocTxtMakeEpub: Boolean(raw.assocTxtMakeEpub),
    closeLibraryOnTxtOpen: typeof raw.closeLibraryOnTxtOpen === "boolean" ? raw.closeLibraryOnTxtOpen : true,
    closeLibraryOnEpubOpen: typeof raw.closeLibraryOnEpubOpen === "boolean" ? raw.closeLibraryOnEpubOpen : true,
    closeLibraryOnToolboxOpen: typeof raw.closeLibraryOnToolboxOpen === "boolean" ? raw.closeLibraryOnToolboxOpen : true,
    closeToolboxOnToolOpen: typeof raw.closeToolboxOnToolOpen === "boolean" ? raw.closeToolboxOnToolOpen : true,
    txtEditorCloseAction: isCloseAction(raw.txtEditorCloseAction) ? raw.txtEditorCloseAction : "library",
    aiProofing,
    aiProviders: normalizeAiProviders(raw.aiProviders, aiProofing),
    txtAiProofing: {
      providerId: String(raw.txtAiProofing?.providerId || ""),
      approvalProviderId: String(raw.txtAiProofing?.approvalProviderId || raw.txtAiProofing?.providerId || ""),
    },
    customRegexRules: normalizeTocRegexRules(raw.customRegexRules),
  };
  return ensureAiProviderSelections(settings);
}

export function loadAppSettings(legacySource: Record<string, any> = {}): GlobalAppSettings {
  const stored = readStoredSettings();
  const merged = { ...legacySource, ...stored };
  if (!Array.isArray(stored.aiProviders) && Array.isArray(legacySource.aiProviders)) {
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

export function ensureAiProviderSelections(settings: GlobalAppSettings): GlobalAppSettings {
  const aiProviders = normalizeAiProviders(settings.aiProviders, settings.aiProofing);
  const firstTextId = aiProviders.find((provider) => provider.kind !== "image")?.id || "";
  const txtAiProofing = {
    providerId: settings.txtAiProofing?.providerId || firstTextId,
    approvalProviderId: settings.txtAiProofing?.approvalProviderId || settings.txtAiProofing?.providerId || firstTextId,
  };
  if (!aiProviders.some((provider) => provider.id === txtAiProofing.providerId && provider.kind !== "image")) {
    txtAiProofing.providerId = firstTextId;
  }
  if (!aiProviders.some((provider) => provider.id === txtAiProofing.approvalProviderId && provider.kind !== "image")) {
    txtAiProofing.approvalProviderId = txtAiProofing.providerId || firstTextId;
  }
  return {
    ...settings,
    aiProofing: normalizeAiProofingConfig(settings.aiProofing),
    aiProviders,
    txtAiProofing,
  };
}

export function saveAppSettings(settings: GlobalAppSettings): GlobalAppSettings {
  const normalized = ensureAiProviderSelections(normalizeAppSettings(settings));
  if (typeof localStorage !== "undefined") {
    writeScopedSettingsRaw(normalized);
  }
  applyTheme(normalized.uiTheme);
  notifySettingsChanged();
  return normalized;
}

export function applyTheme(theme: UiTheme) {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", theme);
  const meta = document.querySelector('meta[name="theme-color"]');
  const colors: Record<UiTheme, string> = {
    modern: "#eef4f8",
    classic: "#f3f3f3",
    dark: "#151b23",
  };
  if (meta) meta.setAttribute("content", colors[theme]);
}
