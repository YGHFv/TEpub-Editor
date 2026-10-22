// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  ObjectUrlRegistry,
  downloadBrowserBlob,
  triggerBrowserDownload,
  validateBrowserFiles,
} from "./webFileWorkflow";

describe("validateBrowserFiles", () => {
  it("accepts extensions case-insensitively and reports rejected files", () => {
    const epub = new File(["book"], "Book.EPUB", {
      type: "application/octet-stream",
    });
    const txt = new File(["text"], "note.txt", { type: "text/plain" });
    const result = validateBrowserFiles([epub, txt], {
      extensions: ["epub"],
      multiple: true,
    });
    expect(result.accepted).toEqual([epub]);
    expect(result.rejected).toEqual([txt]);
    expect(result.message).toContain("1");
  });

  it("limits single-file workflows", () => {
    const first = new File(["a"], "a.epub");
    const second = new File(["b"], "b.epub");
    const result = validateBrowserFiles([first, second], {
      extensions: [".epub"],
      multiple: false,
    });
    expect(result.accepted).toEqual([first]);
    expect(result.rejected).toEqual([second]);
    expect(result.message).toContain("一个文件");
  });
});

describe("ObjectUrlRegistry", () => {
  afterEach(() => vi.restoreAllMocks());

  it("revokes individual and remaining URLs exactly once", () => {
    const create = vi
      .spyOn(URL, "createObjectURL")
      .mockReturnValueOnce("blob:first")
      .mockReturnValueOnce("blob:second");
    const revoke = vi
      .spyOn(URL, "revokeObjectURL")
      .mockImplementation(() => undefined);
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

describe("downloadBrowserBlob", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  /**
   * 移动端两件事都不能退回：
   * 1. 锚点必须先挂进 DOM（游离节点上的 click() 在 Android WebView 里被忽略）；
   * 2. blob URL 不能在点击后立刻回收（下载是异步接管的）。
   * 桌面浏览器对这两点都宽容，所以很容易被"顺手简化"掉 —— 用测试钉住。
   */
  it("attaches the anchor to the DOM before clicking it", () => {
    const attached: (Node | null)[] = [];
    const click = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(function (this: HTMLAnchorElement) {
        attached.push(this.parentNode);
      });
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:x");
    vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => undefined);

    downloadBrowserBlob(new Blob(["a"]), "a.epub");

    expect(click).toHaveBeenCalledTimes(1);
    expect(attached[0]).toBe(document.body);
    // 点击后立刻从 DOM 摘掉，不留下隐藏节点
    expect(document.body.querySelector("a[download]")).toBeNull();
  });

  it("revokes the blob url only after the download has had time to start", () => {
    vi.useFakeTimers();
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      () => undefined,
    );
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:slow");
    const revoke = vi
      .spyOn(URL, "revokeObjectURL")
      .mockImplementation(() => undefined);

    downloadBrowserBlob(new Blob(["a"]), "a.epub");
    expect(revoke).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1_000);
    expect(revoke).not.toHaveBeenCalled();

    vi.advanceTimersByTime(30_000);
    expect(revoke).toHaveBeenCalledWith("blob:slow");
  });
});

describe("triggerBrowserDownload", () => {
  afterEach(() => vi.restoreAllMocks());

  it("sets href and download name on an anchor that is in the document", () => {
    let href = "";
    let name = "";
    let inDocument = false;
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      function (this: HTMLAnchorElement) {
        href = this.getAttribute("href") || "";
        name = this.getAttribute("download") || "";
        inDocument = document.body.contains(this);
      },
    );

    triggerBrowserDownload("blob:y", "chapter.xhtml");

    expect(href).toBe("blob:y");
    expect(name).toBe("chapter.xhtml");
    expect(inDocument).toBe(true);
  });
});
