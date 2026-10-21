/**
 * 量 EPUB 正文的真实高度（预览 iframe 需要撑到这么高）。
 *
 * 绝大多数书直接读 `body.scrollHeight` 就够。但有一类书会给 `html, body`
 * 写 `height: 100%`：这时 body 的盒子高度等于 iframe 视口高度，
 * `scrollHeight` 也跟着 iframe 一起变高。拿它去设 iframe 高度就成了一条
 * 自持回路：
 *
 *     iframe 变高 → body 变高 → 量出来更高 → iframe 再变高
 *
 * 每轮多出 body 的上下外边距，几百毫秒内 iframe 能从 700px 涨到 11000px，
 * 浏览器持续抛 `ResizeObserver loop completed with undelivered notifications`。
 * 配合"全局 error 直接判崩溃"的处理，用户看到的就是编辑器整个变红。
 *
 * 所以先识别「body 被 CSS 撑满视口」这种情况，改量最靠下的子元素底边 ——
 * 那个值只取决于内容排版，与 iframe 自身高度无关，能稳定收敛。
 *
 * 安装版 PreviewPane 与 web 版 EPUB 编辑器共用这一份，避免两边再各修一次。
 */
export function measureEpubContentHeight(innerBody: HTMLElement): number {
  const doc = innerBody.ownerDocument;
  const bodyStyle = getComputedStyle(innerBody);
  const htmlStyle = getComputedStyle(doc.documentElement);
  const marginTop = parseFloat(bodyStyle.marginTop) || 0;
  const marginBottom = parseFloat(bodyStyle.marginBottom) || 0;
  const paddingBottom = parseFloat(bodyStyle.paddingBottom) || 0;

  const viewportHeight = doc.documentElement.clientHeight;
  const bodyHeight = innerBody.getBoundingClientRect().height;
  const heightFromCss =
    bodyStyle.height !== "auto" || htmlStyle.height !== "auto";

  if (heightFromCss && Math.abs(bodyHeight - viewportHeight) < 2) {
    let bottom = 0;
    for (const child of Array.from(innerBody.children)) {
      bottom = Math.max(bottom, child.getBoundingClientRect().bottom);
    }
    // 子元素量不到（空章节、只剩绝对定位）时退回 scrollHeight，
    // 免得把内容高度量成 0、书页缩成一条
    if (bottom > 0) {
      return Math.ceil(bottom + paddingBottom + marginBottom);
    }
  }

  // scrollHeight 不含 body 自己的外边距。书的 CSS 常给 body 上下留 8px，
  // 漏掉这 16px，iframe 就比内容矮一截，正文能滚、章节末尾被裁掉
  return (
    Math.max(innerBody.scrollHeight || 0, innerBody.offsetHeight || 0) +
    marginTop +
    marginBottom
  );
}
