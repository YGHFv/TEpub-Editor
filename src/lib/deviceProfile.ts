/**
 * 「这台设备是不是手机 / 平板」—— 全应用唯一的 UA 判定。
 *
 * 为什么单独一个文件：这份正则在项目里被抄过三遍，而且三份不一样
 * （clientProfile 那份含 "Mobile"、windowChrome 那份不含、epub-editor 页面
 * 又自己写了一份只看 Android|iPhone|iPad|iPod）。结果就是同一个设备在不同
 * 模块里被判成不同形态 —— iPad 桌面模式、安卓 WebView 的移动 UA 都踩过。
 *
 * 为什么不做成从 clientProfile 再导出：clientProfile → platform → tauri.ts
 * → windowChrome，这条链已经在转；windowChrome 反过来 import clientProfile
 * 就成环了，ESM 下会在模块初始化阶段拿到半成品。这里零依赖，谁都能引。
 *
 * 注意：它只回答"设备形态"，不回答"该用哪种布局"。布局判定在
 * clientProfile.resolveClientLayout（还看窗口宽度与用户显式覆盖）。
 */
const MOBILE_UA_PATTERN =
  /Android|webOS|iPhone|iPod|BlackBerry|IEMobile|Opera Mini|Mobile/i;

export function isMobileUserAgent(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  // iPadOS 13+ 默认发桌面版 UA（MacIntel），只能靠触点数量区分
  const iPadLikeMac =
    navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1;
  return MOBILE_UA_PATTERN.test(ua) || iPadLikeMac;
}
