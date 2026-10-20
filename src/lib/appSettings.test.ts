import { afterEach, describe, expect, it } from "vitest";
import {
  DEFAULT_APP_SETTINGS,
  isPreviewScrollbarEnabled,
  loadAppSettings,
  normalizeAppSettings,
  saveAppSettings,
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
// @vitest-environment happy-dom
