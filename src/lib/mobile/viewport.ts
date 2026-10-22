/**
 * 软键盘高度桥接。
 *
 * 手机上和输入框打架的浮层（底部抽屉、编辑区动作条）都是 `position: fixed;
 * bottom: 0`，而 `position: fixed` 跟的是**布局视口**，不是键盘升起后的可视区。
 * 结果：Android WebView 里键盘一弹，抽屉底部的按钮就被压在键盘下面，用户
 * 看不到也点不到「确定 / 关闭」。
 *
 * 这里把「可视区底部比布局视口高出多少」写进 `--ime-height`，浮层改用它：
 *
 *   bottom: var(--ime-height);
 *   max-height: calc(85dvh - var(--ime-height));
 *
 * 只在手机形态下挂载 —— 桌面窗口没有软键盘，Safari 桌面版的 visualViewport
 * 缩放也不该被当成键盘。
 */

type Cleanup = () => void;

/** 悬浮键盘至少要有这么高才当真：浏览器地址栏伸缩也就几十 px，不算键盘 */
const MIN_KEYBOARD_HEIGHT = 120;

export function installViewportBridge(): Cleanup {
  if (typeof window === "undefined" || !window.visualViewport) return () => {};

  const viewport = window.visualViewport;
  const root = document.documentElement;

  const sync = () => {
    const layoutHeight = window.innerHeight;
    // offsetTop 是可视区顶边相对布局视口的位置（键盘不会移动顶边，缩放会），
    // 两者一起算才是「被遮住的高度」，只算高度会在缩放的桌面浏览器里算错。
    const occluded = layoutHeight - viewport.height - viewport.offsetTop;
    const ime = occluded >= MIN_KEYBOARD_HEIGHT ? Math.round(occluded) : 0;
    root.style.setProperty("--ime-height", `${ime}px`);
  };

  sync();
  viewport.addEventListener("resize", sync);
  viewport.addEventListener("scroll", sync);
  window.addEventListener("orientationchange", sync);
  return () => {
    viewport.removeEventListener("resize", sync);
    viewport.removeEventListener("scroll", sync);
    window.removeEventListener("orientationchange", sync);
    root.style.removeProperty("--ime-height");
  };
}
