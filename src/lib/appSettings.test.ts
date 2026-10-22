// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  applyTheme,
  DEFAULT_APP_SETTINGS,
  isPreviewScrollbarEnabled,
  loadAppSettings,
  normalizeAppSettings,
  resolveUiTheme,
  saveAppSettings,
  THEME_RESOLVED_EVENT,
  usesMultiWindow,
} from "./appSettings";

describe("app settings persistence", () => {
  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("normalizes, persists and reloads user-facing settings", () => {
    const initial = loadAppSettings();
    const saved = saveAppSettings({
      ...initial,
      uiTheme: "dark",
      closeToolboxOnToolOpen: false,
      txtEditorCloseAction: "exit",
    });
    const loaded = loadAppSettings();
    expect(loaded.uiTheme).toBe("dark");
    expect(loaded.closeToolboxOnToolOpen).toBe(false);
    expect(loaded.txtEditorCloseAction).toBe("exit");
    expect(saved.aiProviders).toEqual(loaded.aiProviders);
    expect(document.documentElement.dataset.theme).toBe("dark");
  });
});

describe("preview scrollbar setting", () => {
  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it("defaults to hidden", () => {
    expect(DEFAULT_APP_SETTINGS.previewScrollbar).toBe(false);
    expect(normalizeAppSettings({}).previewScrollbar).toBe(false);
    expect(isPreviewScrollbarEnabled()).toBe(false);
  });

  it("keeps an explicit true after save and reload", () => {
    saveAppSettings({
      ...loadAppSettings(),
      previewScrollbar: true,
    });
    expect(loadAppSettings().previewScrollbar).toBe(true);
    expect(isPreviewScrollbarEnabled()).toBe(true);
  });

  it("falls back to false for non-boolean stored values", () => {
    expect(
      normalizeAppSettings({ previewScrollbar: "yes" }).previewScrollbar,
    ).toBe(false);
    expect(normalizeAppSettings({ previewScrollbar: 1 }).previewScrollbar).toBe(
      false,
    );
  });
});

describe("system theme preference", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
    sessionStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-theme-preference");
  });

  function mockSystemDark(dark: boolean) {
    const media = {
      matches: dark,
      media: "(prefers-color-scheme: dark)",
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    } as unknown as MediaQueryList;
    vi.spyOn(window, "matchMedia").mockReturnValue(media);
  }

  it("defaults to following the system", () => {
    expect(DEFAULT_APP_SETTINGS.uiTheme).toBe("system");
    expect(normalizeAppSettings({}).uiTheme).toBe("system");
  });

  it("resolves system into the current effective theme", () => {
    mockSystemDark(true);
    expect(resolveUiTheme("system")).toBe("dark");

    mockSystemDark(false);
    expect(resolveUiTheme("system")).toBe("light");
  });

  it("keeps an explicit light or dark preference regardless of the system", () => {
    mockSystemDark(true);
    expect(resolveUiTheme("light")).toBe("light");
    expect(resolveUiTheme("dark")).toBe("dark");

    applyTheme("system");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.dataset.themePreference).toBe("system");
  });

  it("announces a resolved theme change for previews and readers", () => {
    mockSystemDark(true);
    document.documentElement.dataset.theme = "light";
    const listener = vi.fn();
    window.addEventListener(THEME_RESOLVED_EVENT, listener);

    applyTheme("system");

    expect(listener).toHaveBeenCalledTimes(1);
    expect((listener.mock.calls[0][0] as CustomEvent).detail).toBe("dark");
    window.removeEventListener(THEME_RESOLVED_EVENT, listener);
  });
});

describe("multi-window secondary pages setting", () => {
  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it("defaults to same-window tab mode", () => {
    expect(DEFAULT_APP_SETTINGS.secondaryWindows).toBe(false);
    expect(normalizeAppSettings({}).secondaryWindows).toBe(false);
    expect(usesMultiWindow()).toBe(false);
  });

  it("keeps an explicit true after save and reload", () => {
    saveAppSettings({ ...loadAppSettings(), secondaryWindows: true });
    expect(loadAppSettings().secondaryWindows).toBe(true);
    expect(usesMultiWindow()).toBe(true);
  });
});
