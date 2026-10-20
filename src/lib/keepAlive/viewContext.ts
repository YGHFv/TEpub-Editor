/**
 * 保活视图的上下文。
 *
 * 每个被保活的页面都挂在一个 `KeepAliveView` 里，后者通过 context 提供三样
 * 东西：这个视图自己的 URL（而不是全局 page.url —— 全局值会随别的标签跳
 * 转而变化）、「我当前是不是可见」的订阅、以及设置本视图标题的入口。
 *
 * 页面按需取用；独立窗口（多窗口模式）里的页面拿不到 context，
 * 行为回到原来的路由式渲染。
 */
import { getContext } from "svelte";

export interface KeepAliveViewContext {
  /** 这个视图自己的地址。标签的 id 含查询串，所以视图一旦建立就不再变 */
  url: URL;
  /** 当前是否可见（挂载时立即为 true 的那个视图） */
  isActive(): boolean;
  /**
   * 可见性变化的订阅。返回取消订阅函数。
   * 阅读器用它把整本书的 CSSStyleSheet 从 document 上摘下来/挂回去 ——
   * 多个页面同时挂载时，EPUB 里的 `p { }` 这类全局选择器会污染别的页面。
   */
  onActivationChange(listener: (active: boolean) => void): () => void;
  /**
   * 设置本视图的窗口标题。
   *
   * 保活模式下所有页面同时挂着，各自 `<svelte:head><title>` 会互相覆盖；
   * 标题统一由宿主按"当前可见视图"落盘。阅读器需要它来把书名写进标题。
   */
  setTitle(title: string): void;
}

/** 上下文 key。不做跨库共享，用 Symbol 避免和别的库撞名 */
export const KEEP_ALIVE_CONTEXT_KEY = Symbol("tepub-keep-alive-view");

export function getKeepAliveViewContext(): KeepAliveViewContext | null {
  return (
    getContext<KeepAliveViewContext | null>(KEEP_ALIVE_CONTEXT_KEY) ?? null
  );
}
