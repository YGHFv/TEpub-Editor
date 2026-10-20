/**
 * 保活视图（keep-alive）的可见性与地址查询。
 *
 * 同窗口模式下所有打开过的标签都一直挂载着，只把非当前标签的容器标成
 * `data-ka-hidden="true"`。隐藏的容器是 `visibility:hidden`：不参与命中测试、
 * 拿不到焦点，但**仍保留真实尺寸** —— 阅读器分页、编辑器列宽都不必在切回来
 * 时重量。
 *
 * 两个配套问题要在这里解决：
 * 1. 全局监听器不会随页面隐藏而失效。隐藏页面挂在 window 上的 keydown /
 *    wheel / contextmenu 照样会响（比如隐藏的阅读器把可见编辑器的方向键
 *    拿去翻页），所以这些处理函数先问一句「我是不是当前视图」。
 * 2. 全局 `page.url` 会随任何标签跳转变化。保活页面必须读**自己的**地址，
 *    否则切走再切回来，`?file=` 之类的参数会被别的页面覆盖。
 *
 * 注意：带 `getContext` 的这几个函数**只能在组件初始化时调用一次**
 * （脚本顶层），不能放进 onMount 或事件回调里 —— Svelte 5 不允许在初始化
 * 之外读 context。
 */
import { getContext } from "svelte";
import { page } from "$app/state";
import {
  KEEP_ALIVE_CONTEXT_KEY,
  type KeepAliveViewContext,
} from "./viewContext";

/**
 * 判断某个元素是否位于隐藏视图里。
 *
 * 传组件自己的根元素最准；传事件对象也行（会从 target 往上找）。
 * 不在 keep-alive 宿主里渲染的页面（独立窗口、浏览器里直接打开某个路由）
 * 找不到标记，一律返回 false，行为和以前完全一致。
 */
export function isInHiddenView(
  node: EventTarget | Node | null | undefined,
): boolean {
  if (!node) return false;
  let el: Element | null = null;
  if (node instanceof Element) el = node;
  else if (node instanceof Node) el = node.parentElement;
  return Boolean(el?.closest?.('[data-ka-hidden="true"]'));
}

/** 事件是不是发生在隐藏视图里（供 window 级监听器快速判断） */
export function isHiddenViewEvent(event: Event): boolean {
  return isInHiddenView(event.target);
}

/**
 * 「现在该不该响应全局事件」的判定函数。
 *
 * 在组件初始化时调用一次，返回的闭包放进事件处理里：
 *
 *     const activeOnly = onlyWhenActive();
 *     function onKeydown(e) { if (!activeOnly()) return; ... }
 *
 * 非保活环境（多窗口、浏览器直开路由）永远返回 true。
 */
export function onlyWhenActive(): () => boolean {
  const view = getContext<KeepAliveViewContext | null>(KEEP_ALIVE_CONTEXT_KEY);
  if (!view) return () => true;
  return () => view.isActive();
}

/** 本组件所在的保活视图（初始化时调用一次；独立窗口下为 null） */
export function currentKeepAliveView(): KeepAliveViewContext | null {
  return (
    getContext<KeepAliveViewContext | null>(KEEP_ALIVE_CONTEXT_KEY) ?? null
  );
}

/**
 * 本页面自己的地址。**只能在初始化时调用一次**，把结果存进常量。
 *
 * 保活视图里是创建该视图时的 URL（视图 key 变了就会换组件实例，所以它
 * 对这一个实例是常量）；普通路由渲染时退回全局 `page.url`，保持原来的
 * 响应式行为（多窗口模式下每次导航都会重建页面，常量也够用）。
 */
export function viewUrl(): URL {
  const view = getContext<KeepAliveViewContext | null>(KEEP_ALIVE_CONTEXT_KEY);
  return view ? view.url : page.url;
}

/**
 * 把窗口标题登记到当前视图。
 *
 * 保活下多个页面同时挂着，`<svelte:head><title>` 会互相覆盖，所以阅读器
 * 这类"标题随内容变化"的页面改用这个函数；独立窗口里直接写 document.title。
 *
 * 同样**只能在组件初始化时调用一次**，返回的函数可以反复调用。
 */
export function titleSetter(): (title: string) => void {
  const view = getContext<KeepAliveViewContext | null>(KEEP_ALIVE_CONTEXT_KEY);
  if (!view) {
    return (title: string) => {
      if (typeof document !== "undefined" && title) {
        document.title = title;
      }
    };
  }
  return (title: string) => view.setTitle(title);
}
