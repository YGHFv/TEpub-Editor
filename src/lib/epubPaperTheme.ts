/**
 * 书页（预览/阅读）的深色适配。
 *
 * 书页大多渲染在独立文档里（iframe srcdoc、或阅读页自建节点），拿不到宿主
 * 的 CSS 变量，只能把颜色算出来再注入。这里提供：
 * - 生成书页 HTML 时可用的常量（构建期就把底色写成深色，避免加载瞬间闪白）；
 * - 运行时 `applyPaperTheme(doc, dark)`：按实际计算样式判断要不要覆盖，
 *   书自身定义了配色（比如手绘背景、纯黑底反白字）就保持原样。
 */

/**
 * 深色模式下书页纸张底色。
 * 取 `--color-canvas` 的深色值，让书页和整个应用是同一套深色，
 * 不要用纯黑 —— 纯黑会把界面切成「墨蓝外壳 + 黑洞书页」两块。
 */
export const PAPER_DARK_BG = "#191e26";
/** 深色模式下书页默认文字色（同 --color-text 深色值） */
export const PAPER_DARK_TEXT = "#e2e8f0";

const STYLE_ID = "tepub-paper-theme";

function parseRgb(value: string): [number, number, number] | null {
  const match = value.match(/rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/i);
  if (!match) return null;
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

/** 0（纯黑）～ 1（纯白）。透明返回 null。 */
function luminance(color: string): number | null {
  const trimmed = (color || "").trim().toLowerCase();
  if (!trimmed || trimmed === "transparent") return null;
  const rgba = parseRgb(trimmed);
  if (!rgba) return null;
  const [r, g, b] = rgba;
  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

/** 独立的预览文档：把书页底色直接写进 HTML，避免 srcdoc 首帧白闪 */
export function paperBackground(dark: boolean) {
  return dark ? PAPER_DARK_BG : "#fffdf9";
}

export function paperTextColor(dark: boolean) {
  return dark ? PAPER_DARK_TEXT : "#172033";
}

/**
 * 运行时把已渲染的书页刷成深色/恢复原样。
 *
 * 只在「当前配色与目标主题明显冲突」时覆盖：书自己把 body 刷成深色
 * 或写明彩色背景时不动它，否则会把作者排版破坏掉。
 */
export function applyPaperTheme(
  doc: Document | null | undefined,
  dark: boolean,
) {
  if (!doc || !doc.documentElement) return;
  const existing = doc.getElementById(STYLE_ID);
  if (!dark) {
    existing?.remove();
    doc.documentElement.style.removeProperty("color-scheme");
    return;
  }

  const body = doc.body;
  const bodyStyle = body ? doc.defaultView?.getComputedStyle(body) : null;
  const htmlStyle = doc.defaultView?.getComputedStyle(doc.documentElement);
  const bodyBg = bodyStyle ? luminance(bodyStyle.backgroundColor) : null;
  const htmlBg = htmlStyle ? luminance(htmlStyle.backgroundColor) : null;
  const textLum = bodyStyle ? luminance(bodyStyle.color) : null;
  // 书没设背景（透明）或本来就是深底，就别改；浅底才刷成夜间纸色
  const overrideBodyBg = bodyBg === null || bodyBg > 0.62;
  const overrideHtmlBg = htmlBg === null || htmlBg > 0.62;
  // 文字够亮就不动，深色文字才换成夜间前景色
  const overrideText = textLum !== null && textLum < 0.42;

  const rules = [
    `html { color-scheme: dark; }`,
    overrideHtmlBg ? `html { background: ${PAPER_DARK_BG} !important; }` : "",
    overrideBodyBg ? `body { background: ${PAPER_DARK_BG} !important; }` : "",
    overrideText ? `body { color: ${PAPER_DARK_TEXT} !important; }` : "",
  ].filter(Boolean);

  const css = rules.join("\n");
  if (existing) {
    if (existing.textContent !== css) existing.textContent = css;
    return;
  }
  const style = doc.createElement("style");
  style.id = STYLE_ID;
  style.textContent = css;
  (doc.head ?? doc.documentElement).appendChild(style);
}
