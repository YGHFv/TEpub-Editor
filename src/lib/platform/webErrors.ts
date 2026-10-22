/**
 * Web 端后端命令失败的文案归一化。
 *
 * 浏览器版的后端是 server/web-server.mjs（或纯静态部署）。当它不认识某个命令
 * 时，静态托管常常直接把 index.html 当 200/404 的响应体返回，于是
 * `response.text()` 拿到的是整页 HTML 源码。页面把这段字符串拼进 toast，
 * 用户看到的是一条 `<!doctype html>…` 的源码瀑布，真正的原因（浏览器版没有
 * 这条命令）反而被埋掉。
 *
 * 抽成纯函数是为了能单测：web.ts 里的 requestJson 依赖 fetch，不好在 node 里跑。
 */

/** 静态托管「没有这条路由」时会给出的状态码。 */
const STATIC_FALLBACK_STATUSES = new Set([200, 404, 405, 501]);

/** 整页 HTML（Cloudflare/网关的错误页常以 `<!-- … -->` 开头）。 */
const HTML_BODY = /^\s*(?:<!--[\s\S]*?-->\s*)*<(!doctype|html)/i;

function readMessage(parsed: unknown, raw: string): string {
  // JSON 错误体里的 error / message 不一定是字符串：网关常给
  // `{"error":{"message":"upstream down"}}`，直接 String() 会得到
  // "[object Object]"；`{}` 会得到 "{}"。只接受非空字符串，其余一律
  // 交给调用方回退到原文/状态码。
  if (!parsed || typeof parsed !== "object") return raw;
  const source = parsed as Record<string, unknown>;
  for (const key of ["error", "message"]) {
    const value = source[key];
    if (typeof value === "string" && value.trim()) return value;
  }
  return raw;
}

export function describeCommandFailure(
  command: string,
  status: number,
  detail: string,
) {
  const text = detail || "";
  let message = text;
  try {
    message = readMessage(JSON.parse(text), text);
  } catch {
    // 不是 JSON 就保留原文，下面再判断是不是整页 HTML
  }

  // 必须是终结分支：整页 HTML 无论什么状态码都不该原样进 toast。
  // 以前只有白名单状态码才改写，400/408/413/429 会漏到下面的原文返回，
  // 而这几个恰好是反代/CDN 最常给出的整页 HTML（nginx 默认
  // client_max_body_size 1m、Cloudflare 的 413/429、http 打到 https 端口的 400）。
  //
  // 只有「这个路由在静态托管上不存在」才说是浏览器版能力缺失。
  // nginx 的 502/504、Cloudflare 的 522/524 错误页同样是整页 HTML：
  // 把它们说成「暂不支持该命令」会把一次可重试的服务故障讲成永久缺陷，
  // 用户不会重试也不会报障。
  if (HTML_BODY.test(String(message))) {
    if (STATIC_FALLBACK_STATUSES.has(status)) {
      return `浏览器版暂不支持“${command}”（HTTP ${status}）`;
    }
    if (status === 401 || status === 403) return `请先登录（HTTP ${status}）`;
    if (status === 413) return `文件太大，服务器拒绝接收（HTTP 413）`;
    if (status === 429) return `请求过于频繁，请稍后再试（HTTP 429）`;
    if (status >= 500) return `服务暂时不可用（HTTP ${status}）`;
    return `请求失败（HTTP ${status}）`;
  }
  return String(message).trim() || `Request failed with ${status}`;
}
