import { describe, expect, it } from "vitest";
import {
  PHONE_MAX_WIDTH,
  resolveClientLayout,
  resolvePhoneDevice,
  type ClientSignal,
} from "./clientProfile";

const signal = (partial: Partial<ClientSignal>): ClientSignal => ({
  mobileUA: false,
  width: 1440,
  override: "",
  runtime: "web",
  ...partial,
});

describe("resolveClientLayout", () => {
  it("手机 UA 竖屏 → phone", () => {
    expect(resolveClientLayout(signal({ mobileUA: true, width: 390 }))).toBe(
      "phone",
    );
  });

  it("手机 UA 横屏（844 > 760）仍然是 phone", () => {
    // 这正是 @media (max-width: 760px) 会失效的场景
    expect(resolveClientLayout(signal({ mobileUA: true, width: 844 }))).toBe(
      "phone",
    );
  });

  it("桌面 UA 窄窗口 → phone（沿用旧 web-mobile 语义）", () => {
    expect(resolveClientLayout(signal({ width: 700 }))).toBe("phone");
  });

  it("桌面 UA 宽窗口 → desktop", () => {
    expect(resolveClientLayout(signal({ width: 1440 }))).toBe("desktop");
  });

  it("触屏笔记本（宽、非手机 UA）→ desktop", () => {
    // pointer: coarse 不再单独作为手机信号
    expect(resolveClientLayout(signal({ width: 1400 }))).toBe("desktop");
  });

  it("Android Tauri 壳（手机 UA）→ phone", () => {
    expect(
      resolveClientLayout(signal({ mobileUA: true, width: 412, runtime: "tauri" })),
    ).toBe("phone");
  });

  it("桌面 Tauri 小窗（560×360 查找替换窗）→ desktop", () => {
    // 窗口窄不等于手机：这类小窗在改动前拿不到任何移动样式
    expect(resolveClientLayout(signal({ width: 560, runtime: "tauri" }))).toBe(
      "desktop",
    );
    expect(resolveClientLayout(signal({ width: 540, runtime: "tauri" }))).toBe(
      "desktop",
    );
  });

  it("override=mobile 在宽屏上也是 phone", () => {
    expect(resolveClientLayout(signal({ width: 1440, override: "mobile" }))).toBe(
      "phone",
    );
  });

  it("override=desktop 在窄屏上也是 desktop", () => {
    expect(
      resolveClientLayout(signal({ mobileUA: true, width: 390, override: "desktop" })),
    ).toBe("desktop");
  });

  it("override 优先于手机 UA", () => {
    expect(
      resolveClientLayout(signal({ mobileUA: true, width: 390, override: "desktop" })),
    ).toBe("desktop");
    expect(
      resolveClientLayout(signal({ mobileUA: false, width: 1440, override: "mobile" })),
    ).toBe("phone");
  });

  it("断点边界：760 是 phone，761 是 desktop", () => {
    expect(PHONE_MAX_WIDTH).toBe(760);
    expect(resolveClientLayout(signal({ width: 760 }))).toBe("phone");
    expect(resolveClientLayout(signal({ width: 761 }))).toBe("desktop");
  });

  it("宽度未知（0/NaN）时不判成 phone", () => {
    expect(resolveClientLayout(signal({ width: 0 }))).toBe("desktop");
    expect(resolveClientLayout(signal({ width: Number.NaN }))).toBe("desktop");
  });
});

describe("resolvePhoneDevice", () => {
  // 外壳（收标签条、上底部导航）只认真机或显式覆盖，不跟窗口宽度走：
  // 桌面浏览器拖到 700px 时 layout 会是 phone，但那是电脑，不该没标签条。
  it("桌面 UA 的窄窗口不是手机设备", () => {
    expect(resolvePhoneDevice(signal({ width: 700 }))).toBe(false);
  });

  it("手机 UA 在横屏宽视口仍是手机设备", () => {
    expect(resolvePhoneDevice(signal({ mobileUA: true, width: 844 }))).toBe(
      true,
    );
  });

  it("?view=mobile 在桌面上也按手机设备处理", () => {
    expect(resolvePhoneDevice(signal({ width: 1440, override: "mobile" }))).toBe(
      true,
    );
  });

  it("?view=desktop 在手机上退出手机设备", () => {
    expect(
      resolvePhoneDevice(signal({ mobileUA: true, width: 390, override: "desktop" })),
    ).toBe(false);
  });
});
