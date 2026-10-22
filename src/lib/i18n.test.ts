import { describe, expect, it } from "vitest";
import {
  initializeI18n,
  localizeAiProviderName,
  localizeUiData,
  resolveAppLanguage,
  setI18nPreference,
  t,
} from "./i18n.svelte";

describe("language resolution", () => {
  it("follows the first supported browser language", () => {
    expect(resolveAppLanguage("auto", ["fr-FR", "en-US"])).toBe("en");
    expect(resolveAppLanguage("auto", ["zh-CN", "en-US"])).toBe("zh-CN");
    expect(resolveAppLanguage("auto", ["zh-HK", "en-US"])).toBe("zh-TW");
    expect(resolveAppLanguage("auto", ["zh-Hant-TW"])).toBe("zh-TW");
  });

  it("uses English for unsupported languages", () => {
    expect(resolveAppLanguage("auto", ["fr-FR"])).toBe("en");
  });

  it("always honors an explicit preference", () => {
    expect(resolveAppLanguage("zh-TW", ["en-US"])).toBe("zh-TW");
    expect(resolveAppLanguage("zh-CN", ["zh-TW"])).toBe("zh-CN");
  });
});

describe("runtime translation", () => {
  it("translates and interpolates English messages", () => {
    initializeI18n("en");
    expect(t("共 {count} 个日志文件", { count: 3 })).toBe("3 log files");
    expect(t("未知文案")).toBe("未知文案");
  });

  it("converts Simplified Chinese to Traditional Chinese", () => {
    setI18nPreference("zh-TW");
    expect(t("设置")).toBe("設定");
    expect(t("书库")).toBe("書庫");
  });

  it("leaves Simplified Chinese unchanged", () => {
    setI18nPreference("zh-CN");
    expect(t("设置")).toBe("设置");
  });

  it("localizes known UI data fields without touching data values", () => {
    initializeI18n("en");
    const source = {
      title: "设置",
      detail: "跟随系统",
      id: "settings",
      nested: {
        label: "书库",
        value: "书库",
      },
    };

    expect(localizeUiData(source)).toEqual({
      title: "Settings",
      detail: "Follow system",
      id: "settings",
      nested: {
        label: "Library",
        value: "书库",
      },
    });
  });

  it("localizes generated provider names without changing custom names", () => {
    initializeI18n("en");
    expect(localizeAiProviderName("文字 API")).toBe("Text API");
    expect(localizeAiProviderName("My private endpoint")).toBe(
      "My private endpoint",
    );
  });
});
