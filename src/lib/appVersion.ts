/**
 * 应用版本号，唯一真源是 package.json。
 *
 * 之前各页面自己手写版本字符串（移动端「关于」页写死 `0.6.1`），升版时没人
 * 记得改，界面就一直报着几个版本之前的号。现在由 vite.config.js 在构建期
 * 从 package.json 注入，页面只读这里。
 *
 * 注入值缺失（比如在未走 vite 的裸单测环境里 import）时回退到 "dev"，
 * 不给界面留下 undefined 或空白。
 */
declare const __TEPUB_VERSION__: string | undefined;

export const APP_VERSION =
  typeof __TEPUB_VERSION__ === "string" ? __TEPUB_VERSION__ : "dev";
