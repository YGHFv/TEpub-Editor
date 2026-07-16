import { describe, expect, it } from "vitest";
import { hasUnsavedEpubChanges, hasUnsavedMakeChanges, hasUnsavedTextChanges } from "./unsavedChanges";

describe("unsaved change guards", () => {
  it("tracks text against its saved baseline", () => {
    expect(hasUnsavedTextChanges(false, "draft", "")).toBe(false);
    expect(hasUnsavedTextChanges(true, "draft", "source")).toBe(true);
    expect(hasUnsavedTextChanges(true, "source", "source")).toBe(false);
  });

  it("does not warn in reader mode but covers every EPUB dirty flag", () => {
    expect(hasUnsavedEpubChanges(true, true, true)).toBe(false);
    expect(hasUnsavedEpubChanges(false, true, false, true, false)).toBe(true);
    expect(hasUnsavedEpubChanges(false, false, true)).toBe(false);
  });

  it("keeps the make flow dirty until an export exists", () => {
    expect(hasUnsavedMakeChanges("", "")).toBe(false);
    expect(hasUnsavedMakeChanges("web-local:book.txt", "")).toBe(true);
    expect(hasUnsavedMakeChanges("web-local:book.txt", "book.epub")).toBe(false);
  });
});
