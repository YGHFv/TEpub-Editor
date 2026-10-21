// @vitest-environment happy-dom

/**
 * 这里只测"分支选择"这一层：happy-dom 没有真实排版，getBoundingClientRect
 * 恒为 0，量不出真实像素；但"哪条路径会拿 iframe 高度当输入"正是循环的
 * 根源，用桩对象就能覆盖住。
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { measureEpubContentHeight } from "./contentHeight";

interface StubOptions {
  /** body 的 CSS height（"100%" 用来制造"撑满视口"的书） */
  bodyHeightCss?: string;
  htmlHeightCss?: string;
  bodyRectHeight?: number;
  viewportHeight?: number;
  scrollHeight?: number;
  offsetHeight?: number;
  marginTop?: string;
  marginBottom?: string;
  paddingBottom?: string;
  children?: { bottom: number }[];
}

function makeBody(options: StubOptions = {}) {
  const {
    bodyHeightCss = "auto",
    htmlHeightCss = "auto",
    bodyRectHeight = 0,
    viewportHeight = 700,
    scrollHeight = 900,
    offsetHeight = 900,
    marginTop = "0px",
    marginBottom = "0px",
    paddingBottom = "0px",
    children = [],
  } = options;

  const docElement = { clientHeight: viewportHeight } as unknown as HTMLElement;

  const body = {
    ownerDocument: { documentElement: docElement },
    getBoundingClientRect: () => ({ height: bodyRectHeight }),
    scrollHeight,
    offsetHeight,
    children: children.map((child) => ({
      getBoundingClientRect: () => ({ bottom: child.bottom }),
    })),
  } as unknown as HTMLElement;

  vi.spyOn(globalThis, "getComputedStyle").mockImplementation((element) => {
    const isRoot = element === docElement;
    return {
      height: isRoot ? htmlHeightCss : bodyHeightCss,
      marginTop,
      marginBottom,
      paddingBottom,
    } as unknown as CSSStyleDeclaration;
  });
  return body;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("measureEpubContentHeight", () => {
  it("uses scrollHeight plus body margins for ordinary books", () => {
    const body = makeBody({
      scrollHeight: 1200,
      marginTop: "8px",
      marginBottom: "8px",
    });
    expect(measureEpubContentHeight(body)).toBe(1216);
  });

  it("never uses scrollHeight when body is stretched to the viewport", () => {
    // 这类书的 scrollHeight 会跟着 iframe 一起长，拿它当高度就是自持回路
    const body = makeBody({
      bodyHeightCss: "100%",
      bodyRectHeight: 700,
      viewportHeight: 700,
      scrollHeight: 5000,
      offsetHeight: 5000,
      children: [{ bottom: 1800 }, { bottom: 2400 }],
      marginBottom: "8px",
    });
    expect(measureEpubContentHeight(body)).toBe(2408);
  });

  it("detects html { height: 100% } even when body height is auto", () => {
    const body = makeBody({
      htmlHeightCss: "100%",
      bodyRectHeight: 700,
      viewportHeight: 700,
      scrollHeight: 9000,
      offsetHeight: 9000,
      children: [{ bottom: 1500 }],
    });
    expect(measureEpubContentHeight(body)).toBe(1500);
  });

  it("counts body padding and margin below the last child", () => {
    const body = makeBody({
      bodyHeightCss: "100%",
      bodyRectHeight: 700,
      viewportHeight: 700,
      children: [{ bottom: 1000 }],
      paddingBottom: "10px",
      marginBottom: "6px",
    });
    expect(measureEpubContentHeight(body)).toBe(1016);
  });

  it("falls back to scrollHeight when a stretched body has no measurable children", () => {
    const body = makeBody({
      bodyHeightCss: "100%",
      bodyRectHeight: 700,
      viewportHeight: 700,
      scrollHeight: 700,
      offsetHeight: 700,
      children: [],
    });
    expect(measureEpubContentHeight(body)).toBe(700);
  });

  it("keeps using scrollHeight when body is shorter than the viewport", () => {
    // 内容比视口短的书：body 没有被撑满，仍走常规路径
    const body = makeBody({
      bodyHeightCss: "100%",
      bodyRectHeight: 300,
      viewportHeight: 700,
      scrollHeight: 300,
      offsetHeight: 300,
    });
    expect(measureEpubContentHeight(body)).toBe(300);
  });
});
