import { describe, expect, it } from "vitest";
import { describeCommandFailure } from "./webErrors";

describe("web command error messages", () => {
  it("uses the backend's JSON error text", () => {
    expect(
      describeCommandFailure("load_library", 500, '{"error":"磁盘不可读"}'),
    ).toBe("磁盘不可读");
  });

  it("keeps plain-text bodies", () => {
    expect(describeCommandFailure("load_library", 502, "Bad gateway")).toBe(
      "Bad gateway",
    );
  });

  // 纯静态部署（pnpm build:web 后直接托管）对 /api/commands/* 会回退整页
  // index.html。原样抛出时书库页的 toast 会铺满一屏 HTML 源码。
  it("summarizes an HTML fallback page instead of pasting it", () => {
    const html = "<!doctype html>\n<html lang=\"en\"><head>…";
    expect(describeCommandFailure("load_library", 404, html)).toBe(
      "浏览器版暂不支持“load_library”（HTTP 404）",
    );
    expect(describeCommandFailure("load_library", 200, "<html><body>")).toBe(
      "浏览器版暂不支持“load_library”（HTTP 200）",
    );
  });

  it("falls back to the status when the body is empty", () => {
    expect(describeCommandFailure("x", 503, "")).toBe("Request failed with 503");
  });

  // 网关/反代的 5xx 错误页同样是整页 HTML。把一次可重试的服务故障
  // 说成「浏览器版暂不支持该命令」，用户不会重试也不会报障。
  it("does not call an upstream HTML error page an unsupported command", () => {
    const page = "<html><head><title>502 Bad Gateway</title></head></html>";
    expect(describeCommandFailure("load_library", 502, page)).toBe(
      "服务暂时不可用（HTTP 502）",
    );
    expect(describeCommandFailure("load_library", 401, page)).toBe(
      "请先登录（HTTP 401）",
    );
  });

  // HTML 分支必须是终结分支：nginx/Cloudflare 的 413、429、400 也是整页
  // HTML，以前会原样铺进 toast（181 字符的 nginx 页面）。
  it("summarizes every HTML error page, not just the whitelisted statuses", () => {
    const page = "<html><head><title>413 Request Entity Too Large</title></head></html>";
    expect(describeCommandFailure("load_library", 413, page)).toBe(
      "文件太大，服务器拒绝接收（HTTP 413）",
    );
    expect(describeCommandFailure("load_library", 429, page)).toBe(
      "请求过于频繁，请稍后再试（HTTP 429）",
    );
    expect(describeCommandFailure("load_library", 400, page)).toBe(
      "请求失败（HTTP 400）",
    );
    expect(
      describeCommandFailure(
        "load_library",
        502,
        "<!-- error page -->\n<!DOCTYPE html><html></html>",
      ),
    ).toBe("服务暂时不可用（HTTP 502）");
  });

  // 网关的嵌套错误对象以前会被 String() 成 "[object Object]"，
  // `{}` 会显示成 "{}"，整条真实原因就丢了。
  it("ignores non-string JSON error values", () => {
    expect(
      describeCommandFailure(
        "load_library",
        502,
        '{"error":{"message":"upstream down"}}',
      ),
    ).toBe('{"error":{"message":"upstream down"}}');
    expect(describeCommandFailure("load_library", 500, "{}")).toBe("{}");
    expect(
      describeCommandFailure("load_library", 404, '{"message":"没找到"}'),
    ).toBe("没找到");
  });
});
