// @vitest-environment happy-dom

import { describe, expect, it } from "vitest";
import {
  closestBlock,
  collapseWhitespace,
  describePreviewClick,
  findStrippedMatch,
  stripWhitespace,
} from "./previewSync";

function makeDoc(html: string): Document {
  return new DOMParser().parseFromString(
    `<!doctype html><html><body>${html}</body></html>`,
    "text/html",
  );
}

describe("collapseWhitespace", () => {
  it("folds runs of whitespace and trims", () => {
    expect(collapseWhitespace("  一  二\n\t三  ")).toBe("一 二 三");
  });

  it("treats non-breaking space as whitespace", () => {
    expect(collapseWhitespace("一 二")).toBe("一 二");
  });
});

describe("stripWhitespace", () => {
  it("drops every whitespace character", () => {
    expect(stripWhitespace(" 一 \n 二\t三 ")).toBe("一二三");
  });

  /**
   * 匹配环节是定位的核心，这里只测纯函数。
   * Range 的构造依赖真实浏览器（happy-dom 的跨节点 Range 不可靠），
   * 那条路径用 headless 浏览器验证过。
   */
  describe("findStrippedMatch", () => {
    it("finds a plain substring", () => {
      expect(
        findStrippedMatch("沈棠看得好笑，柔声问：“小妹妹。”", "柔声问"),
      ).toEqual({ at: 7, length: 3 });
    });

    it("ignores whitespace differences on both sides", () => {
      // 编辑器里诗句常带换行与缩进，预览渲染成普通空白
      expect(
        findStrippedMatch("春眠不觉晓处处闻啼鸟", "春眠不觉晓\n  处处闻啼鸟"),
      ).toEqual({ at: 0, length: 10 });
    });

    it("returns the first occurrence for repeated text", () => {
      const text = stripWhitespace("第一处文本中间第一处文本");
      expect(findStrippedMatch(text, "第一处文本")).toEqual({
        at: 0,
        length: 5,
      });
    });

    it("returns null when absent", () => {
      expect(findStrippedMatch("天行剑宗", "并不存在的句子")).toBeNull();
    });

    it("returns null for a blank needle", () => {
      expect(findStrippedMatch("天行剑宗", "   ")).toBeNull();
    });
  });
});

describe("closestBlock", () => {
  it("skips inline wrappers", () => {
    const doc = makeDoc("<p>文字<span>内联</span></p>");
    const span = doc.querySelector("span")!;
    expect(closestBlock(span)?.tagName).toBe("P");
  });
});

describe("describePreviewClick", () => {
  it("extracts text and block context", () => {
    const doc = makeDoc("<p>天行剑宗，位于东域极北之地。</p>");
    const p = doc.querySelector("p")!;
    const payload = describePreviewClick(p);
    expect(payload).not.toBeNull();
    expect(payload!.text).toContain("天行剑宗");
    expect(payload!.context).toContain("东域极北之地");
  });

  it("ignores clicks on links", () => {
    const doc = makeDoc('<p><a href="#n1">注脚</a></p>');
    const link = doc.querySelector("a")!;
    expect(describePreviewClick(link)).toBeNull();
  });

  it("returns null when there is no text", () => {
    const doc = makeDoc("<p></p>");
    const p = doc.querySelector("p")!;
    expect(describePreviewClick(p)).toBeNull();
  });

  it("returns null for a null target", () => {
    expect(describePreviewClick(null)).toBeNull();
  });
});
