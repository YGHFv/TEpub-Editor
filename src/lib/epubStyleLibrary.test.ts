// @vitest-environment happy-dom

import { describe, expect, it } from "vitest";
import {
  EPUB_ILLUSTRATION_STYLES,
  EPUB_STYLE_INTERFACE,
  EPUB_STYLE_MODULES,
  EPUB_TRANSITION_STYLES,
} from "$lib/epubStyleLibrary";

describe("EPUB style library", () => {
  it("exposes illustration and transition categories through the shared module list", () => {
    expect(EPUB_ILLUSTRATION_STYLES.length).toBeGreaterThanOrEqual(4);
    expect(EPUB_TRANSITION_STYLES.length).toBeGreaterThanOrEqual(5);
    expect(new Set(EPUB_STYLE_MODULES.map((style) => style.kind))).toEqual(
      new Set(["header", "title", "illustration", "transition"]),
    );
  });

  it("provides a script-free annotation illustration with EPUB3 and Yuewei-compatible footnote links", () => {
    const style = EPUB_ILLUSTRATION_STYLES.find((item) => item.id === "illustration-annotation-popup");
    expect(style?.target).toBe("annotation-illustration");
    expect(style?.css).toContain(".te-annotation-toggle:checked ~ .te-annotation-popup");
    expect(style?.markup).toContain('<input id="te-annotation-toggle-1"');
    expect(style?.markup).toContain('class="duokan-footnote"');
    expect(style?.markup).toContain('epub:type="noteref" role="doc-noteref" aria-controls="te-annotation-note-1">〔查看插图〕</a>');
    expect(style?.markup).toContain('href="#te-annotation-note-1"');
    expect(style?.markup).toContain('id="te-annotation-note-1" class="te-annotation-popup duokan-footnote-item" epub:type="footnote" role="doc-footnote"');
    expect(style?.markup).toContain('class="duokan-footnote-back"');
    expect(`${style?.css}${style?.markup}`).not.toMatch(/<script|javascript:/i);
  });

  it("opens and closes the annotation image by toggling its checkbox", () => {
    const style = EPUB_ILLUSTRATION_STYLES.find((item) => item.id === "illustration-annotation-popup")!;
    document.body.innerHTML = style.markup || "";
    const toggle = document.querySelector<HTMLInputElement>(".te-annotation-toggle")!;
    const trigger = document.querySelector<HTMLLabelElement>(".te-annotation-trigger")!;
    const backdrop = document.querySelector<HTMLLabelElement>(".te-annotation-backdrop")!;
    expect(toggle.checked).toBe(false);
    trigger.click();
    expect(toggle.checked).toBe(true);
    backdrop.click();
    expect(toggle.checked).toBe(false);
  });

  it("uses standalone SVG illustration samples instead of processed header artwork", () => {
    const centered = EPUB_ILLUSTRATION_STYLES.find((item) => item.id === "illustration-centered-caption");
    expect(centered?.previewHtml).toContain("data:image/svg+xml");
    expect(centered?.previewHtml).not.toMatch(/sample-character-gallery|cloud-gate-ink-banner|bottomFade/i);
  });

  it("includes the classic fg1 divider and marks transitions as isolated-ellipsis replacements", () => {
    const stars = EPUB_TRANSITION_STYLES.find((item) => item.id === "transition-fg1-stars");
    expect(stars?.markup).toBe('<p class="fg1">※※※</p>');
    expect(stars?.css).toContain("p.te-divider-line");
    expect(EPUB_TRANSITION_STYLES.every((style) => style.replacementMode === "isolated-ellipsis")).toBe(true);
  });

  it("offers text, pure CSS, and image transition structures", () => {
    expect(EPUB_TRANSITION_STYLES.some((style) => style.markup?.includes("te-transition-text"))).toBe(true);
    expect(EPUB_TRANSITION_STYLES.some((style) => style.css.includes("::before"))).toBe(true);
    expect(EPUB_TRANSITION_STYLES.some((style) => style.markup?.includes("te-divider-img"))).toBe(true);
    expect(EPUB_STYLE_INTERFACE.transition).toContain("te-divider-line");
    expect(EPUB_STYLE_INTERFACE.transitionImage).toContain("te-divider-img");
  });
});
