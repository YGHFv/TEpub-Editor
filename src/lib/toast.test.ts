import { describe, expect, it } from "vitest";
import { cleanErrorMessage } from "./toast";

describe("toast error text", () => {
  it("strips a leading Error: prefix", () => {
    expect(cleanErrorMessage("Error: 磁盘不可读")).toBe("磁盘不可读");
  });

  it("strips the prefix glued after a Chinese or ASCII colon", () => {
    expect(cleanErrorMessage("加载书库失败：Error: 磁盘不可读")).toBe(
      "加载书库失败：磁盘不可读",
    );
    expect(cleanErrorMessage("加载书库失败: Error: boom")).toBe(
      "加载书库失败: boom",
    );
  });

  it("leaves ordinary messages untouched", () => {
    expect(cleanErrorMessage("已保存")).toBe("已保存");
    expect(cleanErrorMessage("")).toBe("");
  });
});
