// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import {
  applySafeAreaInsets,
  normalizeSafeAreaInsets,
  readAndroidSafeAreaInsets,
} from "./safeArea";

describe("safe area insets", () => {
  afterEach(() => {
    const root = document.documentElement;
    root.style.removeProperty("--safe-top-min");
    root.style.removeProperty("--safe-bottom-min");
    root.style.removeProperty("--safe-left-min");
    root.style.removeProperty("--safe-right-min");
    delete (window as Window & { TepubAndroid?: unknown }).TepubAndroid;
  });

  it("accepts object and JSON forms", () => {
    const expected = { top: 30, bottom: 16, left: 0, right: 0 };
    expect(normalizeSafeAreaInsets(expected)).toEqual(expected);
    expect(normalizeSafeAreaInsets(JSON.stringify(expected))).toEqual(expected);
  });

  it("clamps invalid values to zero instead of breaking CSS", () => {
    expect(
      normalizeSafeAreaInsets({
        top: -8,
        bottom: Number.NaN,
        left: "12.345" as unknown as number,
        right: 27.999,
      }),
    ).toEqual({ top: 0, bottom: 0, left: 12.35, right: 28 });
    expect(normalizeSafeAreaInsets("not-json")).toBeNull();
    expect(normalizeSafeAreaInsets({})).toBeNull();
  });

  it("writes all four native minimums", () => {
    const applied = applySafeAreaInsets(document.documentElement, {
      top: 30,
      bottom: 16,
      left: 4,
      right: 5,
    });
    expect(applied).toBe(true);
    expect(document.documentElement.style.getPropertyValue("--safe-top-min")).toBe(
      "30px",
    );
    expect(
      document.documentElement.style.getPropertyValue("--safe-bottom-min"),
    ).toBe("16px");
    expect(
      document.documentElement.style.getPropertyValue("--safe-left-min"),
    ).toBe("4px");
    expect(
      document.documentElement.style.getPropertyValue("--safe-right-min"),
    ).toBe("5px");
  });

  it("reads the Android bridge when available", () => {
    (window as Window & { TepubAndroid?: unknown }).TepubAndroid = {
      getSafeAreaInsets: () => '{"top":32,"bottom":18,"left":0,"right":0}',
    };

    expect(readAndroidSafeAreaInsets()).toBe(true);
    expect(document.documentElement.style.getPropertyValue("--safe-top-min")).toBe(
      "32px",
    );
  });
});
