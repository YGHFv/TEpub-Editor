export type EpubStyleKind = "header" | "title" | "illustration" | "transition";
export type EpubTitleLayout = "single" | "split";

export type EpubStyleTarget =
  | "header-image"
  | "intro-title"
  | "volume-title"
  | "chapter-title"
  | "illustration"
  | "annotation-illustration"
  | "transition";

export interface EpubStyleModule {
  /** 样式唯一 ID，保存到本地样式库后保持稳定。 */
  id: string;
  /** 样式大类：头图样式或标题样式。 */
  kind: EpubStyleKind;
  /** 样式应用目标，后续制作 EPUB 时用于匹配对应模块。 */
  target: EpubStyleTarget;
  /** 用户可见的样式名称。 */
  name: string;
  /** 样式用途说明，显示在样式列表和详情区。 */
  description: string;
  /** 该样式依赖的标准选择器接口。 */
  selectors: string[];
  /** 制作 EPUB 时如何套用该样式的说明。 */
  usage: string;
  /** 可直接写入 EPUB 的 CSS 内容。 */
  css: string;
  /** 样式库预览用的 XHTML 片段。 */
  previewHtml: string;
  /** 可复制到 EPUB 正文中的标准 XHTML 结构。 */
  markup?: string;
  /** 转场样式默认替换方式。 */
  replacementMode?: "isolated-ellipsis";
  /** 来源：内置样式或用户保存的样式。 */
  sourceKind?: "built-in" | "saved";
  /** 标题结构：single 为单行标题，split 为 number/name 双行标题。 */
  titleLayout?: EpubTitleLayout;
  /** 章节序号 number 部分的 CSS 声明，保存分段标题样式时使用。 */
  titleNumberCss?: string;
  /** 章节名称 name 部分的 CSS 声明，保存分段标题样式时使用。 */
  titleNameCss?: string;
  /** 旧版 A 段字段，仅用于兼容已经保存到 localStorage 的样式。 */
  titleCssA?: string;
  /** 旧版 B 段字段，仅用于兼容已经保存到 localStorage 的样式。 */
  titleCssB?: string;
  /** 头图样式生成后的透明样板图，既用于预览，也可作为后续制作时的遮罩来源。 */
  sampleDataUrl?: string;
  /** 兼容旧数据：单独保存的遮罩样板图。新样式默认只使用 sampleDataUrl。 */
  templateDataUrl?: string;
  /** 保存后的预览头图宽度。 */
  sampleWidth?: number;
  /** 保存后的预览头图高度。 */
  sampleHeight?: number;
  /** 导入样板图的原始宽度。 */
  originalSampleWidth?: number;
  /** 导入样板图的原始高度。 */
  originalSampleHeight?: number;
  /** 头图样式预览时绑定使用的章节标题样式 ID。 */
  boundTitleStyleId?: string;
}

export const EPUB_STYLE_INTERFACE = {
  headerFigure: ".te-header-figure",
  headerImage: ".te-header-image",
  headerCaption: ".te-header-caption",
  introTitle: ".te-intro-title",
  volumeTitle: ".te-volume-title",
  volumeSubtitle: ".te-volume-subtitle",
  chapterTitle: ".te-chapter-title",
  chapterNumber: ".te-chapter-number",
  chapterName: ".te-chapter-name",
  paragraph: "p.te-paragraph",
  illustrationFigure: ".te-illustration",
  illustrationImage: ".te-illustration-image",
  illustrationCaption: ".te-illustration-caption",
  annotationTrigger: ".te-annotation-trigger",
  annotationPopup: ".te-annotation-popup",
  transition: ".te-transition, p.te-divider-line, p.fg1",
  transitionText: ".te-transition-text",
  transitionImage: ".te-divider-image, .te-divider-img",
} as const;

export type EpubStyleInterfaceSlot = keyof typeof EPUB_STYLE_INTERFACE;

export const EPUB_STYLE_INTERFACE_NOTES: Record<EpubStyleInterfaceSlot, string> = {
  headerFigure: "章节头图容器，负责贴边、卡片、边距和遮罩等外层效果。",
  headerImage: "章节头图图片本体，制作 EPUB 时把生成好的头图放到这里。",
  headerCaption: "头图说明文字接口；默认不显示，只有样式明确需要说明时再开启。",
  introTitle: "简介、前言、后记等元信息页标题接口，后续会单独放到元信息样式列表。",
  volumeTitle: "卷首页标题接口，后续会单独放到卷标题样式列表。",
  volumeSubtitle: "卷首页副标题接口，和卷标题样式一起管理。",
  chapterTitle: "章节标题外层接口，单行标题直接套用到这里，双行标题作为 number/name 的容器。",
  chapterNumber: "双行章节标题的章节序号部分，例如“第三章”。",
  chapterName: "双行章节标题的章节名称部分，例如“计划不如变化”。",
  paragraph: "正文段落接口，预览用来观察标题和头图与正文的间距。",
  illustrationFigure: "正文插图容器，控制居中、出血、卡片和上下间距。",
  illustrationImage: "正文插图图片本体，实际制作时替换 src 和 alt。",
  illustrationCaption: "插图说明文字；没有图注时可省略 figcaption。",
  annotationTrigger: "正文中的注释或文字触发入口，链接到对应弹出插图 ID。",
  annotationPopup: "注释插图弹层，使用 CSS :target 实现无脚本打开和关闭。",
  transition: "章节内转场容器；制作时默认替换前后都有正文的单段孤立省略号。",
  transitionText: "带文字的转场内容，例如“与此同时”或时间地点提示。",
  transitionImage: "图片转场容器与图片本体，对应制作模板的 dividerImage 资源位。",
};

const sampleParagraphs = `
  <p class="te-paragraph">夜色沉入城市边缘，风从旧站台吹过，带着潮湿的铁锈味。</p>
  <p class="te-paragraph">她合上手中的书，抬头看见远处灯塔亮起，像一枚缓慢落下的星。</p>
`;

export const EPUB_HEADER_PREVIEW_TITLE_CSS = `.te-chapter-title {
  font-family: "llf", "黑体", sans-serif;
  text-align: center;
  font-weight: 900;
  font-size: 0.8em;
  margin: 1em 0 3em;
  color: #413245;
  line-height: 1.3;
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-chapter-number {
  display: block;
  color: #413245;
}

.te-chapter-name {
  display: block;
  font-family: "llf", "黑体", sans-serif;
  font-size: 1.2em;
  font-weight: 900;
  color: #c2181e;
}`;

function splitTitleMarkup() {
  return `
    <h3 class="te-chapter-title">
      <span class="te-chapter-number">第十二章</span>
      <span class="te-chapter-name">灯塔来信</span>
    </h3>
  `;
}

function titlePreview(titleMarkup = splitTitleMarkup()) {
  return `
    <main class="te-preview-page">
      ${titleMarkup}
      ${sampleParagraphs}
    </main>
  `;
}

const fixedPreviewHeader = new URL("./assets/epub-style-library/fixed-preview-header.png", import.meta.url).href;

const headerTemplateSamples = {
  bottomFade: new URL("./assets/epub-style-library/sample-character-gallery.png", import.meta.url).href,
  tornEdge: new URL("./assets/epub-style-library/sample-sword-duel.png", import.meta.url).href,
  scatterEdge: new URL("./assets/epub-style-library/sample-harbor-studio.png", import.meta.url).href,
  inkEdge: new URL("./assets/epub-style-library/sample-night-guard.png", import.meta.url).href,
  diagonalBrush: new URL("./assets/epub-style-library/sample-court-lineup.png", import.meta.url).href,
  rightMemoryCollage: new URL("./assets/epub-style-library/sample-right-memory-collage.png", import.meta.url).href,
  deliveryBikeCollage: new URL("./assets/epub-style-library/sample-delivery-bike-collage.png", import.meta.url).href,
  deliveryBikeCollageTemplate: new URL("./assets/epub-style-library/template-delivery-bike-collage.png", import.meta.url).href,
  cloudGateInkBanner: new URL("./assets/epub-style-library/sample-cloud-gate-ink-banner.png", import.meta.url).href,
  cloudGateInkBannerTemplate: new URL("./assets/epub-style-library/template-cloud-gate-ink-banner.png", import.meta.url).href,
} as const;

function svgDataUrl(svg: string) {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

const rightMemoryCollageMask = svgDataUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 720">
  <defs>
    <filter id="soft-edge" x="-8%" y="-8%" width="116%" height="116%">
      <feGaussianBlur stdDeviation="1.1"/>
    </filter>
  </defs>
  <g fill="#fff">
    <path filter="url(#soft-edge)" d="M522 90c63-36 169-42 274-30 139 16 235 65 277 145 38 72 27 170-24 247-48 73-134 124-250 144-120 21-255 5-335-39-72-39-105-96-96-163 9-69 57-111 82-164 20-42 19-102 72-140z"/>
    <path d="M501 104c76-30 243-35 372-5 102 24 170 74 202 140 36 75 17 178-48 247-62 66-169 104-300 105-113 1-229-25-297-76-54-41-75-96-52-158 20-55 67-87 91-137 18-38 1-90 32-116z"/>
    <path d="M455 152c44-28 80-51 123-55-23 31-14 59-40 81-31 26-75 17-121 43 14-27 14-53 38-69z"/>
    <path d="M448 500c38 37 98 65 177 83-78 5-145-8-196-35-41-22-67-54-79-93 37 11 67 18 98 45z"/>
    <path d="M933 86c86 14 141 48 176 99-44-11-67 3-102-19-37-23-39-53-74-80z"/>
    <path d="M901 579c83-22 127-59 163-118-6 62-49 115-120 146-48 21-109 31-172 28 42-16 82-43 129-56z"/>
    <path d="M356 133l58 12-46 38-51-31z"/>
    <path d="M372 208l76 8-50 58-64-18z"/>
    <path d="M287 244l61 30-61 31-45-39z"/>
    <path d="M368 336l92 22-80 61-72-37z"/>
    <path d="M290 424l73 5-45 54-62-20z"/>
    <path d="M409 72l44 27-38 25-43-17z"/>
    <path d="M241 184l44 14-26 38-50-12z"/>
    <path d="M312 548l79-13-32 62-69-3z"/>
    <ellipse cx="540" cy="613" rx="42" ry="18" opacity=".82"/>
    <ellipse cx="641" cy="627" rx="55" ry="15" opacity=".72"/>
    <ellipse cx="797" cy="632" rx="64" ry="13" opacity=".64"/>
  </g>
  <g fill="#fff" opacity=".86">
    <circle cx="454" cy="104" r="8"/><circle cx="475" cy="78" r="4"/><circle cx="496" cy="66" r="6"/>
    <circle cx="529" cy="57" r="5"/><circle cx="574" cy="43" r="7"/><circle cx="617" cy="38" r="4"/>
    <circle cx="681" cy="36" r="6"/><circle cx="744" cy="43" r="5"/><circle cx="815" cy="53" r="8"/>
    <circle cx="884" cy="65" r="5"/><circle cx="972" cy="91" r="7"/><circle cx="1035" cy="133" r="6"/>
    <circle cx="1101" cy="236" r="9"/><circle cx="1127" cy="310" r="5"/><circle cx="1112" cy="390" r="7"/>
    <circle cx="1083" cy="462" r="9"/><circle cx="1036" cy="526" r="6"/><circle cx="954" cy="586" r="8"/>
    <circle cx="862" cy="624" r="5"/><circle cx="752" cy="646" r="7"/><circle cx="649" cy="648" r="5"/>
    <circle cx="543" cy="631" r="8"/><circle cx="438" cy="586" r="6"/><circle cx="362" cy="512" r="5"/>
    <circle cx="334" cy="440" r="8"/><circle cx="339" cy="355" r="5"/><circle cx="379" cy="270" r="7"/>
  </g>
  <g fill="#fff" opacity=".58">
    <circle cx="405" cy="129" r="3"/><circle cx="386" cy="159" r="2"/><circle cx="347" cy="190" r="4"/>
    <circle cx="310" cy="210" r="2"/><circle cx="276" cy="239" r="3"/><circle cx="224" cy="260" r="4"/>
    <circle cx="292" cy="318" r="3"/><circle cx="335" cy="319" r="2"/><circle cx="382" cy="314" r="3"/>
    <circle cx="274" cy="397" r="4"/><circle cx="328" cy="407" r="2"/><circle cx="389" cy="453" r="3"/>
    <circle cx="237" cy="474" r="3"/><circle cx="301" cy="508" r="2"/><circle cx="377" cy="564" r="4"/>
    <circle cx="472" cy="613" r="2"/><circle cx="584" cy="656" r="3"/><circle cx="703" cy="668" r="2"/>
    <circle cx="831" cy="655" r="3"/><circle cx="937" cy="622" r="2"/><circle cx="1048" cy="563" r="4"/>
    <circle cx="1126" cy="469" r="3"/><circle cx="1155" cy="341" r="4"/><circle cx="1131" cy="205" r="3"/>
  </g>
</svg>`);

const headerEdgeCss = `.te-header-figure {
  margin: 0 -1.5em 1.6em;
  padding: 0;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  duokan-bleed: lefttopright;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`;

function headerPreview(imageSrc = fixedPreviewHeader) {
  const image = imageSrc
    ? `<img class="te-header-image" src="${imageSrc}" alt="" />`
    : `<div class="te-header-image te-header-placeholder"></div>`;
  return `
    <main class="te-preview-page te-preview-header-page">
      <figure class="te-header-figure" aria-label="头图预览">
        ${image}
      </figure>
      ${splitTitleMarkup()}
      ${sampleParagraphs}
    </main>
  `;
}

function headerTemplateStyle(
  id: string,
  name: string,
  description: string,
  usage: string,
  sampleDataUrl: string,
  sampleWidth: number,
  sampleHeight: number,
  originalSampleWidth: number,
  originalSampleHeight: number,
): EpubStyleModule {
  return {
    id,
    kind: "header",
    target: "header-image",
    name,
    description,
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage,
    css: headerEdgeCss,
    previewHtml: headerPreview(sampleDataUrl),
    sampleDataUrl,
    sampleWidth,
    sampleHeight,
    originalSampleWidth,
    originalSampleHeight,
  };
}

export const EPUB_HEADER_STYLES: EpubStyleModule[] = [
  {
    id: "header-standard-edge",
    kind: "header",
    target: "header-image",
    name: "贴边头图",
    description: "图片贴住页面上、左、右边缘，标题和正文从下方开始。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "制作 EPUB 时把生成好的头图放入 .te-header-figure > img.te-header-image，图片贴页面上沿显示。",
    css: headerEdgeCss,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
  },
  headerTemplateStyle(
    "header-template-bottom-fade",
    "底部渐隐贴边头图",
    "底部透明渐隐，适合让用户上传的头图自然过渡到正文留白。",
    "使用该样板的透明度裁出底部渐隐效果，用户上传的头图内容本身不会被样板图覆盖。",
    headerTemplateSamples.bottomFade,
    1080,
    750,
    1080,
    750,
  ),
  headerTemplateStyle(
    "header-template-torn-edge",
    "底部撕边贴边头图",
    "底部不规则透明撕边，适合有动作感或场景切换感的章节头图。",
    "使用该样板的透明度裁出不规则底边，制作时只保留用户头图经过遮罩后的结果。",
    headerTemplateSamples.tornEdge,
    1080,
    784,
    860,
    624,
  ),
  headerTemplateStyle(
    "header-template-scatter-edge",
    "底部散点贴边头图",
    "底部散点透明过渡，适合较柔和的横向头图样式。",
    "使用该样板的透明度裁出底部散点，样板图只作为形状模板使用。",
    headerTemplateSamples.scatterEdge,
    1080,
    608,
    826,
    465,
  ),
  headerTemplateStyle(
    "header-template-ink-edge",
    "底部墨痕贴边头图",
    "底部墨痕状透明留白，适合边缘更硬朗的章节头图。",
    "使用该样板的透明度裁出墨痕底边，用户上传头图不会叠加样板图内容。",
    headerTemplateSamples.inkEdge,
    1080,
    608,
    1280,
    720,
  ),
  headerTemplateStyle(
    "header-template-diagonal-brush",
    "斜向笔刷贴边头图",
    "斜向笔刷透明边缘，适合需要强烈斜切构图的头图样式。",
    "使用该样板的透明度裁出斜向笔刷边缘，保存的是遮罩后的头图结果。",
    headerTemplateSamples.diagonalBrush,
    1080,
    608,
    1280,
    720,
  ),
  {
    id: "header-template-right-memory-collage",
    kind: "header",
    target: "header-image",
    name: "右侧散边留白头图",
    description: "主体集中在右侧，左侧保留大片留白，并带碎片、颗粒和旧照散边效果。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "使用透明样板把用户头图裁成右侧旧照散边形状，适合年代感、群像、生活流或回忆章节。",
    css: `.te-header-figure {
  margin: 0 -1.5em 1.65em;
  padding: 0;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  duokan-bleed: lefttopright;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(headerTemplateSamples.rightMemoryCollage),
    sampleDataUrl: headerTemplateSamples.rightMemoryCollage,
    templateDataUrl: rightMemoryCollageMask,
    sampleWidth: 1080,
    sampleHeight: 664,
    originalSampleWidth: 1080,
    originalSampleHeight: 664,
    boundTitleStyleId: "title-cinematic-slab",
  },
  {
    ...headerTemplateStyle(
      "header-template-delivery-bike-collage",
      "左叠拼片散边头图",
      "左侧拼片式透明轮廓，主体向中下部展开，适合需要留白和碎片感的章节头图。",
      "使用拼片样板的 alpha 作为蒙版裁切用户头图；内置预览图只是示例填充，制作 EPUB 时会换成用户选择的头图内容。",
      headerTemplateSamples.deliveryBikeCollage,
      1080,
      810,
      1000,
      750,
    ),
    templateDataUrl: headerTemplateSamples.deliveryBikeCollageTemplate,
    boundTitleStyleId: "title-soft-magazine",
  },
  {
    ...headerTemplateStyle(
      "header-template-cloud-gate-ink-banner",
      "墨染横幅散边头图",
      "横幅式透明蒙版，四周带墨染散边，适合需要大场景铺陈和卷首仪式感的章节头图。",
      "使用墨染边缘样板的透明度裁切用户头图；模板本身只负责形状，生成时不会强制保留示例图内容。",
      headerTemplateSamples.cloudGateInkBanner,
      1080,
      650,
      1080,
      650,
    ),
    templateDataUrl: headerTemplateSamples.cloudGateInkBannerTemplate,
    boundTitleStyleId: "title-scroll-border",
  },
  {
    id: "header-card-shadow",
    kind: "header",
    target: "header-image",
    name: "卡片头图",
    description: "保留留白和轻阴影，适合不希望图片铺满页面的章节。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "头图仍使用 .te-header-image，模块只改变外框、圆角和间距。",
    css: `.te-header-figure {
  margin: 1.2em auto 1.8em;
  padding: 0.35em;
  width: 92%;
  border: 1px solid #d9c7a2;
  background: #fffaf0;
  box-shadow: 0 0.45em 1.4em rgba(88, 64, 34, 0.18);
  box-sizing: border-box;
}

.te-header-image {
  display: block;
  width: 100%;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
  },
  {
    id: "header-dark-vignette-edge",
    kind: "header",
    target: "header-image",
    name: "暗角遮罩贴边头图",
    description: "贴边显示并在头图底部增加暗色压底，标题仍按所选标题样式单独渲染。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "章节 XHTML 中先输出 .te-header-figure，再输出章节标题；该样式只处理头图本身。",
    css: `.te-header-figure {
  margin: 0 -1.5em 1.2em;
  min-height: 14em;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  duokan-bleed: lefttopright;
  background: #172033;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: 14em;
  object-fit: cover;
  opacity: 0.9;
}

.te-header-figure::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(to bottom, rgba(23,32,51,0.08), rgba(23,32,51,0.72));
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
  },
  {
    id: "header-fine-frame-edge",
    kind: "header",
    target: "header-image",
    name: "细线装帧头图",
    description: "图片贴近页顶，底部以细线和留白收束，适合文艺、悬疑、现代题材。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "章节开头先输出 .te-header-figure，图片会以装帧细线收束，标题在下方独立排版。",
    css: `.te-header-figure {
  margin: 0 -1.2em 1.9em;
  padding: 0 0 0.42em;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  border-bottom: 1px solid #243447;
  background: #f8fafc;
}

.te-header-figure::after {
  content: "";
  position: absolute;
  left: 16%;
  right: 16%;
  bottom: 0.18em;
  border-bottom: 1px solid #c8a65a;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
    boundTitleStyleId: "title-cinematic-slab",
  },
  {
    id: "header-floating-print",
    kind: "header",
    target: "header-image",
    name: "浮印留白头图",
    description: "保留四周留白和轻微投影，像插页版画，适合古风、奇幻和人物向章节。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "头图不铺满页面，使用 .te-header-figure 的留白与边线形成插页感。",
    css: `.te-header-figure {
  margin: 1.1em auto 2em;
  padding: 0.28em;
  width: 90%;
  box-sizing: border-box;
  border: 1px solid #d7c7a7;
  background: #fffdf7;
  box-shadow: 0 0.35em 1em rgba(34, 25, 18, 0.14);
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-header-image {
  display: block;
  width: 100%;
  height: auto;
  object-fit: cover;
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
    boundTitleStyleId: "title-scroll-border",
  },
  {
    id: "header-cinematic-crop",
    kind: "header",
    target: "header-image",
    name: "电影裁幅头图",
    description: "固定横幅高度，顶部贴边并加深色压边，适合动作、悬疑、科幻章节。",
    selectors: [
      "body.te-chapter-page",
      EPUB_STYLE_INTERFACE.headerFigure,
      EPUB_STYLE_INTERFACE.headerImage,
      EPUB_STYLE_INTERFACE.headerCaption,
    ],
    usage: "将章节头图裁成宽银幕感横幅，标题仍使用绑定标题样式在图下显示。",
    css: `.te-header-figure {
  margin: 0 -1.5em 1.45em;
  height: 13.2em;
  position: relative;
  overflow: hidden;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  background: #0f172a;
}

.te-header-image {
  display: block;
  width: 100%;
  max-width: none;
  height: 13.2em;
  object-fit: cover;
}

.te-header-figure::before,
.te-header-figure::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 1.1em;
  background: rgba(15, 23, 42, 0.82);
}

.te-header-figure::before {
  top: 0;
}

.te-header-figure::after {
  bottom: 0;
}

.te-header-caption {
  display: none;
}`,
    previewHtml: headerPreview(fixedPreviewHeader),
    sampleDataUrl: fixedPreviewHeader,
    sampleWidth: 1920,
    sampleHeight: 1080,
    originalSampleWidth: 1920,
    originalSampleHeight: 1080,
    boundTitleStyleId: "title-night-card",
  },
];

export const EPUB_TITLE_STYLES: EpubStyleModule[] = [
  {
    id: "title-classic-red",
    kind: "title",
    target: "chapter-title",
    name: "经典红章",
    description: "居中双行标题，章节序号偏暗，章节名使用红色强调。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "自动套用到双行章节标题：.te-chapter-title / .te-chapter-number / .te-chapter-name。",
    css: `.te-chapter-title {
  font-family: "Microsoft YaHei", sans-serif;
  text-align: center;
  margin: 2em 0 3em;
  font-size: 1.2em;
  font-weight: 900;
  color: #c2181e;
}

.te-chapter-number {
  display: block;
  color: #413245;
  font-size: 0.82em;
  line-height: 1.35;
}

.te-chapter-name {
  display: block;
  color: #c2181e;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `color: #413245;
font-size: 0.82em;
line-height: 1.35;`,
    titleNameCss: `color: #c2181e;`,
  },
  {
    id: "title-ink-line",
    kind: "title",
    target: "chapter-title",
    name: "墨线章题",
    description: "黑白细线分隔，适合正文密集、低装饰的作品。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "自动套用到双行章节标题，章节序号和章节名使用标准 span。",
    css: `.te-chapter-title {
  margin: 2.4em auto 2.8em;
  padding: 0.9em 0;
  width: 82%;
  border-top: 1px solid #1f2937;
  border-bottom: 1px solid #1f2937;
  color: #111827;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: center;
  text-indent: 0;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.45em;
  color: #6b7280;
  font-size: 0.78em;
}

.te-chapter-name {
  display: block;
  font-size: 1.18em;
  font-weight: 700;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.45em;
color: #6b7280;
font-size: 0.78em;`,
    titleNameCss: `font-size: 1.18em;
font-weight: 700;`,
  },
  {
    id: "title-purple-red-emphasis",
    kind: "title",
    target: "chapter-title",
    name: "暗紫红重点章题",
    description: "居中双行标题，章节序号用暗紫色，章节名用红色强调。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "由 h2.texthE1 / b 转换为标准章节标题接口，自动套用到双行章题。",
    css: EPUB_HEADER_PREVIEW_TITLE_CSS,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `font-family: "llf", "黑体", sans-serif;
font-weight: 900;
font-size: 0.8em;
color: #413245;
line-height: 1.3;`,
    titleNameCss: `font-family: "llf", "黑体", sans-serif;
font-size: 1.2em;
font-weight: 900;
color: #c2181e;`,
  },
  {
    id: "title-teal-left-heading",
    kind: "title",
    target: "chapter-title",
    name: "青蓝左齐章题",
    description: "左对齐双行标题，章节序号为黑色小字，章节名为青蓝色。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "由 h2.head / h2.head span 转换为标准章节标题接口，适合低装饰章节开头。",
    css: `.te-chapter-title {
  font-size: 1.1em;
  line-height: 1.2;
  font-weight: bold;
  text-align: left;
  font-family: "st", "黑体", "ht", sans-serif;
  margin: 1em 0;
  padding-bottom: 1em;
  text-indent: 0;
  duokan-text-indent: 0;
  color: #02586d;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.35em;
  font-family: "st", "黑体", "ht", sans-serif;
  font-size: 0.8em;
  color: #000;
  font-weight: bold;
}

.te-chapter-name {
  display: block;
  color: #02586d;
  font-weight: bold;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `font-family: "st", "黑体", "ht", sans-serif;
font-size: 0.8em;
color: #000;
font-weight: bold;`,
    titleNameCss: `font-family: "st", "黑体", "ht", sans-serif;
font-size: 1.1em;
line-height: 1.2;
font-weight: bold;
text-align: left;
color: #02586d;`,
  },
  {
    id: "title-vermilion-center",
    kind: "title",
    target: "chapter-title",
    name: "朱红居中章题",
    description: "居中双行标题，序号较小，标题名使用朱红色强调。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "由 h2.chapter-title / .chapter-number 转换为标准章节标题接口，自动套用到双行章题。",
    css: `.te-chapter-title {
  margin-top: 0;
  margin-bottom: 1.5em;
  color: #ab1d22;
  font-size: 1.2em;
  line-height: 1.3;
  text-align: center;
  font-family: "zdy1";
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-chapter-number {
  display: block;
  font-size: 0.7em;
  color: #ab1d22;
  font-family: "zdy3";
  font-weight: 500;
}

.te-chapter-name {
  display: block;
  color: #ab1d22;
  font-family: "zdy1";
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `font-size: 0.7em;
color: #ab1d22;
font-family: "zdy3";
font-weight: 500;`,
    titleNameCss: `color: #ab1d22;
font-family: "zdy1";`,
  },
  {
    id: "title-teal-number-badge",
    kind: "title",
    target: "chapter-title",
    name: "青蓝编号章题",
    description: "左对齐双行标题，章节序号使用青蓝底色编号块。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "由 h2.head / span.num 转换为标准章节标题接口，编号块自动套用到 number。",
    css: `.te-chapter-title {
  font-size: 1em;
  color: #0d335d;
  text-align: left;
  line-height: 1.3;
  padding: 0 4px;
  margin: 0 0 2em;
  font-family: "zdy5";
  text-indent: 0;
  duokan-text-indent: 0;
}

.te-chapter-number {
  display: inline-block;
  margin-bottom: 0.6em;
  font-family: "zdy5";
  padding: 0.5px 2px;
  color: #ffffff;
  font-size: x-small;
  background-color: #1d5a6c;
  border-radius: 0;
  border: 1px solid #684c7f;
}

.te-chapter-name {
  display: block;
  color: #0d335d;
  font-family: "zdy5";
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `font-family: "zdy5";
padding: 0.5px 2px;
color: #ffffff;
font-size: x-small;
background-color: #1d5a6c;
border-radius: 0;
border: 1px solid #684c7f;`,
    titleNameCss: `color: #0d335d;
font-family: "zdy5";`,
  },
  {
    id: "title-cinematic-slab",
    kind: "title",
    target: "chapter-title",
    name: "电影字幕章题",
    description: "深蓝主标题配金色序号线，克制、有画面感，适合悬疑、都市、科幻。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "使用标准双行章节标题结构，序号作为上方小标题，章名作为醒目的主标题。",
    css: `.te-chapter-title {
  width: 82%;
  margin: 1.5em auto 2.6em;
  padding: 0.75em 0 0.8em;
  border-top: 2px solid #1f2937;
  border-bottom: 1px solid #c8a65a;
  color: #172033;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: center;
  text-indent: 0;
  line-height: 1.25;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.48em;
  color: #b8860b;
  font-size: 0.72em;
  font-weight: 700;
}

.te-chapter-name {
  display: block;
  color: #172033;
  font-size: 1.26em;
  font-weight: 900;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.48em;
color: #b8860b;
font-size: 0.72em;
font-weight: 700;`,
    titleNameCss: `color: #172033;
font-size: 1.26em;
font-weight: 900;`,
  },
  {
    id: "title-scroll-border",
    kind: "title",
    target: "chapter-title",
    name: "书卷双线章题",
    description: "暖色双线和居中标题，像纸本装帧页，适合古风、奇幻、文学向作品。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "双行标题居中显示，序号淡化，章名以深墨色突出。",
    css: `.te-chapter-title {
  width: 78%;
  margin: 2em auto 2.7em;
  padding: 0.85em 0.3em;
  border-top: 1px solid #b58b52;
  border-bottom: 1px solid #b58b52;
  background: #fffaf0;
  color: #2b2118;
  font-family: "Title", "Microsoft YaHei", serif;
  text-align: center;
  text-indent: 0;
  line-height: 1.28;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.5em;
  color: #8a6a3d;
  font-size: 0.72em;
  font-weight: 600;
}

.te-chapter-name {
  display: block;
  color: #2b2118;
  font-size: 1.18em;
  font-weight: 800;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.5em;
color: #8a6a3d;
font-size: 0.72em;
font-weight: 600;`,
    titleNameCss: `color: #2b2118;
font-size: 1.18em;
font-weight: 800;`,
  },
  {
    id: "title-seal-left",
    kind: "title",
    target: "chapter-title",
    name: "朱印左标题",
    description: "左对齐章名搭配朱色印章式序号，适合武侠、历史、东方幻想。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "序号作为小印章，章名左对齐显示，适合章节开头信息密度较高的排版。",
    css: `.te-chapter-title {
  margin: 1.6em 0 2.4em;
  padding-left: 0.3em;
  color: #222222;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: left;
  text-indent: 0;
  line-height: 1.35;
}

.te-chapter-number {
  display: inline-block;
  margin-bottom: 0.55em;
  padding: 0.22em 0.45em;
  border: 1px solid #a32020;
  background: #a32020;
  color: #fffdf7;
  font-size: 0.72em;
  font-weight: 800;
}

.te-chapter-name {
  display: block;
  border-left: 4px solid #a32020;
  padding-left: 0.65em;
  color: #222222;
  font-size: 1.18em;
  font-weight: 900;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `padding: 0.22em 0.45em;
border: 1px solid #a32020;
background: #a32020;
color: #fffdf7;
font-size: 0.72em;
font-weight: 800;`,
    titleNameCss: `border-left: 4px solid #a32020;
padding-left: 0.65em;
color: #222222;
font-size: 1.18em;
font-weight: 900;`,
  },
  {
    id: "title-soft-magazine",
    kind: "title",
    target: "chapter-title",
    name: "清爽杂志章题",
    description: "浅青分隔与大留白，阅读感轻，适合现代、治愈、日常题材。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "双行标题居中，序号用小号灰色，标题用青蓝强调。",
    css: `.te-chapter-title {
  width: 86%;
  margin: 1.8em auto 2.8em;
  padding: 0 0 0.9em;
  border-bottom: 3px double #8bc5c1;
  color: #165a64;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: center;
  text-indent: 0;
  line-height: 1.32;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.55em;
  color: #64748b;
  font-size: 0.72em;
  font-weight: 600;
}

.te-chapter-name {
  display: block;
  color: #165a64;
  font-size: 1.16em;
  font-weight: 800;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.55em;
color: #64748b;
font-size: 0.72em;
font-weight: 600;`,
    titleNameCss: `color: #165a64;
font-size: 1.16em;
font-weight: 800;`,
  },
  {
    id: "title-night-card",
    kind: "title",
    target: "chapter-title",
    name: "夜色章卡",
    description: "深色标题卡片与亮色序号，适合紧张、悬疑、赛博或暗色题材。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "标题整体作为深色章卡显示，和电影裁幅头图搭配使用效果更强。",
    css: `.te-chapter-title {
  width: 84%;
  margin: 1.6em auto 2.6em;
  padding: 0.9em 1em;
  box-sizing: border-box;
  background: #172033;
  color: #f8fafc;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: left;
  text-indent: 0;
  line-height: 1.35;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.48em;
  color: #f5c542;
  font-size: 0.72em;
  font-weight: 800;
}

.te-chapter-name {
  display: block;
  color: #f8fafc;
  font-size: 1.16em;
  font-weight: 900;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.48em;
color: #f5c542;
font-size: 0.72em;
font-weight: 800;`,
    titleNameCss: `color: #f8fafc;
font-size: 1.16em;
font-weight: 900;`,
  },
  {
    id: "title-minimal-quiet",
    kind: "title",
    target: "chapter-title",
    name: "素净留白章题",
    description: "极简居中标题，低装饰、强留白，适合长篇阅读和正文密集作品。",
    selectors: [
      EPUB_STYLE_INTERFACE.chapterTitle,
      EPUB_STYLE_INTERFACE.chapterNumber,
      EPUB_STYLE_INTERFACE.chapterName,
    ],
    usage: "序号和章名均居中，使用细线留出呼吸感，不干扰正文阅读。",
    css: `.te-chapter-title {
  margin: 2.6em auto 3.2em;
  padding-bottom: 0.9em;
  width: 70%;
  border-bottom: 1px solid #d0d7de;
  color: #1f2937;
  font-family: "Title", "Microsoft YaHei", sans-serif;
  text-align: center;
  text-indent: 0;
  line-height: 1.32;
}

.te-chapter-number {
  display: block;
  margin-bottom: 0.45em;
  color: #94a3b8;
  font-size: 0.7em;
  font-weight: 600;
}

.te-chapter-name {
  display: block;
  color: #1f2937;
  font-size: 1.08em;
  font-weight: 700;
}`,
    previewHtml: titlePreview(),
    titleLayout: "split",
    titleNumberCss: `margin-bottom: 0.45em;
color: #94a3b8;
font-size: 0.7em;
font-weight: 600;`,
    titleNameCss: `color: #1f2937;
font-size: 1.08em;
font-weight: 700;`,
  },
];

const illustrationPreviewImage = svgDataUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 900 600">
  <rect width="900" height="600" fill="#dcecf4"/>
  <rect y="365" width="900" height="235" fill="#8eb5a2"/>
  <circle cx="720" cy="120" r="54" fill="#fff4c7"/>
  <path d="M0 390L170 220l120 110 145-190 180 210 130-120 155 160v210H0z" fill="#5f7f78"/>
  <path d="M0 430c160-48 250-24 390 18 140 41 269 39 510-14v166H0z" fill="#b7d5c5"/>
  <path d="M110 470h680v18H110z" fill="#51483e"/>
  <path d="M180 398h210v72H180z" fill="#9b5e46"/>
  <path d="M160 398l125-82 125 82z" fill="#6e3f35"/>
  <rect x="215" y="422" width="44" height="48" fill="#f6d787"/>
  <rect x="302" y="422" width="44" height="48" fill="#f6d787"/>
  <path d="M545 488c24-80 72-118 142-118s118 38 142 118" fill="none" stroke="#fffdf7" stroke-width="14"/>
</svg>`);
const illustrationWidePreviewImage = svgDataUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 520">
  <rect width="1200" height="520" fill="#cadde9"/>
  <circle cx="940" cy="118" r="62" fill="#fff1b8"/>
  <path d="M0 350L165 210l125 90 170-188 168 196 135-118 177 142 130-95 120 113v170H0z" fill="#6f8792"/>
  <path d="M0 392c196-55 342-31 520 18 170 47 362 48 680-8v118H0z" fill="#8db5aa"/>
  <path d="M0 455h1200v65H0z" fill="#547a70"/>
  <path d="M90 420h1020" stroke="#f6edd8" stroke-width="8" stroke-dasharray="24 16"/>
</svg>`);
const transitionDividerPreview = svgDataUrl(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 90">
  <g fill="none" stroke="#8a6a3d" stroke-width="2">
    <path d="M35 45h205M360 45h205"/>
    <path d="M262 45l18-18 18 18-18 18zM302 45l18-18 18 18-18 18z"/>
  </g>
  <circle cx="300" cy="45" r="5" fill="#a32020"/>
</svg>`);

function illustrationPreview(markup: string) {
  return `<main class="te-preview-page">
    <p class="te-paragraph">夜色沉入城市边缘，她在旧站台旁停下脚步。</p>
    ${markup}
    <p class="te-paragraph">风从长街另一端吹来，照片里的灯火仍像从前一样明亮。</p>
  </main>`;
}

function transitionPreview(markup: string) {
  return `<main class="te-preview-page">
    <p class="te-paragraph">雨声渐渐远去，站台上的人群也散了。</p>
    ${markup}
    <p class="te-paragraph">三天后，她在另一座城市收到了那封迟来的信。</p>
  </main>`;
}

export const EPUB_ILLUSTRATION_STYLES: EpubStyleModule[] = [
  {
    id: "illustration-centered-caption",
    kind: "illustration",
    target: "illustration",
    name: "居中图注插图",
    description: "常见的正文居中插图，限制宽度并在下方显示简洁图注。",
    selectors: [
      EPUB_STYLE_INTERFACE.illustrationFigure,
      EPUB_STYLE_INTERFACE.illustrationImage,
      EPUB_STYLE_INTERFACE.illustrationCaption,
    ],
    usage: "将 XHTML 中的图片路径、替代文字和图注替换为实际内容，适合人物、场景和资料图。",
    css: `.te-illustration {
  width: 86%;
  margin: 1.6em auto;
  padding: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  page-break-inside: avoid;
}

.te-illustration-image {
  display: block;
  width: 100%;
  height: auto;
  margin: 0 auto;
  border: 0;
}

.te-illustration-caption {
  display: block;
  margin-top: 0.65em;
  color: #64748b;
  font-size: 0.78em;
  line-height: 1.5;
  text-align: center;
  text-indent: 0;
}`,
    markup: `<figure class="te-illustration">
  <img class="te-illustration-image" src="../Images/illustration-01.jpg" alt="旧站台夜景" />
  <figcaption class="te-illustration-caption">图 1　旧站台的最后一班列车</figcaption>
</figure>`,
    previewHtml: illustrationPreview(`<figure class="te-illustration">
      <img class="te-illustration-image" src="${illustrationPreviewImage}" alt="旧站台夜景" />
      <figcaption class="te-illustration-caption">图 1　旧站台的最后一班列车</figcaption>
    </figure>`),
  },
  {
    id: "illustration-paper-card",
    kind: "illustration",
    target: "illustration",
    name: "纸张卡片插图",
    description: "带留白、细边框和轻阴影的卡片式插图，适合信件、档案和回忆画面。",
    selectors: [
      EPUB_STYLE_INTERFACE.illustrationFigure,
      EPUB_STYLE_INTERFACE.illustrationImage,
      EPUB_STYLE_INTERFACE.illustrationCaption,
    ],
    usage: "插入标准 figure 结构并替换图片；电子墨水设备会自然退化为边框卡片。",
    css: `.te-illustration {
  width: 82%;
  margin: 1.8em auto;
  padding: 0.7em;
  box-sizing: border-box;
  border: 1px solid #d7c8ad;
  background: #fffaf0;
  box-shadow: 0 0.2em 0.8em rgba(72, 52, 34, 0.14);
  text-align: center;
  text-indent: 0;
  page-break-inside: avoid;
}

.te-illustration-image {
  display: block;
  width: 100%;
  height: auto;
  border: 0;
}

.te-illustration-caption {
  margin-top: 0.7em;
  color: #6f5638;
  font-size: 0.76em;
  line-height: 1.5;
  text-align: center;
}`,
    markup: `<figure class="te-illustration">
  <img class="te-illustration-image" src="../Images/archive-photo.jpg" alt="档案照片" />
  <figcaption class="te-illustration-caption">档案编号 A-17</figcaption>
</figure>`,
    previewHtml: illustrationPreview(`<figure class="te-illustration">
      <img class="te-illustration-image" src="${illustrationPreviewImage}" alt="档案照片" />
      <figcaption class="te-illustration-caption">档案编号 A-17</figcaption>
    </figure>`),
  },
  {
    id: "illustration-full-bleed",
    kind: "illustration",
    target: "illustration",
    name: "通栏出血插图",
    description: "横向铺满版心并向两侧出血，适合章节中段的大场景和地图。",
    selectors: [EPUB_STYLE_INTERFACE.illustrationFigure, EPUB_STYLE_INTERFACE.illustrationImage],
    usage: "使用横幅图片；部分阅读器不支持 duokan-bleed 时仍会按容器宽度正常显示。",
    css: `.te-illustration {
  margin: 1.8em -1.5em;
  padding: 0;
  line-height: 0;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  duokan-bleed: leftright;
  page-break-inside: avoid;
}

.te-illustration-image {
  display: block;
  width: 100%;
  max-width: none;
  height: auto;
  margin: 0;
  border: 0;
}`,
    markup: `<figure class="te-illustration">
  <img class="te-illustration-image" src="../Images/panorama-01.jpg" alt="城市全景" />
</figure>`,
    previewHtml: illustrationPreview(`<figure class="te-illustration">
      <img class="te-illustration-image" src="${illustrationWidePreviewImage}" alt="城市全景" />
    </figure>`),
  },
  {
    id: "illustration-annotation-popup",
    kind: "illustration",
    target: "annotation-illustration",
    name: "注释点击弹图",
    description: "点击正文词语或注释链接，以无脚本弹层显示对应插图。",
    selectors: [EPUB_STYLE_INTERFACE.annotationTrigger, EPUB_STYLE_INTERFACE.annotationPopup],
    usage: "checkbox 的 id 必须与所有 label 的 for 一致；点击文字、注释、遮罩或关闭按钮即可打开或收起图片。",
    css: `.te-annotation-trigger {
  color: #17699a;
  text-decoration: none;
  border-bottom: 1px dotted currentColor;
  cursor: pointer;
}

.te-annotation-toggle {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  overflow: hidden;
  pointer-events: none;
}

.te-annotation-popup {
  display: none;
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 999;
  align-items: center;
  justify-content: center;
  padding: 1.2em;
  box-sizing: border-box;
  text-indent: 0;
}

.te-annotation-toggle:checked ~ .te-annotation-popup {
  display: flex;
}

.te-annotation-backdrop {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  background: rgba(15, 23, 42, 0.78);
}

.te-annotation-figure {
  position: relative;
  z-index: 1;
  width: 90%;
  max-width: 34em;
  max-height: 88vh;
  margin: 0;
  padding: 0.8em;
  box-sizing: border-box;
  overflow: auto;
  border-radius: 0.35em;
  background: #fffdf8;
  text-align: center;
}

.te-annotation-image {
  display: block;
  max-width: 100%;
  max-height: 70vh;
  width: auto;
  height: auto;
  margin: 0 auto;
}

.te-annotation-caption {
  margin: 0.7em 2em 0;
  color: #475569;
  font-size: 0.78em;
  line-height: 1.5;
}

.te-annotation-close {
  position: absolute;
  top: 0.35em;
  right: 0.45em;
  width: 1.8em;
  height: 1.8em;
  border-radius: 50%;
  background: #172033;
  color: #ffffff;
  font: bold 1em/1.8 sans-serif;
  text-align: center;
  cursor: pointer;
}`,
    markup: `<input id="te-annotation-toggle-1" class="te-annotation-toggle" type="checkbox" />
<p class="te-paragraph">她在<label class="te-annotation-trigger" for="te-annotation-toggle-1">旧站台<a id="te-annotation-ref-1" class="duokan-footnote" href="#te-annotation-note-1" epub:type="noteref" role="doc-noteref" aria-controls="te-annotation-note-1">〔查看插图〕</a></label>旁停下脚步。</p>
<aside id="te-annotation-note-1" class="te-annotation-popup duokan-footnote-item" epub:type="footnote" role="doc-footnote">
  <label class="te-annotation-backdrop" for="te-annotation-toggle-1" aria-label="关闭插图"></label>
  <figure class="te-annotation-figure">
    <label class="te-annotation-close" for="te-annotation-toggle-1" aria-label="关闭">×</label>
    <img class="te-annotation-image" src="../Images/note-station.jpg" alt="旧站台" />
    <figcaption class="te-annotation-caption">旧站台改造前的资料照片</figcaption>
    <a class="duokan-footnote-back" href="#te-annotation-ref-1">返回</a>
  </figure>
</aside>`,
    previewHtml: `<main class="te-preview-page">
      <input id="te-annotation-toggle-1" class="te-annotation-toggle" type="checkbox" />
      <p class="te-paragraph">她在<label class="te-annotation-trigger" for="te-annotation-toggle-1" epub:type="noteref" role="doc-noteref" aria-controls="te-annotation-note-1">旧站台</label>旁停下脚步，手中的车票已经褪色。<label class="te-annotation-trigger" for="te-annotation-toggle-1" epub:type="noteref" role="doc-noteref" aria-controls="te-annotation-note-1">〔查看插图〕</label></p>
      <p class="te-paragraph">点击蓝色文字或注释即可查看图片。</p>
      <aside id="te-annotation-note-1" class="te-annotation-popup" epub:type="footnote" role="doc-footnote">
        <label class="te-annotation-backdrop" for="te-annotation-toggle-1" aria-label="关闭插图"></label>
        <figure class="te-annotation-figure">
          <label class="te-annotation-close" for="te-annotation-toggle-1" aria-label="关闭">×</label>
          <img class="te-annotation-image" src="${illustrationPreviewImage}" alt="旧站台" />
          <figcaption class="te-annotation-caption">旧站台改造前的资料照片</figcaption>
        </figure>
      </aside>
    </main>`,
  },
];

export const EPUB_TRANSITION_STYLES: EpubStyleModule[] = [
  {
    id: "transition-fg1-stars",
    kind: "transition",
    target: "transition",
    name: "居中三星转场",
    description: "制作功能内置的经典居中 ※※※，兼容旧 class fg1 与标准 te-divider-line。",
    selectors: [EPUB_STYLE_INTERFACE.transition],
    usage: "制作 EPUB 时默认用它替换前后都有正文的单段孤立省略号。",
    replacementMode: "isolated-ellipsis",
    css: `p.fg1,
p.te-divider-line {
  margin: 1.4em 0;
  padding: 0;
  color: #4b5563;
  font-size: 0.9em;
  line-height: 1.4;
  letter-spacing: 0.6em;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
}`,
    markup: `<p class="fg1">※※※</p>`,
    previewHtml: transitionPreview(`<p class="fg1">※※※</p>`),
  },
  {
    id: "transition-css-diamond",
    kind: "transition",
    target: "transition",
    name: "CSS 菱形转场",
    description: "不使用文字和图片，仅用伪元素绘制三枚渐次菱形。",
    selectors: [EPUB_STYLE_INTERFACE.transition],
    usage: "纯 CSS 转场，可直接替换孤立省略号，黑白屏和电子墨水设备也能保持清晰。",
    replacementMode: "isolated-ellipsis",
    css: `.te-transition--diamond {
  margin: 1.7em 0;
  padding: 0;
  height: 1em;
  line-height: 1;
  text-align: center;
  text-indent: 0;
}

.te-transition--diamond::before {
  content: "◆ ◇ ◆";
  color: #8a6a3d;
  font-size: 0.72em;
  letter-spacing: 0.5em;
}`,
    markup: `<p class="te-transition te-transition--diamond" aria-label="场景转换"></p>`,
    previewHtml: transitionPreview(`<p class="te-transition te-transition--diamond" aria-label="场景转换"></p>`),
  },
  {
    id: "transition-text-label",
    kind: "transition",
    target: "transition",
    name: "文字线条转场",
    description: "在左右细线之间显示“与此同时”等短文本。",
    selectors: [EPUB_STYLE_INTERFACE.transition, EPUB_STYLE_INTERFACE.transitionText],
    usage: "替换 te-transition-text 的文字即可表达时间、地点或叙事视角转换。",
    replacementMode: "isolated-ellipsis",
    css: `.te-transition--text {
  display: flex;
  align-items: center;
  gap: 0.8em;
  margin: 1.8em 0;
  color: #64748b;
  font-size: 0.76em;
  line-height: 1.4;
  text-align: center;
  text-indent: 0;
}

.te-transition--text::before,
.te-transition--text::after {
  content: "";
  flex: 1;
  height: 1px;
  background: #cbd5e1;
}

.te-transition-text {
  white-space: nowrap;
}`,
    markup: `<p class="te-transition te-transition--text"><span class="te-transition-text">与此同时</span></p>`,
    previewHtml: transitionPreview(`<p class="te-transition te-transition--text"><span class="te-transition-text">与此同时</span></p>`),
  },
  {
    id: "transition-css-fade-line",
    kind: "transition",
    target: "transition",
    name: "CSS 渐隐短线",
    description: "无文本的轻量渐隐横线，适合现代、日常和长篇阅读。",
    selectors: [EPUB_STYLE_INTERFACE.transition],
    usage: "纯 CSS 样式，直接替换孤立省略号，不依赖额外资源。",
    replacementMode: "isolated-ellipsis",
    css: `.te-transition--fade-line {
  width: 58%;
  height: 1px;
  margin: 1.9em auto;
  padding: 0;
  background: linear-gradient(90deg, transparent, #94a3b8 22%, #94a3b8 78%, transparent);
  line-height: 0;
  text-indent: 0;
}`,
    markup: `<p class="te-transition te-transition--fade-line" aria-label="场景转换"></p>`,
    previewHtml: transitionPreview(`<p class="te-transition te-transition--fade-line" aria-label="场景转换"></p>`),
  },
  {
    id: "transition-image-divider",
    kind: "transition",
    target: "transition",
    name: "图片装饰转场",
    description: "使用独立 PNG、JPG 或 SVG 小图替代孤立省略号。",
    selectors: [EPUB_STYLE_INTERFACE.transitionImage],
    usage: "把图片导入制作模板的 dividerImage 资源位，制作时会自动以该图片替换孤立省略号。",
    replacementMode: "isolated-ellipsis",
    css: `.te-divider-image {
  margin: 1.6em 0;
  padding: 0;
  line-height: 1;
  text-align: center;
  text-indent: 0;
  duokan-text-indent: 0;
  page-break-inside: avoid;
}

.te-divider-img {
  display: inline-block;
  width: 12em;
  max-width: 72%;
  height: auto;
  border: 0;
  vertical-align: middle;
}`,
    markup: `<div class="te-divider-image">
  <img class="te-divider-img" src="../Images/divider.png" alt="场景转换" />
</div>`,
    previewHtml: transitionPreview(`<div class="te-divider-image">
      <img class="te-divider-img" src="${transitionDividerPreview}" alt="场景转换" />
    </div>`),
  },
];

export const EPUB_STYLE_MODULES: EpubStyleModule[] = [
  ...EPUB_HEADER_STYLES,
  ...EPUB_TITLE_STYLES,
  ...EPUB_ILLUSTRATION_STYLES,
  ...EPUB_TRANSITION_STYLES,
];
