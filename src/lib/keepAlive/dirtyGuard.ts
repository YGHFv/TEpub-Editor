/**
 * 保活视图的"别卸载我"登记表。
 *
 * 手机外壳（src/routes/+layout.svelte）会把标签登记表裁到 N 条，因为
 * KeepAliveViews 的存活集合就是标签集合 —— 不裁的话，逛一页就多挂一个
 * 活组件，一本解压后的书几十 MB，Android WebView 直接被系统回收。
 *
 * 但裁剪的副作用是**静默卸载**：用户从 EPUB 编辑器切到工具箱、再切到图片
 * 工具，第三次跳转时编辑器就成了"最久没访问"的那一个，被裁掉 —— 里面未暂存
 * 的修改随之消失，连一句提示都没有。桌面版的裁剪由用户点关闭标签触发，
 * 手机版的裁剪是自动的，所以必须给页面一个否决权。
 *
 * 约定：页面在 onMount 里登记自己的 tab id 和一个"现在能扔吗"的判定函数，
 * onDestroy 里注销。判定返回 true = 还有没保存的东西，请别卸载我。
 *
 * 刻意做成模块级 Map 而不是 Svelte store：没有界面需要订阅它，读取只发生在
 * 一次导航的裁剪计算里。
 */

type DirtyCheck = () => boolean;

const guards = new Map<string, DirtyCheck>();

/** 登记一个保护。同一 id 重复登记会覆盖（保活视图只会有一个实例） */
export function registerKeepAliveGuard(id: string, check: DirtyCheck) {
  guards.set(id, check);
}

export function unregisterKeepAliveGuard(id: string) {
  guards.delete(id);
}

/**
 * 这个标签现在能不能被裁剪掉。
 * 没登记 = 可以（大部分页面没有未保存状态）。
 * 判定函数自己抛错时按"可以"处理：宁可淘汰一个页面，也不能因为某个页面的
 * bug 让整个导航卡住。
 */
export function canEvictKeepAlive(id: string): boolean {
  const check = guards.get(id);
  if (!check) return true;
  try {
    return !check();
  } catch {
    return true;
  }
}

/** 测试用：清空登记表 */
export function resetKeepAliveGuards() {
  guards.clear();
}
