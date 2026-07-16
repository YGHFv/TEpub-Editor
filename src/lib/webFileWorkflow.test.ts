import { afterEach, describe, expect, it, vi } from "vitest";
import { ObjectUrlRegistry, validateBrowserFiles } from "./webFileWorkflow";

describe("validateBrowserFiles", () => {
  it("accepts extensions case-insensitively and reports rejected files", () => {
    const epub = new File(["book"], "Book.EPUB", { type: "application/octet-stream" });
    const txt = new File(["text"], "note.txt", { type: "text/plain" });
    const result = validateBrowserFiles([epub, txt], { extensions: ["epub"], multiple: true });
    expect(result.accepted).toEqual([epub]);
    expect(result.rejected).toEqual([txt]);
    expect(result.message).toContain("1");
  });

  it("limits single-file workflows", () => {
    const first = new File(["a"], "a.epub");
    const second = new File(["b"], "b.epub");
    const result = validateBrowserFiles([first, second], { extensions: [".epub"], multiple: false });
    expect(result.accepted).toEqual([first]);
    expect(result.rejected).toEqual([second]);
    expect(result.message).toContain("一个文件");
  });
});

describe("ObjectUrlRegistry", () => {
  afterEach(() => vi.restoreAllMocks());

  it("revokes individual and remaining URLs exactly once", () => {
    const create = vi.spyOn(URL, "createObjectURL")
      .mockReturnValueOnce("blob:first")
      .mockReturnValueOnce("blob:second");
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => undefined);
    const registry = new ObjectUrlRegistry();
    const first = registry.create(new Blob(["a"]));
    registry.create(new Blob(["b"]));
    registry.revoke(first);
    registry.revoke(first);
    registry.clear();
    expect(create).toHaveBeenCalledTimes(2);
    expect(revoke.mock.calls).toEqual([["blob:first"], ["blob:second"]]);
    expect(registry.size).toBe(0);
  });
});
