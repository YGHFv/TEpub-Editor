import { describe, expect, it } from "vitest";
import { i18nMarkupPreprocess } from "../../scripts/i18n-markup-preprocess.js";

describe("i18n markup preprocessor", () => {
  it("wraps static text and translatable attributes", () => {
    const content = `<script lang="ts"></script>
<h1 title="设置">书库</h1>
<pre>不要翻译</pre>`;
    const result = i18nMarkupPreprocess().markup({
      content,
      filename: "Demo.svelte",
    });

    expect(result?.code).toContain(
      'import { t as __tepubT } from "$lib/i18n.svelte";',
    );
    expect(result?.code).toContain('title={__tepubT("设置")}');
    expect(result?.code).toContain('{__tepubT("书库")}');
    expect(result?.code).toContain("<pre>不要翻译</pre>");
  });

  it("reuses an existing t import alias", () => {
    const content = `<script lang="ts">
import { t as translate } from "$lib/i18n.svelte";
</script>
<p>设置</p>`;
    const result = i18nMarkupPreprocess().markup({
      content,
      filename: "Demo.svelte",
    });

    expect(result?.code).toContain('{translate("设置")}');
    expect(result?.code).not.toContain("__tepubT");
  });

  it("translates static page titles in svelte:head", () => {
    const content = `<script lang="ts"></script>
<svelte:head><title>EPUB 结构诊断 - TEpub Editor</title></svelte:head>`;
    const result = i18nMarkupPreprocess().markup({
      content,
      filename: "Demo.svelte",
    });

    expect(result?.code).toContain(
      '<title>{__tepubT("EPUB 结构诊断 - TEpub Editor")}</title>',
    );
  });
});
