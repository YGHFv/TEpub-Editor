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
export const toastError = (message: string) => showToast(message, "error");
