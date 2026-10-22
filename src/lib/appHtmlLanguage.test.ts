import fs from "node:fs";
import { Window } from "happy-dom";
import { describe, expect, it } from "vitest";

const html = fs.readFileSync(new URL("../app.html", import.meta.url), "utf8");
const inlineScript = html.match(/<script>([\s\S]*?)<\/script>/)?.[1];

function firstPaintLanguage(
  languages: string[],
  storedSettings?: Record<string, unknown>,
  runtime: "web" | "tauri" = "web",
) {
  if (!inlineScript) throw new Error("app.html inline script not found");
  const window = new Window({ url: "http://localhost/" });
  Object.defineProperty(window.navigator, "languages", {
    value: languages,
    configurable: true,
  });
  Object.defineProperty(window.navigator, "language", {
    value: languages[0],
    configurable: true,
  });
  window.matchMedia = (() => ({
    matches: false,
    media: "",
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent() {
      return false;
    },
  })) as unknown as typeof window.matchMedia;
  if (runtime === "tauri") {
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {},
      configurable: true,
    });
  }
  if (storedSettings) {
    const storage =
      runtime === "tauri" ? window.localStorage : window.sessionStorage;
    const key =
      runtime === "tauri" ? "app-settings" : "tepub-web-guest-settings";
    storage.setItem(key, JSON.stringify(storedSettings));
  }
  window.eval(inlineScript);
  return window.document.documentElement.lang;
}

describe("first-paint language detection", () => {
  it("matches automatic browser detection", () => {
    expect(firstPaintLanguage(["en-US"])).toBe("en");
    expect(firstPaintLanguage(["zh-TW"])).toBe("zh-TW");
    expect(firstPaintLanguage(["zh-CN"])).toBe("zh-CN");
    expect(firstPaintLanguage(["fr-FR"])).toBe("en");
  });

  it("honors an explicit saved preference", () => {
    expect(firstPaintLanguage(["zh-TW"], { uiLanguage: "en" })).toBe("en");
    expect(
      firstPaintLanguage(["en-US"], { uiLanguage: "zh-TW" }, "tauri"),
    ).toBe("zh-TW");
  });
});
