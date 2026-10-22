import { writable } from "svelte/store";

export type ToastType = "info" | "success" | "error";

export interface ToastItem {
  id: number;
  type: ToastType;
  message: string;
}

const dismissDelayMs: Record<ToastType, number> = {
  info: 4000,
  success: 4000,
  error: 7000,
};

export const toasts = writable<ToastItem[]>([]);

let nextId = 1;

/**
 * 页面里普遍写成 `toastError("加载书库失败：" + e)`，而 `String(e)` 会带上
 * `Error: ` 前缀，提示读起来像调试信息。这里统一剥掉一层 —— 既处理句首，
 * 也处理拼接在中文冒号 / 英文冒号之后的情况。
 */
export function cleanErrorMessage(message: string) {
  return String(message ?? "").replace(/(^|[：:]\s*)Error:\s*/i, "$1");
}

export function showToast(message: string, type: ToastType = "info") {
  const id = nextId++;
  toasts.update((items) => [...items, { id, type, message }]);
  setTimeout(() => dismissToast(id), dismissDelayMs[type]);
  return id;
}

export function dismissToast(id: number) {
  toasts.update((items) => items.filter((item) => item.id !== id));
}

export const toastInfo = (message: string) => showToast(message, "info");
export const toastSuccess = (message: string) => showToast(message, "success");
/**
 * 只有手机形态才改写错误文案。
 *
 * 直接读根节点属性，不走 clientProfile 的导入链（toast.ts 被大量模块引用，
 * 没必要为此把 platform 拉进来）。桌面保持 HEAD 的原文案，逐字节不变。
 */
function isPhoneLayoutNow() {
  return (
    typeof document !== "undefined" &&
    document.documentElement.dataset.tepubLayout === "phone"
  );
}

export const toastError = (message: string) =>
  showToast(isPhoneLayoutNow() ? cleanErrorMessage(message) : message, "error");
