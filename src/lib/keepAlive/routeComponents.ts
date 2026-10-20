/**
 * 二级页面的「保活」宿主。
 *
 * SvelteKit 没有 keep-alive：`goto()` 虽然是 SPA 导航，但被路由到的页面组件
 * 会整个销毁再重建。于是每次切标签，TXT 编辑器丢了光标和滚动位置、EPUB
 * 编辑器丢了打开的书和标签页、阅读器要从头解析整本书 —— 用户看到的就是
 * 「切回去像重新加载，进度没了」。
 *
 * 这里的做法很直接：不再让路由决定"渲染哪个页面"，而是把**打开过的标签
 * 全部挂载着**，只把非当前标签的容器隐藏（visibility:hidden）。隐藏的容器
 * 仍然占满 app-frame-body 的真实尺寸，所以各页的列宽测量、阅读器分页、
 * 编辑器高度都不用在切回来时重算。
 *
 * 页面组件按需动态 import：没用到的页面不会进初始包，保留路由级代码分割。
 */
import type { Component } from "svelte";

type RouteModule = { default: Component<any> };

/** 应用内路径（去掉 base 前缀、去掉尾部斜杠）→ 页面组件加载器 */
const LOADERS: Record<string, () => Promise<RouteModule>> = {
  "/": () => import("../../routes/toolbox/+page.svelte"),
  "/toolbox": () => import("../../routes/toolbox/+page.svelte"),
  "/library": () => import("../../routes/library/+page.svelte"),
  "/editor": () => import("../../routes/editor/+page.svelte"),
  "/epub-editor": () => import("../../routes/epub-editor/+page.svelte"),
  "/reader": () => import("../../routes/reader/+page.svelte"),
  "/epub-metadata": () => import("../../routes/epub-metadata/+page.svelte"),
  "/search-replace": () => import("../../routes/search-replace/+page.svelte"),
  "/batch-progress": () => import("../../routes/batch-progress/+page.svelte"),
  "/mobile": () => import("../../routes/mobile/+page.svelte"),
  "/mobile/decrypt": () => import("../../routes/mobile/decrypt/+page.svelte"),
  "/mobile/edit": () => import("../../routes/mobile/edit/+page.svelte"),
  "/mobile/make": () => import("../../routes/mobile/make/+page.svelte"),
  "/mobile/metadata": () => import("../../routes/mobile/metadata/+page.svelte"),
  "/toolbox/epub-editor": () =>
    import("../../routes/toolbox/epub-editor/+page.svelte"),
  "/toolbox/text-editor": () =>
    import("../../routes/toolbox/text-editor/+page.svelte"),
  "/toolbox/epub-style-library": () =>
    import("../../routes/toolbox/epub-style-library/+page.svelte"),
  "/toolbox/make-epub": () =>
    import("../../routes/toolbox/make-epub/+page.svelte"),
  "/toolbox/image-tools": () =>
    import("../../routes/toolbox/image-tools/+page.svelte"),
  "/toolbox/online-image-download": () =>
    import("../../routes/toolbox/online-image-download/+page.svelte"),
  "/toolbox/epub-advanced": () =>
    import("../../routes/toolbox/epub-advanced/+page.svelte"),
  "/toolbox/epub-process": () =>
    import("../../routes/toolbox/epub-process/+page.svelte"),
  "/toolbox/epub-diagnose": () =>
    import("../../routes/toolbox/epub-diagnose/+page.svelte"),
  "/toolbox/font-process": () =>
    import("../../routes/toolbox/font-process/+page.svelte"),
};

const loaded = new Map<string, Component<any>>();
const pending = new Map<string, Promise<Component<any> | null>>();

export function hasRouteComponent(path: string): boolean {
  return Boolean(LOADERS[path]);
}

export function cachedRouteComponent(path: string): Component<any> | null {
  return loaded.get(path) ?? null;
}

/** 加载某个路径的页面组件；同一个路径只加载一次，失败后不再重试 */
export function loadRouteComponent(
  path: string,
): Promise<Component<any> | null> {
  const hit = loaded.get(path);
  if (hit) return Promise.resolve(hit);
  const inflight = pending.get(path);
  if (inflight) return inflight;
  const loader = LOADERS[path];
  if (!loader) return Promise.resolve(null);
  const task = loader()
    .then((mod) => {
      loaded.set(path, mod.default);
      return mod.default;
    })
    .catch((error) => {
      console.error(`页面模块加载失败：${path}`, error);
      return null;
    })
    .finally(() => {
      pending.delete(path);
    });
  pending.set(path, task);
  return task;
}
