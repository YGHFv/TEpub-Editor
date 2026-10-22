import { afterEach, describe, expect, it } from "vitest";
import {
  canEvictKeepAlive,
  registerKeepAliveGuard,
  resetKeepAliveGuards,
  unregisterKeepAliveGuard,
} from "./dirtyGuard";

describe("keep-alive dirty guard", () => {
  afterEach(() => resetKeepAliveGuards());

  it("allows evicting pages that registered nothing", () => {
    expect(canEvictKeepAlive("/toolbox/image-tools")).toBe(true);
  });

  it("blocks eviction while the page reports unsaved work", () => {
    let dirty = true;
    registerKeepAliveGuard("/toolbox/epub-editor", () => dirty);
    expect(canEvictKeepAlive("/toolbox/epub-editor")).toBe(false);
    // 保存之后立刻允许淘汰，不必重新登记
    dirty = false;
    expect(canEvictKeepAlive("/toolbox/epub-editor")).toBe(true);
  });

  it("stops blocking after the page unmounts", () => {
    registerKeepAliveGuard("/toolbox/text-editor", () => true);
    expect(canEvictKeepAlive("/toolbox/text-editor")).toBe(false);
    unregisterKeepAliveGuard("/toolbox/text-editor");
    expect(canEvictKeepAlive("/toolbox/text-editor")).toBe(true);
  });

  /** 一个页面的判定抛错不能把整次导航拖死 */
  it("treats a throwing check as 'may evict'", () => {
    registerKeepAliveGuard("/toolbox/epub-editor", () => {
      throw new Error("boom");
    });
    expect(canEvictKeepAlive("/toolbox/epub-editor")).toBe(true);
  });

  it("keeps guards independent per tab id", () => {
    registerKeepAliveGuard("/reader?file=a.epub", () => true);
    expect(canEvictKeepAlive("/reader?file=a.epub")).toBe(false);
    expect(canEvictKeepAlive("/reader?file=b.epub")).toBe(true);
  });
});
