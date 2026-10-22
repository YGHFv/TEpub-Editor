export interface SafeAreaInsets {
  top: number;
  bottom: number;
  left: number;
  right: number;
}

type AndroidSafeAreaBridge = {
  getSafeAreaInsets?: () =>
    | SafeAreaInsets
    | Partial<SafeAreaInsets>
    | string
    | null;
};

function cssPixels(value: unknown) {
  const number = Number(value);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.round(number * 100) / 100);
}

/**
 * Android 原生侧返回的是物理像素换算后的 CSS 像素。这里同时接受 JSON
 * 字符串和对象，方便原生桥与测试共用同一份清洗逻辑。
 */
export function normalizeSafeAreaInsets(
  raw: SafeAreaInsets | Partial<SafeAreaInsets> | string | null | undefined,
): SafeAreaInsets | null {
  if (raw == null) return null;
  let value: unknown = raw;
  if (typeof raw === "string") {
    try {
      value = JSON.parse(raw);
    } catch {
      return null;
    }
  }
  if (!value || typeof value !== "object") return null;
  const insets = value as Partial<SafeAreaInsets>;
  if (
    insets.top == null &&
    insets.bottom == null &&
    insets.left == null &&
    insets.right == null
  ) {
    return null;
  }
  return {
    top: cssPixels(insets.top),
    bottom: cssPixels(insets.bottom),
    left: cssPixels(insets.left),
    right: cssPixels(insets.right),
  };
}

export function applySafeAreaInsets(
  root: HTMLElement,
  raw: SafeAreaInsets | Partial<SafeAreaInsets> | string | null | undefined,
): boolean {
  const insets = normalizeSafeAreaInsets(raw);
  if (!insets) return false;
  root.style.setProperty("--safe-top-min", `${insets.top}px`);
  root.style.setProperty("--safe-bottom-min", `${insets.bottom}px`);
  root.style.setProperty("--safe-left-min", `${insets.left}px`);
  root.style.setProperty("--safe-right-min", `${insets.right}px`);
  return true;
}

/**
 * 读取 Android 壳暴露的真实 WindowInsets。
 *
 * WebView 的 env(safe-area-inset-*) 在 edge-to-edge Android 上只覆盖刘海裁切，
 * 拿不到状态栏/手势条高度。24px 的静态兜底又会在 30px 状态栏机型上漏出
 * 一条滚动文本。原生桥返回实际值后，CSS 的 max(env(), --safe-*-min) 就能
 * 在所有机型上正确覆盖。
 */
export function readAndroidSafeAreaInsets(): boolean {
  if (typeof window === "undefined" || typeof document === "undefined")
    return false;
  const bridge = (window as Window & { TepubAndroid?: AndroidSafeAreaBridge })
    .TepubAndroid;
  if (typeof bridge?.getSafeAreaInsets !== "function") return false;
  try {
    return applySafeAreaInsets(
      document.documentElement,
      bridge.getSafeAreaInsets(),
    );
  } catch {
    return false;
  }
}
