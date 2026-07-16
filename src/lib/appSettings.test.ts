import { afterEach, describe, expect, it } from "vitest";
import { loadAppSettings, saveAppSettings } from "./appSettings";

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
// @vitest-environment happy-dom
